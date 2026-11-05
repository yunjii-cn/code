// versions.json 契约测试（node:test，无第三方依赖）
import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import {
  buildManifest,
  computeSha256,
  validateManifest,
} from "./lib/versions-manifest.mjs";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const CLI = join(SCRIPT_DIR, "generate-versions.mjs");

const NEW_VERSION = "2026.11.05.1905";
const BASELINE = "2026.11.05.1800";
const ARTIFACT_NAME = `aw-desktop-v${NEW_VERSION}.exe`;

function makeWorkspace({ artifactBytes = "agentwork-release-payload" } = {}) {
  const root = mkdtempSync(join(tmpdir(), "aw-versions-"));
  const artifactsDir = join(root, "artifacts");
  mkdirSync(artifactsDir, { recursive: true });
  if (artifactBytes !== null) {
    writeFileSync(join(artifactsDir, ARTIFACT_NAME), artifactBytes);
  }
  return { root, artifactsDir, cleanup: () => rmSync(root, { recursive: true, force: true }) };
}

function sha256OfFile(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function validEntry(artifactsDir, overrides = {}) {
  const artifact = join(artifactsDir, ARTIFACT_NAME);
  return {
    version: NEW_VERSION,
    date: "2026-11-05T11:05:00Z",
    message: "首个可在线更新的版本",
    changes: ["修复登录凭据读取"],
    platform: "windows-x86_64",
    name: ARTIFACT_NAME,
    download_url: `https://example.com/releases/${NEW_VERSION}/${ARTIFACT_NAME}`,
    sha256: sha256OfFile(artifact),
    size: readFileSync(artifact).length,
    ...overrides,
  };
}

test("接受字段齐全且哈希匹配的清单", () => {
  const ws = makeWorkspace();
  try {
    const entry = validEntry(ws.artifactsDir);
    const result = validateManifest([entry], {
      baseline: BASELINE,
      artifactsDir: ws.artifactsDir,
    });
    assert.equal(result.ok, true, `应通过，实际错误：${result.errors.join("; ")}`);
    assert.deepEqual(result.errors, []);
  } finally {
    ws.cleanup();
  }
});

test("拒绝空清单", () => {
  const result = validateManifest([], { baseline: BASELINE });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("; "), /空|为空|至少/);
});

test("拒绝缺少必填字段（sha256）", () => {
  const ws = makeWorkspace();
  try {
    const entry = validEntry(ws.artifactsDir);
    delete entry.sha256;
    const result = validateManifest([entry], { baseline: BASELINE });
    assert.equal(result.ok, false);
    assert.match(result.errors.join("; "), /sha256/);
  } finally {
    ws.cleanup();
  }
});

test("拒绝哈希与产物不符", () => {
  const ws = makeWorkspace();
  try {
    const entry = validEntry(ws.artifactsDir, { sha256: "0".repeat(64) });
    const result = validateManifest([entry], {
      baseline: BASELINE,
      artifactsDir: ws.artifactsDir,
    });
    assert.equal(result.ok, false);
    assert.match(result.errors.join("; "), /sha256|SHA-256/);
  } finally {
    ws.cleanup();
  }
});

test("拒绝哈希格式非法（大写或长度不足）", () => {
  const ws = makeWorkspace();
  try {
    const entry = validEntry(ws.artifactsDir, { sha256: "ABC".repeat(21) + "D" });
    const result = validateManifest([entry], { baseline: BASELINE });
    assert.equal(result.ok, false);
    assert.match(result.errors.join("; "), /sha256/);
  } finally {
    ws.cleanup();
  }
});

test("拒绝版本低于基线（版本回退）", () => {
  const ws = makeWorkspace();
  try {
    const entry = validEntry(ws.artifactsDir, {
      version: "2026.11.05.1700",
      name: "aw-desktop-v2026.11.05.1700.exe",
      download_url: `https://example.com/releases/2026.11.05.1700/aw-desktop-v2026.11.05.1700.exe`,
    });
    const result = validateManifest([entry], { baseline: BASELINE });
    assert.equal(result.ok, false);
    assert.match(result.errors.join("; "), /基线|回退|版本/);
  } finally {
    ws.cleanup();
  }
});

test("拒绝与基线同版本（原地重发）", () => {
  const ws = makeWorkspace();
  try {
    const result = validateManifest([validEntry(ws.artifactsDir)], { baseline: NEW_VERSION });
    assert.equal(result.ok, false);
    assert.match(result.errors.join("; "), /基线|回退/);
  } finally {
    ws.cleanup();
  }
});

