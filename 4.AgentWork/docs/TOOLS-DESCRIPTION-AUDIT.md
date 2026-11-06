# `tools/list` 描述与实际行为全量对照（v1.14；v1.17 复审：§12 超时/回执相关 22 条，§13 其余 65 条；v1.18 变更见 §14）

- 对照对象：`apps/mcp-bridge/src/server.rs` 中 `tools()` 返回的**全部工具**的 `description` 与 `inputSchema`（含每个字段的 `description`、`type`、范围、`required`、`additionalProperties`）。
- 权威契约：[`PM-AW-DISPATCH-CONTRACT.md`](./PM-AW-DISPATCH-CONTRACT.md)（本批 v1.14；v1.17 复审见 §12 与 §13；v1.18 见 §14）。
- 本批性质：**只修描述与契约措辞，不改任何行为、不改回执字段**。
- 判定标签（仅这三种，逐句打标）：
  - **与实现一致**：描述句子能在实现里找到对应的无条件行为。
  - **不一致（已修）**：v1.14 之前描述与实现相反/缺失，**本批已改**。
  - **未实现却写了**：描述承诺了实现里根本不存在的约束或能力。

> 口径提示：`tools/list` 当前是 **6 个工具**（`health` / `status` / `task.execute` / `receipt.get` / `runs.list` / `runs.prune`）。"四/五个工具"是 v1.5 之前（`runs.list` 尚未加入）的口径；本对照按当前实现全量覆盖。

---

## 0. 结论先行

| 统计 | 数量 |
|---|---|
| 逐句拆出的可判定断言（不含第 7 节的结构性约束） | **87** |
| 与实现一致 | **85** |
| 不一致（已修） | **2** |
| 未实现却写了 | **0** |

**2 条"不一致（已修）"**：

| # | 位置 | v1.14 之前的描述 | 实际实现（v1.7 起） | 本批修正 |
|---|---|---|---|---|
| 1 | `task.execute` → `env.description` | "…PATH 会整体覆盖继承值…" | `parse_env` 对 `key.eq_ignore_ascii_case("PATH")` **看见即拒绝**（`invalid_env`），继承 PATH 永远保留 | 改为 fail-closed 口径：任意大小写一律拒绝、继承 PATH 永远保留、不整体覆盖、不静默忽略、追加目录走 `path_prepend` |
| 2 | `task.execute` → `inputSchema.properties.path_prepend` | **整个字段缺失**（schema 里没有声明） | v1.9 已实现：数组、字面顺序前插、平台分隔符、参与幂等签名、16 项/4096 字符、不校验存在性 | 补齐 schema 与完整 description |

另有一条**契约内部**不一致（不属于 `tools/list`，同批修正，见第 8 节）：

| 位置 | 问题 | 修正 |
|---|---|---|
| 契约 §3.1 第 2、3 条 | 签名口径只写 `task_id + cmd + cwd + timeout_secs + env`，漏了 v1.9 起已参与签名的 `path_prepend`，与 §3.5 各说一套 | 两处都补上 `path_prepend`，与 §3.5 单一措辞 |

**"未实现却写了"= 0 条**：本批没有发现描述承诺了不存在的能力。唯一需要说明的结构性差异见 §7.2（`additionalProperties: false` 属声明性约束，服务端不做拒绝），它不是"承诺了不存在的行为"，但明确记录在此，避免读者误读。

---

## 1. `health`

| # | 断言（原文） | 实现依据 | 判定 |
|---|---|---|---|
| 1 | "返回 MCP 桥存活状态、版本与构建时间（只读）" | `health_payload`（server.rs）：返回 `status:"ok"`、`server.{name,version,buildTime}`；函数体内无任何写盘 | 与实现一致 |
| 2 | schema：`{"type":"object","properties":{},"additionalProperties":false}` | `call_tool` 的 `health` 分支只调用 `health_payload()`，不读任何参数 | 与实现一致 |

---

## 2. `status`

| # | 断言（原文） | 实现依据 | 判定 |
|---|---|---|---|
| 1 | "只读汇总桌面端关键状态" | `status_payload`：只读 `settings.json` 与 `_session/`，无写操作 | 与实现一致 |
| 2 | "配置文件是否就绪" | `summarize_settings` → `settingsPresent`（文件可读、可解析为 JSON 对象即为 true） | 与实现一致 |
| 3 | "AI 是否启用" | `aiEnabled` = `settings["ai_enabled"]` 的布尔值，缺失为 `null` | 与实现一致 |
| 4 | "启用模型数量" | `enabledModelsCount` = `settings["enabled_models"]` 数组长度，缺失为 `null` | 与实现一致 |
| 5 | "仓库路径是否配置" | `repoPathConfigured` = `settings["repo_path"]` 非空字符串 | 与实现一致 |
| 6 | "initialize 审计（记录可能多份，按 receivedAt 聚合，顶层回显最近一条 clientInfo）" | `session_audit`：扫描 `initialize.json` 与 `initialize-*.json`，按 `receivedAt` 升序，返回 `{clientInfo, receivedAt, total, records[]}` | **不一致（已修）**：v1.13 改为逐份落盘后，描述仍只提"最近一次"，未声明多份与聚合；v1.14 补齐 |
| 7 | "仅审计弱识别，不构成身份" | `record_client_info` 的边界注释与 `session_audit` 的 `source` 注释；全链路无任何准入/鉴权读取该记录 | 与实现一致 |
| 8 | "不回显任何配置值" | `summarize_settings` 只输出布尔/计数/`null`，不含任何字符串配置值 | 与实现一致 |

---

## 3. `task.execute`

### 3.1 工具级 `description`

| # | 断言（原文） | 实现依据 | 判定 |
|---|---|---|---|
| 1 | "接单执行：由派单方给出命令" | `execute_task`：`cmd` 由参数给出，经 `shell_command` 执行 | 与实现一致 |
| 2 | "执行并把 stdout/stderr … 写成可审计回执" | 子进程 stdout/stderr 分别落到 `attempt-N/stdout.txt` / `stderr.txt`，`write_handback` 落 `handback.json` | 与实现一致 |
| 3 | "退出码" | `handback.exit_code` = `ExitStatus::code()`（`blocked` 时为 `null`） | 与实现一致 |
| 4 | "耗时" | `handback.duration_ms` = `started.elapsed()` | 与实现一致 |
| 5 | "产物哈希" | `handback.artifacts[{kind,locator,hash}]`，hash 为 `sha256:<hex>` | 与实现一致 |
| 6 | "可审计回执" | 回执落盘 + `receipt.get` 可读回；目录布局 `runs/<task_id>/attempt-N/` | 与实现一致 |
| 7 | "含实际 cwd" | `handback.cwd` = `canonical_cwd` | 与实现一致 |
| 8 | "生效 timeout_secs" | `handback.timeout_secs` = 实际生效值（含默认 120） | 与实现一致 |
| 9 | "env_keys" | `handback.env_keys` = `env_pairs` 的键（不含值） | 与实现一致 |
| 10 | "changed_files 是否截断" | `handback.changed_files_truncated` = 差集超过 20 条 | 与实现一致 |
| 11 | "AW 不定义判据、不做验收" | bridge 只产出事实回执，`status` 仅进程结果；无验收逻辑、无写权威 | 与实现一致 |

