// 构建版本号生成器（打包时间的唯一来源）
//
// 用途：在打包前生成 apps/desktop/.build-version，供 vite（前端）与
// src-tauri/build.rs（Rust）读取同一个值，保证窗口标题与界面版本号一致。
//
// 优先级：VER_BUILD_TIME 环境变量 > 现有时间戳文件 > 当前时间
// - 显式传入 VER_BUILD_TIME 时不做任何改写，便于可复现构建 / CI。
// - 默认每次打包重新生成「封装时间」，因此打包产物不可复现（时间戳变化是设计意图）；
//   该时间戳文件已被 .gitignore 忽略，不进入版本库。
//
// 开发模式不使用该文件：vite dev 固定注入 "dev"，由前端改用启动时间。

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const stampFile = path.join(appRoot, ".build-version");
const VERSION_RE = /^\d{4}\.\d{2}\.\d{2}\.\d{4}$/;

function format(date) {
  const pad = (n) => String(n).padStart(2, "0");
  return `${date.getFullYear()}.${pad(date.getMonth() + 1)}.${pad(date.getDate())}.${pad(
    date.getHours()
  )}${pad(date.getMinutes())}`;
}

const fromEnv = process.env.VER_BUILD_TIME?.trim();
if (fromEnv && !VERSION_RE.test(fromEnv)) {
  console.error(`FAIL VER_BUILD_TIME 格式不符合 YYYY.MM.DD.HHMM（实际长度 ${fromEnv.length}）`);
  process.exit(1);
}

const version = fromEnv || format(new Date());
fs.writeFileSync(stampFile, `${version}\n`, "utf8");
console.log(`build-version: ${version}${fromEnv ? "（来自 VER_BUILD_TIME）" : ""}`);
