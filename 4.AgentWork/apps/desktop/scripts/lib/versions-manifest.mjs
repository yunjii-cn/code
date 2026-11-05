// versions.json 契约校验与生成（契约见 docs/RELEASE-CHANNEL.md）
import { createHash } from "node:crypto";
import { existsSync, readFileSync, renameSync, statSync, unlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export const VERSION_PATTERN = /^\d{4}\.\d{2}\.\d{2}\.\d{4}$/;
export const VERSION_STAMP_EXAMPLE = "2026.11.05.1905";
export const ALLOWED_PLATFORMS = ["windows-x86_64"];
export const REQUIRED_FIELDS = [
  "version",
  "date",
  "message",
  "platform",
  "name",
  "download_url",
  "sha256",
  "size",
];

const DATE_PATTERN = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/;
const SHA256_PATTERN = /^[0-9a-f]{64}$/;

/** 解析 15 位时间戳版本，非法返回 null */
export function parseVersionStamp(version) {
  if (typeof version !== "string" || !VERSION_PATTERN.test(version)) {
    return null;
  }
  const [year, month, day, hhmm] = version.split(".").map((part) => Number(part));
  const hour = Math.floor(hhmm / 100);
  const minute = hhmm % 100;
  if (month < 1 || month > 12) return null;
  if (day < 1 || day > 31) return null;
  if (hour > 23 || minute > 59) return null;
  return [year, month, day, hhmm];
}

/** 版本比较：a > b 返回正数，a < b 返回负数 */
export function compareVersions(a, b) {
  const left = parseVersionStamp(a);
  const right = parseVersionStamp(b);
  if (!left || !right) {
    throw new Error(`版本号非法，无法比较：${a} / ${b}`);
  }
  for (let i = 0; i < left.length; i += 1) {
    if (left[i] !== right[i]) {
      return left[i] - right[i];
    }
  }
  return 0;
}

/** 计算文件 SHA-256（小写十六进制） */
export function computeSha256(path) {
  const hash = createHash("sha256");
  hash.update(readFileSync(path));
  return hash.digest("hex");
}

export function artifactNameFor(version, platform = "windows-x86_64") {
  if (platform !== "windows-x86_64") {
    throw new Error(`v1 仅支持 windows-x86_64 产物，收到：${platform}`);
  }
  return `aw-desktop-v${version}.exe`;
}

/**
 * 校验清单。返回 { ok, errors }；不抛出，便于 CLI 统一处理。
 * options.baseline 必填：本条清单必须严格高于该版本。
 * options.artifactsDir 可选：提供时重算产物哈希与大小。
 */
export function validateManifest(manifest, options = {}) {
  const errors = [];
  const { baseline, artifactsDir } = options;

  if (!Array.isArray(manifest)) {
    return { ok: false, errors: ["顶层必须是数组（消费端按数组解析，[0] 为最新版）"] };
  }
  if (manifest.length === 0) {
    return { ok: false, errors: ["清单为空：至少需要 1 个条目，拒绝写出空清单"] };
  }

  if (typeof baseline !== "string" || !parseVersionStamp(baseline)) {
    errors.push(`缺少合法的基线版本 baseline（格式 ${VERSION_STAMP_EXAMPLE}）`);
  }

  manifest.forEach((entry, index) => {
    const prefix = `[${index}]`;
    if (typeof entry !== "object" || entry === null || Array.isArray(entry)) {
      errors.push(`${prefix} 条目必须是对象`);
      return;
    }

    for (const field of REQUIRED_FIELDS) {
      if (!(field in entry)) {
        errors.push(`${prefix} 缺少必填字段 ${field}`);
      }
    }

    if ("version" in entry) {
      if (!parseVersionStamp(entry.version)) {
        errors.push(`${prefix} version 格式非法：需为 ${VERSION_STAMP_EXAMPLE}（YYYY.MM.DD.HHMM）`);
      } else if (typeof baseline === "string" && parseVersionStamp(baseline)) {
        if (compareVersions(entry.version, baseline) <= 0) {
          errors.push(
            `${prefix} 版本 ${entry.version} 未高于基线 ${baseline}：拒绝版本回退或原地重发`,
          );
        }
      }
    }

    if ("date" in entry) {
      const validShape = typeof entry.date === "string" && DATE_PATTERN.test(entry.date);
      const validDate = validShape && !Number.isNaN(Date.parse(entry.date));
      if (!validDate) {
        errors.push(`${prefix} date 必须是 ISO-8601 UTC（YYYY-MM-DDTHH:MM:SSZ）`);
      }
    }

    if ("message" in entry && (typeof entry.message !== "string" || !entry.message.trim())) {
      errors.push(`${prefix} message 必须是非空字符串`);
    }

    if ("changes" in entry) {
      const validChanges =
        Array.isArray(entry.changes) && entry.changes.every((item) => typeof item === "string");
      if (!validChanges) {
        errors.push(`${prefix} changes 必须是字符串数组`);
      }
    }

    if ("platform" in entry && !ALLOWED_PLATFORMS.includes(entry.platform)) {
      errors.push(
        `${prefix} platform 不支持：${entry.platform}（v1 允许 ${ALLOWED_PLATFORMS.join(", ")}）`,
      );
    }

    if ("version" in entry && "name" in entry && parseVersionStamp(entry.version)) {
      const expected = `aw-desktop-v${entry.version}.exe`;
      if (entry.name !== expected) {
        errors.push(`${prefix} name 必须等于 ${expected}`);
      }
    }

    if ("download_url" in entry) {
      validateDownloadUrl(entry.download_url, entry.name, prefix, errors);
    }

    if ("sha256" in entry && !(typeof entry.sha256 === "string" && SHA256_PATTERN.test(entry.sha256))) {
      errors.push(`${prefix} sha256 必须是小写 64 位十六进制`);
    }

    if ("size" in entry && !(Number.isSafeInteger(entry.size) && entry.size > 0)) {
      errors.push(`${prefix} size 必须是正整数`);
    }

    if (artifactsDir && typeof entry.name === "string" && entry.name) {
      verifyArtifact(entry, artifactsDir, prefix, errors);
    }
  });

  for (let index = 1; index < manifest.length; index += 1) {
    const previous = manifest[index - 1];
    const current = manifest[index];
    if (!previous || !current) continue;
    if (!parseVersionStamp(previous.version) || !parseVersionStamp(current.version)) continue;
    const order = compareVersions(previous.version, current.version);
    if (order === 0) {
      errors.push(`[${index}] 版本 ${current.version} 重复`);
    } else if (order < 0) {
      errors.push(`[${index}] 版本必须严格降序：${current.version} 大于前一条 ${previous.version}`);
    }
  }

  return { ok: errors.length === 0, errors };
}

function validateDownloadUrl(url, name, prefix, errors) {
  if (typeof url !== "string") {
    errors.push(`${prefix} download_url 必须是字符串`);
    return;
  }
  let parsed;
  try {
    parsed = new URL(url);
  } catch {
    errors.push(`${prefix} download_url 不是合法 URL`);
    return;
  }
  if (parsed.protocol !== "https:") {
    errors.push(`${prefix} download_url 必须是 https:// 绝对地址`);
  }
  if (parsed.search || parsed.hash) {
    errors.push(`${prefix} download_url 不得带 query 或 fragment`);
  }
  const lastSegment = parsed.pathname.split("/").filter(Boolean).pop();
  if (typeof name === "string" && name && lastSegment !== name) {
    errors.push(`${prefix} download_url 最后一段必须等于 name（${name}）`);
  }
}

function verifyArtifact(entry, artifactsDir, prefix, errors) {
  const path = join(artifactsDir, entry.name);
  if (!existsSync(path)) {
    errors.push(`${prefix} 产物缺失：${entry.name}`);
    return;
  }
  const actualSha = computeSha256(path);
  if (typeof entry.sha256 === "string" && SHA256_PATTERN.test(entry.sha256)) {
    if (actualSha !== entry.sha256) {
      errors.push(`${prefix} sha256 与产物不符（期望 ${entry.sha256}，实际 ${actualSha}）`);
    }
  }
  const actualSize = statSync(path).size;
  if (Number.isSafeInteger(entry.size) && entry.size !== actualSize) {
    errors.push(`${prefix} size 与产物不符（期望 ${entry.size}，实际 ${actualSize}）`);
  }
}

/**
 * 从产物构建单条清单并自校验；校验失败抛出异常（CLI 据此非 0 退出）。
 */
export function buildManifest(options) {
  const {
    version,
    date,
    message,
    changes = [],
    platform = "windows-x86_64",
    artifactsDir,
    baseUrl,
    baseline,
  } = options;

  const name = artifactNameFor(version, platform);
  const artifactPath = join(artifactsDir, name);
  if (!existsSync(artifactPath)) {
    throw new Error(`产物缺失：${artifactPath}`);
  }

  const entry = {
    version,
    date,
    message,
    changes,
    platform,
    name,
    download_url: `${String(baseUrl).replace(/\/+$/, "")}/${name}`,
    sha256: computeSha256(artifactPath),
    size: statSync(artifactPath).size,
  };

  const manifest = [entry];
  const result = validateManifest(manifest, { baseline, artifactsDir });
  if (!result.ok) {
    throw new Error(`清单校验失败：\n - ${result.errors.join("\n - ")}`);
  }
  return manifest;
}

/** 原子写出清单：先写临时文件，成功后改名；失败清理临时文件 */
export function writeManifestAtomic(manifest, outPath) {
  const tempPath = `${outPath}.tmp-${process.pid}`;
  try {
    writeFileSync(tempPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
    renameSync(tempPath, outPath);
  } catch (error) {
    if (existsSync(tempPath)) {
      try {
        unlinkSync(tempPath);
      } catch {
        // 清理失败不遮蔽原始错误
      }
    }
    throw error;
  }
}