### 3.2 schema — `task_id`

| # | 断言（原文） | 实现依据 | 判定 |
|---|---|---|---|
| 12 | "派单标识（字母或数字开头，仅含字母数字 . _ -，≤64）" | `is_valid_task_id`：首字符 ASCII 字母/数字，其余 ∈ `[A-Za-z0-9._-]`，字节长度 ≤64（字符集全为 ASCII，字节长度=字符长度） | 与实现一致 |

### 3.3 schema — `cmd`

| # | 断言（原文） | 实现依据 | 判定 |
|---|---|---|---|
| 13 | "派单方指定的执行/验证命令" | `cmd` 原样作为 shell 命令执行（Windows `cmd /C`，其他 `sh -c`），并原样写入 `verify_cmd` | 与实现一致 |

### 3.4 schema — `cwd`

| # | 断言（原文） | 实现依据 | 判定 |
|---|---|---|---|
| 14 | "工作目录，默认进程当前目录" | `cwd` 缺省或空串 → `std::env::current_dir()` | 与实现一致 |
| 15 | "回执中的 locator 始终为相对路径" | `artifacts.locator` = `runs/<task_id>/attempt-N/stdout.txt`；无绝对路径 | 与实现一致 |

### 3.5 schema — `timeout_secs`

| # | 断言（原文） | 实现依据 | 判定 |
|---|---|---|---|
| 16 | "超时秒数，默认 120" | 缺省 `unwrap_or(120)`；范围 `1..=600`（schema 同） | 与实现一致 |
| 17 | "超时记 blocked 并给出原因" | 超时 → `timed_out=true` → `blocked_reason = "timeout：…"` → `status="blocked"` | 与实现一致 |

### 3.6 schema — `env`（本批重点）

| # | 断言（原文） | 实现依据 | 判定 |
|---|---|---|---|
| 18 | "可选" | `args.get("env")` 缺失 → `Ok(Vec::new())` | 与实现一致 |
| 19 | "追加/覆盖子进程环境变量" | `command.envs(...)` 只对显式键生效，其余继承父进程 | 与实现一致 |
| 20 | "字符串 map" | `raw.as_object()` 否则 `invalid_env`；值必须 `as_str()` 否则 `invalid_env` | 与实现一致 |
| 21 | "仅非 PATH 变量" | `parse_env` 对 `PATH` 任意大小写变体直接拒绝 | 与实现一致 |
| 22 | "未列出的变量继承 bridge 进程环境" | `Command` 默认继承父环境，实现只追加 `envs`，不清空环境 | 与实现一致 |
| 23 | "显式同名变量覆盖继承值" | `envs` 覆盖同名变量（仅限非 PATH 键） | 与实现一致 |
| 24 | "PATH 任意大小写变体一律拒绝（invalid_env）" | `key.eq_ignore_ascii_case("PATH")` → `invalid_env`，在 `create_dir_all` 之前 | 与实现一致 |
| 25 | "继承的 PATH 永远保留" | 无 `path_prepend` 时 PATH 原样继承；有 `path_prepend` 时 `std::env::var_os("PATH")` 原样拼在末尾 | 与实现一致（注：继承 PATH 为空串时不拼接，语义等价于"没有可保留内容"） |
| 26 | "不会整体覆盖" | 拒绝 `env.PATH`；拼接只做"前插 + 继承" | 与实现一致（**修正后**措辞，v1.14 之前写的是肯定式"会整体覆盖"→ 见 §0） |
| 27 | "也不会被静默忽略" | 看见 `PATH` 即返回错误，不忽略、不降级 | 与实现一致 |
| 28 | "需要追加目录请用 path_prepend" | 唯一追加路径是 `path_prepend` 分支 | 与实现一致 |
| 29 | "值为空串表示置空，不是 unset" | `Command::env(key, "")` 令变量存在且为空串；无删除变量能力 | 与实现一致 |
| 30 | "不做任何自动推断" | `parse_env` 只做校验与透传，不注入任何默认值 | 与实现一致 |

### 3.7 schema — `path_prepend`（本批补齐）

| # | 断言（原文） | 实现依据 | 判定 |
|---|---|---|---|
| 31 | "可选" | 缺失 → `Ok(Vec::new())` | 与实现一致 |
| 32 | "前插到子进程 PATH 的目录列表" | 非空时 `command.env("PATH", joined)`，前插段在继承段之前 | 与实现一致 |
| 33 | "字符串数组"（`type=array`, `items=string`） | `value.as_array()` + 逐元素 `as_str()`，否则 `invalid_path_prepend` | 与实现一致 |
| 34 | "按数组字面顺序排在最前" | 遍历 `path_prepend` 顺序写入 `joined`，无排序/去重 | 与实现一致 |
| 35 | "其后才是继承的 PATH" | 前插段写完后才 `joined.push(inherited)` | 与实现一致 |
| 36 | "平台分隔符自动使用（Windows 为分号，其他平台为冒号）" | `let separator = if cfg!(windows) { ';' } else { ':' };` | 与实现一致 |
| 37 | "空数组与缺省完全等价（签名相同）" | 空数组提前返回空 `Vec`；签名里 `path_prepend: []` 与缺省值同为 `json!([])` | 与实现一致 |
| 38 | "参与幂等签名：同 task_id 重投必须完全一致" | 签名含 `"path_prepend": path_prepend.clone()`；`replay_existing` 比对；旧签名缺该字段补 `[]` 后再比 | 与实现一致 |
| 39 | "上限 16 项" | `MAX_ENTRIES = 16`，超出 → `invalid_path_prepend` | 与实现一致 |
| 40 | "单项 ≤4096 字符" | `MAX_ENTRY_LEN = 4096`，超出 → `invalid_path_prepend` | 与实现一致 |
| 41 | "不做目录存在性校验" | 解析只校验类型/长度/NUL，无 `is_dir()` 调用 | 与实现一致 |
| 42 | "非法形态（非字符串数组/空元素/NUL/超量/超长）在占号前以 invalid_path_prepend 拒绝" | `parse_path_prepend` 在 `create_dir_all` / `create_dir` 之前调用；五类非法形态各有具名错误 | 与实现一致 |

---

## 4. `receipt.get`

| # | 断言（原文） | 实现依据 | 判定 |
|---|---|---|---|
| 43 | "按 task_id 读回执行回执" | `receipt_payload`：`required_str(args,"task_id")` + `is_valid_task_id` | 与实现一致 |
| 44 | "默认返回最新一次" | `include_history` 缺省 false → 返回 `latest.clone()`（`collect_attempts` 按 attempt 序号升序取最后一个） | 与实现一致 |
| 45 | "include_history=true 时返回 attempts 次数" | 返回 `attempts: records.len()` | 与实现一致 |
| 46 | "全部历史" | 返回 `history[]`（全部 `handback.json`） | 与实现一致 |
| 47 | "每次 exit_code 与耗时" | `history` 元素即 handback 原文，含 `exit_code` 与 `duration_ms`；`latest` 同 | 与实现一致 |
| 48 | schema `task_id`："task.execute 使用的派单标识" | 同一 `is_valid_task_id` 校验 | 与实现一致 |
| 49 | schema `include_history`："是否返回全部尝试记录，默认 false" | `as_bool().unwrap_or(false)` | 与实现一致 |