test("拒绝非 https 或文件名不一致的 download_url", () => {
  const ws = makeWorkspace();
  try {
    const base = validEntry(ws.artifactsDir);
    for (const url of [
      `http://example.com/releases/${NEW_VERSION}/${ARTIFACT_NAME}`,
      `https://example.com/releases/${NEW_VERSION}/other.exe`,
      `https://example.com/releases/${NEW_VERSION}/${ARTIFACT_NAME}?token=abc`,
    ]) {
      const result = validateManifest([{ ...base, download_url: url }], { baseline: BASELINE });
      assert.equal(result.ok, false, `应拒绝 ${url}`);
      assert.match(result.errors.join("; "), /download_url/);
    }
  } finally {
    ws.cleanup();
  }
});

test("buildManifest 从产物计算 SHA256 与 size", async () => {
  const ws = makeWorkspace();
  try {
    const manifest = await buildManifest({
      version: NEW_VERSION,
      date: "2026-11-05T11:05:00Z",
      message: "测试摘要",
      changes: [],
      platform: "windows-x86_64",
      artifactsDir: ws.artifactsDir,
      baseUrl: `https://example.com/releases/${NEW_VERSION}`,
      baseline: BASELINE,
    });
    assert.equal(manifest.length, 1);
    assert.equal(manifest[0].sha256, sha256OfFile(join(ws.artifactsDir, ARTIFACT_NAME)));
    assert.equal(manifest[0].size, readFileSync(join(ws.artifactsDir, ARTIFACT_NAME)).length);
    assert.equal(manifest[0].name, ARTIFACT_NAME);
  } finally {
    ws.cleanup();
  }
});

test("产物缺失时 CLI 非 0 退出且不写出文件", () => {
  const ws = makeWorkspace({ artifactBytes: null });
  try {
    const out = join(ws.root, "versions.json");
    const run = spawnSync(
      process.execPath,
      [
        CLI,
        "--artifacts", ws.artifactsDir,
        "--base-url", `https://example.com/releases/${NEW_VERSION}`,
        "--baseline", BASELINE,
        "--version", NEW_VERSION,
        "--date", "2026-11-05T11:05:00Z",
        "--message", "测试摘要",
        "--out", out,
      ],
      { encoding: "utf8", cwd: ws.root },
    );
    assert.notEqual(run.status, 0, "缺产物必须非 0 退出");
    assert.equal(existsSync(out), false, "失败时不得写出清单");
  } finally {
    ws.cleanup();
  }
});

test("CLI 生成清单且哈希与产物一致", () => {
  const ws = makeWorkspace();
  try {
    const out = join(ws.root, "versions.json");
    const run = spawnSync(
      process.execPath,
      [
        CLI,
        "--artifacts", ws.artifactsDir,
        "--base-url", `https://example.com/releases/${NEW_VERSION}`,
        "--baseline", BASELINE,
        "--version", NEW_VERSION,
        "--date", "2026-11-05T11:05:00Z",
        "--message", "测试摘要",
        "--out", out,
      ],
      { encoding: "utf8", cwd: ws.root },
    );
    assert.equal(run.status, 0, `CLI 应成功，stderr：${run.stderr}`);
    const manifest = JSON.parse(readFileSync(out, "utf8"));
    assert.equal(manifest.length, 1);
    assert.equal(manifest[0].sha256, sha256OfFile(join(ws.artifactsDir, ARTIFACT_NAME)));
  } finally {
    ws.cleanup();
  }
});

test("CLI 拒绝低于基线的版本且不写出文件", () => {
  const ws = makeWorkspace();
  try {
    const out = join(ws.root, "versions.json");
    const run = spawnSync(
      process.execPath,
      [
        CLI,
        "--artifacts", ws.artifactsDir,
        "--base-url", `https://example.com/releases/${NEW_VERSION}`,
        "--baseline", "2026.11.06.1000",
        "--version", NEW_VERSION,
        "--date", "2026-11-05T11:05:00Z",
        "--message", "测试摘要",
        "--out", out,
      ],
      { encoding: "utf8", cwd: ws.root },
    );
    assert.notEqual(run.status, 0, "版本回退必须非 0 退出");
    assert.equal(existsSync(out), false, "校验失败不得写出清单");
  } finally {
    ws.cleanup();
  }
});

test("computeSha256 返回小写 64 位十六进制", async () => {
  const ws = makeWorkspace();
  try {
    const digest = await computeSha256(join(ws.artifactsDir, ARTIFACT_NAME));
    assert.match(digest, /^[0-9a-f]{64}$/);
  } finally {
    ws.cleanup();
  }
});
