// 构建版本注入断言（不联网、不启动服务、只读产物）
//
// 断言目标：打包产物里的版本号必须是「封装时间」注入值，而不是
//   1) 字面量 "dev"（开发回退值）
//   2) 运行时 now() 生成的启动时间
// 判定依据：注入值存在 => 前端 version.ts 走 injected 分支（避开 localStorage/now 分支），
//           后端 option_env!("VER_BUILD_TIME") 为 Some（避开 chrono::Local::now() 分支）。
// 因此只要两侧产物里都是同一个非 "dev" 的封装时间，就说明两条 now 回退都没生效。
//
// 用法：node scripts/assert-build-version.mjs
// 前置：已执行 `node scripts/build-version.mjs && pnpm build` 与 `cargo check -p aw-desktop`

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const awRoot = path.resolve(appRoot, "../.."); // 4.AgentWork
const STAMP_FILE = path.join(appRoot, ".build-version");
const VERSION_RE = /^\d{4}\.\d{2}\.\d{2}\.\d{4}$/;

const failures = [];

// 1) 时间戳文件本身
let stamp = "";
if (!fs.existsSync(STAMP_FILE)) {
  failures.push(`缺少构建时间戳文件：${path.relative(awRoot, STAMP_FILE)}`);
} else {
  stamp = fs.readFileSync(STAMP_FILE, "utf8").trim();
  if (stamp === "dev") {
    failures.push('时间戳为 "dev"（开发回退值），未注入封装时间');
  } else if (!VERSION_RE.test(stamp)) {
    failures.push(`时间戳格式不符合 YYYY.MM.DD.HHMM（实际长度 ${stamp.length}）`);
  }
}

// 2) 前端 vite 产物
if (stamp && VERSION_RE.test(stamp)) {
  const assetsDir = path.join(appRoot, "dist", "assets");
  const bundles = fs.existsSync(assetsDir)
    ? fs.readdirSync(assetsDir).filter((f) => f.endsWith(".js"))
    : [];
  if (bundles.length === 0) {
    failures.push("缺少前端构建产物 dist/assets/*.js，请先执行 pnpm build");
  } else {
    const injected = bundles.some((f) =>
      fs.readFileSync(path.join(assetsDir, f), "utf8").includes(JSON.stringify(stamp))
    );
    if (!injected) failures.push("前端产物中找不到注入的版本号字面量（说明仍是 dev 或启动时间回退）");
  }
}

// 3) Rust 侧 rustc-env（build.rs 输出可在 cargo 的 build 目录中核对）
if (stamp) {
  const buildDir = path.join(awRoot, "target", "debug", "build");
  const outputs = fs.existsSync(buildDir)
    ? fs
        .readdirSync(buildDir, { withFileTypes: true })
        .filter((d) => d.isDirectory() && d.name.startsWith("aw-desktop-"))
        .map((d) => path.join(buildDir, d.name, "output"))
        .filter((f) => fs.existsSync(f))
        .sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs)
    : [];
  if (outputs.length === 0) {
    failures.push("缺少 aw-desktop 构建脚本输出，请先执行 cargo check -p aw-desktop");
  } else {
    const out = fs.readFileSync(outputs[0], "utf8");
    if (!out.includes(`cargo:rustc-env=VER_BUILD_TIME=${stamp}`)) {
      failures.push("Rust 构建脚本未向 rustc 注入与时间戳一致的 VER_BUILD_TIME");
    }
  }
}

if (failures.length > 0) {
  console.error("FAIL");
  failures.forEach((f) => console.error(` - ${f}`));
  process.exit(1);
}
console.log(
  `PASS 注入版本=${stamp}（前端产物与 rustc-env 一致，既非 "dev" 也非启动时间回退）`
);