---

## 5. `runs.list`

| # | 断言（原文） | 实现依据 | 判定 |
|---|---|---|---|
| 50 | "只读枚举 runs 目录下的任务" | `runs_list` 只 `read_dir` 与读文件，无写盘、不建目录 | 与实现一致 |
| 51 | "按 task_id 升序" | `task_ids.sort()`（ASCII task_id，字节序即字典序） | 与实现一致 |
| 52 | "offset/limit 确定分页" | `pagination_args`：`offset` 缺省 0、`limit` 缺省 20 且 `1..=100`，`skip(offset).take(limit)` | 与实现一致 |
| 53 | "排除 _session 等预留目录" | `name.starts_with('_')` 或非法 task_id 一律跳过；`.` 开头的 `.prune-trash` 因非法 task_id 同样被排除（"等"覆盖） | 与实现一致 |
| 54 | "只回 env 键名" | `task_summary` 从 `dispatch.json["env"]` 取 `map.keys()` | 与实现一致 |
| 55 | "不回 env 值" | 摘要只输出 `env_keys`；`dispatch.json` 里值本身也只是 `sha256:<hex>` 摘要，无明文 | 与实现一致 |
| 56 | "目录枚举不是执行状态的权威来源" | `runs_list` 文档注释与实现一致：只镜像磁盘现状 | 与实现一致 |
| 57 | "判定请用 receipt.get" | `task_summary` 的 `status` 等字段仅取自 `handback.json`，且契约 §5 明写权威来源为 `receipt.get` | 与实现一致 |
| 58 | schema `offset`："跳过多少条，默认 0"（`minimum: 0`） | 缺省 0；`as_u64()` 非负，负值/非整数 → `invalid_pagination` | 与实现一致 |
| 59 | schema `limit`："本页条数，默认 20"（`1..=100`） | 缺省 20；越界 → `invalid_pagination` | 与实现一致 |

---

## 6. `runs.prune`

| # | 断言（原文） | 实现依据 | 判定 |
|---|---|---|---|
| 60 | "显式证据清理" | 唯一入口是 `tools/call` 的 `runs.prune`；无定时器、无启动钩子 | 与实现一致 |
| 61 | "默认 dry_run=true 只列出候选、零写入" | `dry_run` 缺省 true，分支在 `append_prune_record` / 删除之前直接返回，不建目录不写文件 | 与实现一致 |
| 62 | "真删必须同时 dry_run=false 与 confirm=true" | `!dry_run && !confirm` → `confirm_required` | 与实现一致 |
| 63 | "只删非最新 attempt 的整个目录" | 候选只取 `attempt-N`（`N < latest`）；删除单位是 `attempt-N` 目录（`delete_attempt_dir`） | 与实现一致 |
| 64 | "最新 attempt 永不进入候选" | `prune_candidates` 先 `latest` 记 `latest_attempt_retained`，循环过滤 `*number < latest` | 与实现一致 |
| 65 | "年龄只认 handback 的 finishedAt" | `handback.get("finishedAt")`；不使用 mtime/目录顺序/编号 | 与实现一致 |
| 66 | "缺字段一律不删" | 缺 `finishedAt` → `age_unknown`；缺 `dispatch.json` → `missing_dispatch`；缺 `handback.json` → `unfinished_or_missing_handback`；均跳过 | 与实现一致 |
| 67 | "删除前先写 runs/prune.jsonl 记账" | 先 `append_prune_record(plan)`（失败即一个都不删），再逐项删除，最后写 `result` | 与实现一致 |
| 68 | "部分失败如实报 partial 且 isError=true" | `status = if failed_total == 0 {"ok"} else {"partial"}`；`call_tool` 对 `partial` 置 `isError=true` | 与实现一致 |
| 69 | "系统内没有定时/启动自动清理" | 全仓无清理定时器/启动调用；`runs.prune` 仅由 `tools/call` 触发 | 与实现一致 |
| 70 | schema `older_than_days`："必填…" | 缺失 → `invalid_retention`；在一切读写之前 | 与实现一致 |
| 71 | schema `older_than_days`："只清理 finishedAt 距今不少于该天数的 attempt（1..=3650）" | 范围校验 `1..=3650`；`age_days < older_than_days` 跳过 | 与实现一致 |
| 72 | schema `older_than_days`："不提供默认阈值"（v1.18 改为"推荐 90 天、仍须显式传入"，见 §14.2） | 缺失即报错，不落任何默认值 | 与实现一致 |
| 73 | schema `dry_run`："默认 true：只列候选、不删不写" | 缺省 true；dry-run 返回前零写入 | 与实现一致 |
| 74 | schema `dry_run`："false 表示要求真删（还需 confirm=true）" | `dry_run=false` 且无 confirm → `confirm_required` | 与实现一致 |
| 75 | schema `confirm`："默认 false；真删必须显式 true" | 缺省 false；`!dry_run && !confirm` 拒绝 | 与实现一致 |
| 76 | schema `confirm`："单独 true 不隐含真删" | `dry_run` 缺省为 true，单独 confirm 仍走 dry-run 分支 | 与实现一致 |
| 77 | schema `limit`："本批最多处理的候选数，默认 20"（`1..=100`） | 缺省 20；越界 → `invalid_retention`；`take(limit)` | 与实现一致 |

---

## 7. schema 结构性约束（逐工具）

### 7.1 `inputSchema.type = object` 与 `required`

| # | 断言 | 实现依据 | 判定 |
|---|---|---|---|
| 78 | 六个工具的 `inputSchema.type` 均为 `object` | 六个 schema 均声明 `type=object`；`call_tool` 在缺失 `arguments` 时按 `{}` 处理，非对象值也透传（不额外校验） | 与实现一致（v1.18 更正：原"`call_tool` 要求为对象"表述过强，见 §13.6 注 5） |
| 79 | `task.execute.required = ["task_id","cmd"]`：`task_id` 缺失即错误 | `required_str("task_id")` → 工具错误 | 与实现一致 |
| 80 | `task.execute.required = ["task_id","cmd"]`：`cmd` 缺失按 schema 属必填 | 缺 `cmd` **不报工具错误**，而返回 `blocked` 回执（`command_not_found:`），见契约 §6 | 与实现一致（口径按契约 §6 的 blocked 回执，schema `required` 只约束调用方；服务端容错是既有行为，本批不改） |
| 81 | `receipt.get.required = ["task_id"]` | 缺失 → 工具错误 | 与实现一致 |
| 82 | `runs.prune.required = ["older_than_days"]` | 缺失 → `invalid_retention` | 与实现一致 |
| 83 | `health` / `status` / `runs.list` 的 `properties` 为空或仅可选字段 | `health`/`status` 不读参数；`runs.list` 两字段均可缺省 | 与实现一致 |

### 7.2 `additionalProperties: false`

| # | 断言 | 实现依据 | 判定 |
|---|---|---|---|
| 84 | 六个工具的 `inputSchema.additionalProperties` 均为 `false`（客户端不得传未知字段） | 服务端**不校验未知字段**：`parse_env`/`parse_path_prepend`/`pagination_args` 等只读已知键，未知参数被忽略且不影响任何行为 | 与实现一致（**声明性约束**：约束调用方，服务端不另做拒绝。若把该字段理解为"服务端必须报错"，则属未实现——本批不改行为，按声明性约束记录） |

