// updater fail-closed 断言（不联网、不启动服务）
//
// 断言目标：versions.json 发布流不可达/不存在时，更新入口必须显式禁用并给出原因，
// 而不是静默失败或弹出空白页；可达时不得误伤正常入口。
//
// 用法：node scripts/assert-updater-failclosed.mjs

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { resolveUpdateAvailability } from "../src/lib/update-availability.ts";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const viewSource = fs.readFileSync(
  path.join(appRoot, "src", "views", "UpdateManager.tsx"),
  "utf8"
);

const failures = [];

// 1) 不可达 -> 必须禁用，且带「发布流未配置」原因
const unreachable = resolveUpdateAvailability({
  ok: false,
  error: "发布流未配置：versions.json 请求失败（404）",
});
if (!unreachable.disabled) {
  failures.push("versions.json 不可达时未回退为不可用（fail-open）");
} else {
  if (!String(unreachable.reasonKey ?? "").includes("publishStreamMissing")) {
    failures.push("不可用时未给出「发布流未配置」原因键");
  }
  if (!unreachable.detail.includes("404")) {
    failures.push("不可用原因未保留原始错误详情");
  }
}

// 2) 可达 -> 不得禁用
const reachable = resolveUpdateAvailability({ ok: true });
if (reachable.disabled) {
  failures.push("发布流可达时不应禁用更新入口");
}

// 3) 组件必须真正接上可用性（禁用 + 展示原因）
if (!viewSource.includes("availability.disabled")) {
  failures.push("UpdateManager 未把可用性接入按钮 disabled");
}
if (!viewSource.includes("t(availability.reasonKey)")) {
  failures.push("UpdateManager 未展示不可用原因文案");
}

if (failures.length > 0) {
  console.error("FAIL");
  failures.forEach((f) => console.error(` - ${f}`));
  process.exit(1);
}
console.log("PASS 不可达时入口禁用并给出「发布流未配置」原因，可达时正常");
