// versions.json 本地生成器（不联网、不上传、不接 CI）
// 契约见 docs/RELEASE-CHANNEL.md
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { buildManifest, writeManifestAtomic } from "./lib/versions-manifest.mjs";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const BUILD_VERSION_FILE = join(SCRIPT_DIR, "..", ".build-version");

function usage(message) {
  if (message) {
    console.error(message);
  }
  console.error(
    [
      "用法：",
      "  node generate-versions.mjs --artifacts <目录> --base-url <https://...> --baseline <版本>",
      "    [--version <YYYY.MM.DD.HHMM>] [--date <ISO-UTC>] [--message <摘要>]",
      "    [--changes <条目>]... [--platform windows-x86_64] [--out versions.json]",
      "",
      "--baseline 必须显式给出（本次发布前用户可能已安装的版本），无内置默认值。",
    ].join("\n"),
  );
  process.exit(2);
}

function parseArgs(argv) {
  const options = { changes: [] };
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith("--")) {
      usage(`无法识别的参数：${token}`);
    }
    const key = token.slice(2);
    const value = argv[index + 1];
    if (value === undefined || value.startsWith("--")) {
      usage(`参数 --${key} 缺少取值`);
    }
    index += 1;
    if (key === "changes") {
      options.changes.push(value);
      continue;
    }
    if (Object.prototype.hasOwnProperty.call(options, key)) {
      usage(`参数 --${key} 重复`);
    }
    options[key] = value;
  }
  return options;
}

function readBuildVersion() {
  if (!existsSync(BUILD_VERSION_FILE)) {
    usage(`缺少 ${BUILD_VERSION_FILE}，请先执行构建或显式传入 --version`);
  }
  const value = readFileSync(BUILD_VERSION_FILE, "utf8").trim();
  if (!value) {
    usage(`${BUILD_VERSION_FILE} 为空，请显式传入 --version`);
  }
  return value;
}

function main() {
  const argv = process.argv.slice(2);
  const options = parseArgs(argv);

  if (!options.artifacts) usage("缺少 --artifacts <产物目录>");
  if (!options["base-url"]) usage("缺少 --base-url <https://...>");
  if (!options.baseline) usage("缺少 --baseline <版本>（禁止使用内置默认值）");
  if (!options.date) usage("缺少 --date <YYYY-MM-DDTHH:MM:SSZ>（发布时刻不取当前时间）");

  const artifactsDir = resolve(options.artifacts);
  const version = options.version ?? readBuildVersion();
  const platform = options.platform ?? "windows-x86_64";
  const outPath = resolve(options.out ?? "versions.json");

  let manifest;
  try {
    manifest = buildManifest({
      version,
      date: options.date,
      message: options.message ?? `版本 ${version} 发布`,
      changes: options.changes,
      platform,
      artifactsDir,
      baseUrl: options["base-url"],
      baseline: options.baseline,
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (message.includes("产物缺失")) {
      console.error(message);
      process.exit(2);
    }
    console.error(message);
    process.exit(3);
  }

  writeManifestAtomic(manifest, outPath);
  const entry = manifest[0];
  console.log(
    `已生成 ${outPath}：version=${entry.version} platform=${entry.platform} size=${entry.size} sha256=${entry.sha256}`,
  );
}

main();