---

## 8. 契约与描述单一措辞核对

| # | 位置 | 核对结果 | 判定 |
|---|---|---|---|
| 85 | §3.2 `env` ↔ `tools/list` `env.description` | 拒绝任意大小写 PATH、`invalid_env`、占号前拒绝、继承 PATH 永远保留、不静默忽略、追加目录走 `path_prepend` —— 两处逐条一致 | 与实现一致 |
| 86 | §3.5 `path_prepend` ↔ `tools/list` `path_prepend` schema | 数组、字面顺序、平台分隔符、空数组等价缺省、参与幂等签名、16/4096、不校验存在性、占号前拒绝 —— 两处逐条一致 | 与实现一致（且属本批"工具描述同步"行的落地） |
| 87 | §3.1 第 2、3 条 ↔ §3.5 幂等签名 | v1.14 之前 §3.1 的签名口径**漏写** `path_prepend`，与 §3.5 及实现相反 | **不一致（已修）**：§3.1 两处均补上 `path_prepend` |

---

## 9. 本批"真红 → 改 → 绿"证据

### 9.1 红（改之前）

新增两条用例，在**未修改的实现**上运行：

```
env_description_is_fail_closed_like_implementation --- FAILED
schema_declares_path_prepend_with_contract_limits --- FAILED

test result: FAILED. 59 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
exit code 101
```

失败证据（原文）：

- `env` 描述含 "…PATH 会整体覆盖继承值…"，与 `parse_env` 的 fail-closed 实现相反。
- `inputSchema.properties` 中**不存在** `path_prepend`，而实现自 v1.9 起已支持并参与签名。

### 9.2 改后的一次断言口径修正（如实记录）

描述改成"不会整体覆盖"之后，原断言 `!env.contains("整体覆盖")` 反而命中**否定句**，用例仍红（`60 passed; 1 failed; exit 101`）。
这是断言写法过严，不是实现问题。处理方式：**保留用例**，把断言从"禁止出现该词"改为"禁止出现肯定式说法"（`PATH 会整体覆盖` / `PATH 可整体覆盖` / `会整体覆盖继承值` / `覆盖继承值；PATH`），并**新增**一条正向断言"必须出现 `不会整体覆盖`"，防止以后再次写回肯定式措辞。**没有删除任何用例。**

### 9.3 绿

```
test result: ok. 61 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`cargo check -p aw-mcp-bridge` 无 warning。

---

## 10. 本批改动边界（自查）

| 约束 | 结果 |
|---|---|
| 不改任何行为 | 仅改 `tools()` 中的描述字符串与新增 schema 字段；`parse_env` / `parse_path_prepend` / `execute_task` / `runs_prune` 逻辑零改动 |
| 不改回执字段 | `handback` 生成代码零改动 |
| 不碰 `#5` 清理 | `runs_prune` 逻辑零改动；未删除任何既有证据 |
| 不碰 `#3` 进程树杀灭 | 未改动；状态仍为 `NOT_PROVEN`，超时文案保持 v1.11 的诚实口径 |
| 用例只增不减 | 新增 2 条用例；既有的 1 条断言收紧 + 1 条正向断言新增，无删除 |
| 提交分区 | A：描述/契约（`server.rs` + `tests/mcp_contract.rs` + 契约文档）；B：本对照文档 |

---

## 11. 仍存在的已知缺口（不在本批范围，不得被本批描述"修好"所掩盖）

1. **`#3` 整棵进程树杀灭仍不可判定**：v1.16 已按窄定义实现**受控集合**清理（POSIX 进程组 / Windows Job），但逃逸后代（`setsid`、`spawn → Assign` 窗口、`BREAKAWAY`）不可见；`#3` 仍为 `NOT_PROVEN`。`timeout:` 文案与契约 §4 已如实说明；`tools/list` 描述已同步至同一口径（见 §12）。
2. **`additionalProperties: false` 服务端不强制（v1.18 已定为默认策略）**：见 §7.2 / §14.3；保持忽略以兼容现有调用方；若未来强制，升版并新增 `unknown_argument`，默认不做。
3. **清理的阈值/分档/`task_id` 退役纪律（v1.18 已定默认）**：统一 90 天为推荐默认、仍须显式传入；默认不分档（分档推荐值仅入契约，未实现）；清理过的 `task_id` 退役。见契约 §8.9。
4. **认证/准入/并发上限不存在**：`clientInfo` 仅审计、不参与准入（v1.18 已定为默认，参与所需字段见契约 §2.3）；并发上限归 PM 侧排队（契约 §10 第 3 行）；多份 `initialize` 记录不等于多会话、不构成身份。

---

## 12. v1.17 复审：超时 / 回执 / `blocked_reason` 相关描述（2026-11-06）

- 复审对象：`tools/list` 中与超时、回执、`blocked_reason` 相关的描述，对齐 **v1.17 实现**（`timeout_reason` 三态 + Windows `spawn→Assign` 窗口）。
- 口径：**不删任何基线断言**；本次范围内逐句重判，范围外沿用 v1.14 判定。基线总数仍为 **87**。

### 12.1 结论与基线对账

| 统计 | 数量 |
|---|---|
| 上次基线可判定断言总数 | **87**（未增未删） |
| 本次重判（超时 / 回执 / `blocked_reason` 相关） | **22** |
| ├ 与实现一致 | **20** |
| ├ 不一致（已修） | **2**（#2、#17） |
| ├ 未实现却写了 | **0** |
| └ 不可判定 | **0** |
| 未重判（不在本次范围，沿用 v1.14 判定） | 65 |
| 合计 | 87（22 + 65） |

范围 22 条 = `task.execute` 工具级描述全部 11 条（#1-#11，因本次改动该字符串，需整句复核）+ `timeout_secs` 2 条（#16-#17）+ `receipt.get` 7 条（#43-#49）+ `runs.list` 状态权威 2 条（#56-#57）。

**不可判定 = 0 的理由**：这 22 条都对应可静态定位的代码路径（`execute_task` / `write_handback` / `timeout_reason` / `process_scope` / `receipt_payload` / `runs_list`），没有依赖运行时不可观测信息的断言，因此没有需要标"不可判定"的条目。

**未实现却写了 = 0 的理由**：新描述中的每一句都有实现落点——"只向受控集合发送终止信号"对应 Unix `kill(-pgid)` / Windows `TerminateJobObject`；"仅当观测到集合内无存活进程才写「已清理受控集合」"对应 `CleanupOutcome::Cleaned` 只在 `ESRCH` / `ActiveProcesses == 0` 时产生；"`spawn→Assign` 窗口"对应既有的 W2d 取舍与回执文案；"不承诺终止整棵进程树"是对 `#3` 保持 `NOT_PROVEN` 的如实边界声明。

### 12.2 逐条判定（范围内 22 条）

