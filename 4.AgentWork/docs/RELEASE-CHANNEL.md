# 发布流元数据契约（versions.json）

> 状态：契约 v1，仅覆盖 **Windows 免安装 EXE** 单一产物。
> 消费端：`apps/desktop/src-tauri/src/commands/updater.rs`（按数组解析，`[0]` 为最新版）。
> 生成端：`apps/desktop/scripts/generate-versions.mjs` + `apps/desktop/scripts/lib/versions-manifest.mjs`。
> 本文档是唯一权威契约；代码注释不承担契约职责。

## 1. 文件形态

- 文件名：`versions.json`，UTF-8，无 BOM。
- 顶层是 **JSON 数组**（不是对象），按 `version` 严格降序排列，`[0]` 为最新版本。
- 数组不得为空。空数组视为非法清单，生成器必须拒绝写出。

```json
[
  {
    "version": "2026.11.05.1905",
    "date": "2026-11-05T11:05:00Z",
    "message": "首个可在线更新的版本",
    "changes": ["修复登录凭据读取", "updater 改为 fail-closed"],
    "platform": "windows-x86_64",
    "name": "aw-desktop-v2026.11.05.1905.exe",
    "download_url": "https://example.com/releases/2026.11.05.1905/aw-desktop-v2026.11.05.1905.exe",
    "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
    "size": 18446732
  }
]
```

## 2. 字段表

| 字段 | 必填 | 类型 | 约束 |
|---|:--:|---|---|
| `version` | 是 | string | 15 位时间戳版本，格式 `YYYY.MM.DD.HHMM`；月 01–12、日 01–31、时 00–23、分 00–59；必须 **严格大于** 基线版本（见 §4） |
| `date` | 是 | string | ISO-8601 UTC，格式 `YYYY-MM-DDTHH:MM:SSZ` |
| `message` | 是 | string | 非空摘要，面向用户的一句话 |
| `changes` | 否 | string[] | 变更条目；缺省视为空数组 |
| `platform` | 是 | string | v1 只允许 `windows-x86_64`；其余平台键为保留字，校验时显式拒绝（暂无打包产物格式） |
| `name` | 是 | string | 产物文件名，必须严格等于 `aw-desktop-v{version}.exe` |
| `download_url` | 是 | string | 绝对 `https://` 地址；不得带 query/fragment；路径最后一段必须等于 `name` |
| `sha256` | 是 | string | 产物内容的 SHA-256，**小写** 64 位十六进制 |
| `size` | 是 | integer | 产物字节数，正整数，`<= Number.MAX_SAFE_INTEGER` |

消费端当前只读取 `version / date / message / changes / name / download_url / size`；
`platform` 与 `sha256` 是 v1 新增的发布元数据，消费端尚未校验，属已知缺口（见 §7）。

## 3. 版本号格式与构建时间来源

- 版本号唯一来源：`apps/desktop/.build-version`（由 `apps/desktop/scripts/build-version.mjs` 在构建时写入，内容为 15 位时间戳）。
- 产物文件名中的版本号必须与 `version` 字段一致。
- `date` 是**发布时间**，由发布者显式传入（`--date`），不在生成时取当前时间，避免生成器不可复现。
- 生成器默认从 `.build-version` 读取 `version`；可用 `--version` 覆盖（仅用于回放/测试）。

## 4. 基线（当前版本）与版本回退

- 基线 = 本次发布前用户可能已安装的版本，由调用方通过 `--baseline` 明确给出，不得有内置默认值。
- 校验规则：每个条目的 `version` 必须 **严格大于** 基线；低于或等于基线一律拒绝。
- 这保证「版本回退」和「原地重发同版本」不会进入清单。

## 5. 校验规则（生成器强制，违反即拒绝写出）

1. 顶层是数组且至少 1 个条目。
2. 每个条目的必填字段齐全，类型正确，未知字段不参与校验但原样保留。
3. `version` 符合 §2 格式且取值区间合法。
4. `date` 符合 ISO-8601 UTC 格式。
5. `platform` 在 v1 允许集合内。
6. `name` 与 `version` 自洽（`aw-desktop-v{version}.exe`）。
7. `download_url` 为 https 绝对地址，末段等于 `name`。
8. `sha256` 为小写 64 位十六进制；提供产物目录时，必须与**重新计算**的 SHA-256 完全一致（不符即拒绝）。
9. `size` 为正整数；提供产物目录时，必须与产物的实际字节数一致。
10. 条目按 `version` 严格降序，不允许重复版本。
11. 每个条目的 `version` 严格大于基线。
12. 产物缺失、无法读取或哈希计算失败 → 生成器以非 0 退出，且**不写出**任何文件。

## 6. 生成流程与退出码

```text
node apps/desktop/scripts/generate-versions.mjs \
  --artifacts <产物目录> \
  --base-url https://example.com/releases/<version> \
  --baseline 2026.11.05.1800 \
  [--version <覆盖版本>] \
  [--date 2026-11-05T11:05:00Z] \
  [--message "摘要"] \
  [--changes "改动1" --changes "改动2"] \
  [--out versions.json]
```

- 产物定位：`<产物目录>/aw-desktop-v{version}.exe`，必须存在且唯一。
- `download_url` 由 `--base-url` 与产物文件名拼接（去除重复斜杠）。
- 写出方式：先写临时文件，全部校验通过后再原子改名；校验失败时删除临时文件。
- 退出码：`0` 成功；`2` 参数/产物错误；`3` 契约校验失败。

## 7. 已知缺口（本轮不修，不允许被当作"已打通"）

1. **无真实产物**：仓库当前没有 release bundle（`target/release/bundle` 不存在），生成器只能用构造出的临时文件验证流程，不能证明真实安装包链路可用。
2. **无上传/CI**：契约与生成器均不联网、不上传、不接 workflow；托管地址由发布者自行提供。
3. **消费端不校验哈希**：`updater.rs` 只使用 `download_url`/`size`，未校验 `sha256`；契约中的哈希目前仅由生成器保证。
4. **版本口径不一致**：消费端 `has_update` 用 `app.package_info().version`（`tauri.conf.json` 中的 semver `0.1.0`）与清单中的 15 位时间戳比较，时间戳必然"更大"。该问题需单独一轮把桌面端版本口径统一为 `.build-version`，本轮不改。