| # | 位置 | 断言（v1.14 原文） | v1.17 实现依据 | 判定 |
|---|---|---|---|---|
| 1 | `task.execute` 描述 | "接单执行：由派单方给出命令" | `execute_task`：`cmd` 由参数给出，经 `shell_command` 执行 | 与实现一致 |
| 2 | `task.execute` 描述 | "执行并把 stdout/stderr … 写成可审计回执（含实际 cwd、生效 timeout_secs、env_keys、changed_files 是否截断）" | 回执字段齐全；**但超时分支的清理结果（三态 + Windows 窗口）未在描述中披露** | **不一致（已修）** |
| 3 | `task.execute` 描述 | "退出码" | `handback.exit_code`（`blocked` 时为 `null`） | 与实现一致 |
| 4 | `task.execute` 描述 | "耗时" | `handback.duration_ms` | 与实现一致 |
| 5 | `task.execute` 描述 | "产物哈希" | `handback.artifacts[{kind,locator,hash}]` | 与实现一致 |
| 6 | `task.execute` 描述 | "可审计回执" | `runs/<task_id>/attempt-N/` + `receipt.get` 可读回 | 与实现一致 |
| 7 | `task.execute` 描述 | "含实际 cwd" | `handback.cwd` | 与实现一致 |
| 8 | `task.execute` 描述 | "生效 timeout_secs" | `handback.timeout_secs`（含默认 120） | 与实现一致 |
| 9 | `task.execute` 描述 | "env_keys" | `handback.env_keys` | 与实现一致 |
| 10 | `task.execute` 描述 | "changed_files 是否截断" | `handback.changed_files_truncated` | 与实现一致 |
| 11 | `task.execute` 描述 | "AW 不定义判据、不做验收" | 无验收逻辑；`status` 仅进程结果 | 与实现一致 |
| 16 | `timeout_secs` 描述 | "超时秒数，默认 120" | `unwrap_or(120)`；范围 `1..=600` | 与实现一致 |
| 17 | `timeout_secs` 描述 | "超时记 blocked 并给出原因" | 实现按三态给原因（`Cleaned` / `StillAlive` / `Unavailable`），Windows 另写 `spawn→Assign` 窗口；描述未写口径与边界 | **不一致（已修）** |
| 43 | `receipt.get` 描述 | "按 task_id 读回执行回执" | `receipt_payload` + `is_valid_task_id` | 与实现一致 |
| 44 | `receipt.get` 描述 | "默认返回最新一次" | `include_history` 缺省 false → `latest` | 与实现一致 |
| 45 | `receipt.get` 描述 | "include_history=true 时返回 attempts 次数" | `attempts = records.len()` | 与实现一致 |
| 46 | `receipt.get` 描述 | "全部历史" | `history[]` 为全部 `handback.json` | 与实现一致 |
| 47 | `receipt.get` 描述 | "每次 exit_code 与耗时" | `history`/`latest` 均为 handback 原文（含 `exit_code`、`duration_ms`、`blocked_reason`） | 与实现一致 |
| 48 | `receipt.get` schema | "task.execute 使用的派单标识" | 同一 `is_valid_task_id` | 与实现一致 |
| 49 | `receipt.get` schema | "是否返回全部尝试记录，默认 false" | `as_bool().unwrap_or(false)` | 与实现一致 |
| 56 | `runs.list` 描述 | "目录枚举不是执行状态的权威来源" | 只镜像磁盘现状 | 与实现一致 |
| 57 | `runs.list` 描述 | "判定请用 receipt.get" | `task_summary.status` 取自 `handback.json` | 与实现一致 |

补充说明（不改变判定）：`receipt.get` 对历史 `handback.json` **原样返回、不改写**（含旧 `timeout：…已终止进程` 文案），描述未做相反承诺，故 #43-#47 不因 v1.17 文案变更而改变判定。

### 12.3 两条不一致：改前 / 改后原文

**#2 `task.execute` 工具级 `description`**

改前：
```text
接单执行：由派单方给出命令，执行并把 stdout/stderr、退出码、耗时、产物哈希写成可审计回执（含实际 cwd、生效 timeout_secs、env_keys、changed_files 是否截断）；AW 不定义判据、不做验收
```

改后：
```text
接单执行：由派单方给出命令，执行并把 stdout/stderr、退出码、耗时、产物哈希写成可审计回执（含实际 cwd、生效 timeout_secs、env_keys、changed_files 是否截断）；AW 不定义判据、不做验收。超时只向受控集合（POSIX 进程组 / Windows Job Object）发送终止信号，仅当观测到集合内无存活进程才写「已清理受控集合」，并注明逃逸进程是否仍在运行未验证；Windows 下 spawn→Assign 窗口内派生的进程可能未被纳入作业。不承诺终止整棵进程树
```

**#17 `task.execute` → `timeout_secs.description`**

改前：
```text
超时秒数，默认 120；超时记 blocked 并给出原因
```

改后：
```text
超时秒数，默认 120；超时记 blocked 并给出原因。原因按受控集合清理结果三态给出：已清理受控集合 / 受控集合内仍有存活进程 / 受控集合未能建立或无法查询（此时仅向直接子进程发送终止信号）；逃逸进程是否仍在运行未验证；不承诺终止整棵进程树
```

### 12.4 验证证据

新增用例 `timeout_description_matches_controlled_set_semantics`（用 `tools/list` 实际返回断言，不读源码字符串），固定上表两条措辞与边界；用例只增不减。

```text
Windows：
running 3 tests  -> test result: ok. 3 passed; 0 failed
running 63 tests -> test result: ok. 63 passed; 0 failed（原 62 + 本次 1）
[evidence] task.execute.description=…超时只向受控集合（POSIX 进程组 / Windows Job Object）…spawn→Assign 窗口…不承诺终止整棵进程树
[evidence] timeout_secs.description=…三态…已清理受控集合 / 受控集合内仍有存活进程 / 受控集合未能建立或无法查询…不承诺终止整棵进程树

POSIX（WSL，verify.sh run，离线）：
running 3 tests  -> test result: ok. 3 passed; 0 failed
running 62 tests -> test result: ok. 62 passed; 0 failed（原 61 + 本次 1）
[INFO] 四类 POSIX 用例全部通过，证据行齐全
```

`cargo check -p aw-mcp-bridge` 通过；本次仅改两条描述字符串 + 1 条用例 + 本文档，**未改实现、未改契约**。

---

## 13. v1.17 复审：其余 65 条（2026-11-06）

- 复审对象：§12 未覆盖的其余 65 条 —— `health` 2、`status` 8、`task.execute` #12-#15（4）与 #18-#42（25）、`runs.list` #50-#55 / #58-#59（8）、`runs.prune` #60-#77（18）。
- 口径：**不删任何基线断言**；逐条对齐 v1.17 实现。v1.17 只改超时文案与两条描述（见 §12），这 65 条对应的实现与描述均未改动，因此本次是复核而非改写。
- 编号说明：§1 / §2 各自从 #1 起编号，与 §3 前 8 条**重号**；本节目录用 `H-` / `S-` 前缀，其余沿用 §3-§6 节内编号。重号是原文档的历史编号方式，不影响基线总数。

### 13.1 结论与基线对账

| 统计 | 数量 |
|---|---|
| 基线可判定断言（§1-§6） | **87** |
| §12 已复审（超时 / 回执 / `blocked_reason`） | **22** |
| §13 本次复审（其余） | **65** |
| ├ 与实现一致 | **65** |
| ├ 不一致（已修） | **0** |
| ├ 未实现却写了 | **0** |
| └ 不可判定 | **0** |
| 合计 | **87（22 + 65）** |

65 的构成：`health` 2 + `status` 8 + `task.execute` #12-#15（4）+ #18-#42（25）+ `runs.list` #50-#55 / #58-#59（8）+ `runs.prune` #60-#77（18）= **65**。

**不一致 = 0 的解释**：本次范围内没有发现描述与实现相反或缺失的条目；v1.17 的两处描述修正已在 §12 处理，不属于这 65 条。**未实现却写了 = 0**：每条断言都能在 `server.rs` 找到对应行为（下表给出依据）。**不可判定 = 0**：这 65 条都对应可静态定位的代码路径与既有集成用例，不依赖运行时不可观测信息。

### 13.2 `health` 与 `status`

| # | 断言（原文） | v1.17 实现依据 | 判定 |
|---|---|---|---|
| H-1 | "返回 MCP 桥存活状态、版本与构建时间（只读）" | `health_payload` 返回 `status:"ok"`、`server.{name,version,buildTime}`；函数体内无任何写盘 | 与实现一致 |
| H-2 | schema：`{"type":"object","properties":{},"additionalProperties":false}` | `call_tool` 的 `health` 分支不读任何参数；schema 声明空 `properties` | 与实现一致 |
| S-1 | "只读汇总桌面端关键状态" | `status_payload` 只读 `settings.json` 与 `_session/`，无写操作 | 与实现一致 |
| S-2 | "配置文件是否就绪" | `summarize_settings` → `settingsPresent`：文件可读且可解析为 JSON 对象 | 与实现一致 |
| S-3 | "AI 是否启用" | `aiEnabled` = `settings["ai_enabled"]` 的布尔值，缺失为 `null` | 与实现一致 |
| S-4 | "启用模型数量" | `enabledModelsCount` = `settings["enabled_models"]` 数组长度，缺失为 `null` | 与实现一致 |
| S-5 | "仓库路径是否配置" | `repoPathConfigured` = `settings["repo_path"]` 非空白字符串 | 与实现一致 |
| S-6 | "initialize 审计（记录可能多份，按 receivedAt 聚合，顶层回显最近一条 clientInfo）" | `session_audit` 扫描 `initialize.json` 与 `initialize-*.json`，按 `receivedAt` 升序，返回 `{clientInfo, receivedAt, total, records[]}`，顶层取最后一条 | 与实现一致（v1.14 已修，v1.17 未变） |
| S-7 | "仅审计弱识别，不构成身份" | `record_client_info` 的边界注释；全链路无准入/鉴权读取该记录 | 与实现一致 |
| S-8 | "不回显任何配置值" | `summarize_settings` 只输出布尔/计数/`null`（含 `aiConfigPresent` 布尔），不含任何字符串配置值 | 与实现一致 |

### 13.3 `task.execute` schema（#12-#15、#18-#42）

| # | 断言（原文） | v1.17 实现依据 | 判定 |
|---|---|---|---|
| 12 | "派单标识（字母或数字开头，仅含字母数字 . _ -，≤64）" | `is_valid_task_id`：首字符 ASCII 字母/数字，其余 ∈ `[A-Za-z0-9._-]`，字节长度 ≤64（字符集全 ASCII，字节长度=字符长度） | 与实现一致 |
| 13 | "派单方指定的执行/验证命令" | `required_str` 取字符串并 `trim` 后作为 shell 命令执行（Windows `cmd /C`，其他 `sh -c`），并写入 `verify_cmd` | 与实现一致（注：首尾空白被 `trim`，描述未承诺逐字保留空白） |
| 14 | "工作目录，默认进程当前目录" | `cwd` 缺失或 `trim` 后为空 → `std::env::current_dir()` | 与实现一致 |
| 15 | "回执中的 locator 始终为相对路径" | `artifacts.locator` = `runs/<task_id>/attempt-N/stdout.txt` / `stderr.txt`，无绝对路径 | 与实现一致 |
| 18 | `env`："可选" | `args.get("env")` 缺失 → 空 `Vec` | 与实现一致 |
| 19 | "追加/覆盖子进程环境变量" | `command.envs(...)` 只对显式键生效，其余继承父进程 | 与实现一致 |
| 20 | "字符串 map" | `raw.as_object()` 否则 `invalid_env`；值必须 `as_str()` 否则 `invalid_env` | 与实现一致 |
| 21 | "仅非 PATH 变量" | `parse_env` 对 `PATH` 任意大小写变体直接拒绝 | 与实现一致 |
| 22 | "未列出的变量继承 bridge 进程环境" | `Command` 默认继承父环境，实现只追加 `envs`，不清空环境 | 与实现一致 |
| 23 | "显式同名变量覆盖继承值" | `envs` 覆盖同名变量（仅限非 PATH 键） | 与实现一致 |
| 24 | "PATH 任意大小写变体一律拒绝（invalid_env）" | `key.eq_ignore_ascii_case("PATH")` → `invalid_env`，且在创建目录之前 | 与实现一致 |
| 25 | "继承的 PATH 永远保留" | 无 `path_prepend` 时 PATH 原样继承；有 `path_prepend` 时 `std::env::var_os("PATH")` 非空则原样拼在末尾 | 与实现一致（继承值为空串时不拼接，语义等价于"没有可保留内容"） |
| 26 | "不会整体覆盖" | 拒绝 `env.PATH`；拼接只做"前插 + 继承" | 与实现一致 |
| 27 | "也不会被静默忽略" | 看见 `PATH` 即返回错误，不忽略、不降级 | 与实现一致 |
| 28 | "需要追加目录请用 path_prepend" | 唯一追加路径是 `path_prepend` 分支 | 与实现一致 |
| 29 | "值为空串表示置空，不是 unset" | `Command::env(key, "")` 令变量存在且为空串；无删除变量能力 | 与实现一致 |
| 30 | "不做任何自动推断" | `parse_env` 只做校验与透传，不注入任何默认值 | 与实现一致 |
| 31 | `path_prepend`："可选" | 缺失 → 空 `Vec` | 与实现一致 |
| 32 | "前插到子进程 PATH 的目录列表" | 非空时 `command.env("PATH", joined)`，前插段在继承段之前 | 与实现一致 |
| 33 | "字符串数组"（`type=array`, `items=string`） | `value.as_array()` + 逐元素 `as_str()`，否则 `invalid_path_prepend` | 与实现一致 |
| 34 | "按数组字面顺序排在最前" | 按 `path_prepend` 顺序写入 `joined`，无排序/去重 | 与实现一致 |
| 35 | "其后才是继承的 PATH" | 前插段写完后才 `joined.push(inherited)` | 与实现一致 |
| 36 | "平台分隔符自动使用（Windows 为分号，其他平台为冒号）" | `let separator = if cfg!(windows) { ';' } else { ':' };` | 与实现一致 |
| 37 | "空数组与缺省完全等价（签名相同）" | 空数组提前返回空 `Vec`；签名里 `path_prepend: []` 与缺省值同为 `json!([])` | 与实现一致 |
| 38 | "参与幂等签名：同 task_id 重投必须完全一致" | 签名含 `"path_prepend": path_prepend.clone()`；`replay_existing` 比对；旧签名缺该字段补 `[]` 后再比 | 与实现一致 |
| 39 | "上限 16 项" | `MAX_ENTRIES = 16`，超出 → `invalid_path_prepend` | 与实现一致 |
| 40 | "单项 ≤4096 字符" | `MAX_ENTRY_LEN = 4096`，超出 → `invalid_path_prepend` | 与实现一致 |
| 41 | "不做目录存在性校验" | 解析只校验类型/长度/NUL，无 `is_dir()` 调用 | 与实现一致 |
| 42 | "非法形态（非字符串数组/空元素/NUL/超量/超长）在占号前以 invalid_path_prepend 拒绝" | `parse_path_prepend` 在 `create_dir_all` / `create_dir` 之前调用；五类非法形态各有具名错误 | 与实现一致 |

### 13.4 `runs.list`（#50-#55、#58-#59）

| # | 断言（原文） | v1.17 实现依据 | 判定 |
|---|---|---|---|
| 50 | "只读枚举 runs 目录下的任务" | `runs_list` 只 `read_dir` 与读文件，无写盘、不建目录 | 与实现一致 |
| 51 | "按 task_id 升序" | `task_ids.sort()`（ASCII task_id，字节序即字典序） | 与实现一致 |
| 52 | "offset/limit 确定分页" | `pagination_args`：`offset` 缺省 0、`limit` 缺省 20 且 `1..=100`，`skip(offset).take(limit)` | 与实现一致 |
| 53 | "排除 _session 等预留目录" | `name.starts_with('_')` 或非法 task_id 一律跳过；`.` 开头的 `.prune-trash` 因非法 task_id 同样被排除（"等"覆盖） | 与实现一致 |
| 54 | "只回 env 键名" | `task_summary` 从 `dispatch.json["env"]` 取 `map.keys()` | 与实现一致 |
| 55 | "不回 env 值" | 摘要只输出 `env_keys`；`dispatch.json` 里值本身也只是 `sha256:<hex>` 摘要，无明文 | 与实现一致 |
| 58 | schema `offset`："跳过多少条，默认 0"（`minimum: 0`） | 缺省 0；`as_u64()` 非负，负值/非整数 → `invalid_pagination` | 与实现一致（注：无上限，schema 只声明 `minimum: 0`） |
| 59 | schema `limit`："本页条数，默认 20"（`1..=100`） | 缺省 20；越界 → `invalid_pagination` | 与实现一致 |

### 13.5 `runs.prune`（#60-#77）

| # | 断言（原文） | v1.17 实现依据 | 判定 |
|---|---|---|---|
| 60 | "显式证据清理" | 唯一入口是 `tools/call` 的 `runs.prune`；无定时器、无启动钩子 | 与实现一致 |
| 61 | "默认 dry_run=true 只列出候选、零写入" | `dry_run` 缺省 true，分支在 `append_prune_record` / 删除之前直接返回，不建目录不写文件 | 与实现一致 |
| 62 | "真删必须同时 dry_run=false 与 confirm=true" | `!dry_run && !confirm` → `confirm_required`（未删除、未写审计） | 与实现一致 |
| 63 | "只删非最新 attempt 的整个目录" | 候选只取 `attempt-N`（`N < latest`）；删除单位是 `attempt-N` 目录（`delete_attempt_dir` 整目录 rename + remove） | 与实现一致 |
| 64 | "最新 attempt 永不进入候选" | `prune_candidates` 先记 `latest_attempt_retained`，循环过滤 `*number < latest` | 与实现一致 |
| 65 | "年龄只认 handback 的 finishedAt" | `handback.get("finishedAt")`；不使用 mtime / 目录顺序 / 编号 | 与实现一致 |
| 66 | "缺字段一律不删" | 缺 `finishedAt` → `age_unknown`；缺 `dispatch.json` → `missing_dispatch`；缺 `handback.json` → `unfinished_or_missing_handback`；另有 `unsafe_attempt_path` / `invalid_finished_at` / `future_timestamp` / `not_old_enough` 同样跳过 | 与实现一致 |
| 67 | "删除前先写 runs/prune.jsonl 记账" | 非空批次先 `append_prune_record(plan)`（失败即一个都不删），再逐项删除，最后写 `result` | 与实现一致（注：空批次没有删除发生，不写审计） |
| 68 | "部分失败如实报 partial 且 isError=true" | `failed_total > 0` → `status="partial"`；`call_tool` 对 `partial` 置 `isError=true` | 与实现一致 |
| 69 | "系统内没有定时/启动自动清理" | 全仓仅 `tools/call` 调用 `runs_prune`；无清理定时器/启动钩子 | 与实现一致 |
| 70 | schema `older_than_days`："必填…" | 缺失 → `invalid_retention`；在一切读写之前 | 与实现一致 |
| 71 | schema `older_than_days`："只清理 finishedAt 距今不少于该天数的 attempt（1..=3650）" | 范围校验 `1..=3650`；`age_days < older_than_days` 跳过 | 与实现一致 |
| 72 | schema `older_than_days`："不提供默认阈值"（v1.18 改为"推荐 90 天、仍须显式传入"，见 §14.2） | 缺失即报错，不落任何默认值 | 与实现一致 |
| 73 | schema `dry_run`："默认 true：只列候选、不删不写" | 缺省 true；dry-run 返回前零写入 | 与实现一致 |
| 74 | schema `dry_run`："false 表示要求真删（还需 confirm=true）" | `dry_run=false` 且无 confirm → `confirm_required` | 与实现一致 |
| 75 | schema `confirm`："默认 false；真删必须显式 true" | 缺省 false；`!dry_run && !confirm` 拒绝 | 与实现一致 |
| 76 | schema `confirm`："单独 true 不隐含真删" | `dry_run` 缺省为 true，单独 confirm 仍走 dry-run 分支 | 与实现一致 |
| 77 | schema `limit`："本批最多处理的候选数，默认 20"（`1..=100`） | 缺省 20；越界 → `invalid_retention`；`take(limit)` | 与实现一致 |

### 13.6 口径注记（不改变判定）

1. `cmd` / `cwd` 的首尾空白会经 `required_str` / `trim` 去除；描述只承诺"派单方给出的命令 / 工作目录"，未承诺逐字保留空白，故 #13 / #14 判定不变。
2. `receipt.get` 的 `latest` / `history` 只含**已落盘**的 `handback.json`；进行中的 attempt 尚无回执、不进入历史。§12 #44 / #46 判定不变。
3. `runs.list` 的 `offset` 无上限（schema 只声明 `minimum: 0`）；负值与非整数由 `as_u64()` 拒绝为 `invalid_pagination`，#58 判定不变。
4. `runs.prune` 在**没有候选**时不写 `prune.jsonl`（没有删除发生）；"先记账后删除"对非空批次成立。#67 判定不变。
5. 基线外 #78 的 v1.14 依据措辞"`call_tool` 要求 `arguments` 为对象"**过强**：实现是"缺失按 `{}`、非对象也透传"，只有 schema 声明为 `object`。此处如实更正依据措辞，#78 的声明性判定不变（见 §13.7）。

### 13.7 基线外 #78-#87 的现状复核（不计入 87）

- **#78-#83（声明性结构）**：新增用例 `schema_structure_declares_required_fields_and_bounds` 用 `tools/list` 实际返回守护六个工具的 `type=object`、`properties`、`required` 与数值边界（`timeout_secs` 1..=600、`runs.list` limit 1..=100、`runs.prune` older 1..=3650 / limit 1..=100）。声明与实现一致。
- **#84（`additionalProperties: false` 服务端不强制）**：v1.18 已定为**默认策略**（契约 §10.1）：保持忽略以兼容现有调用方；服务端只读已知键，未知参数被忽略。若改为强制拒绝，会新增 `unknown_argument` 并改变调用方兼容性，需升版，默认不做。见 §14.3。
- **#85-#87（契约措辞）**：#85 / #86 两处逐条一致；#87（§3.1 签名口径漏 `path_prepend`）已在 v1.14 修复。本次复核未发现新的"描述与实现不符"条目。
- 这 10 条不计入 87 基线（基线只统计 §1-§6）；此处复核不改变基线总数。

### 13.8 验证证据（两平台）

新增用例只增不减，使用 `tools/list` 的实际返回断言，不读源码字符串。

```text
Windows：
cargo check -q -p aw-mcp-bridge（通过）
running 3 tests  -> test result: ok. 3 passed; 0 failed
running 64 tests -> test result: ok. 64 passed; 0 failed（原 63 + 本次 1）

POSIX（WSL，verify.sh run，离线，AW_VENDOR=/root/vendor）：
running 3 tests  -> test result: ok. 3 passed; 0 failed
[evidence] exited pgid=1010 group_empty=true
[evidence] setsid escaped_pid=1014 escaped_alive=true original_group_empty=true
[evidence] stopped pgid=1012 kill(-pgid,0)_exists=true
running 63 tests -> test result: ok. 63 passed; 0 failed（原 62 + 本次 1）
[INFO] 四类 POSIX 用例全部通过，证据行齐全
```

本次仅新增 1 条用例 + 本文档 §13：**未改实现、未改 `tools/list` 描述、未改契约版本（仍 v1.17）**。

---

## 14. v1.18 变更复审：默认决策与 schema 同步（2026-11-06）

v1.18 只做两类可直接落地的变更，**不改执行/超时行为**：把"待 PM 确认"转成有依据的默认决策（契约 §10），并把已实现的限制写进 schema。

### 14.1 改前 / 改后

| 位置 | 改前 | 改后 |
|---|---|---|
| `path_prepend` schema | `"items": {"type":"string"}`，无 `maxItems` / `maxLength` | `"maxItems": 16`，`"items": {"type": "string", "maxLength": 4096}` |
| `runs.prune.older_than_days` 描述 | "必填：只清理 finishedAt 距今不少于该天数的 attempt；不提供默认阈值" | "必填：只清理 finishedAt 距今不少于该天数的 attempt。默认建议 90 天（覆盖一般验收/审计窗口，偏保守）；仍须显式传入，不提供隐式默认阈值，缺省按 invalid_retention 拒绝" |

`path_prepend` 的字段描述本身未改（仍写"上限 16 项、单项 ≤4096 字符"）；v1.18 只是让 schema 声明与实现校验同源。

### 14.2 对 87 条基线的影响

- **#39（上限 16）/ #40（单项 ≤4096）**：实现不变；schema 现在声明同一上限。判定仍"与实现一致"，并从"仅描述"升级为"描述 + schema 双重声明"。
- **#70（必填）**：不变，仍 `required` 且缺失 `invalid_retention`。
- **#71（1..=3650 且距今不少于）**：不变，范围与判龄逻辑未改。
- **#72（"不提供默认阈值"）**：语义不变——**没有隐式默认**，缺省仍拒绝；新增的只是"推荐 90 天"的调用方建议。为避免读者误读，v1.18 把措辞改为"不提供隐式默认阈值"。判定仍"与实现一致"。
- 其余 82 条未触及（87 - #39/#40/#70/#71/#72 共 5 条 = 82）；**87 条基线不增不减**，未出现"描述与实现不符"的新条目。

### 14.3 默认决策（契约 §10 / §2.3 / §8.9 / §10.1）

| 项 | 默认 | 依据 / 可配置方式 |
|---|---|---|
| `additionalProperties` | 保持**忽略未知参数**（schema 仍声明 `false`） | 兼容 `_meta`、SDK 序列化的 null 可选字段、camelCase 别名与自定义元数据；强制拒绝会把这些今天可正常执行的调用变成错误。不提供运行时开关（schema 静态、行为需确定）；若未来强制，升版并新增 `unknown_argument`。新增用例 `unknown_arguments_are_ignored_by_default` 锁定默认 |
| `runs.prune` 阈值/分档 | 统一 **90 天推荐默认**，**仍须显式传入**；默认**不分档**；分档推荐值 `ok=30` / `failed=180` / `blocked=365` 仅入契约、**未实现** | 90 天覆盖一般验收/审计窗口且比 30 天保守；"没写阈值"不能变成"按默认值删"（缺省仍 `invalid_retention`）。每次调用可配 `older_than_days` / `limit` / `dry_run` / `confirm`；分档需新增状态过滤契约，当前不做 |
| `clientInfo` 准入 | 默认**不参与准入**，仅审计 | 自报 `name`/`version`/`title` 无签名、无进程绑定、无会话票据，按它放行等于虚假安全。若参与，至少需要传输/进程锚点、PM/MH 签发的会话票据、主体标识、授权范围（契约 §2.3）；`clientInfo` 只作展示元数据 |
| `path_prepend` 限制 | **写入 schema**（`maxItems=16`、`items.maxLength=4096`） | 与实现校验同源；客户端可在调用前发现超量/超长，减少无效派单。上限是契约常量，改动需升版；调用方仍可在 0..16 项、每项 ≤4096 内自由配置 |

### 14.4 验证证据（两平台）

```text
Windows：
cargo check -q -p aw-mcp-bridge（通过）
running 3 tests  -> test result: ok. 3 passed; 0 failed
running 65 tests -> test result: ok. 65 passed; 0 failed（原 64 + 本次 1）

POSIX（WSL，verify.sh run，离线，AW_VENDOR=/root/vendor）：
running 3 tests  -> test result: ok. 3 passed; 0 failed
[evidence] exited pgid=912 group_empty=true
[evidence] setsid escaped_pid=916 escaped_alive=true original_group_empty=true
[evidence] stopped pgid=914 kill(-pgid,0)_exists=true
running 64 tests -> test result: ok. 64 passed; 0 failed（原 63 + 本次 1）
[INFO] 四类 POSIX 用例全部通过，证据行齐全
```

新增用例 `unknown_arguments_are_ignored_by_default` 锁定 `additionalProperties` 默认忽略；`schema_structure_declares_required_fields_and_bounds` 增补 `maxItems` / `maxLength` 断言。用例只增不减。

本次改动：`server.rs`（`path_prepend` schema + `older_than_days` 描述）、1 条新用例、契约 §2.3/§8.9/§10/§11、本文档 §14。**`#3` 仍 `NOT_PROVEN`；W2c 未重开；回执仍只写已观测状态，逃逸进程保持"未验证"。**
