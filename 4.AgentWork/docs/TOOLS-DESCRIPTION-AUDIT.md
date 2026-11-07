# `tools/list` 描述与实际行为全量对照（v1.14；v1.17 复审：§12 超时/回执相关 22 条，§13 其余 65 条；v1.18 见 §14；v1.19 校正见 §15；v1.20 见 §16；v1.21 见 §17；v1.24 §23；v1.25 §24）

- 对照对象：`apps/mcp-bridge/src/server.rs` 中 `tools()` 返回的**全部工具**的 `description` 与 `inputSchema`（含每个字段的 `description`、`type`、范围、`required`、`additionalProperties`）。
- 权威契约：[`PM-AW-DISPATCH-CONTRACT.md`](./PM-AW-DISPATCH-CONTRACT.md)（本批 v1.14；v1.17 复审见 §12 与 §13；v1.18 见 §14；v1.19 校正见 §15；v1.20 见 §16；v1.21 见 §17；v1.24 见 §23；v1.25 见 §24；v1.26 见 §25；v1.27 见 §26；v1.28 见 §27；v1.29 见 §28）。
- 本批性质（v1.14）：**只修描述与契约措辞，不改任何行为、不改回执字段**；v1.25 例外见 §24（回执纯加 `arguments_diagnostics`，仍不改成功/失败）。
- 判定标签（仅这三种，逐句打标）：
  - **与实现一致**：描述句子能在实现里找到对应的无条件行为。
  - **不一致（已修）**：v1.14 之前描述与实现相反/缺失，**本批已改**。
  - **未实现却写了**：描述承诺了实现里根本不存在的约束或能力。

> 口径提示：`tools/list` 当前是 **8 个工具**（`health` / `status` / `capabilities` / `task.execute` / `receipt.get` / `runs.list` / `runs.inflight` / `runs.prune`）。"四/五个工具"是 v1.5 之前（`runs.list` 尚未加入）的口径；本对照按当前实现全量覆盖。

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
| 72 | schema `older_than_days`："不提供默认阈值"（v1.18 改为"推荐 365 天、仍须显式传入"，见 §14.2） | 缺失即报错，不落任何默认值 | 与实现一致 |
| 73 | schema `dry_run`："默认 true：只列候选、不删不写" | 缺省 true；dry-run 返回前零写入 | 与实现一致 |
| 74 | schema `dry_run`："false 表示要求真删（还需 confirm=true）" | `dry_run=false` 且无 confirm → `confirm_required` | 与实现一致 |
| 75 | schema `confirm`："默认 false；真删必须显式 true" | 缺省 false；`!dry_run && !confirm` 拒绝 | 与实现一致 |
| 76 | schema `confirm`："单独 true 不隐含真删" | `dry_run` 缺省为 true，单独 confirm 仍走 dry-run 分支 | 与实现一致 |
| 77 | schema `limit`："本批最多处理的候选数，默认 100"（`1..=100`；v1.19 由 20 校正为 100） | 缺省 100；越界 → `invalid_retention`；`take(limit)` | 与实现一致 |

---

## 7. schema 结构性约束（逐工具）

### 7.1 `inputSchema.type = object` 与 `required`

| # | 断言 | 实现依据 | 判定 |
|---|---|---|---|
| 78 | 六个工具的 `inputSchema.type` 均为 `object` | 六个 schema 均声明 `type=object`；`call_tool` 在缺失 `arguments` 时按 `{}` 处理，非对象值也透传（不额外校验） | 与实现一致（v1.18 更正：原"`call_tool` 要求为对象"表述过强，见 §13.6 注 5） |
| 79 | `task.execute.required = ["task_id"]`（v1.26 起；`cmd`/`argv` 由 `anyOf` 二选一）：`task_id` 缺失即错误 | `required_str("task_id")` → 工具错误 | 与实现一致 |
| 80 | `task.execute.anyOf = [{required:["cmd"]}, {required:["mode","argv"]}]`：dispatched 需 `cmd`；standalone 需 `mode`+`argv` | 缺 `cmd` 的 dispatched 调用**不报工具错误**，而返回 `blocked` 回执（`command_not_found:`）；`mode` 非法 → `invalid_mode`；standalone 缺 `argv`/`cwd`/`acceptance` → 具名 `standalone_*` blocked 回执（§2.6.1） | 与实现一致（口径按契约 §2.6 / §6；schema 只声明结构，服务端容错是既有行为） |
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

1. **`#3` 整棵进程树杀灭仍不可判定**：v1.16 已按窄定义实现**受控集合**清理（POSIX 进程组 / Windows Job），但逃逸后代（`setsid`、`BREAKAWAY`、加入作业失败）不可见；v1.20 已关闭"恢复执行前派生后代"的 Windows 窗口，但**未提供逃逸观测点**；`#3` 仍为 `NOT_PROVEN`。`timeout:` 文案与契约 §4 已如实说明；`tools/list` 描述已同步至同一口径（见 §16）。
2. **`additionalProperties: false` 服务端不强制（v1.18 已定为默认策略）**：见 §7.2 / §14.3；保持忽略以兼容现有调用方；若未来强制，升版并新增 `unknown_argument`，默认不做。
3. **清理的阈值/分档/`task_id` 退役纪律（v1.18 已定默认；v1.19 按 PM 口径校正）**：统一 365 天为推荐默认、仍须显式传入；默认不分档（分档推荐值仅入契约，未实现）；清理过的 `task_id` 退役。见契约 §8.9。
4. **认证/准入/并发上限不存在**：`clientInfo` 仅审计、不参与准入（v1.18 已定为默认，参与所需字段见契约 §2.3）；并发上限归 PM 侧排队（契约 §10 第 3 行）；多份 `initialize` 记录不等于多会话、不构成身份。

---

## 12. v1.17 复审：超时 / 回执 / `blocked_reason` 相关描述（2026-11-06）

- 复审对象：`tools/list` 中与超时、回执、`blocked_reason` 相关的描述，对齐 **v1.17 实现**（`timeout_reason` 三态 + Windows `spawn→Assign` 窗口）。**v1.20 已把该窗口改为"挂起态创建 → 入作业 → 恢复执行"，本节为历史记录，当前口径以 §16 为准。**
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
| 11 | `task.execute` 描述 | "AW 不定义判据、不做验收"（该句限定 dispatched） | dispatched：无验收逻辑，`status` 仅进程结果；standalone：按任务声明的 `acceptance` 判定，三态结论落盘 `verdict.json`（v1.26）；dispatched 回执仍无 verdict 字段 | 与实现一致（v1.26 分模式表述） |
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
| 72 | schema `older_than_days`："不提供默认阈值"（v1.18 改为"推荐 365 天、仍须显式传入"，见 §14.2） | 缺失即报错，不落任何默认值 | 与实现一致 |
| 73 | schema `dry_run`："默认 true：只列候选、不删不写" | 缺省 true；dry-run 返回前零写入 | 与实现一致 |
| 74 | schema `dry_run`："false 表示要求真删（还需 confirm=true）" | `dry_run=false` 且无 confirm → `confirm_required` | 与实现一致 |
| 75 | schema `confirm`："默认 false；真删必须显式 true" | 缺省 false；`!dry_run && !confirm` 拒绝 | 与实现一致 |
| 76 | schema `confirm`："单独 true 不隐含真删" | `dry_run` 缺省为 true，单独 confirm 仍走 dry-run 分支 | 与实现一致 |
| 77 | schema `limit`："本批最多处理的候选数，默认 100"（`1..=100`；v1.19 由 20 校正为 100） | 缺省 100；越界 → `invalid_retention`；`take(limit)` | 与实现一致 |

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

> v1.19 已按 PM 口径校正 `runs.prune` 的推荐值（365 天 / limit 100 / 先 dry-run / 不分档），见 §15；本节保留 v1.18 的历史记录，当前值以 §15 为准。

v1.18 只做两类可直接落地的变更，**不改执行/超时行为**：把"待 PM 确认"转成有依据的默认决策（契约 §10），并把已实现的限制写进 schema。

### 14.1 改前 / 改后

| 位置 | 改前 | 改后 |
|---|---|---|
| `path_prepend` schema | `"items": {"type":"string"}`，无 `maxItems` / `maxLength` | `"maxItems": 16`，`"items": {"type": "string", "maxLength": 4096}` |
| `runs.prune.older_than_days` 描述 | "必填：只清理 finishedAt 距今不少于该天数的 attempt；不提供默认阈值" | "必填：只清理 finishedAt 距今不少于该天数的 attempt。PM 口径统一推荐 365 天；仍须显式传入，不提供隐式默认阈值，缺省按 invalid_retention 拒绝"（v1.18 初版曾写 90 天，v1.19 已校正） |

`path_prepend` 的字段描述本身未改（仍写"上限 16 项、单项 ≤4096 字符"）；v1.18 只是让 schema 声明与实现校验同源。

### 14.2 对 87 条基线的影响

- **#39（上限 16）/ #40（单项 ≤4096）**：实现不变；schema 现在声明同一上限。判定仍"与实现一致"，并从"仅描述"升级为"描述 + schema 双重声明"。
- **#70（必填）**：不变，仍 `required` 且缺失 `invalid_retention`。
- **#71（1..=3650 且距今不少于）**：不变，范围与判龄逻辑未改。
- **#72（"不提供默认阈值"）**：语义不变——**没有隐式默认**，缺省仍拒绝；新增的只是"推荐 365 天"的调用方建议（v1.18 初版曾写 90 天，v1.19 已按 PM 口径校正）。为避免读者误读，措辞改为"不提供隐式默认阈值"。判定仍"与实现一致"。
- 其余 82 条未触及（87 - #39/#40/#70/#71/#72 共 5 条 = 82）；**87 条基线不增不减**，未出现"描述与实现不符"的新条目。

### 14.3 默认决策（契约 §10 / §2.3 / §8.9 / §10.1）

| 项 | 默认 | 依据 / 可配置方式 |
|---|---|---|
| `additionalProperties` | 保持**忽略未知参数**（schema 仍声明 `false`） | 兼容 `_meta`、SDK 序列化的 null 可选字段、camelCase 别名与自定义元数据；强制拒绝会把这些今天可正常执行的调用变成错误。不提供运行时开关（schema 静态、行为需确定）；若未来强制，升版并新增 `unknown_argument`。新增用例 `unknown_arguments_are_ignored_by_default` 锁定默认 |
| `runs.prune` 阈值/分档 | 统一 **365 天推荐值**、`limit` 缺省 **100**，**older 仍须显式传入**；先 dry-run 预览再真删；默认**不分档**；`ok=30` / `failed=180` / `blocked=365` 仅为"未实现、需另立契约"的历史参考 | PM 口径已定为统一 365 天；"没写阈值"不能变成"按默认值删"（缺省仍 `invalid_retention`）。每次调用可配 `older_than_days` / `limit` / `dry_run` / `confirm`；分档需新增状态过滤契约，当前不做 |
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

---

## 15. v1.19 校正：`runs.prune` 默认包络按 PM 口径（2026-11-06）

> 本节取代 §14 中 `runs.prune` 的推荐值；`additionalProperties` / `clientInfo` / `path_prepend` 三项不变。

### 15.1 变更

| 项 | v1.18 | v1.19（当前） |
|---|---|---|
| 推荐阈值 | 90 天（已从契约删除） | **365 天** |
| `limit` 缺省 | 20 | **100** |
| 操作顺序 | 默认 dry-run + 两段式确认 | 同前，并明确"先 dry-run 预览，再真删" |
| 分档 | 30 / 180 / 365 作为推荐值 | **不做**；30 / 180 / 365 仅作"未实现、需另立契约"的历史参考 |

- 契约中不再保留"90 天"作为默认建议；`older_than_days` 仍必填，缺省 → `invalid_retention`，365 只是推荐值，不是隐式默认。
- 删除条件未变：只删非最新 attempt、只认 `finishedAt`、缺字段不删、先记账后删除。
- 分档未实现：当前接口没有状态过滤参数，不新增删除维度。

### 15.2 对 87 条基线

- **#70 / #71 / #72**：语义不变；#72 的"不提供隐式默认"仍是事实，365 只是推荐值。
- **#77**：`limit` 缺省由 20 校正为 100，范围仍 `1..=100`，判定"与实现一致"。
- **87 条基线不增不减**，未出现"描述与实现不符"的新条目。

### 15.3 验证证据（两平台）

```text
Windows：
cargo check -q -p aw-mcp-bridge（通过）
running 3 tests  -> test result: ok. 3 passed; 0 failed
running 65 tests -> test result: ok. 65 passed; 0 failed

POSIX（WSL，verify.sh run，离线，AW_VENDOR=/root/vendor）：
running 3 tests  -> test result: ok. 3 passed; 0 failed
[evidence] exited pgid=959 group_empty=true
[evidence] setsid escaped_pid=963 escaped_alive=true original_group_empty=true
[evidence] stopped pgid=961 kill(-pgid,0)_exists=true
running 64 tests -> test result: ok. 64 passed; 0 failed
[INFO] 四类 POSIX 用例全部通过，证据行齐全
```

本次改动：`server.rs`（`runs.prune` 的 `limit` 缺省 100、`older_than_days` 描述改为 365）、`prune_defaults_to_dry_run_and_lists_candidates_without_writing` 增补 `limit=100` 断言、契约升 **v1.19**、本文档 §15。**`#3` 仍 `NOT_PROVEN`；W2c 未重开；回执仍只写已观测状态。**

---

## 16. v1.20 窗口收窄：Windows 挂起创建 → 入作业 → 恢复执行（2026-11-06）

> 本节取代 §12/§14 中关于 Windows `spawn→Assign` 窗口的表述；`runs.prune` 的 v1.19 结论不变。

### 16.1 变更

| 项 | v1.19 | v1.20（当前） |
|---|---|---|
| Windows 启动路径 | `spawn` 后 `Assign`（W2d，窗口披露） | **`CREATE_SUSPENDED` → `Assign` → `Toolhelp` 定位主线程 → `ResumeThread`**（W2a） |
| 窗口内派生后代 | 可能不在作业内 | **恢复执行前不产生后代**；该窗口对后代闭合（有确定性夹具） |
| 回执 Windows 文案 | `Windows 下 spawn→Assign 窗口内派生的进程可能未被纳入作业` | `Windows 下子进程以挂起态创建并先加入作业再恢复执行（恢复前不会派生子进程）；BREAKAWAY 或加入作业失败的作业外进程仍不可观测` |
| 恢复失败 | 无此路径 | 终止挂起进程，`blocked_reason` 记 **`process_resume_failed`**，不把“从未执行”当命令结果 |
| 逃逸判定 | 未验证 | **未验证**（`#3` 仍 `NOT_PROVEN`）；新增 §6.8 机器可判定门槛，四条缺一即保留“未验证” |

### 16.2 对 87 条基线

- **#2 / #10（`task.execute` 超时披露）**：描述改为挂起创建/入作业/恢复执行，并明确残余逃逸；判定仍“与实现一致”。
- **#13 / #14 / #15（`timeout_secs` 口径）**：语义不变；仍只承诺受控集合、不承诺整棵树。
- **#50（`blocked_reason` 前缀）**：新增 `process_resume_failed`，前缀集合只增不减；其余前缀不变。
- **87 条基线不增不减**；无“描述与实现不符”的新条目；`spawn→Assign` 不再出现在任何当前描述中（仅历史章节保留并标注）。

### 16.3 验证证据（两平台，先红后绿）

```text
红（实现前，Windows）：
suspended_start_defers_execution_until_job_assign -> FAILED
  "加入作业前子进程不得执行（否则可派生作业外后代）"
timeout_reason_states_action_not_tree_death -> FAILED（仍含 spawn→Assign，未写挂起态/加入作业）
timeout_description_matches_controlled_set_semantics -> FAILED（描述未含挂起态/加入作业）
test result: FAILED. 2 passed; 2 failed

绿（实现后，Windows）：
cargo check -q -p aw-mcp-bridge（通过）
running 4 tests  -> test result: ok. 4 passed; 0 failed
running 65 tests -> test result: ok. 65 passed; 0 failed（原 64 + 本次 1）
[evidence] suspended_start marker_before_assign=false marker_after_resume=true attached=true ActiveProcesses_before_terminate=Some(2)
[evidence] suspended_start terminate=Cleaned ActiveProcesses_after=Some(0)

POSIX（WSL HostGoTest，verify.sh run，离线，AW_VENDOR=/root/vendor）：
running 3 tests  -> test result: ok. 3 passed; 0 failed
[evidence] exited pgid=967 group_empty=true
[evidence] setsid escaped_pid=971 escaped_alive=true original_group_empty=true
[evidence] stopped pgid=969 kill(-pgid,0)_exists=true
[evidence] stopped after_SIGKILL group_empty=true
running 64 tests -> test result: ok. 64 passed; 0 failed
[evidence] grandchild_reason=timeout：超过 1s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证
[evidence] timeout_reason=timeout：超过 1s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证
[INFO] 四类 POSIX 用例全部通过，证据行齐全
```

本次改动：`server.rs`（`CREATE_SUSPENDED` + Toolhelp 恢复 + `attach_error` + 回执文案）、`Cargo.toml`（仅启用 `windows-sys` 已有的 `Win32_System_Diagnostics_ToolHelp` feature，未新增包/版本）、新增 Windows 单元用例 `suspended_start_defers_execution_until_job_assign`、两处文案断言改为新口径、契约升 **v1.20**、设计文档 §4.2/§4.5/§6.5/§6.8 与本文档 §16。**`#3` 仍 `NOT_PROVEN`；W2c 未重开；回执仍只写已观测状态，逃逸保持“未验证”。**

---

## 17. v1.21 状态过滤：`runs.prune.status` 多值白名单（2026-11-06）

> 本节依据 [RUNS-PRUNE-STATUS-TIER-PROPOSAL.md](./RUNS-PRUNE-STATUS-TIER-PROPOSAL.md)；`runs.prune` 的其余删除条件（最新 attempt 永不删、只认 `finishedAt`、缺字段不删、两段式确认、先记账后删除）**全部不变**；分档仍未实现。

### 17.1 描述与 schema 变更

| 位置 | 改前（v1.20） | 改后（v1.21） |
|---|---|---|
| `runs.prune` 工具级 `description` | 无状态过滤表述 | 新增：`status` 是**可选多值白名单**，**显式传入才生效**，缺省不过滤（与旧行为一致）；不接受分档/档位常量 |
| `runs.prune` schema `status` | **不存在** | `{"type":"array","minItems":1,"uniqueItems":true,"items":{"type":"string","enum":["ok","failed","blocked"]}}`；字段描述写明"显式才生效、与 `older_than_days` 为 AND、空数组/null/未知值 `invalid_retention`、无默认状态、无服务端分档" |
| `limit` 字段描述 | "本批最多处理的候选数，默认 100" | 追加"**在 `status` 过滤之后生效**"（`candidates_total` 为过滤后总数） |
| dry-run 输出 | 无 `status_filter` | 新增 `status_filter`（缺省为 `null`）、候选 JSON 新增 `status` |
| `skipped[]` 原因码 | `latest_attempt_retained` / `age_unknown` / `missing_dispatch` / `unfinished_or_missing_handback` | 新增 **`status_unknown`**（回执缺 `status` 或值不认识）与 **`status_mismatch`**（有状态但不在白名单）；仅传 `status` 时产生 |
| 审计行 `runs/prune.jsonl` | plan/result 无过滤上下文 | `status_filter` / `older_than_days` / `limit` / `batch_id` 四件套在 **plan 与 result 两行**都记录 |

**能力门禁（写死在描述里）**：旧服务端会按 `additionalProperties:false` 的声明性约束**静默忽略** `status`，从而把本不该删的候选一起删掉。因此 PM **必须先读 `tools/list` 确认 `runs.prune.inputSchema.properties.status` 存在，才允许携带 `status` 并真删**；schema 不含 `status` 时只能 dry-run、不得真删。服务端不提供 `tier`，未知键仍按 v1.18 的默认策略忽略（不报错、不生效）。

### 17.2 对 87 条基线

- **#60 / #69（显式、无自动清理）**：不变；仍只有 `tools/call` 一条入口。
- **#61 / #62 / #76（dry-run 缺省、两段式确认）**：不变；`status` 不改变确认门。
- **#63 / #64（只删非最新 attempt、最新永不进入候选）**：不变；`status` 白名单**不能**把最新 attempt 拉进候选（case 11 锁定）。
- **#65 / #66（只认 `finishedAt`、缺字段不删）**：不变；缺 `status` 时新增 `status_unknown` 跳过（只会更保守）。
- **#67（先记账后删除）**：不变；plan 行先落盘、失败即一个都不删。
- **#68（partial + isError）**：不变；状态过滤下仍如实报 `partial`（case 20）。
- **#70 / #71 / #72（`older_than_days` 必填与范围）**：不变；仍必填、仍 `1..=3650`、仍无隐式默认，365 只是推荐值。
- **#77（`limit` 缺省 100、`1..=100`）**：缺省与范围不变，措辞补充"在状态过滤之后生效"，判定仍"与实现一致"。
- **#84（`additionalProperties` 声明性忽略）**：不变；`status` 是**入 schema 的已知键**，因此"忽略未知键"不适用于它——传了就按白名单生效，非法就 fail-closed。
- **87 条基线不增不减**，无"描述与实现不符"的新条目。

### 17.3 新增断言（#88–#90，不计入 87 基线的历史编号）

| # | 断言 | 实现依据 | 判定 |
|---|---|---|---|
| 88 | schema 声明 `status`：`array` / `minItems=1` / `uniqueItems` / `items.enum=[ok,failed,blocked]` | `tools()` 中 `runs.prune` 的 `properties.status`，与 `parse status_filter` 的校验同源 | 与实现一致（case 1） |
| 89 | 描述写明"显式才生效、缺省不过滤、与年龄 AND、`limit` 后置、无默认状态/无分档、非法即 `invalid_retention`" | 工具级 `description` 与 `status` 字段描述；行为见 `prune_candidates` 与 `limit.take()` | 与实现一致（case 23） |
| 90 | `skipped[]` 新增 `status_unknown` / `status_mismatch`，且只在不删一侧出现 | `prune_candidates`：缺/未知状态 → `status_unknown`；不在白名单 → `status_mismatch`；两者都不进入候选 | 与实现一致（case 12 / 13 / 3） |

### 17.4 提案 23 条用例 → 实现映射

| 组（测试函数） | 用例 | 断言点 |
|---|---|---|
| `prune_status_schema_and_legacy_compatibility` | 1, 2, 21, 23 | schema 形状；缺省 `status` 与 v1.20 候选集合逐项一致；不传 `status` 时旧字段/旧行为不变；描述语义与实现一致 |
| `prune_status_whitelist_selection` | 3, 4, 10, 11, 12, 13, 14, 15 | 单值只删匹配项；多值取并集；与 `older_than_days` 为 AND；最新 attempt 不受白名单影响；缺 `status` → `status_unknown`；未知值 → `status_unknown`；`limit` 在过滤后生效（`candidates_total` 为过滤后总数）；`limit` 1/100 通过、0/101 → `invalid_retention` |
| `prune_status_validation_is_fail_closed` | 5, 6, 7, 8, 9, 16, 17 | 空数组 / 未知值 / 非数组 / `null` → `invalid_retention`，**零删除零审计**；重复值去重后正常执行并回显去重结果；过滤下 dry-run 仍零写入；缺 `confirm` → `confirm_required` 且零删除零审计 |
| `prune_status_audit_and_mixed_batch` | 18, 19 | plan 与 result 两行都记录 `status_filter` / `older_than_days` / `limit` / `batch_id`；混合批按白名单删、`blocked` 保留、计数正确 |
| `prune_status_partial_failure_is_reported_as_partial`（Windows-only） | 20 | 状态过滤下部分失败仍 `partial` + `isError=true`，其余照删；POSIX 无法用文件锁稳定制造部分失败，故该组只在 Windows 跑 |
| `prune_status_capability_gate_and_no_server_side_tier` | 22 | schema 含 `status` 是 PM 侧能力门禁的唯一依据；`tier` 被忽略、无服务端分档；schema 不含 `status` 时 PM 必须禁止携带并真删 |

### 17.5 先红后绿（原文）

红：以**本批改动前**的 `server.rs`（v1.20，`git restore --source=b98bc81` 对照复跑）执行 6 组新用例，原文如下（仅省略重复的 `note: run with RUST_BACKTRACE=1...` 与交错输出的 `test ... FAILED` 行；断言文本与左右值逐字保留）：

```text
running 6 tests

thread 'prune_status_schema_and_legacy_compatibility' panicked at apps\mcp-bridge\tests\mcp_contract.rs:2620:5:
assertion `left == right` failed: case 1
  left: Null
 right: "array"

thread 'prune_status_validation_is_fail_closed' panicked at apps\mcp-bridge\tests\mcp_contract.rs:2594:5:
assertion `left == right` failed: 该形态必须拒绝：{"candidates":[{"age_days":40,"attempt":1,"dispatchedAt":"2026-01-01T00:00:00Z","finishedAt":"2026-09-27T11:56:43Z","path":"runs/t-guard/attempt-1","task_id":"t-guard"}],"candidates_total":1,"dry_run":true,"evaluated_at":"2026-11-06T11:56:43Z","limit":100,"older_than_days":30,"skipped":[{"attempt":2,"reason":"latest_attempt_retained","task_id":"t-guard"}],"skipped_total":1,"state":"ready","truncated":false}
  left: Bool(false)
 right: true
test prune_status_validation_is_fail_closed ... FAILED

thread 'prune_status_capability_gate_and_no_server_side_tier' panicked at apps\mcp-bridge\tests\mcp_contract.rs:3009:5:
case 22：schema 必须声明 status，这是 PM 侧能力门禁的唯一依据

thread 'prune_status_partial_failure_is_reported_as_partial' panicked at apps\mcp-bridge\tests\mcp_contract.rs:2995:5:
assertion `left == right` failed: case 20
  left: Null
 right: Array [String("failed")]

thread 'prune_status_audit_and_mixed_batch' panicked at apps\mcp-bridge\tests\mcp_contract.rs:2928:5:
assertion `left == right` failed: case 19
  left: Null
 right: Array [String("ok"), String("failed")]

thread 'prune_status_whitelist_selection' panicked at apps\mcp-bridge\tests\mcp_contract.rs:2704:5:
assertion `left == right` failed: case 3
  left: Null
 right: Array [String("failed")]

failures:
    prune_status_audit_and_mixed_batch
    prune_status_capability_gate_and_no_server_side_tier
    prune_status_partial_failure_is_reported_as_partial
    prune_status_schema_and_legacy_compatibility
    prune_status_validation_is_fail_closed
    prune_status_whitelist_selection

test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 65 filtered out; finished in 0.06s
error: test failed, to rerun pass `-p aw-mcp-bridge --test mcp_contract`
```

红阶段同时暴露了"旧服务端静默忽略 `status`"的实际后果：case 5/6/7/8 的非法 `status` 没有被拒绝，反而照常列出候选并进入删除路径——这正是能力门禁必须存在的原因，不是测试写法问题，断言未放宽。

绿：恢复本批实现后（文件 SHA256 与红/绿对照前后一致，`8389EFDD…18D5`），同一命令原文：

```text
running 6 tests
[case 1] schema status=array(minItems=1,uniqueItems,enum=[ok,failed,blocked]) PASS
[case 23] runs.prune 描述与 status 语义一致 PASS
[case 5] 空数组 → invalid_retention，零删除零审计 PASS
[case 6] 未知值/大小写不符 → invalid_retention PASS
[case 7] 非数组 → invalid_retention PASS
[case 8] null → invalid_retention，不当作不过滤 PASS
[case 9] 重复值去重后正常执行，status_filter 回显去重结果 PASS
[case 16] status 过滤的 dry-run 仍零写入 PASS
[case 17] status 合法但缺 confirm → confirm_required，零删除零审计 PASS
[case 2] 缺省 status 与 v1.20 候选集合一致 PASS
[case 21] 不传 status 时旧字段/旧行为不变 PASS
[case 20] status 过滤下 partial 仍 partial + isError=true，其余照删 PASS
[case 19] 混合白名单按状态删除，blocked 保留、计数正确 PASS
[case 22] schema 含 status=能力位；不含时 PM 必须禁止携带并真删 PASS
[case 22] 无服务端 tier；未知参数被忽略，故 PM 必须靠 schema 门禁 PASS
[case 18] plan/result 均记录 status_filter、older_than_days、limit、batch_id PASS
[case 3] status=[failed] 只删 failed，非白名单进 status_mismatch PASS
[case 4] 多值白名单取并集，failed 不进入候选 PASS
[case 10] status + older_than_days 为 AND，未满阈值不删 PASS
[case 11] 最新 attempt 永不删，status 白名单不能选中 PASS
[case 12] 回执缺 status → status_unknown，不删 PASS
[case 13] 未知状态值 → status_unknown，不删 PASS
[case 14] limit 在 status 过滤之后生效，candidates_total 为过滤后总数 PASS
[case 15] limit 1/100 通过，0/101 → invalid_retention PASS
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 65 filtered out; finished in 0.10s
```

### 17.6 两平台全量证据

```text
Windows（cargo test -p aw-mcp-bridge -- --nocapture）：
running 4 tests  -> test result: ok. 4 passed; 0 failed（process_scope 单元）
running 71 tests -> test result: ok. 71 passed; 0 failed（原 65 + 本次 6 组；含 Windows-only case 20）

POSIX（WSL HostGoTest，verify.sh run，离线，AW_VENDOR=/root/vendor）：
running 3 tests  -> test result: ok. 3 passed; 0 failed
[evidence] exited pgid=921 group_empty=true
[evidence] setsid escaped_pid=925 escaped_alive=true original_group_empty=true
[evidence] stopped pgid=923 kill(-pgid,0)_exists=true
[evidence] stopped after_SIGKILL group_empty=true
running 69 tests -> test result: ok. 69 passed; 0 failed（原 64 + 本次 6 组 - Windows-only 1 组）
[INFO] 四类 POSIX 用例全部通过，证据行齐全
```

本次改动：`server.rs`（`status` 校验/去重、`prune_candidates` 状态判定、`skipped` 原因码、dry-run/plan/result 的 `status_filter`、`tools/list` schema 与描述）、`tests/mcp_contract.rs`（6 组 23 条用例）、契约升 **v1.21**（新增 §8.10）、本文档 §17。**未新增依赖、未改删除条件、未放宽任何断言；分档（`ok=30` / `failed=180` / `blocked=365`）仍未实现；`#3` 仍 `NOT_PROVEN`。**

---

## 18. v1.21 真实数据核验与 `_session` 排除（2026-11-06）

> 本节只记录实跑结果与机器判据，**契约版本不变（仍 v1.21）**：无参数、无返回字段、无描述变更。

### 18.1 真实 `runs/` 与规模

- bridge 默认 runs 目录 = 可执行文件同级 `runs/`：本机为 `target\debug\runs`（唯一生产 runs）。`target\debug\deps\runs` 是测试二进制产物，只有 `_session`、无任务目录，不计入。
- 现状：**13 个任务目录**（12 个有 attempt，1 个空目录 `smoke-1`）、**13 个 attempt 目录**；`_session` 文件 **11 个**（本节两次 `initialize` 各 +1，9→10→11）。
- 父会话提到的「57 个 `_session` 文件」在本机未复现：对 `E:\软件开发`、`D:\`、`AppData` 按目录名 `_session` 检索，文件数 >25 的目录为 0（最大是 `target\debug\deps\runs\_session`=22，测试产物）。以下只报告本机实测数字。

### 18.2 三次 dry-run 实测（`older_than_days=365`、`limit=100`）

| `status_filter` | `candidates_total` | `skipped_total` | `status_unknown` | 跳过原因分布 |
|---|---|---|---|---|
| `["ok"]` | 0 | 13 | 0 | `latest_attempt_retained`=12，`missing_dispatch`=1 |
| `["failed"]` | 0 | 13 | 0 | 同上 |
| `["blocked"]` | 0 | 13 | 0 | 同上 |
| 不传（基线） | 0 | 13 | — | 同上 |

三档与基线完全一致：`state=ready`、`candidates_total=0`。

### 18.3 为什么三档都是 0（真实构成）

- 13 个 attempt 的状态构成：`ok`=7、`failed`=2、`blocked`=2、无 handback=2。
- 其中 12 个是各自任务的**最新 attempt** → 永不删，在状态判定之前就被 `latest_attempt_retained` 跳过。
- 唯一非最新 attempt（`smoke-2/attempt-1`，`status=failed`）缺 `dispatch.json` → 在年龄/状态判定之前就 `missing_dispatch` 跳过。
- 有回执的 attempt 的 `finishedAt` 均为 2026-11-05（约 1 天），远未到 365 天；`smoke-2` 两条为遗留空串。

**`status_unknown` = 0 的原因**：不是"没有缺失状态"，而是所有 older attempt 都在更早的检查点被跳过；`status_unknown` 只在"过了年龄线且回执缺 `status`/值不认识"时才出现。据此，真数据在今天、在 365 天阈值下，**任何状态档位都删不到东西**；分档上线也不会删。

### 18.4 `_session` 的机器判据

- 判据收敛到单一函数（`server.rs`）：`is_task_dir_name(name)` 要求名字不以 `_` 或 `.` 开头，且 `is_valid_task_id(name)` 为真；`runs.list` 与 `runs.prune` **共用**该判据。
- 效果：`_session`、`.prune-trash` 等目录即使内含完整 attempt 链（够旧、状态可删），也**不会**进入 `total_tasks` / `total`，不会成为候选，不会产生 `skipped` 记录，也不会被 dry-run/真删处理。
- 与"上层过滤"的区别：判据在枚举入口，任何客户端（PM 或第三方）都绕不过去，不依赖调用方先过滤。
- `_session` **不会**被当成孤儿：代码中没有"孤儿"概念，attempt 计数只从任务目录下的 `attempt-N` 取，`_session` 的 initialize 记录既不进列表也不进计数。

锁定用例 `reserved_directories_are_never_tasks_in_list_or_prune`：`_session` 与 `.prune-trash` 各放两个 400 天前、状态可删的 attempt，另放合法任务 `t-live`（两个 attempt）。断言 `total_tasks=1` / `total=2` / 候选仅 `t-live/attempt-1` / `skipped` 仅 `latest_attempt_retained`，且 dry-run 零写入零删除。

### 18.5 先红后绿（原文）

红：把 `is_task_dir_name` 临时放宽为恒真（对照），同一用例原文：

```text
running 1 test

thread 'reserved_directories_are_never_tasks_in_list_or_prune' panicked at apps\mcp-bridge\tests\mcp_contract.rs:1940:5:
assertion `left == right` failed: 只有 t-live 是任务：{"limit":20,"offset":0,"state":"ready","tasks":[{"attempts":2,"blocked_reason":null,"duration_ms":null,"env_keys":["SECRET_TOKEN"],"exit_code":0,"handback":true,"latest_attempt":2,"status":"failed","task_id":".prune-trash"},{"attempts":2,"blocked_reason":null,"duration_ms":null,"env_keys":["SECRET_TOKEN"],"exit_code":0,"handback":true,"latest_attempt":2,"status":"ok","task_id":"_session"},{"attempts":2,"blocked_reason":null,"duration_ms":null,"env_keys":["SECRET_TOKEN"],"exit_code":0,"handback":true,"latest_attempt":2,"status":"ok","task_id":"t-live"}],"total":6,"total_tasks":3}
  left: Number(3)
 right: 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test reserved_directories_are_never_tasks_in_list_or_prune ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 71 filtered out; finished in 0.04s
```

绿：恢复判据后（文件 SHA256 `9C91EA05…3694` 前后一致）：

```text
running 1 test
test reserved_directories_are_never_tasks_in_list_or_prune ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 71 filtered out; finished in 0.03s
```

说明：`_`/`.` 开头目录的排除在既有实现里已存在，本次**没有行为缺陷可修**（只把两处判据收敛成一个函数）；红是对照红，用来证明新用例确实绑在判据上（放宽判据即失败），不是空断言。

### 18.6 两平台与结论

```text
Windows：4 unit + 72 integration，0 failed
POSIX（verify.sh run）：3 unit + 70 integration，0 failed，四类证据行齐全
```

本次改动：`server.rs`（判据收敛为 `is_task_dir_name`，行为不变）、`tests/mcp_contract.rs`（+1 条用例）、本文档 §18。**契约版本不变、未放宽判据、未新增依赖；分档仍未实现；`#3` 仍 `NOT_PROVEN`。**

---

## 19. `runs.prune` 删除回执字段核验与删除后重跑（2026-11-06）

> 本节只核验、**不改实现**（新用例仅锁定既有行为）；**契约版本不变（仍 v1.21）**。

### 19.1 当前回执（真实响应）

沙箱 = 生产 `runs/` 的副本 + 一个探针任务 `probe-receipt`（attempt 1..3，`finishedAt=2024-01-01`，状态 `failed`/`ok`/`ok`），由真实 bridge 可执行文件走 MCP stdio 执行；生产目录未被改动。

`deleted[]` 单项原文：

```json
{"age_days":1040,"attempt":1,"dispatchedAt":"2026-11-05T18:58:51Z","finishedAt":"2024-01-01T00:00:00Z","path":"runs/probe-receipt/attempt-1","status":"failed","task_id":"probe-receipt"}
```

顶层：`batch_id=prune-1793969836752-46320`、`older_than_days=365`、`status_filter=null`、`limit=100`、`evaluated_at`、`deleted_total=2`、`failed_total=0`、`failed=[]`；审计 `prune.jsonl` 的 plan/result 两行字段与响应一致。

### 19.2 字段缺口（事后核对所需）

| # | 缺口 | 后果 |
|---|---|---|
| 1 | 无内容指纹：没有 `handback.json` / `dispatch.json` 的 sha256，也没有字节数 | 无法证明"被删的就是当时看到的那份回执"，也无法估算释放空间 |
| 2 | 无去向：`deleted[]` 只有原始 `path`，没有 `trash_path` / `deleted_at` / 状态标注 | 瞬时路径已不存在，但字段仍写原路径，读者无法判断它去了哪里 |
| 3 | `failed[]` 不可区分两种失败：`rename` 失败（原路径仍在）与 `remove_dir_all` 失败（已在 `.prune-trash`）都只给 `path` + 自由文本 `error` | 同一 `path` 既可能是"还在原处"，也可能是"已进暂存区"，容易被误读成已删除 |
| 4 | 无 `trashed[]` / `trashed_total`：留在 `.prune-trash` 的条目没有结构化出口 | 只能从 error 文本里抠信息 |
| 5 | 档位/阈值/`batch_id` 只在批次顶层，条目不自包含 | 跨批次或单独核对某一条时要回连批次上下文 |
| 6 | `deleted[]` 与 `failed[]` 条目没有显式 `state`，且 `status` 可为 `null` | 单看条目无法区分"已删除 / 未删除（原处）/ 未删除（暂存）" |

### 19.3 建议形状（兼容优先：旧字段全保留，只增不改）

`deleted[]` 条目（新增字段，`path` 保留为兼容别名）：

```json
{
  "task_id": "...", "attempt": 1,
  "state": "deleted",
  "path": "runs/<task>/attempt-1",
  "source_path": "runs/<task>/attempt-1",
  "trash_path": "runs/.prune-trash/<batch_id>/<task>-attempt-1",
  "deleted_at": "<ISO8601>",
  "rule": { "older_than_days": 365, "status_filter": ["ok"], "limit": 100, "batch_id": "..." },
  "fingerprint": {
    "status": "failed", "finishedAt": "...", "dispatchedAt": "...",
    "handback_sha256": "...", "dispatch_sha256": "...",
    "bytes": { "handback": 1234, "dispatch": 567, "total": 1801 }
  }
}
```

`failed[]` 条目（必须显式二选一，禁止裸 `path`）：

```json
{
  "task_id": "...", "attempt": 1,
  "state": "retained_at_source" | "retained_in_trash",
  "path": "runs/<task>/attempt-1",
  "source_path": "runs/<task>/attempt-1",
  "trash_path": null | "runs/.prune-trash/<batch_id>/<task>-attempt-1",
  "error": "...",
  "fingerprint": { "…与 deleted 同一形状…" }
}
```

新增 `trashed[]`（仅 `remove_dir_all` 失败时出现）与 `trashed_total`，条目：`state:"trashed_not_deleted"` + `trash_path` + `bytes` + `fingerprint`。

语义约束（写进未来契约时必须明确）：`deleted[]` 是**唯一**表示"已从 `runs/` 移除"的集合；`.prune-trash` 里的一切都不算删除；`failed[]` 与 `trashed[]` 都不得使用 `deleted` 字样或 `state:"deleted"`。

### 19.4 删除后重跑（真实响应）

| 步骤 | 响应 |
|---|---|
| 真删前 `runs.list` | `total_tasks=14`、`total=16`（探针 3 个 attempt） |
| 真删（`older_than_days=365`、`limit=100`、`confirm=true`） | `deleted_total=2`、`failed_total=0`、`batch_id=prune-1793969836752-46320` |
| 真删后 `runs.list` | `total=14`（探针只剩 attempt-3） |
| 重跑 dry-run（同参数） | `candidates_total=0`、`skipped_total=14`（`latest_attempt_retained`=13、`missing_dispatch`=1），探针相关仅 attempt-3 的 `latest_attempt_retained` |
| 重跑 dry-run（`status=["ok"]`） | 同上：`candidates_total=0`、`skipped_total=14` |

结论：**当前行为符合**——已删 attempt 不再出现在候选，`skipped` 没有因删除产生新噪声；`.prune-trash` 只留一个空的父目录（0 条目，且不以任务身份进入任何计数）。

### 19.5 锁定用例与先红后绿

用例 `prune_rerun_after_delete_has_no_phantom_candidates_or_skips`：一个任务 3 个 attempt（400 天前），真删 2 个后重跑，断言候选为 0、`skipped_total=1` 且首条是 attempt-3 的 `latest_attempt_retained`、attempt 1/2 不出现在任何候选或跳过项；`.prune-trash` 无残留条目。

红（对照：把 `delete_attempt_dir` 临时改成不删除直接返回 `Ok(())`）：

```text
running 1 test

thread 'prune_rerun_after_delete_has_no_phantom_candidates_or_skips' panicked at apps\mcp-bridge\tests\mcp_contract.rs:3429:5:
assertion failed: !runs.path().join("t-rerun").join("attempt-1").exists()
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test prune_rerun_after_delete_has_no_phantom_candidates_or_skips ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 72 filtered out; finished in 0.01s
```

绿（恢复后，文件 SHA256 `9C91EA05…3694` 前后一致）：

```text
running 1 test
test prune_rerun_after_delete_has_no_phantom_candidates_or_skips ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 72 filtered out; finished in 0.02s
```

### 19.6 两平台与结论

```text
Windows：4 unit + 73 integration，0 failed
POSIX（verify.sh run）：3 unit + 71 integration，0 failed，四类证据行齐全
```

本次改动：`tests/mcp_contract.rs`（+1 条用例）、本文档 §19。**实现未改、契约版本不变、未放宽判据；字段缺口只给建议形状，未改代码；分档仍未实现；`#3` 仍 `NOT_PROVEN`。**

## 20. v1.22 删除回执落地：纯加法字段 + 代码保证（2026-11-06）

### 20.1 版本与改动范围

契约由 v1.21 升到 **v1.22**，纯加法：

- **旧字段全部保留、不改名、不删除**：`task_id` / `attempt` / `path` / `status` / `dispatchedAt` / `finishedAt` / `age_days`、顶层 `deleted_total` / `failed_total`、`failed[].error`、plan 行全部字段。
- **新增条目级字段**：`state` / `source_path` / `trash_path` / `deleted_at` / `rule` / `fingerprint`。
- **新增集合与计数**：`trashed[]`、`trashed_total`。
- `runs.prune` 的入参 schema 未变（无新参数），`tools/list` 描述不改，描述审计基线不因本次变动而变。
- `.prune-trash` 相关内容仍不进入 `deleted[]`，也不进入任何任务/候选计数。

### 20.2 字段最终形状

`deleted[]` 条目（唯一表示"已从 `runs/` 移除"的集合）：

```json
{
  "task_id": "t-fp",
  "attempt": 1,
  "path": "runs/t-fp/attempt-1",
  "status": "failed",
  "dispatchedAt": "2026-01-01T00:00:00Z",
  "finishedAt": "2025-10-02T13:12:07Z",
  "age_days": 400,
  "state": "deleted",
  "source_path": "runs/t-fp/attempt-1",
  "trash_path": "runs/.prune-trash/<batch_id>/t-fp-attempt-1",
  "deleted_at": "2026-11-06T13:12:07Z",
  "rule": {
    "older_than_days": 365,
    "status_filter": ["failed"],
    "limit": 100,
    "batch_id": "<batch_id>"
  },
  "fingerprint": {
    "status": "failed",
    "finishedAt": "2025-10-02T13:12:07Z",
    "dispatchedAt": "2026-01-01T00:00:00Z",
    "handback_sha256": "…",
    "dispatch_sha256": "…",
    "bytes": { "handback": 1028, "dispatch": 202, "total": 1230 }
  }
}
```

`failed[]` 条目（`state` 只能是 `retained_at_source`，`trash_path` 恒为 `null`）：

```json
{
  "task_id": "…", "attempt": 1,
  "path": "runs/<task>/attempt-1",
  "source_path": "runs/<task>/attempt-1",
  "state": "retained_at_source",
  "trash_path": null,
  "error": "…",
  "rule": { "older_than_days": …, "status_filter": …, "limit": …, "batch_id": … },
  "fingerprint": { "…与 deleted 同一形状…" }
}
```

`trashed[]` 条目（`state` 只能是 `trashed_not_deleted`，`trash_path` 非空）：

```json
{
  "task_id": "…", "attempt": 1,
  "path": "runs/<task>/attempt-1",
  "source_path": "runs/<task>/attempt-1",
  "state": "trashed_not_deleted",
  "trash_path": "runs/.prune-trash/<batch_id>/<task>-attempt-1",
  "error": "…",
  "rule": { "…" },
  "fingerprint": { "…" }
}
```

顶层：`deleted_total` / `failed_total` / `trashed_total` 三集合计数，且 `deleted_total + failed_total + trashed_total = 本次实际处理的候选数`；`status` 仅在 `failed_total + trashed_total == 0` 时为 `ok`，否则为 `partial`。

`rule` 在**每一条条目里自包含**（`older_than_days`、`status_filter`、`limit`、`batch_id` 四件套），跨批次核对不需要回连顶层或 `prune.jsonl` 的 plan/result 行。

### 20.3 语义约束由代码保证（可指认位置）

| 约束 | 代码位置 | 机制 |
|---|---|---|
| `deleted[]` 是唯一"已移除"集合 | `server.rs` 删除分支 | 只有 `DeleteOutcome::Deleted` 才 push 进 `deleted` 并写入 `state:"deleted"`；`RetainedAtSource` → `failed`，`RetainedInTrash` → `trashed` |
| 未删除条目不可能自称 `deleted` | `RetainedState` / `retained_entry` | 枚举只有 `AtSource` / `InTrash` 两个取值，`as_str()` 分别返回 `retained_at_source` / `trashed_not_deleted`；未删除条目只能经 `retained_entry(...)` 构造，类型层面构造不出 `"deleted"` |
| `.prune-trash` 里的一切不算删除 | `is_task_dir_name` | 目录名以 `.` 开头直接排除，`.prune-trash` 不成为任务、不进候选、不进任何计数 |
| 三集合互斥且穷尽 | 真删循环 | 每个候选恰走一个 `DeleteOutcome` 分支；三个计数各自 `len()`，合计等于本批 `batch.len()` |
| 部分失败必须报 `partial` | 结果行 | `status = ok` 当且仅当 `failed_total + trashed_total == 0`；`isError=true` |
| 记账失败文案不误导 | `prune_audit_failed` | 文案为"已删除 N 个、失败 M 个、暂存 K 个" |

对应用例：

- `receipt_tests::retained_entries_never_use_deleted_state`（单元）：`AtSource` / `InTrash` 两种变体逐项断言 `state` 正确、`state != "deleted"`、序列化后不含 `"deleted"` 字样、`rule` 自包含、`trash_path` 只在 `InTrash` 时非空。
- `receipt_tests::missing_files_yield_null_fingerprint_parts`（单元）：缺文件必须 `(Null, None)`。
- `prune_rerun_after_delete_has_no_phantom_candidates_or_skips`（集成）：删除后重跑无幽灵候选、无新增 `skipped` 噪声。
- `prune_receipt_deleted_entries_carry_fingerprint_and_rule`（集成）：旧字段保留 + 六个新字段 + 指纹真实值 + 删除后重跑。
- `prune_status_partial_failure_is_reported_as_partial`（集成，Windows）：`failed[0]` 的 `state` / `trash_path` / `source_path` / `rule` / `fingerprint` 逐项断言，并断言 `failed[]` 与 `trashed[]` 不含 `state:"deleted"` 或 `"deleted"` 字样。

### 20.4 指纹实现与事后核对

- 在**删除前**对 `attempt-<n>/handback.json` 与 `dispatch.json` 各做一次 `fs::read`，计算 `sha2::Sha256` 与字节数，写进回执的 `fingerprint`；`bytes.total` 仅在两文件都存在时求和。
- 文件缺失时写 `null`（`handback_sha256` / `dispatch_sha256` / `bytes.handback` / `bytes.dispatch` / `bytes.total`），**不编造值**，也不让删除因此失败。
- 事后核对口径：拿备份/镜像里的同名文件重算 sha256 与字节数，与回执 `fingerprint` 逐项比对；与 `status` / `finishedAt` / `dispatchedAt` 三项元数据同条给出，避免只对内容不对批次。

### 20.5 真实 MCP 响应原文

沙箱：`runs/` 为生产数据副本 + 探针任务 `probe-receipt`（`attempt-1..3`，`finishedAt=2024-01-01`，状态 `failed/ok/ok`）；真实 `aw-mcp-bridge.exe` 走 stdio JSON-RPC，`older_than_days=365`、`limit=100`、`dry_run=false`、`confirm=true`。

`deleted[0]` 原文（单行，字段顺序为 serde_json Map 顺序，不代表契约顺序）：

```text
{"age_days":1040,"attempt":1,"deleted_at":"2026-11-06T13:09:51Z","dispatchedAt":"2026-11-05T18:58:51Z","fingerprint":{"bytes":{"dispatch":202,"handback":1028,"total":1230},"dispatch_sha256":"fb034531b3d23cb6c4ce4d68dfe1c781bebb2ad5a46869d0ff2bb4abac3f3f0d","dispatchedAt":"2026-11-05T18:58:51Z","finishedAt":"2024-01-01T00:00:00Z","handback_sha256":"a7e2e98ae0f3d0596d32a32bb01de348220dd441d8321ebdfa282d57a2102fb6","status":"failed"},"finishedAt":"2024-01-01T00:00:00Z","path":"runs/probe-receipt/attempt-1","rule":{"batch_id":"prune-1793970591456-172644","limit":100,"older_than_days":365,"status_filter":null},"source_path":"runs/probe-receipt/attempt-1","state":"deleted","status":"failed","task_id":"probe-receipt","trash_path":"runs/.prune-trash/prune-1793970591456-172644/probe-receipt-attempt-1"}
```

汇总行：

```text
deleted_total=2 failed_total=0 trashed_total=0
```

要点：`state` 与 `path` / `source_path` / `trash_path` 同条给出；`deleted_at` 逐条时间戳；`rule` 自包含本批四件套（本批未带 `status`，故 `status_filter=null`）；指纹三项（`handback` / `dispatch` / `total`）均为真实字节数，sha256 与删除前文件一致。

### 20.6 先红后绿（红原文原样）

红 A：临时去掉 `deleted[]` 条目里的 `fingerprint` 写入（断言未改）：

```text
running 1 test
test prune_receipt_deleted_entries_carry_fingerprint_and_rule ... FAILED

failures:

---- prune_receipt_deleted_entries_carry_fingerprint_and_rule stdout ----


thread 'prune_receipt_deleted_entries_carry_fingerprint_and_rule' panicked at apps\mcp-bridge\tests\mcp_contract.rs:3571:5:
assertion `left == right` failed: handback sha256 必须与删除前文件一致：{"age_days":400,"attempt":1,"deleted_at":"2026-11-06T13:12:07Z","dispatchedAt":"2026-01-01T00:00:00Z","finishedAt":"2025-10-02T13:12:07Z","path":"runs/t-fp/attempt-1","rule":{"batch_id":"prune-1793970727624-146604","limit":100,"older_than_days":365,"status_filter":["failed"]},"source_path":"runs/t-fp/attempt-1","state":"deleted","status":"failed","task_id":"t-fp","trash_path":"runs/.prune-trash/prune-1793970727624-146604/t-fp-attempt-1"}
  left: None
 right: Some("874f5223ed39df10c850b2f8dc966f29920acc2dc1d5dae940dd4f25eaaf9a29")

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 73 filtered out; finished in 0.02s
```

红 B：临时把 `RetainedState::AtSource` 的 `as_str()` 改成返回 `"deleted"`（断言未改）：

```text
running 1 test
test server::receipt_tests::retained_entries_never_use_deleted_state ... FAILED

---- server::receipt_tests::retained_entries_never_use_deleted_state stdout ----


thread 'server::receipt_tests::retained_entries_never_use_deleted_state' panicked at apps\mcp-bridge\src\server.rs:2302:9:
assertion `left == right` failed
  left: String("deleted")
 right: "retained_at_source"

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s
```

绿：两处扰动原样恢复（`server.rs` SHA256 恢复为 `B1155E5E…D908FB5`，与扰动前一致），Windows 全量：

```text
running 6 tests
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 74 tests
test result: ok. 74 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 20.7 两平台数字

```text
Windows（cargo test -p aw-mcp-bridge）：6 unit + 74 integration，0 failed
POSIX（WSL HostGoTest，verify.sh run，离线）：5 unit + 72 integration，0 failed，四类证据行齐全
```

差额说明：Windows 多 1 条单元与 2 条集成，均为 `cfg(windows)` 用例（作业对象挂起/恢复证据、独占共享模式制造"删一半"失败），POSIX 不适用。

### 20.8 未变项

- `tools/list` 的 `runs.prune` 入参 schema 与描述未变（本次纯回执侧加法）。
- `dry_run` 缺省零写入、两段式确认、先记账后删除、最新 attempt 永不删、"宁可不删"均未削弱。
- 分档（30/180/365）仍未实现、需另立契约。
- `#3`（逃逸进程是否仍在运行）仍 `NOT_PROVEN`。

## 21. 描述审计自动门禁（2026-11-06）

### 21.1 一条命令

```text
cargo test -p aw-mcp-bridge description_gate
```

同一次 `cargo test` 会同时跑单元与集成两类门禁用例：单元侧校验「快照漂移 + 行断言 + 三态文案」，集成侧校验依赖运行时行为的断言（`additionalProperties` 只声明不强制、`status` 能力位与过滤、`_session`、`limit` 缺省值）。

### 21.2 机器可校验表（本文档即唯一来源）

下表中 `contains` / `not_contains` 由用例直接对 `tools/list` 的**实际返回**逐项断言；`tools_sha256` 是对整个 `tools/list` JSON 的规范化序列化做 SHA-256 的**漂移指纹**。任何一处描述或 schema 改动而本文档没有同步更新，`description_gate_snapshot_matches_audit` 立即失败。

| ID | 工具 | 路径 | 必须包含 | 不得包含 | 实现侧断言 |
|---|---|---|---|---|---|
| G1 | `task.execute` | `description` | 受控集合（POSIX 进程组 / Windows Job Object）；终止只尝试一次（不重试）；仅当观测到集合内无存活进程才写「已清理受控集合」；逃逸进程是否仍在运行未验证；不承诺终止整棵进程树；挂起态创建、先加入作业再恢复执行 | 已终止进程树；无残留；会终止整棵进程树；自动重试 | `description_gate_timeout_three_states_match_description` |
| G2 | `task.execute` | `inputSchema.properties.timeout_secs.description` | 默认 120；三态；终止只尝试一次（不重试）；已清理受控集合；受控集合内仍有存活进程；受控集合未能建立或无法查询；仅向直接子进程发送终止信号；逃逸进程是否仍在运行未验证；不承诺终止整棵进程树 | 已终止进程树；无残留；会终止整棵进程树；自动重试 | `description_gate_timeout_three_states_match_description` |
| G3 | `runs.prune` | `description` | 显式传入才生效；缺省不过滤；无默认状态；无服务端分档；与 older_than_days 为 AND | 默认 90 天；默认分档；分档 30 | `description_gate_behavior_status_capability_and_filtering` |
| G4 | `runs.prune` | `inputSchema.properties.status.description` | 显式传入才生效；缺省不过滤；空数组/null/未知值按 invalid_retention 拒绝；无默认状态、无服务端分档 | 默认状态 ok | `description_gate_behavior_status_capability_and_filtering` |
| G5 | `runs.prune` | `inputSchema.properties.limit.description` | 默认 100；在 status 过滤与 exclude 排除之后生效 | — | `description_gate_behavior_status_capability_and_filtering`、`description_gate_behavior_session_and_limit_defaults` |
| G6 | `runs.list` | `description` | 排除 _session 等预留目录 | — | `description_gate_behavior_session_and_limit_defaults` |
| G7 | `runs.list` | `inputSchema.properties.limit.description` | 默认 20 | — | `description_gate_behavior_session_and_limit_defaults` |
| G8 | `health` | `description` | 只读 | — | `description_gate_behavior_additional_properties_declared_only` |
| G9 | `status` | `description` | 只读；不回显任何配置值 | — | `description_gate_behavior_additional_properties_declared_only` |
| G10 | `receipt.get` | `description` | 默认返回最新一次 | — | `description_gate_snapshot_matches_audit` |
| G11 | `status` | `description` | 连接级计数 connections；unit=connection；进程内无会话概念；active/total/peak 恒为 1；initialize_total 随 initialize 次数累加；多份 initialize 不等于多会话；仅观测，不做准入或上限 | 活跃会话数；会话上限；准入拦截 | `description_gate_behavior_connection_counters_real_stdio` |
| G12 | `task.execute` | `description` | mode=standalone；verdict.json；undetermined；无法判定绝不默认通过；attempt < max_attempts；AW 不定义判据、不做验收 | 自动重试；会终止整棵进程树；已终止进程树；无残留 | `description_gate_standalone_mode_boundary` |
| G13 | `task.execute` | `inputSchema.properties.mode.description` | 缺省 dispatched；AW 只执行不验收；拒绝 cmd/env/path_prepend；同一 task_id 不得跨模式复用；mode_conflict | 自动推断模式 | `description_gate_standalone_mode_boundary` |
| G14 | `task.execute` | `inputSchema.properties.acceptance.description` | standalone 必填；file_exists；file_content_match；git_diff_non_empty；standalone_path_escape；未声明验收不得默认通过 | 视为通过 | `description_gate_standalone_mode_boundary` |
| G15 | `runs.prune` | `inputSchema.properties.exclude.description` | 按条目精确排除；task_id；attempt；不得重复；命名校验；invalid_exclusion；not_in_candidates；枚举入口生效 | 默认排除；自动排除 | `description_gate_snapshot_matches_audit`、`description_gate_rows_hold_on_tools` |
| G16 | `task.execute` | `inputSchema.properties.argv.description` | argv[1..]；携带路径；按规范化后的真实位置校验；standalone_argv_path_escape；standalone_argv_path_unknown；argv_path_scan；不按路径校验；占号前 | 不做作用域校验；忽略参数中的路径 | `description_gate_snapshot_matches_audit`、`description_gate_rows_hold_on_tools` |
| G17 | `capabilities` | `description` | 机器可读；holds（true/false/null）；status；scope；reason_code；platforms_verified；platforms_unverified；automatic；不依赖描述散文 | 仅见文档；无法机器判定 | `description_gate_snapshot_matches_audit`、`description_gate_rows_hold_on_tools`、`capabilities_declares_machine_readable_boundaries` |
| G18 | `runs.inflight` | `description` | runs/.inflight/<task_id>.json；action=list；stale_reason；pid_alive；pid_missing_grace；deadline_expired；unreadable；dry_run=true；confirm=true；活跃标记永不删除；pid+startedAt；would_remove；不进入 runs.prune 候选 | 自动清扫；自动删除 | `description_gate_snapshot_matches_audit`、`description_gate_rows_hold_on_tools`、`runs_inflight_lists_and_cleans_only_stale_markers` |
| G19 | `runs.inflight` | `inputSchema.properties.action.description` | list；clean；stale=true | 默认 clean；隐式删除 | `description_gate_snapshot_matches_audit`、`description_gate_rows_hold_on_tools` |

机器可读块（供用例解析；修改请连同 `tools_sha256` 一起重算，并保留 `description-gate` 标记）：

<!-- description-gate:begin -->
```json
{
  "version": "description-gate/1",
  "tools_sha256": "36dbcfc9c9ac83eede1a4ef581801b0b4dcb9b955761d5e3fcb085ea093af993",
  "command": "cargo test -p aw-mcp-bridge description_gate",
  "rows": [
    {
      "id": "G1",
      "tool": "task.execute",
      "path": "description",
      "contains": ["受控集合（POSIX 进程组 / Windows Job Object）", "终止只尝试一次（不重试）", "仅当观测到集合内无存活进程才写「已清理受控集合」", "逃逸进程是否仍在运行未验证", "不承诺终止整棵进程树", "挂起态创建、先加入作业再恢复执行"],
      "not_contains": ["已终止进程树", "无残留", "会终止整棵进程树", "自动重试"]
    },
    {
      "id": "G2",
      "tool": "task.execute",
      "path": "inputSchema.properties.timeout_secs.description",
      "contains": ["默认 120", "三态", "终止只尝试一次（不重试）", "已清理受控集合", "受控集合内仍有存活进程", "受控集合未能建立或无法查询", "仅向直接子进程发送终止信号", "逃逸进程是否仍在运行未验证", "不承诺终止整棵进程树"],
      "not_contains": ["已终止进程树", "无残留", "会终止整棵进程树", "自动重试"]
    },
    {
      "id": "G3",
      "tool": "runs.prune",
      "path": "description",
      "contains": ["显式传入才生效", "缺省不过滤", "无默认状态", "无服务端分档", "与 older_than_days 为 AND", "按条目精确排除", "计数不变式", "excluded_total"],
      "not_contains": ["默认 90 天", "默认分档", "分档 30"]
    },
    {
      "id": "G4",
      "tool": "runs.prune",
      "path": "inputSchema.properties.status.description",
      "contains": ["显式传入才生效", "缺省不过滤", "空数组/null/未知值按 invalid_retention 拒绝", "无默认状态、无服务端分档"],
      "not_contains": ["默认状态 ok"]
    },
    {
      "id": "G5",
      "tool": "runs.prune",
      "path": "inputSchema.properties.limit.description",
      "contains": ["默认 100", "在 status 过滤与 exclude 排除之后生效"],
      "not_contains": []
    },
    {
      "id": "G6",
      "tool": "runs.list",
      "path": "description",
      "contains": ["排除 _session 等预留目录"],
      "not_contains": []
    },
    {
      "id": "G7",
      "tool": "runs.list",
      "path": "inputSchema.properties.limit.description",
      "contains": ["默认 20"],
      "not_contains": []
    },
    {
      "id": "G8",
      "tool": "health",
      "path": "description",
      "contains": ["只读"],
      "not_contains": []
    },
    {
      "id": "G9",
      "tool": "status",
      "path": "description",
      "contains": ["只读", "不回显任何配置值"],
      "not_contains": []
    },
    {
      "id": "G10",
      "tool": "receipt.get",
      "path": "description",
      "contains": ["默认返回最新一次"],
      "not_contains": []
    },
    {
      "id": "G11",
      "tool": "status",
      "path": "description",
      "contains": ["连接级计数 connections", "unit=connection", "进程内无会话概念", "active/total/peak 恒为 1", "initialize_total 随 initialize 次数累加", "多份 initialize 不等于多会话", "仅观测，不做准入或上限"],
      "not_contains": ["活跃会话数", "会话上限", "准入拦截"]
    },
    {
      "id": "G12",
      "tool": "task.execute",
      "path": "description",
      "contains": ["mode=standalone", "verdict.json", "undetermined", "无法判定绝不默认通过", "attempt < max_attempts", "AW 不定义判据、不做验收"],
      "not_contains": ["自动重试", "会终止整棵进程树", "已终止进程树", "无残留"]
    },
    {
      "id": "G13",
      "tool": "task.execute",
      "path": "inputSchema.properties.mode.description",
      "contains": ["缺省 dispatched", "AW 只执行不验收", "拒绝 cmd/env/path_prepend", "同一 task_id 不得跨模式复用", "mode_conflict"],
      "not_contains": ["自动推断模式"]
    },
    {
      "id": "G14",
      "tool": "task.execute",
      "path": "inputSchema.properties.acceptance.description",
      "contains": ["standalone 必填", "file_exists", "file_content_match", "git_diff_non_empty", "standalone_path_escape", "未声明验收不得默认通过"],
      "not_contains": ["视为通过"]
    },
    {
      "id": "G15",
      "tool": "runs.prune",
      "path": "inputSchema.properties.exclude.description",
      "contains": ["按条目精确排除", "task_id", "attempt", "不得重复", "命名校验", "invalid_exclusion", "not_in_candidates", "枚举入口生效"],
      "not_contains": ["默认排除", "自动排除"]
    },
    {
      "id": "G16",
      "tool": "task.execute",
      "path": "inputSchema.properties.argv.description",
      "contains": ["argv[1..]", "携带路径", "按规范化后的真实位置校验", "standalone_argv_path_escape", "standalone_argv_path_unknown", "argv_path_scan", "不按路径校验", "占号前"],
      "not_contains": ["不做作用域校验", "忽略参数中的路径"]
    },
    {
      "id": "G17",
      "tool": "capabilities",
      "path": "description",
      "contains": ["机器可读", "holds（true/false/null）", "status", "scope", "reason_code", "platforms_verified", "platforms_unverified", "automatic", "不依赖描述散文"],
      "not_contains": ["仅见文档", "无法机器判定"]
    },
    {
      "id": "G18",
      "tool": "runs.inflight",
      "path": "description",
      "contains": ["runs/.inflight/<task_id>.json", "action=list", "stale_reason", "pid_alive", "pid_missing_grace", "deadline_expired", "unreadable", "dry_run=true", "confirm=true", "活跃标记永不删除", "pid+startedAt", "would_remove", "不进入 runs.prune 候选"],
      "not_contains": ["自动清扫", "自动删除"]
    },
    {
      "id": "G19",
      "tool": "runs.inflight",
      "path": "inputSchema.properties.action.description",
      "contains": ["list", "clean", "stale=true"],
      "not_contains": ["默认 clean", "隐式删除"]
    }
  ]
}
```
<!-- description-gate:end -->

### 21.3 覆盖与"任何一处改动即失败"

- 每个工具的 `description` 至少被一条行断言覆盖；行中的工具名必须真实存在、路径必须解析为字符串，否则用例失败（防止表格与实现脱节）。
- `tools_sha256` 覆盖**全部**工具描述与全部 `inputSchema`（含 `additionalProperties`、`status` 的 `enum/minItems/uniqueItems`、`limit` 的 `default` 语义措辞等）：即使改动没有破坏任何 `contains`，指纹也会失败，迫使本次改动同步本文档并重新判定。
- 重算指纹方法：跑一次门禁，失败信息里的 `实际=<sha256>` 即为新值；把它写回本块，同时按变更补行/改行。

### 21.4 覆盖的断言清单（对应本次重点）

| 重点 | 自动断言 |
|---|---|
| 三态回执文案 | `timeout_reason` 的三个 `CleanupOutcome` 分支各自必须出现对应措辞；描述必须列出三态；任何分支都不得出现"已终止进程树/无残留" |
| 超时作用域 | 描述必须写明受控集合（POSIX 进程组 / Windows Job Object）、"不承诺终止整棵进程树"、"逃逸进程……未验证"；不得包含"会终止整棵进程树" |
| `status` 能力位 | schema 必须声明 `status`（`array`、`minItems=1`、`uniqueItems=true`、`enum=[ok,failed,blocked]`）；行为上显式白名单确实过滤，未知 `tier` 被忽略（无服务端分档） |
| `additionalProperties` 只声明不强制 | 全部工具的 schema 全部声明 `additionalProperties:false`；同时带未知参数调用不被拒绝、未知参数不生效 |
| `_session` 不作为任务 | `runs.list` 与 `runs.prune` 在有 `_session` 的 runs 目录下都不把它当任务/候选 |
| `limit` 缺省值 | `runs.list` 缺省回显 `limit=20` 且只回 20 条；`runs.prune` 缺省回显 `limit=100`；`limit` 在 `status` 过滤之后生效（过滤后截断、`truncated=true`） |
| 连接计数（v1.23） | 描述必须写明连接级 `connections`、无会话概念、`active/total/peak` 恒为 1、`initialize_total` 累加、只观测；真实 stdio 进程两次 `initialize` 后必须读到 `initialize_total=2` 且 `total/peak/active=1` |
| 终止只尝试一次（v1.24） | `task.execute` 与 `timeout_secs` 描述必须写明「终止只尝试一次（不重试）」「只有观测到集合内无存活进程才写「已清理受控集合」」；不得出现「自动重试」；两平台压力用例必须证明一次终止后判据归零（POSIX `kill(-pgid,0)==ESRCH`、Windows `ActiveProcesses==0`） |
| 能力边界可判定（v1.32） | `capabilities.entries[]` 必须逐条含 `id`/`holds`/`status`/`scope`/`reason_code`/`platforms_verified`/`platforms_unverified`/`automatic`；`status` 必须落在枚举内；`holds=false` 必须有非空原因码；`#3` 整树、逃逸枚举、POSIX 覆盖、TOCTOU、自动清扫五个限制必须有具名条目；`entries_total`/`not_proven_total` 可核对 |
| 残留标记可观察可清（v1.32） | `runs.inflight action=list` 必须对构造的 `pid` 消失+超时标记给出 `stale=true`/`stale_reason=pid_missing`；`pid` 存活的活跃标记即使 deadline 已过也必须 `stale=false`；默认 `clean` 只出 `would_remove`；真删需 `confirm`；清理后残留文件消失、活跃文件保留、二次清理幂等；正常执行结束无残留；坏 JSON 保留并记 `unreadable` |
| PM 判活口径统一（v1.32.1） | `runs.inflight action=list` 必须回传 `liveness_rule`：`authoritative_fields` 恰为 `["markers[].task_id","markers[].stale"]`；`cleanup_fields` 恰为 `["markers[].removable"]`；`diagnostic_fields` 含 `pid`/`pid_alive`/`stale_reason`/`age_secs`；`not_liveness_evidence` 含 `attempt_missing_handback`/`attempt_missing_dispatch`/`raw_marker_file_presence`；`fallback.allowed=false`；契约 §2.8 写死同一口径 |
| 残标顺带清扫（v1.33） | `runs.inflight action=list` 必须回传 `sweep_rule`：`triggers` 恰含 `task_start`/`runs.prune`/`runs.inflight.clean`；`never_removed` 含 `active_pid_alive`/`unreadable`；`audit=runs/inflight-sweep.jsonl`；`idempotent=true`；`limits` 含跨进程无事务/判活与删除之间可加入新进程/坏标记不自愈；三条路径必须实跑：同 `task_id` 重启只清自身残标、`prune` 真删前清残标后候选不再被跳过、`clean` 写 plan+result 且重复触发零删除零新增审计；坏 JSON 在启动路径保留 |

### 21.5 仍然只能靠人的部分（不做自动覆盖）

| 项 | 为什么不能自动验证 |
|---|---|
| Windows 挂起态创建 → 入作业 → 恢复执行、BREAKAWAY 作业外进程不可观测 | 需要 Windows 主机与真实 Job Object 行为；POSIX 主机上无法观测，门禁只能校验描述措辞，环境证据由 `cfg(windows)` 用例单独承担 |
| "AW 不定义判据、不做验收" | 对全仓的否定性断言，有限用例无法证明"不存在验收逻辑"；依赖代码评审与审计复核 |
| "仅审计弱识别，不构成身份" | 同上，属跨链路否定性断言；门禁无法枚举所有未来读取路径 |
| PM 侧能力门禁（先读 schema 再决定是否携带 `status`） | 属调用方行为；服务端只能保证 schema 事实存在，无法验证 PM 是否遵守 |
| "一次终止即足够"的跨环境普适性（v1.24） | 门禁与两平台用例只能证明本机（Windows 作业对象 / WSL Ubuntu 24.04 内核）在这些夹具下成立；内核调度、D 状态、NFS 等等待无法在本环境穷举。可自动验证的是"未观测到空只能报 StillAlive"与"一次终止后判据归零"两个方向 |
| `additionalProperties:false` 对严格客户端的效果 | 严格客户端可能在发送前就拒绝未知参数，服务端观测不到；门禁只验证服务端"自己不拒绝" |
| 残留判定对真实崩溃的普适性（v1.32） | 用例构造的是"pid 数字不存在 + 时间已过"的确定性夹具；真实崩溃后 pid 复用、进程僵死/挂起、时间回拨等无法穷举。能自动验证的是"存活 pid 绝不判残留"与"消失 pid + 超宽限必判残留"两个方向 |
| POSIX 活性判定的平台差异（v1.32.1） | WSL 全量用例在 Linux 上跑通，但 POSIX 上 rename 活跃 attempt 目录、进程组逃逸等语义未单独构造；`platform.posix_inflight_semantics_verified` 保持 `partial`。macOS 仍未验证 |
| 三态文案在真实超时下的分支归属 | 需要真实进程/竞态；门禁只保证文案函数与描述一致，真实分支由 POSIX/Windows 证据用例覆盖 |
| 回执内容（stdout/stderr 正文、耗时、产物哈希）的运行时正确性 | 依赖真实执行环境与文件系统状态；门禁只对齐描述措辞，正确性由执行层用例承担 |

### 21.6 扰动证明（红原文原样）

环境：Windows，命令 `cargo test -p aw-mcp-bridge description_gate`，断言未放宽。

红 A：把 `timeout_secs.description` 的"不承诺终止整棵进程树"改成"会终止整棵进程树"（行断言与漂移指纹同时失败）：

```text
test server::description_gate::description_gate_rows_hold_on_tools ... FAILED
test server::description_gate::description_gate_snapshot_matches_audit ... FAILED

thread 'server::description_gate::description_gate_rows_hold_on_tools' panicked at apps\mcp-bridge\src\server.rs:2412:17:
G2：task.execute 的 inputSchema.properties.timeout_secs.description 缺少必需措辞「不承诺终止整棵进程树」：超时秒数，默认 120；超时记 blocked 并给出原因。原因按受控集合清理结果三态给出：已清理受控集合 / 受控集合内仍有存活进程 / 受控集合未能建立或无法查询（此时仅向直接子进程发送终止信号）；逃逸进程是否仍在运行未验证；会终止整棵进程树

thread 'server::description_gate::description_gate_snapshot_matches_audit' panicked at apps\mcp-bridge\src\server.rs:2384:9:
assertion `left == right` failed: tools/list 描述或 schema 已改动：请同步 TOOLS-DESCRIPTION-AUDIT.md §21（行断言 + tools_sha256）。实际=ac049762557fa423c763f3a55d4a644c133ec59e7a7d1243b599d0764c7bdbce

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s
```

红 B：只把 `health` 描述的"（只读）"改成"（只读查询）"——不破坏任何 `contains`，但漂移指纹必须失败：

```text
test server::description_gate::description_gate_timeout_three_states_match_description ... ok
test server::description_gate::description_gate_rows_hold_on_tools ... ok
test server::description_gate::description_gate_snapshot_matches_audit ... FAILED

thread 'server::description_gate::description_gate_snapshot_matches_audit' panicked at apps\mcp-bridge\src\server.rs:2384:9:
assertion `left == right` failed: tools/list 描述或 schema 已改动：请同步 TOOLS-DESCRIPTION-AUDIT.md §21（行断言 + tools_sha256）。实际=3f9ccb0395b8216a33e6760cb52c95575dfb91479f2efa9b517fab6a2a9b694f

test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s
```

恢复两处扰动后（描述逐字还原，`tools_sha256` 回到 `96441bc4…35a736`）：

```text
running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out
running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 74 filtered out
```

### 21.7 两平台数字

```text
Windows（cargo test -p aw-mcp-bridge）：9 unit + 78 integration，0 failed（门禁 3 + 4）
POSIX（WSL HostGoTest，verify.sh run 离线）：8 unit + 76 integration，0 failed，四类证据行齐全（门禁 3 + 4）
```

差额仍为 `cfg(windows)` 用例。POSIX 侧 `verify.sh` 已同步把本文档复制进裁剪 workspace，否则 `include_str!` 会让门禁用例在 POSIX 上无法编译。

结论：门禁把"描述与实现对齐"从"靠人记得更新"变成**可执行、可失败**；上表列出的环境依赖项与否定性断言仍只能靠人，已如实标注，不假装覆盖。

## 22. 「多份 initialize ≠ 多会话」的可观测口径（v1.23）

### 22.1 实测方法与结论

方法：启动**真实** `aw-mcp-bridge.exe`（stdio），同一进程内先发两条 `initialize`（`clientInfo.name` 分别为 `count-probe-1` / `count-probe-2`），再发 `tools/call status`。实测输出（`status.connections` 原文）：

```text
{"active":1,"initialize_total":2,"note":"进程内无会话概念：stdio 单连接，active/total/peak 恒为 1；同一连接重复 initialize 只累加 initialize_total，不新增连接；仅观测，不做准入或上限","peak":1,"scope":"process","total":1,"unit":"connection"}
```

结论：**bridge 进程内没有"会话"概念，只有一条 stdio 连接**。多份 `initialize` 不是多个会话，也不产生多条连接；`initialize_total` 是唯一随重复握手变化的量。因此计数按**连接级**命名，不使用"会话数"措辞。

依据（不是读代码推断）：`main` 只从 stdin 读循环、EOF 即退出，没有 accept/重连路径；上述实测中两条 `initialize` 都返回协议版本，而 `total`/`peak`/`active` 保持 1、`initialize_total` 为 2。

### 22.2 暴露字段

`status` 结果新增 `connections`（纯加法；`runs.list` 不变）：

| 字段 | 类型 | 含义 | stdio 下的值 |
|---|---|---|---|
| `unit` | string | 计数单位 | `"connection"` |
| `scope` | string | 作用域 | `"process"` |
| `active` | number | 当前活跃连接数 | `1`（进程存活期间唯一 stdin 连接） |
| `total` | number | 累计连接数 | `1` |
| `peak` | number | 并发峰值 | `1`（没有第二条连接路径，故恒为 1） |
| `initialize_total` | number | 本连接累计 `initialize` 次数 | 随调用累加 |
| `note` | string | 口径说明 | 固定文案，明写无会话概念与只观测 |

实现：[connections_payload](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs)。`active`/`total` 由"首条消息即连接建立"的原子标记推导，`peak` 由该标记置 1，`initialize_total` 在每次收到 `initialize` 时原子累加。**不拦截**：没有准入、没有连接上限，`clientInfo` 仍只落审计、不参与任何判断。

### 22.3 与描述门禁打通

- `tools/list` 的 `status` 描述新增连接级计数说明；`tools_sha256` 由 `96441bc4…` 变为 `884952c7…`，未同步文档即失败。
- 新增门禁行 `G11`：要求描述包含「连接级计数 connections」「unit=connection」「进程内无会话概念」「active/total/peak 恒为 1」「initialize_total 随 initialize 次数累加」「多份 initialize 不等于多会话」「仅观测，不做准入或上限」；禁止「活跃会话数」「会话上限」「准入拦截」。
- 新增行为用例 `description_gate_behavior_connection_counters_real_stdio`：**真启动二进制**、走真实 stdio，断言两次 `initialize` 后 `initialize_total=2` 且 `active/total/peak=1`。描述或计数任一改动都会让门禁失败。

### 22.4 扰动证明（红原文原样）

红 A：去掉 `initialize_total` 累加（行为扰动），真实 stdio 用例失败：

```text
test description_gate_behavior_connection_counters_real_stdio ... FAILED

thread 'description_gate_behavior_connection_counters_real_stdio' panicked at apps\mcp-bridge\tests\mcp_contract.rs:3934:5:
assertion `left == right` failed: 两次 initialize 只累加 initialize_total，不新增连接/session：{"connections":{"active":1,"initialize_total":0,"note":"进程内无会话概念：stdio 单连接，active/total/peak 恒为 1；同一连接重复 initialize 只累加 initialize_total，不新增连接；仅观测，不做准入或上限","peak":1,"scope":"process","total":1,"unit":"connection"},…}
  left: Number(0)
 right: 2

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 77 filtered out; finished in 0.29s
```

红 B：把描述里的「仅观测，不做准入或上限」改成「并对连接数做准入拦截」（描述扰动），行断言与快照同时失败：

```text
thread 'server::description_gate::description_gate_rows_hold_on_tools' panicked at apps\mcp-bridge\src\server.rs:2439:17:
G11：status 的 description 缺少必需措辞「仅观测，不做准入或上限」：只读汇总桌面端关键状态：…；并对连接数做准入拦截）；不回显任何配置值

thread 'server::description_gate::description_gate_snapshot_matches_audit' panicked at apps\mcp-bridge\src\server.rs:2411:9:
assertion `left == right` failed: tools/list 描述或 schema 已改动：请同步 TOOLS-DESCRIPTION-AUDIT.md §21（行断言 + tools_sha256）。实际=03e1c3dd7b4755ea59985ec1173e44408839ab740d8a6b19a94f11905e3dc840
  left: "03e1c3dd7b4755ea59985ec1173e44408839ab740d8a6b19a94f11905e3dc840"
 right: "884952c7686d305356dc0f41b52af5579835edf54ea37f0e7b7378293b093092"

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s
```

两处扰动已逐字恢复，门禁回到 3 + 4 全绿。

### 22.5 两平台数字与边界

```text
Windows：9 unit + 78 integration，0 failed
POSIX （WSL HostGoTest，verify.sh run 离线）：8 unit + 76 integration，0 failed，四类证据行齐全
```

边界（如实标注，不扩大口径）：

- 计数的"连接"指 **stdio 进程内唯一连接**，不是 TCP/socket 多连接；`peak` 恒为 1 是因为根本没有第二条连接路径，而不是"测出来恰好为 1"。
- 计数是**进程级**：不同 bridge 进程各自独立，没有跨进程共享或汇总。
- 只观测：不参与准入、不做上限、不做限流；`clientInfo` 仍不参与准入。
- 真实 stdio 用例会把 `initialize` 审计文件写到可执行文件同级的 `runs/_session`（测试目录，非生产 runs）；不影响计数断言。

## 23. 超时终止「单次尝试」实测与门禁（v1.24）

### 23.1 结论

**一次终止即足够，本批不新增第二次尝试。** 实测：POSIX 68 轮（高速 fork / 2GB 进程 16ms fork 窗口 / 400 个已存在子进程）+ Windows 20 轮（成员持续 `start /b` 拉起后代）均未出现"一次 SIGKILL / 一次 `TerminateJobObject` 后集合内仍有存活"。能构造出的 `StillAlive` 场景（僵尸成员、cgroup 冻结）重试也无效：僵尸对再次 SIGKILL 是 no-op，冻结进程在首次 SIGKILL 后已是 Z，解冻后才消失。完整夹具、原始输出与"fork 为何不会留下幸存者"见 [TIMEOUT-PROCESS-TREE-DESIGN.md](./TIMEOUT-PROCESS-TREE-DESIGN.md) §6.9。

回执语义未变松：只有观测到 POSIX `kill(-pgid,0)==ESRCH` / Windows `ActiveProcesses==0` 才写 `Cleaned`；任何分支仍不出现"已终止进程树"；逃逸进程仍写"未验证"；`#3` 仍 `NOT_PROVEN`，W2c 未重开。

### 23.2 描述与门禁对应

| 门禁行 | 覆盖内容 | 本批变化 |
|---|---|---|
| G1 | `task.execute` 的 `description` | 新增必需措辞「终止只尝试一次（不重试）」；`not_contains` 新增「自动重试」 |
| G2 | `task.execute.inputSchema.properties.timeout_secs.description` | 新增必需措辞「终止只尝试一次（不重试）」「只有观测到集合内无存活进程才写「已清理受控集合」」；`not_contains` 新增「自动重试」 |

`tools_sha256` 由 `884952c7686d305356dc0f41b52af5579835edf54ea37f0e7b7378293b093092` 更新为 `10128b3ed88a4b4df5d419dbf61e02b151a8832f4cd5eb2e52791a67851d3895`。新增/改动的两行同时受行断言、每个工具描述覆盖检查与快照哈希三重约束。

### 23.3 门禁扰动（红）原文

把 `task.execute` 描述里的「终止只尝试一次（不重试）」改成「终止会自动重试直到清理」，行断言与快照同时失败：

```text
thread 'server::description_gate::description_gate_rows_hold_on_tools' panicked at apps\mcp-bridge\src\server.rs:2513:17:
G1：task.execute 的 description 缺少必需措辞「终止只尝试一次（不重试）」：接单执行：…超时只向受控集合（POSIX 进程组 / Windows Job Object）发送终止信号；终止会自动重试直到清理，仅当观测到集合内无存活进程才写「已清理受控集合」…

thread 'server::description_gate::description_gate_snapshot_matches_audit' panicked at apps\mcp-bridge\src\server.rs:2485:9:
assertion `left == right` failed: tools/list 描述或 schema 已改动：请同步 TOOLS-DESCRIPTION-AUDIT.md §21（行断言 + tools_sha256）。实际=3600f8fb17c828dc892fff3da43f73926159fbe514206528ad93d8ebd2839c8c
  left: "3600f8fb17c828dc892fff3da43f73926159fbe514206528ad93d8ebd2839c8c"
 right: "10128b3ed88a4b4df5d419dbf61e02b151a8832f4cd5eb2e52791a67851d3895"
```

扰动已逐字恢复，门禁回到 3 + 4 全绿。

### 23.4 用例变异（红）原文

新增的两个压力用例不是空转：把实现临时削弱后各自失败（变异已逐字还原）。

POSIX：把 `kill(-pgid, SIGKILL)` 改成只杀组长 `kill(pgid, SIGKILL)`，子进程留在组内：

```text
[evidence] fork_pressure pgid=12971 group_exists_before=true outcome=StillAlive elapsed_ms=3533

thread 'server::process_scope::imp::tests::single_sigkill_suffices_under_fork_pressure' panicked at apps/mcp-bridge/src/server.rs:1511:17:
assertion `left == right` failed: 一次 SIGKILL 后受控集合必须清空
  left: StillAlive
 right: Cleaned
```

Windows：把 `TerminateJobObject` 改成只 `child.kill()`，后代留在作业内：

```text
[evidence] spawn_pressure trial=0 ActiveProcesses_before=Some(2) outcome=Cleaned ActiveProcesses_after=Some(0) elapsed_ms=23
[evidence] spawn_pressure trial=1 ActiveProcesses_before=Some(5) outcome=StillAlive ActiveProcesses_after=Some(3) elapsed_ms=3012

thread 'server::process_scope::imp::tests::single_terminate_suffices_under_spawn_pressure' panicked at apps\mcp-bridge\src\server.rs:1950:21:
assertion `left == right` failed: 一次 TerminateJobObject 后作业必须清空
  left: StillAlive
 right: Cleaned
```

### 23.5 两平台数字与边界

```text
Windows：10 unit + 78 integration，0 failed
POSIX （WSL HostGoTest，verify.sh run 离线）：9 unit + 76 integration，0 failed，四类证据行齐全
```

边界（如实标注）：

- 门禁只能自动验证"描述与 tools/list 一致""未观测到空只能报 StillAlive""本机夹具下一次终止后判据归零"；跨内核/调度状态的普适性列入 §21.5。
- POSIX 的 2GB fork 夹具把 fork 窗口放大到约 16ms 仍 0 幸存；结论依赖内核"致命信号 pending 时中止 fork"，不是概率性断言。
- Windows 的作业成员关系在 `CreateProcess` 时原子继承且本实现不允许 `BREAKAWAY`；`CREATE_SUSPENDED` 关闭"恢复前派生"窗口。
- 真实 D 状态（I/O 不可中断）无法在本环境构造；SIGKILL 一旦 pending 不会丢失，重发同一信号不能加速。

## 24. 参数诊断 `arguments_diagnostics`（v1.25）

### 24.1 结论与边界

`additionalProperties: false` 仍**只声明、不强制**：未知参数照旧被忽略，调用照旧成功/失败。v1.25 只把"忽略"变成**可观测**——命中工具后，若有未知参数或显式 `null` 的可选字段，回执 JSON 追加 `arguments_diagnostics`；没有可报告项时该字段**不出现**（不产生噪声）。同一份 JSON 同步写 stderr 单行日志。

判定口径（调用方无需读源码）：

| `kind` | 含义 | 排查结论 |
|---|---|---|
| `unknown` | 规范化为小写、去掉 `_`/`-` 后对不上任何已声明参数 | 多半是参数名拼错；`declared_arguments` 给出正确名单 |
| `alias` | 能对上某个已声明参数（如 `dryRun` → `dry_run`） | 客户端用了 camelCase/snake_case 别名；`alias_of` 指出目标 |
| `client_metadata` | `_meta` | 客户端多塞的元数据，不是拼写错误 |
| `null_optional` | schema 声明为可选、显式传 `null` | SDK 序列化行为，不是错误，调用照旧 |

**别名不生效**：诊断只指出 `alias_of`，服务端不会替调用方改名（`dryRun` 仍不会驱动 `dry_run`）。**敏感值不落诊断**：值只给 `{type,length}`（字符串字符数 / 数组项数 / 对象键数），`_meta` 只报键数，不报嵌套键与值。未知工具名不做诊断（调用本身返回未知工具错误）。`tools/list` 的 `description`/`inputSchema` 未改，门禁 `tools_sha256` 不变。

### 24.2 字段

| 字段 | 说明 |
|---|---|
| `tool` | 本次调用的工具名 |
| `declared_arguments` | 该工具已声明的参数名 |
| `unknown_arguments[]` | `name` / `kind` / `alias_of`（仅 alias）/ `value{type,length}` |
| `null_optional_arguments[]` | `name` / `kind="null_optional"` / `declared_type` / `value{type:"null",length:null}` |
| `note` | 固定文案：只记录参数名与值类型/长度，不改变成功/失败 |

### 24.3 用例先红后绿

新增 5 条集成用例：完全未知参数名、camelCase 别名、显式 `null` 可选字段（必须成功）、无多余参数不产生诊断、值不泄漏；另把既有"未知参数被忽略"用例改为精确断言（payload 无 `error` 字段 + 诊断存在），替代原来对 `"unknown_argument"` 子串的弱检查。

红：临时把 `apps/mcp-bridge/src/server.rs` 的实现改动 stash（回到 v1.24 行为），同一批新用例原样失败：

```text
running 5 tests

thread 'arguments_diagnostics_flags_unknown_names_without_rejecting' panicked at apps\mcp-bridge\tests\mcp_contract.rs:331:5:
assertion `left == right` failed
  left: Null
 right: "runs.list"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'arguments_diagnostics_never_leaks_values' panicked at apps\mcp-bridge\tests\mcp_contract.rs:310:28:
unknown_arguments 必须是数组：{"limit":20,"offset":0,"state":"empty","tasks":[],"total":0,"total_tasks":0}

thread 'arguments_diagnostics_marks_camel_case_alias_not_typo' panicked at apps\mcp-bridge\tests\mcp_contract.rs:310:28:
unknown_arguments 必须是数组：{"candidates":[],"candidates_total":0,"dry_run":true,"evaluated_at":"2026-11-06T16:57:28Z","limit":100,"older_than_days":365,"skipped":[],"skipped_total":0,"state":"empty","status_filter":null,"truncated":false}
test arguments_diagnostics_is_absent_for_clean_calls ... ok
test arguments_diagnostics_flags_unknown_names_without_rejecting ... FAILED
test arguments_diagnostics_marks_camel_case_alias_not_typo ... FAILED
test arguments_diagnostics_never_leaks_values ... FAILED

thread 'arguments_diagnostics_records_null_optionals_and_stays_successful' panicked at apps\mcp-bridge\tests\mcp_contract.rs:395:10:
null_optional_arguments 必须是数组
test arguments_diagnostics_records_null_optionals_and_stays_successful ... FAILED

failures:

failures:
    arguments_diagnostics_flags_unknown_names_without_rejecting
    arguments_diagnostics_marks_camel_case_alias_not_typo
    arguments_diagnostics_never_leaks_values
    arguments_diagnostics_records_null_optionals_and_stays_successful

test result: FAILED. 1 passed; 4 failed; 0 ignored; 0 measured; 78 filtered out; finished in 0.62s

error: test failed, to rerun pass `-p aw-mcp-bridge --test mcp_contract`
```

反向用例（无多余参数）在实现前就是绿的：它证明的是"实现后也不产生噪声"，不是新增能力的红。

绿（同一批用例，恢复实现后）：

```text
running 5 tests
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 78 filtered out; finished in 0.63s
```

全量：`cargo test -p aw-mcp-bridge` → Windows 10 unit + 83 integration，0 failed。

### 24.4 真实 stdio 双探针（原样回执）

同一个工具 `runs.list`，一次正确参数 `{"limit":5}`，一次故意拼错 `{"lmtt":5}`；直接向 `target/debug/aw-mcp-bridge.exe` 的 stdio 各写一行 JSON-RPC，stdout/stderr 分开捕获。探针只调用只读 `runs.list`，未写任何 runs 数据。

正确参数（stdout；stderr 为空）：

```json
{"id":1,"jsonrpc":"2.0","result":{"content":[{"text":"{\"limit\":5,\"offset\":0,\"state\":\"ready\",\"tasks\":[{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":null,\"env_keys\":[\"PM_E2E_SLEEP_MS\",\"PM_E2E_TAG\",\"PM_E2E_WORK\"],\"exit_code\":null,\"handback\":false,\"latest_attempt\":1,\"status\":null,\"task_id\":\"pm-conc-a-20261105T195253Z\"},{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":2300,\"env_keys\":[\"PM_E2E_SLEEP_MS\",\"PM_E2E_TAG\",\"PM_E2E_WORK\"],\"exit_code\":0,\"handback\":true,\"latest_attempt\":1,\"status\":\"ok\",\"task_id\":\"pm-conc-a-20261105T195328Z\"},{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":2317,\"env_keys\":[\"PM_E2E_SLEEP_MS\",\"PM_E2E_TAG\",\"PM_E2E_WORK\"],\"exit_code\":0,\"handback\":true,\"latest_attempt\":1,\"status\":\"ok\",\"task_id\":\"pm-conc-a-20261105T195355Z\"},{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":null,\"env_keys\":[\"PM_E2E_NOTE\",\"PM_E2E_SLEEP_MS\",\"PM_E2E_TAG\",\"PM_E2E_WORK\"],\"exit_code\":null,\"handback\":false,\"latest_attempt\":1,\"status\":null,\"task_id\":\"pm-conc-b-20261105T195253Z\"},{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":4311,\"env_keys\":[\"PM_E2E_NOTE\",\"PM_E2E_SLEEP_MS\",\"PM_E2E_TAG\",\"PM_E2E_WORK\"],\"exit_code\":0,\"handback\":true,\"latest_attempt\":1,\"status\":\"ok\",\"task_id\":\"pm-conc-b-20261105T195328Z\"}],\"total\":13,\"total_tasks\":13}","type":"text"}],"isError":false}}
```

故意拼错 `lmtt`（stdout；`isError:false`，调用照常成功，`limit` 回落到默认 20；`tasks` 因默认 limit 不同比上一次多）：

```json
{"id":2,"jsonrpc":"2.0","result":{"content":[{"text":"{\"arguments_diagnostics\":{\"declared_arguments\":[\"limit\",\"offset\"],\"note\":\"只记录参数名与值类型/长度，不记录值；诊断不改变调用的成功/失败结果\",\"null_optional_arguments\":[],\"tool\":\"runs.list\",\"unknown_arguments\":[{\"kind\":\"unknown\",\"name\":\"lmtt\",\"value\":{\"length\":null,\"type\":\"number\"}}]},\"limit\":20,\"offset\":0,\"state\":\"ready\",\"tasks\":[{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":null,\"env_keys\":[\"PM_E2E_SLEEP_MS\",\"PM_E2E_TAG\",\"PM_E2E_WORK\"],\"exit_code\":null,\"handback\":false,\"latest_attempt\":1,\"status\":null,\"task_id\":\"pm-conc-a-20261105T195253Z\"},{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":2300,\"env_keys\":[\"PM_E2E_SLEEP_MS\",\"PM_E2E_TAG\",\"PM_E2E_WORK\"],\"exit_code\":0,\"handback\":true,\"latest_attempt\":1,\"status\":\"ok\",\"task_id\":\"pm-conc-a-20261105T195328Z\"},{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":2317,\"env_keys\":[\"PM_E2E_SLEEP_MS\",\"PM_E2E_TAG\",\"PM_E2E_WORK\"],\"exit_code\":0,\"handback\":true,\"latest_attempt\":1,\"status\":\"ok\",\"task_id\":\"pm-conc-a-20261105T195355Z\"},{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":null,\"env_keys\":[\"PM_E2E_NOTE\",\"PM_E2E_SLEEP_MS\",\"PM_E2E_TAG\",\"PM_E2E_WORK\"],\"exit_code\":null,\"handback\":false,\"latest_attempt\":1,\"status\":null,\"task_id\":\"pm-conc-b-20261105T195253Z\"},{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":4311,\"env_keys\":[\"PM_E2E_NOTE\",\"PM_E2E_SLEEP_MS\",\"PM_E2E_TAG\",\"PM_E2E_WORK\"],\"exit_code\":0,\"handback\":true,\"latest_attempt\":1,\"status\":\"ok\",\"task_id\":\"pm-conc-b-20261105T195328Z\"},{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":4304,\"env_keys\":[\"PM_E2E_NOTE\",\"PM_E2E_SLEEP_MS\",\"PM_E2E_TAG\",\"PM_E2E_WORK\"],\"exit_code\":0,\"handback\":true,\"latest_attempt\":1,\"status\":\"ok\",\"task_id\":\"pm-conc-b-20261105T195355Z\"},{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":2258,\"env_keys\":[\"PM_E2E_WORK\"],\"exit_code\":0,\"handback\":true,\"latest_attempt\":1,\"status\":\"ok\",\"task_id\":\"pm-conc-same-20261105T195355Z\"},{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":109,\"env_keys\":[],\"exit_code\":0,\"handback\":true,\"latest_attempt\":1,\"status\":\"ok\",\"task_id\":\"pm-e2e-20261105T185851Z\"},{\"attempts\":1,\"blocked_reason\":null,\"duration_ms\":373,\"env_keys\":[\"PM_E2E_PID_DIR\"],\"exit_code\":1,\"handback\":true,\"latest_attempt\":1,\"status\":\"failed\",\"task_id\":\"pm-timeout-20261105T192252Z\"},{\"attempts\":1,\"blocked_reason\":\"timeout：超过 3s 未结束，已终止进程\",\"duration_ms\":3018,\"env_keys\":[\"PM_E2E_PID_DIR\"],\"exit_code\":null,\"handback\":true,\"latest_attempt\":1,\"status\":\"blocked\",\"task_id\":\"pm-timeout-20261105T192336Z\"},{\"attempts\":1,\"blocked_reason\":\"timeout：超过 3s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证；Windows 下 spawn→Assign 窗口内派生的进程可能未被纳入作业\",\"duration_ms\":3012,\"env_keys\":[\"PM_E2E_PID_DIR\"],\"exit_code\":null,\"handback\":true,\"latest_attempt\":1,\"status\":\"blocked\",\"task_id\":\"pm-timeout-20261105T220037Z\"},{\"attempts\":0,\"blocked_reason\":null,\"duration_ms\":null,\"env_keys\":[],\"exit_code\":null,\"handback\":false,\"latest_attempt\":null,\"status\":null,\"task_id\":\"smoke-1\"},{\"attempts\":2,\"blocked_reason\":null,\"duration_ms\":55,\"env_keys\":[],\"exit_code\":0,\"handback\":true,\"latest_attempt\":2,\"status\":\"ok\",\"task_id\":\"smoke-2\"}],\"total\":13,\"total_tasks\":13}","type":"text"}],"isError":false}}
```

对应 stderr（单行 JSON，与回执内字段同源）：

```text
[aw-mcp-bridge] arguments_diagnostics {"declared_arguments":["limit","offset"],"note":"只记录参数名与值类型/长度，不记录值；诊断不改变调用的成功/失败结果","null_optional_arguments":[],"tool":"runs.list","unknown_arguments":[{"kind":"unknown","name":"lmtt","value":{"length":null,"type":"number"}}]}
```

### 24.5 未覆盖与限制（如实标注）

- 别名识别是**规范化匹配**（忽略大小写与 `_`/`-`），不是编辑距离：`timeout_sec`（少一个 s）会被判 `unknown` 而不是 `alias`；`declared_arguments` 足以让调用方自行对照，但本版不提供"最接近参数名"建议。
- 只对 `arguments` 为 JSON 对象的调用做诊断；非对象 `arguments` 不新增诊断，也不改变原有行为。
- 诊断只写 stderr 与回执，不落 `runs/` 审计文件；当前没有"跨调用汇总诊断"的持久化。
- 本次只在 Windows 实跑；POSIX 未复跑（本改动不涉及平台分支），未验证项不写成已验证。

## 25. v1.26：运行模式显式化与独立判定（`dispatched` / `standalone`）

### 25.1 契约与实现要点

- `task.execute` 新增可选 `mode`：缺省 `dispatched`（PM 派单，**行为零变化**）；`standalone` 必须显式。
- standalone 必填 `cwd` + `argv` + 非空 `acceptance`，拒绝 `cmd`/`env`/`path_prepend`；执行不经 shell、限定 cwd。
- 判定逐 attempt 落盘 `runs/<task_id>/attempt-N/verdict.json`：`criteria[].spec/observed/passed`、`observations{exit_code,stdout_digest,stderr_digest,stdout_preview,changed_files,...}`、`missing[]`、`decision{action,reason,last_failure}`；结论三态 `passed|failed|undetermined`，`undetermined` 绝不默认通过且不重试。
- `max_attempts` 1..5（默认 3）：仅 `failed` 且未耗尽时重试；每次 attempt 独立目录、不覆盖；耗尽 `retry_exhausted`。
- 同 `task_id` 跨模式 → `mode_conflict`，零写入。dispatched 回执不新增 `mode/verdict_*/attempts_total`，`dispatch.json` 不写 `mode`。
- `tools/list` 的 `task.execute` 描述与 schema（`mode`/`argv`/`acceptance`/`max_attempts`、`required=["task_id"]` + `anyOf`）同步更新；门禁新增 G12–G14，`tools_sha256=26481e12…`。

### 25.2 先红：实现前的原始失败输出

在当前代码（HEAD = v1.25）上，先只提交测试文件、把 `src/server.rs` 暂存回 HEAD 后运行；以下为原始输出（未修改断言、未放宽）。

7 条 standalone 用例：

```text
running 7 tests
test standalone_and_dispatched_cannot_share_one_task_id ... FAILED
test standalone_failure_retries_without_overwriting_records ... FAILED
test standalone_passes_and_persists_verdict_with_observations ... FAILED
test standalone_rejects_shell_fields_and_out_of_scope_paths ... FAILED
test standalone_replay_returns_final_result_without_new_attempts ... FAILED
test standalone_requires_cwd_argv_and_acceptance_before_claiming ... FAILED
test standalone_undetermined_is_never_reported_as_pass ... FAILED

---- standalone_and_dispatched_cannot_share_one_task_id stdout ----
thread 'standalone_and_dispatched_cannot_share_one_task_id' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4634:5:
assertion `left == right` failed: 同一目录不得既被 PM 当权威又被 AW 自判：{"...","blocked_reason":"command_not_found: 缺少 cmd（必填字符串）","status":"blocked",...}
  left: Bool(false)
 right: true

---- standalone_failure_retries_without_overwriting_records stdout ----
thread 'standalone_failure_retries_without_overwriting_records' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4298:5:
assertion `left == right` failed: {"...","blocked_reason":"command_not_found: 缺少 cmd（必填字符串）","status":"blocked",...}
  left: String("blocked")
 right: "failed"

---- standalone_passes_and_persists_verdict_with_observations stdout ----
thread 'standalone_passes_and_persists_verdict_with_observations' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4214:5:
assertion `left == right` failed: {"...","blocked_reason":"command_not_found: 缺少 cmd（必填字符串）","status":"blocked",...}
  left: String("blocked")
 right: "ok"

---- standalone_rejects_shell_fields_and_out_of_scope_paths stdout ----
thread 'standalone_rejects_shell_fields_and_out_of_scope_paths' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4492:5:
assertion `left == right` failed
  left: String("ok")
 right: "blocked"

---- standalone_replay_returns_final_result_without_new_attempts stdout ----
thread 'standalone_replay_returns_final_result_without_new_attempts' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4571:5:
assertion `left == right` failed
  left: Null
 right: 1

---- standalone_requires_cwd_argv_and_acceptance_before_claiming stdout ----
thread 'standalone_requires_cwd_argv_and_acceptance_before_claiming' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4409:5:
assertion `left == right` failed
  left: Null
 right: "standalone"

---- standalone_undetermined_is_never_reported_as_pass stdout ----
thread 'standalone_undetermined_is_never_reported_as_pass' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4363:5:
assertion `left == right` failed
  left: Null
 right: "undetermined"

test result: FAILED. 0 passed; 7 failed; 0 ignored; 0 measured; 84 filtered out; finished in 0.73s
```

结构用例（schema 门禁同样先红）：

```text
thread 'schema_structure_declares_required_fields_and_bounds' panicked at apps\mcp-bridge\tests\mcp_contract.rs:222:5:
assertion `left == right` failed
  left: Array [String("task_id"), String("cmd")]
 right: Array [String("task_id")]

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 90 filtered out; finished in 0.00s
```

说明：`{...}` 是为可读性省略的同一回执其余字段；断言文字与 `left/right` 逐字保留。每个 standalone 调用前的 `arguments_diagnostics` stderr（`mode/argv/acceptance` 被当成未知参数）未重复粘贴。

### 25.3 转绿与全量回归

```text
running 7 tests
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 84 filtered out; finished in 1.67s

cargo test -p aw-mcp-bridge --lib            → 10 passed; 0 failed; 0 ignored
cargo test -p aw-mcp-bridge --test mcp_contract → 91 passed; 0 failed; 0 ignored
```

`dispatched_handback_stays_free_of_verdict_fields` 是回归锁：断言 dispatched 回执无 `mode/verdict_conclusion/verdict_locator/attempts_total/max_attempts`、attempt 目录无 `verdict.json`、`dispatch.json` 无 `mode`。

### 25.4 真实 stdio 双单（原样证据）

直接向 `target/debug/aw-mcp-bridge.exe` 的 stdio 写 JSON-RPC；工作目录为两个临时 git 仓库（`D:\Temp\aw-sa-ok-20261107` / `D:\Temp\aw-sa-fail-20261107`），非测试桩。

成功单 `standalone-ok-20261107`，`max_attempts=1`：`attempt-1/verdict.json`

```json
{
  "acceptance": [
    { "path": "out.txt", "type": "file_exists" },
    { "contains": "hello", "path": "out.txt", "type": "file_content_match" }
  ],
  "attempt": 1,
  "criteria": [
    { "index": 1, "observed": { "bytes": 7, "exists": true, "path": "out.txt" }, "passed": true, "spec": { "path": "out.txt", "type": "file_exists" } },
    { "index": 2, "observed": { "bytes": 7, "contains": "hello", "exists": true, "matched": true, "path": "out.txt" }, "passed": true, "spec": { "contains": "hello", "path": "out.txt", "type": "file_content_match" } }
  ],
  "decided_at": "2026-11-06T17:32:08Z",
  "decision": { "action": "terminate", "last_failure": null, "reason": "acceptance_met" },
  "missing": [],
  "mode": "standalone",
  "observations": {
    "changed_files": ["out.txt"],
    "changed_files_truncated": false,
    "duration_ms": 520,
    "exit_code": 0,
    "spawn_failed": null,
    "stderr_bytes": 0,
    "stderr_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "stderr_preview": "",
    "stderr_truncated": false,
    "stdout_bytes": 0,
    "stdout_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "stdout_preview": "",
    "stdout_truncated": false,
    "timed_out": false
  },
  "task_id": "standalone-ok-20261107",
  "verdict": "passed",
  "version": "standalone-verdict/1"
}
```

成功单回执 `attempt-1/handback.json`：

```json
{
  "argv": ["cmd.exe", "/C", "echo hello>out.txt"],
  "artifacts": [
    { "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "kind": "stdout", "locator": "runs/standalone-ok-20261107/attempt-1/stdout.txt" },
    { "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "kind": "stderr", "locator": "runs/standalone-ok-20261107/attempt-1/stderr.txt" },
    { "hash": "sha256:33dc97af8fc129a578aca1dbbc618810008a254b3f1e6eb2b6713eea6bde3f04", "kind": "verdict", "locator": "runs/standalone-ok-20261107/attempt-1/verdict.json" }
  ],
  "attempt": 1,
  "attempts_total": 1,
  "blocked_reason": null,
  "changed_files": ["out.txt"],
  "changed_files_truncated": false,
  "commit": null,
  "cwd": "\\\\?\\D:\\Temp\\aw-sa-ok-20261107",
  "duration_ms": 520,
  "env_keys": [],
  "exit_code": 0,
  "finishedAt": "2026-11-06T17:32:08Z",
  "max_attempts": 1,
  "mode": "standalone",
  "path_prepend": [],
  "status": "ok",
  "stderr_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "stdout_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "task_id": "standalone-ok-20261107",
  "timeout_secs": 120,
  "verdict_conclusion": "passed",
  "verdict_locator": "runs/standalone-ok-20261107/attempt-1/verdict.json",
  "verify_cmd": "cmd.exe /C echo hello>out.txt"
}
```

失败单 `standalone-fail-20261107`，`max_attempts=2`，验收要求文件含 `EXPECTED`、程序写入 `WRONG`。

`attempt-1/verdict.json`（失败 → 重试）：

```json
{
  "acceptance": [{ "contains": "EXPECTED", "path": "out.txt", "type": "file_content_match" }],
  "attempt": 1,
  "criteria": [
    { "index": 1, "observed": { "bytes": 7, "contains": "EXPECTED", "exists": true, "matched": false, "path": "out.txt" }, "passed": false, "spec": { "contains": "EXPECTED", "path": "out.txt", "type": "file_content_match" } }
  ],
  "decided_at": "2026-11-06T17:32:08Z",
  "decision": { "action": "retry", "last_failure": "acceptance_not_met", "reason": "acceptance_not_met" },
  "missing": [],
  "mode": "standalone",
  "observations": {
    "changed_files": ["out.txt"],
    "changed_files_truncated": false,
    "duration_ms": 397,
    "exit_code": 0,
    "spawn_failed": null,
    "stderr_bytes": 0,
    "stderr_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "stderr_preview": "",
    "stderr_truncated": false,
    "stdout_bytes": 0,
    "stdout_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "stdout_preview": "",
    "stdout_truncated": false,
    "timed_out": false
  },
  "task_id": "standalone-fail-20261107",
  "verdict": "failed",
  "version": "standalone-verdict/1"
}
```

`attempt-2/verdict.json`（耗尽 → 终止）：

```json
{
  "acceptance": [{ "contains": "EXPECTED", "path": "out.txt", "type": "file_content_match" }],
  "attempt": 2,
  "criteria": [
    { "index": 1, "observed": { "bytes": 7, "contains": "EXPECTED", "exists": true, "matched": false, "path": "out.txt" }, "passed": false, "spec": { "contains": "EXPECTED", "path": "out.txt", "type": "file_content_match" } }
  ],
  "decided_at": "2026-11-06T17:32:09Z",
  "decision": { "action": "terminate", "last_failure": "acceptance_not_met", "reason": "retry_exhausted" },
  "missing": [],
  "mode": "standalone",
  "observations": {
    "changed_files": [],
    "changed_files_truncated": false,
    "duration_ms": 386,
    "exit_code": 0,
    "spawn_failed": null,
    "stderr_bytes": 0,
    "stderr_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "stderr_preview": "",
    "stderr_truncated": false,
    "stdout_bytes": 0,
    "stdout_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "stdout_preview": "",
    "stdout_truncated": false,
    "timed_out": false
  },
  "task_id": "standalone-fail-20261107",
  "verdict": "failed",
  "version": "standalone-verdict/1"
}
```

失败单最终回执 `attempt-2/handback.json`：

```json
{
  "argv": ["cmd.exe", "/C", "echo WRONG>out.txt"],
  "artifacts": [
    { "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "kind": "stdout", "locator": "runs/standalone-fail-20261107/attempt-2/stdout.txt" },
    { "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "kind": "stderr", "locator": "runs/standalone-fail-20261107/attempt-2/stderr.txt" },
    { "hash": "sha256:7df3f21883a119f43e878e4dd043a22cadbf20c37ef1405e2ab11a4594132214", "kind": "verdict", "locator": "runs/standalone-fail-20261107/attempt-2/verdict.json" }
  ],
  "attempt": 2,
  "attempts_total": 2,
  "blocked_reason": null,
  "changed_files": [],
  "changed_files_truncated": false,
  "commit": null,
  "cwd": "\\\\?\\D:\\Temp\\aw-sa-fail-20261107",
  "duration_ms": 386,
  "env_keys": [],
  "exit_code": 0,
  "finishedAt": "2026-11-06T17:32:09Z",
  "max_attempts": 2,
  "mode": "standalone",
  "path_prepend": [],
  "status": "failed",
  "stderr_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "stdout_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "task_id": "standalone-fail-20261107",
  "timeout_secs": 120,
  "verdict_conclusion": "failed",
  "verdict_locator": "runs/standalone-fail-20261107/attempt-2/verdict.json",
  "verify_cmd": "cmd.exe /C echo WRONG>out.txt"
}
```

`attempt-1/handback.json` 同样存在（`status=failed`、`verdict_locator=runs/standalone-fail-20261107/attempt-1/verdict.json`、无 `attempts_total`），证明重试没有覆盖上一次记录。

### 25.5 干净重建后的重跑（本次最终验证）

去除 fmt 噪声、按 HEAD 重新应用语义编辑并 `cargo build -p aw-mcp-bridge` 之后，再次真跑两个独立任务（全新 task_id 与目录，非测试桩、非回放）：

- 成功单 `standalone-ok-b`：cwd `D:\Temp\aw-sa-ok-b`（临时 git 仓库），`max_attempts=1`。
- 失败单 `standalone-fail-b`：cwd `D:\Temp\aw-sa-fail-b`（临时 git 仓库），`max_attempts=2`，验收要求文件含 `EXPECTED`、程序写 `WRONG`。

成功单 `attempt-1/verdict.json`：

```json
{
  "acceptance": [
    { "path": "out.txt", "type": "file_exists" },
    { "contains": "hello", "path": "out.txt", "type": "file_content_match" }
  ],
  "attempt": 1,
  "criteria": [
    { "index": 1, "observed": { "bytes": 7, "exists": true, "path": "out.txt" }, "passed": true, "spec": { "path": "out.txt", "type": "file_exists" } },
    { "index": 2, "observed": { "bytes": 7, "contains": "hello", "exists": true, "matched": true, "path": "out.txt" }, "passed": true, "spec": { "contains": "hello", "path": "out.txt", "type": "file_content_match" } }
  ],
  "decided_at": "2026-11-06T17:51:45Z",
  "decision": { "action": "terminate", "last_failure": null, "reason": "acceptance_met" },
  "missing": [],
  "mode": "standalone",
  "observations": {
    "changed_files": ["out.txt"],
    "changed_files_truncated": false,
    "duration_ms": 226,
    "exit_code": 0,
    "spawn_failed": null,
    "stderr_bytes": 0,
    "stderr_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "stderr_preview": "",
    "stderr_truncated": false,
    "stdout_bytes": 0,
    "stdout_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "stdout_preview": "",
    "stdout_truncated": false,
    "timed_out": false
  },
  "task_id": "standalone-ok-b",
  "verdict": "passed",
  "version": "standalone-verdict/1"
}
```

成功单 `attempt-1/handback.json`：

```json
{
  "argv": ["cmd.exe", "/C", "echo hello>out.txt"],
  "artifacts": [
    { "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "kind": "stdout", "locator": "runs/standalone-ok-b/attempt-1/stdout.txt" },
    { "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "kind": "stderr", "locator": "runs/standalone-ok-b/attempt-1/stderr.txt" },
    { "hash": "sha256:a4b1ba0c80fb9e8a2423c173562359e1f68fffc1a0c82f0bd095714ad93982b6", "kind": "verdict", "locator": "runs/standalone-ok-b/attempt-1/verdict.json" }
  ],
  "attempt": 1,
  "attempts_total": 1,
  "blocked_reason": null,
  "changed_files": ["out.txt"],
  "changed_files_truncated": false,
  "commit": null,
  "cwd": "\\\\?\\D:\\Temp\\aw-sa-ok-b",
  "duration_ms": 226,
  "env_keys": [],
  "exit_code": 0,
  "finishedAt": "2026-11-06T17:51:45Z",
  "max_attempts": 1,
  "mode": "standalone",
  "path_prepend": [],
  "status": "ok",
  "stderr_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "stdout_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "task_id": "standalone-ok-b",
  "timeout_secs": 120,
  "verdict_conclusion": "passed",
  "verdict_locator": "runs/standalone-ok-b/attempt-1/verdict.json",
  "verify_cmd": "cmd.exe /C echo hello>out.txt"
}
```

失败单 `attempt-1/verdict.json`（不通过 → 重试）：

```json
{
  "acceptance": [{ "contains": "EXPECTED", "path": "out.txt", "type": "file_content_match" }],
  "attempt": 1,
  "criteria": [
    { "index": 1, "observed": { "bytes": 7, "contains": "EXPECTED", "exists": true, "matched": false, "path": "out.txt" }, "passed": false, "spec": { "contains": "EXPECTED", "path": "out.txt", "type": "file_content_match" } }
  ],
  "decided_at": "2026-11-06T17:51:50Z",
  "decision": { "action": "retry", "last_failure": "acceptance_not_met", "reason": "acceptance_not_met" },
  "missing": [],
  "mode": "standalone",
  "observations": {
    "changed_files": ["out.txt"],
    "changed_files_truncated": false,
    "duration_ms": 183,
    "exit_code": 0,
    "spawn_failed": null,
    "stderr_bytes": 0,
    "stderr_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "stderr_preview": "",
    "stderr_truncated": false,
    "stdout_bytes": 0,
    "stdout_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "stdout_preview": "",
    "stdout_truncated": false,
    "timed_out": false
  },
  "task_id": "standalone-fail-b",
  "verdict": "failed",
  "version": "standalone-verdict/1"
}
```

`attempt-1/handback.json`（`status=failed`、无 `attempts_total`，证明中间尝试未被覆盖）：

```json
{
  "argv": ["cmd.exe", "/C", "echo WRONG>out.txt"],
  "artifacts": [
    { "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "kind": "stdout", "locator": "runs/standalone-fail-b/attempt-1/stdout.txt" },
    { "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "kind": "stderr", "locator": "runs/standalone-fail-b/attempt-1/stderr.txt" },
    { "hash": "sha256:ca9bc840456fe43f110431329cff46f85c69a46f1724a114baf8b1cc1a7b97c8", "kind": "verdict", "locator": "runs/standalone-fail-b/attempt-1/verdict.json" }
  ],
  "attempt": 1,
  "blocked_reason": null,
  "changed_files": ["out.txt"],
  "changed_files_truncated": false,
  "commit": null,
  "cwd": "\\\\?\\D:\\Temp\\aw-sa-fail-b",
  "duration_ms": 183,
  "env_keys": [],
  "exit_code": 0,
  "finishedAt": "2026-11-06T17:51:50Z",
  "max_attempts": 2,
  "mode": "standalone",
  "path_prepend": [],
  "status": "failed",
  "stderr_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "stdout_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "task_id": "standalone-fail-b",
  "timeout_secs": 120,
  "verdict_conclusion": "failed",
  "verdict_locator": "runs/standalone-fail-b/attempt-1/verdict.json",
  "verify_cmd": "cmd.exe /C echo WRONG>out.txt"
}
```

失败单 `attempt-2/verdict.json`（耗尽 → 终止）：

```json
{
  "acceptance": [{ "contains": "EXPECTED", "path": "out.txt", "type": "file_content_match" }],
  "attempt": 2,
  "criteria": [
    { "index": 1, "observed": { "bytes": 7, "contains": "EXPECTED", "exists": true, "matched": false, "path": "out.txt" }, "passed": false, "spec": { "contains": "EXPECTED", "path": "out.txt", "type": "file_content_match" } }
  ],
  "decided_at": "2026-11-06T17:51:51Z",
  "decision": { "action": "terminate", "last_failure": "acceptance_not_met", "reason": "retry_exhausted" },
  "missing": [],
  "mode": "standalone",
  "observations": {
    "changed_files": [],
    "changed_files_truncated": false,
    "duration_ms": 187,
    "exit_code": 0,
    "spawn_failed": null,
    "stderr_bytes": 0,
    "stderr_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "stderr_preview": "",
    "stderr_truncated": false,
    "stdout_bytes": 0,
    "stdout_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "stdout_preview": "",
    "stdout_truncated": false,
    "timed_out": false
  },
  "task_id": "standalone-fail-b",
  "verdict": "failed",
  "version": "standalone-verdict/1"
}
```

失败单最终回执 `attempt-2/handback.json`：

```json
{
  "argv": ["cmd.exe", "/C", "echo WRONG>out.txt"],
  "artifacts": [
    { "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "kind": "stdout", "locator": "runs/standalone-fail-b/attempt-2/stdout.txt" },
    { "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "kind": "stderr", "locator": "runs/standalone-fail-b/attempt-2/stderr.txt" },
    { "hash": "sha256:eb21d94faab0784f1eb9ed77e1e77a9c0345049957d6923239251dbb666ab34b", "kind": "verdict", "locator": "runs/standalone-fail-b/attempt-2/verdict.json" }
  ],
  "attempt": 2,
  "attempts_total": 2,
  "blocked_reason": null,
  "changed_files": [],
  "changed_files_truncated": false,
  "commit": null,
  "cwd": "\\\\?\\D:\\Temp\\aw-sa-fail-b",
  "duration_ms": 187,
  "env_keys": [],
  "exit_code": 0,
  "finishedAt": "2026-11-06T17:51:51Z",
  "max_attempts": 2,
  "mode": "standalone",
  "path_prepend": [],
  "status": "failed",
  "stderr_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "stdout_digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "task_id": "standalone-fail-b",
  "timeout_secs": 120,
  "verdict_conclusion": "failed",
  "verdict_locator": "runs/standalone-fail-b/attempt-2/verdict.json",
  "verify_cmd": "cmd.exe /C echo WRONG>out.txt"
}
```

### 25.6 未覆盖与限制（如实标注）

- 本次只在 Windows 10 实跑与回归；POSIX 分支未复跑（standalone 执行路径无平台特定代码，但未验证不写成已验证）。
- 演示仓库为空仓（无 HEAD），`commit=null`；`changed_files` 仍按 `git status --porcelain` 差值正确抓到 `out.txt`。
- `git_diff_non_empty` 以 `git status --porcelain` 判定（含未跟踪文件）；非 git 仓库 → `undetermined`，已有用例覆盖，但**未在真实双单中演示**。
- 失败单 `attempt-2` 的 `changed_files` 为空：第 1 次已创建 `out.txt`，第 2 次是改写同一文件，按"新增文件差值"口径不重复计入；这是既有 `changed_files` 语义，不是本次新增缺陷。
- 未实现：跨调用的判定汇总、`acceptance` 的表达式/断言组合、对 `argv` 参数中的路径做逐项作用域校验（当前只校验 `argv[0]` 显式路径与验收路径）。

## 26. v1.27：`runs.prune` 按条目精确排除（`exclude`）

### 26.1 能力位与 schema

`tools/list` 的 `runs.prune.inputSchema.properties.exclude` 是本轮唯一能力位。形状：

```json
{
  "type": "array",
  "minItems": 1,
  "maxItems": 100,
  "uniqueItems": true,
  "items": {
    "type": "object",
    "properties": {
      "task_id": { "type": "string", "minLength": 1, "maxLength": 64 },
      "attempt": { "type": "integer", "minimum": 1 }
    },
    "required": ["task_id"],
    "additionalProperties": false
  }
}
```

`tools_sha256` 由 `26481e124bbf5326df50b354b55121d7b3bd459fee89a251cf0fbb43cf758595` 变为 `f380e5ada343ce3a1fd6d7d4d39242d4f6e1ac2f18f3f774be10c5766990e6e0`；§21 新增 G15 行（`exclude.description` 的 contains/not_contains）并把 G3、G5 措辞同步为"按条目精确排除 / 在 status 过滤与 exclude 排除之后生效"。

### 26.2 实现口径（写死）

- 排除在 `prune_candidates` 的**候选枚举入口**一次完成：合格条目命中排除后只进 `excluded[]`，不进 `candidates[]`、不占 `limit`；`eligible_total` 仍把其计入 `candidates_total`。
- 解析阶段 fail-closed：非数组、空数组、>100 条、条目不对象、缺 `task_id`、`attempt` 非 `>=1` 整数、重复 `(task_id, attempt)`、同一 `task_id` 整任务与单次混用，全部 `invalid_exclusion`，解析发生在任何写入之前（零删除、零审计）。
- `task_id` 走与任务目录相同的 `is_task_dir_name`：拒路径分隔符、`..`、绝对路径、通配符、`_`/`.` 前缀与超 64 字符。
- 计数：`excluded_total` 只数 `reason=excluded_by_request`；`not_in_candidates_total` 数 `reason=not_in_candidates`；`exclude_requests_total = 两者之和`；`batch_candidates_total = deleted+failed+trashed+excluded_total`；`not_processed_total = candidates_total - batch_candidates_total`。
- 边界选择：排除不存在或不在候选集的 id **不报错**，逐条记 `not_in_candidates`、`path=null`、`isError=false`。

### 26.3 先红：当前代码上的原始失败（断言未放宽）

命令：`cargo test -p aw-mcp-bridge --test mcp_contract prune_exclude`（server.rs 临时回到 HEAD `c842b6a`，测试保留）。

```text
running 5 tests
test prune_exclude_schema_declares_capability_and_constraints ... FAILED
test prune_exclude_rejects_unsafe_or_ambiguous_entries_without_writes ... FAILED
test prune_exclude_reports_not_in_candidates_and_keeps_processing ... FAILED
test prune_exclude_one_candidate_diverts_it_and_keeps_bytes ... FAILED
test prune_exclude_all_candidates_yields_zero_deletes ... FAILED

---- prune_exclude_schema_declares_capability_and_constraints stdout ----
thread 'prune_exclude_schema_declares_capability_and_constraints' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4756:5:
assertion `left == right` failed: exclude 必须是数组：{...旧 schema，properties 只有 confirm/dry_run/limit/older_than_days/status...}
  left: Null
 right: "array"

---- prune_exclude_rejects_unsafe_or_ambiguous_entries_without_writes stdout ----
[aw-mcp-bridge] arguments_diagnostics {"declared_arguments":["confirm","dry_run","limit","older_than_days","status"],...,"tool":"runs.prune","unknown_arguments":[{"kind":"unknown","name":"exclude","value":{"length":1,"type":"array"}}]}
thread 'prune_exclude_rejects_unsafe_or_ambiguous_entries_without_writes' panicked at apps\mcp-bridge\tests\mcp_contract.rs:2862:5:
assertion `left == right` failed: 该形态必须拒绝：{... "candidates_total":1, "dry_run":true ...}
  left: Bool(false)
 right: true

---- prune_exclude_reports_not_in_candidates_and_keeps_processing stdout ----
[aw-mcp-bridge] arguments_diagnostics {... "unknown_arguments":[{"kind":"unknown","name":"exclude","value":{"length":3,"type":"array"}}]}
thread 'prune_exclude_reports_not_in_candidates_and_keeps_processing' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4970:5:
assertion `left == right` failed: 没有候选被请求排除
  left: Null
 right: 0

---- prune_exclude_one_candidate_diverts_it_and_keeps_bytes stdout ----
thread 'prune_exclude_one_candidate_diverts_it_and_keeps_bytes' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4817:5:
assertion `left == right` failed
  left: Null
 right: 1

---- prune_exclude_all_candidates_yields_zero_deletes stdout ----
thread 'prune_exclude_all_candidates_yields_zero_deletes' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4915:5:
assertion `left == right` failed
  left: Number(3)
 right: 0

test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 92 filtered out
```

无参数回归用例同样先红（新字段尚不存在）：

```text
test prune_without_exclude_keeps_legacy_behavior_and_invariant ... FAILED
thread '...' panicked at apps\mcp-bridge\tests\mcp_contract.rs:5104:5:
assertion `left == right` failed
  left: Null
 right: 0
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 96 filtered out
```

### 26.4 转绿

`cargo test -p aw-mcp-bridge`：`10 passed`（lib）+ `97 passed; 0 failed`（integration），其中 21 条 prune 用例（含 6 条新增：schema、排除单条、全排除、`not_in_candidates`、越界/歧义拒绝、无参数回归）。

### 26.5 真跑三次（stdio 直连 `target\debug\aw-mcp-bridge.exe`，原始回执）

夹具：`runs/r-a|r-b|r-c` 各有 `attempt-1`（finishedAt 40 天前）与 `attempt-2`（最新，保留），即 3 条候选。

① 排除 1 条（`r-b/attempt-1`），真删：

```text
{"id":2,"jsonrpc":"2.0","result":{"content":[{"text":"{\"batch_candidates_total\":3,\"batch_id\":\"prune-1793988442636-120876\",\"candidates_total\":3,\"deleted\":[{\"age_days\":40,\"attempt\":1,...\"task_id\":\"r-a\"...},{\"age_days\":40,\"attempt\":1,...\"task_id\":\"r-c\"...}],\"deleted_total\":2,\"dry_run\":false,\"evaluated_at\":\"2026-11-06T18:07:22Z\",\"exclude_filter\":[{\"attempt\":1,\"task_id\":\"r-b\"}],\"exclude_requests_total\":1,\"excluded\":[{\"attempt\":1,\"path\":\"runs/r-b/attempt-1\",\"reason\":\"excluded_by_request\",\"task_id\":\"r-b\"}],\"excluded_total\":1,\"failed\":[],\"failed_total\":0,\"limit\":100,\"not_in_candidates_total\":0,\"not_processed_total\":0,\"older_than_days\":30,\"status\":\"ok\",\"status_filter\":null,\"trashed\":[],\"trashed_total\":0,\"truncated\":false}","type":"text"}],"isError":false}}
```

被排除条目逐文件 SHA256 前后一致（`BYTES_UNCHANGED: True`）：

```text
BEFORE: dispatch.json=2106C81B8DAC54C42F02AFF891C18C0E471A56B235AC6F10065D6AF267B9A9D5; handback.json=0104D90A55E82DD2057BA088EB496AFA4630D823FB5DE570A4D2EF4800549F7B; stderr.txt=B08F13CF26731249045B1C0B34213C0CC6226CEEFEA0F7B4D0FE3308B45FE66C; stdout.txt=09C8628F306839F90CE5BC2B0E7C7129DE33A1434E122869B6F083A383160B54
AFTER:  dispatch.json=2106C81B8DAC54C42F02AFF891C18C0E471A56B235AC6F10065D6AF267B9A9D5; handback.json=0104D90A55E82DD2057BA088EB496AFA4630D823FB5DE570A4D2EF4800549F7B; stderr.txt=B08F13CF26731249045B1C0B34213C0CC6226CEEFEA0F7B4D0FE3308B45FE66C; stdout.txt=09C8628F306839F90CE5BC2B0E7C7129DE33A1434E122869B6F083A383160B54
```

② 排除全部候选，真删：

```text
{"id":2,"jsonrpc":"2.0","result":{"content":[{"text":"{\"batch_candidates_total\":3,\"candidates_total\":3,\"deleted\":[],\"deleted_total\":0,\"dry_run\":false,\"evaluated_at\":\"2026-11-06T18:07:31Z\",\"exclude_filter\":[{\"attempt\":1,\"task_id\":\"r-a\"},{\"attempt\":1,\"task_id\":\"r-b\"},{\"attempt\":1,\"task_id\":\"r-c\"}],\"exclude_requests_total\":3,\"excluded\":[{\"attempt\":1,\"path\":\"runs/r-a/attempt-1\",\"reason\":\"excluded_by_request\",\"task_id\":\"r-a\"},{\"attempt\":1,\"path\":\"runs/r-b/attempt-1\",\"reason\":\"excluded_by_request\",\"task_id\":\"r-b\"},{\"attempt\":1,\"path\":\"runs/r-c/attempt-1\",\"reason\":\"excluded_by_request\",\"task_id\":\"r-c\"}],\"excluded_total\":3,\"failed\":[],\"failed_total\":0,\"limit\":100,\"not_in_candidates_total\":0,\"not_processed_total\":0,\"older_than_days\":30,\"status\":\"ok\",\"status_filter\":null,\"trashed\":[],\"trashed_total\":0,\"truncated\":false}","type":"text"}],"isError":false}}
prune.jsonl exists: False
r-a/attempt-1 exists: True
r-b/attempt-1 exists: True
r-c/attempt-1 exists: True
```

③ 排除不存在的 id（`r-missing/attempt-1`），真删：

```text
{"id":2,"jsonrpc":"2.0","result":{"content":[{"text":"{\"batch_candidates_total\":3,\"candidates_total\":3,\"deleted\":[{\"age_days\":40,\"attempt\":1,...\"task_id\":\"r-a\"...},{\"age_days\":40,\"attempt\":1,...\"task_id\":\"r-b\"...},{\"age_days\":40,\"attempt\":1,...\"task_id\":\"r-c\"...}],\"deleted_total\":3,\"dry_run\":false,\"evaluated_at\":\"2026-11-06T18:07:04Z\",\"exclude_filter\":[{\"attempt\":1,\"task_id\":\"r-missing\"}],\"exclude_requests_total\":1,\"excluded\":[{\"attempt\":1,\"path\":null,\"reason\":\"not_in_candidates\",\"task_id\":\"r-missing\"}],\"excluded_total\":0,\"failed\":[],\"failed_total\":0,\"limit\":100,\"not_in_candidates_total\":1,\"not_processed_total\":0,\"older_than_days\":30,\"status\":\"ok\",\"status_filter\":null,\"trashed\":[],\"trashed_total\":0,\"truncated\":false}","type":"text"}],"isError":false}}
```

三次都满足：`deleted_total + failed_total + trashed_total + excluded_total == batch_candidates_total`，且 `candidates_total == batch_candidates_total + not_processed_total`。

### 26.6 未覆盖与限制（如实标注）

- 未验证 POSIX 分支（本机 Windows 10；排除逻辑无平台专有代码，但未跑不写成已验证）。
- 未验证"排除与 `status` 白名单、`limit` 同时叠加"的真实磁盘场景（叠加逻辑有单测覆盖，未做真实夹具演示）。
- `excluded[]` 的 `path` 为相对路径字符串，不保证该目录此刻仍存在（排除是"不动它"，不是"验证它存在"）；`not_in_candidates` 一律 `path=null`。
- 未实现"排除请求与磁盘现状二次核对"（例如排除后目录被人为删除仍报 `excluded_by_request`）；该场景不在本轮范围。

## 27. v1.28：standalone `argv` 参数携带路径的作用域校验

### 27.1 能力位、识别范围与门禁

本轮不新增参数，只把 `task.execute` 的 `argv[1..]` 纳入作用域校验（契约 §2.6.8）。`tools/list` 的 `argv.description` 增加识别范围、原因码与回执字段；`tools_sha256` 由 `f380e5ada343ce3a1fd6d7d4d39242d4f6e1ac2f18f3f774be10c5766990e6e0` 变为 `919be60becbc358ae0335b17b7a38d3feb063e7acc4deb4f6312204167c52922`，§21 新增 G16 行。

识别规则与盲区逐条写死，摘要：

- 校验：独立参数值、`--flag=value`、`=`/`:` 分隔、一层配对引号、相对/`..`/`//`、绝对路径、盘符、UNC、`\\?\`/`\\.\` 逐字前缀、`%HH` 编码（最多两轮解码）、含空格路径。
- 不校验（明确盲区）：URL（含 `file://`）、含 `^ $ ( ) [ ] { } | * + ?` 的正则/通配符、无分隔符普通标量、破折号开关名、Windows 单字母斜杠开关（`/C`）、未展开的 `%NAME%`/`$NAME`/`${...}`/`~`（后者记 unknown）。
- 无法判定策略：**放行并记 `standalone_argv_path_unknown`**（回执/verdict 的 `argv_path_scan.unknown[]`），理由是不让 URL/正则/环境变量等合法参数被 fail-closed 误拒；未知必须可见，不写成"已验证安全"。
- 越界：`standalone_argv_path_escape`，在占号前拒绝（不建 attempt、不启动进程）。

### 27.2 实现口径（写死）

- 扫描入口在 `StandaloneSpec::parse`，位于 `cwd`/`argv`/`acceptance` 校验之后、`execute_standalone` 创建任务目录之前；`?` 直接转成 `status=blocked` 预检回执。
- 归一化按真实位置：相对路径从 `cwd` 逐组件前进，已存在组件即 `canonicalize`；符号链接按目标判断，指向界外立即拒绝；绝对路径先字面归一化证明是否可能落在 `cwd` 内，再用 `canonicalize` / 最深已存在祖先验证；跨卷/UNC 前缀不等即拒绝。
- Windows 组件比较忽略大小写，`\\?\`/`\\.\` 视同盘符前缀；`/C` 这类单字母开关显式排除。
- `argv_path_scan` 只出现在 standalone 回执与每次 `verdict.json`；dispatched 下 `argv` 仍直接 `dispatched_input_rejected`，既有回执字段与行为不变。

### 27.3 先红：HEAD `9402117` 上的原始失败（断言未放宽）

做法：`git worktree add --detach D:\Temp\aw-v128-red 9402117`，只把新用例 `tests/mcp_contract.rs` 放入工作树，`CARGO_TARGET_DIR` 复用现有 target 编译运行；跑完立即 `worktree remove --force`，当前源码哈希核对为 `158A28F2...930E8`。

命令：`cargo test -p aw-mcp-bridge --test mcp_contract standalone_argv_ -- --nocapture`

```text
running 3 tests

thread 'standalone_argv_scan_records_unknown_forms_instead_of_silently_passing' panicked at apps\mcp-bridge\tests\mcp_contract.rs:5336:28:
unknown 必须是数组：{"argv":["cmd.exe","/C","echo ok","%USERPROFILE%\\outside\\x"],"artifacts":[{"hash":"sha256:769094ef5d241d020cce5fcfdd88ae5faa0100dd1ba45668877c002114512720","kind":"stdout","locator":"runs/argv-unknown/attempt-1/stdout.txt"},{"hash":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","kind":"stderr","locator":"runs/argv-unknown/attempt-1/stderr.txt"},{"hash":"sha256:5c36fecd0749ca29357ca86134e3d67103300d5325a8dad1bff34507b203df9b","kind":"verdict","locator":"runs/argv-unknown/attempt-1/verdict.json"}],"attempt":1,"attempts_total":1,"blocked_reason":null,"changed_files":[],"changed_files_truncated":false,"commit":null,"cwd":"\\\\?\\D:\\Temp\\.tmpnXMewY","duration_ms":343,"env_keys":[],"exit_code":0,"finishedAt":"2026-11-06T18:37:34Z","max_attempts":1,"mode":"standalone","path_prepend":[],"status":"ok","stderr_digest":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","stdout_digest":"sha256:769094ef5d241d020cce5fcfdd88ae5faa0100dd1ba45668877c002114512720","task_id":"argv-unknown","timeout_secs":120,"verdict_conclusion":"passed","verdict_locator":"runs/argv-unknown/attempt-1/verdict.json","verify_cmd":"cmd.exe /C echo ok %USERPROFILE%\\outside\\x"}
test standalone_argv_scan_records_unknown_forms_instead_of_silently_passing ... FAILED

thread 'standalone_argv_scope_rejects_escapes_before_execution' panicked at apps\mcp-bridge\tests\mcp_contract.rs:5295:9:
assertion `left == right` failed: argv-dotdot 参数携带的越界路径必须在执行前拒绝：{"argv":["cmd.exe","/C","echo ok","--out=../escape.txt"],"artifacts":[{"hash":"sha256:b87aadb9dbbbf8e6c909452bbe2e4a7f9b7588e7d4ee06cb1a6a3233d6b0cec1","kind":"stdout","locator":"runs/argv-dotdot/attempt-1/stdout.txt"},{"hash":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","kind":"stderr","locator":"runs/argv-dotdot/attempt-1/stderr.txt"},{"hash":"sha256:208b062ff02a332bfb0f835c5e8a83290e2e54be3093f7a110f1e25c7fe62df6","kind":"verdict","locator":"runs/argv-dotdot/attempt-1/verdict.json"}],"attempt":1,"attempts_total":1,"blocked_reason":null,"changed_files":[],"changed_files_truncated":false,"commit":null,"cwd":"\\\\?\\D:\\Temp\\.tmpeyENOb","duration_ms":718,"env_keys":[],"exit_code":0,"finishedAt":"2026-11-06T18:37:34Z","max_attempts":1,"mode":"standalone","path_prepend":[],"status":"ok","stderr_digest":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","stdout_digest":"sha256:b87aadb9dbbbf8e6c909452bbe2e4a7f9b7588e7d4ee06cb1a6a3233d6b0cec1","task_id":"argv-dotdot","timeout_secs":120,"verdict_conclusion":"passed","verdict_locator":"runs/argv-dotdot/attempt-1/verdict.json","verify_cmd":"cmd.exe /C echo ok --out=../escape.txt"}
  left: String("ok")
 right: "blocked"
test standalone_argv_scope_rejects_escapes_before_execution ... FAILED

thread 'standalone_argv_scope_accepts_normal_arguments' panicked at apps\mcp-bridge\tests\mcp_contract.rs:5202:9:
assertion `left == right` failed: argv-name 扫描必须写明作用域：null
  left: Null
 right: String("\\\\?\\D:\\Temp\\.tmpv2PDwr")
test standalone_argv_scope_accepts_normal_arguments ... FAILED

failures:
    standalone_argv_scan_records_unknown_forms_instead_of_silently_passing
    standalone_argv_scope_accepts_normal_arguments
    standalone_argv_scope_rejects_escapes_before_execution

test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 97 filtered out; finished in 2.90s
```

红的关键点：`--out=../escape.txt` 在旧代码下**真的执行了**，`status=ok`、`exit_code=0`、`blocked_reason=null`。

### 27.4 转绿与回归

`cargo test -p aw-mcp-bridge`：`10 passed`（lib）+ `100 passed; 0 failed`（integration）。其中新增 3 条用例（8 正例、8 反例、1 unknown 形态），`dispatched_handback_stays_free_of_verdict_fields` 等既有 dispatched 用例全绿。

### 27.5 真跑 6+ 正例与 6 反例（stdio 直连 `target\debug\aw-mcp-bridge.exe`，原始回执）

夹具：`D:\Temp\aw-v128-dafd1256\work` 为 cwd；`outside` 为界外目录；`work\link` 是指向 `outside` 的符号链接。所有 standalone 调用的固定前缀为 `["cmd.exe","/C","echo ok"]`。

**正例（全部 `status=ok`、`exit_code=0`）**

`p1-name`（`--name=foo`）：

```text
{"argv":["cmd.exe","/C","echo ok","--name=foo"],"argv_path_scan":{"checked":[],"checked_total":0,"scope":"\\\\?\\D:\\Temp\\aw-v128-dafd1256\\work","unknown":[],"unknown_total":0},"attempt":1,"attempts_total":1,"blocked_reason":null,"exit_code":0,"mode":"standalone","status":"ok","task_id":"p1-name","verdict_conclusion":"passed"}
```

`p2-count`（`--count=3`）：

```text
{"argv":["cmd.exe","/C","echo ok","--count=3"],"argv_path_scan":{"checked":[],"checked_total":0,"scope":"\\\\?\\D:\\Temp\\aw-v128-dafd1256\\work","unknown":[],"unknown_total":0},"attempt":1,"attempts_total":1,"blocked_reason":null,"exit_code":0,"mode":"standalone","status":"ok","task_id":"p2-count","verdict_conclusion":"passed"}
```

`p3-space`（`--out=my dir\out.txt`）：按路径校验且 `in_scope`：

```text
{"argv":["cmd.exe","/C","echo ok","--out=my dir\\out.txt"],"argv_path_scan":{"checked":[{"class":"relative_path","decision":"in_scope","encoded":false,"index":4,"raw":"--out=my dir\\out.txt","resolved":"\\\\?\\D:\\Temp\\aw-v128-dafd1256\\work\\my dir\\out.txt","value":"my dir\\out.txt","via":"flag_equals"}],"checked_total":1,"scope":"\\\\?\\D:\\Temp\\aw-v128-dafd1256\\work","unknown":[],"unknown_total":0},"attempt":1,"attempts_total":1,"blocked_reason":null,"exit_code":0,"mode":"standalone","status":"ok","task_id":"p3-space","verdict_conclusion":"passed"}
```

`p4-separate`（`--out out.txt` 独立参数值）：

```text
{"argv":["cmd.exe","/C","echo ok","--out","out.txt"],"argv_path_scan":{"checked":[],"checked_total":0,"scope":"\\\\?\\D:\\Temp\\aw-v128-dafd1256\\work","unknown":[],"unknown_total":0},"attempt":1,"attempts_total":1,"blocked_reason":null,"exit_code":0,"mode":"standalone","status":"ok","task_id":"p4-separate","verdict_conclusion":"passed"}
```

`p5-url`（`--url=https://example.com/a/b`）：

```text
{"argv":["cmd.exe","/C","echo ok","--url=https://example.com/a/b"],"argv_path_scan":{"checked":[],"checked_total":0,"scope":"\\\\?\\D:\\Temp\\aw-v128-dafd1256\\work","unknown":[],"unknown_total":0},"attempt":1,"attempts_total":1,"blocked_reason":null,"exit_code":0,"mode":"standalone","status":"ok","task_id":"p5-url","verdict_conclusion":"passed"}
```

`p6-regex`（`--pattern=^hello$`）：

```text
{"argv":["cmd.exe","/C","echo ok","--pattern=^hello$"],"argv_path_scan":{"checked":[],"checked_total":0,"scope":"\\\\?\\D:\\Temp\\aw-v128-dafd1256\\work","unknown":[],"unknown_total":0},"attempt":1,"attempts_total":1,"blocked_reason":null,"exit_code":0,"mode":"standalone","status":"ok","task_id":"p6-regex","verdict_conclusion":"passed"}
```

`p7-abs-inside`（cwd 内绝对路径）：`resolved` 为已存在祖先（目标文件尚未创建）：

```text
{"argv":["cmd.exe","/C","echo ok","D:\\Temp\\aw-v128-dafd1256\\work\\out.txt"],"argv_path_scan":{"checked":[{"class":"windows_drive_path","decision":"in_scope","encoded":false,"index":4,"raw":"D:\\Temp\\aw-v128-dafd1256\\work\\out.txt","resolved":"D:\\Temp\\aw-v128-dafd1256\\work","value":"D:\\Temp\\aw-v128-dafd1256\\work\\out.txt","via":"bare"}],"checked_total":1,"scope":"\\\\?\\D:\\Temp\\aw-v128-dafd1256\\work","unknown":[],"unknown_total":0},"attempt":1,"attempts_total":1,"blocked_reason":null,"exit_code":0,"mode":"standalone","status":"ok","task_id":"p7-abs-inside","verdict_conclusion":"passed"}
```

**反例（全部 `status=blocked`、`attempt=null`、`artifacts=[]`、未创建任何运行目录）**

`n1-dotdot`（`--out=../escape.txt`）：

```text
{"artifacts":[],"attempt":null,"blocked_reason":"standalone_argv_path_escape: argv 第 4 项（--out=../escape.txt）解析后越出 working directory：上跳后为 \\\\?\\D:\\Temp\\aw-v128-dafd1256","changed_files":[],"commit":null,"duration_ms":0,"exit_code":null,"mode":"standalone","status":"blocked","stderr_digest":null,"stdout_digest":null,"task_id":"n1-dotdot","verify_cmd":""}
```

`n2-abs`（界外绝对路径）：

```text
{"artifacts":[],"attempt":null,"blocked_reason":"standalone_argv_path_escape: argv 第 4 项（D:\\Temp\\aw-v128-dafd1256\\outside\\escape.txt）解析后越出 working directory：字面归一化后为 D:\\Temp\\aw-v128-dafd1256\\outside\\escape.txt","changed_files":[],"commit":null,"duration_ms":0,"exit_code":null,"mode":"standalone","status":"blocked","stderr_digest":null,"stdout_digest":null,"task_id":"n2-abs","verify_cmd":""}
```

`n3-flag`（`--flag=../../x`）：

```text
{"artifacts":[],"attempt":null,"blocked_reason":"standalone_argv_path_escape: argv 第 4 项（--flag=../../x）解析后越出 working directory：上跳后为 \\\\?\\D:\\Temp\\aw-v128-dafd1256","changed_files":[],"commit":null,"duration_ms":0,"exit_code":null,"mode":"standalone","status":"blocked","stderr_digest":null,"stdout_digest":null,"task_id":"n3-flag","verify_cmd":""}
```

`n4-symlink`（`--out=link/target.txt`，`link` 指向界外）：

```text
{"artifacts":[],"attempt":null,"blocked_reason":"standalone_argv_path_escape: argv 第 4 项（--out=link/target.txt）解析后越出 working directory：符号链接 \\\\?\\D:\\Temp\\aw-v128-dafd1256\\work\\link 指向 \\\\?\\D:\\Temp\\aw-v128-dafd1256\\outside","changed_files":[],"commit":null,"duration_ms":0,"exit_code":null,"mode":"standalone","status":"blocked","stderr_digest":null,"stdout_digest":null,"task_id":"n4-symlink","verify_cmd":""}
```

`n5-unc`（`\\server\share\evil`）：

```text
{"artifacts":[],"attempt":null,"blocked_reason":"standalone_argv_path_escape: argv 第 4 项（\\\\server\\share\\evil）解析后越出 working directory：字面归一化后为 \\\\server\\share\\evil","changed_files":[],"commit":null,"duration_ms":0,"exit_code":null,"mode":"standalone","status":"blocked","stderr_digest":null,"stdout_digest":null,"task_id":"n5-unc","verify_cmd":""}
```

`n6-encoded`（`..%2f..%2fescape.txt`）：

```text
{"artifacts":[],"attempt":null,"blocked_reason":"standalone_argv_path_escape: argv 第 4 项（..%2f..%2fescape.txt）解析后越出 working directory：上跳后为 \\\\?\\D:\\Temp\\aw-v128-dafd1256","changed_files":[],"commit":null,"duration_ms":0,"exit_code":null,"mode":"standalone","status":"blocked","stderr_digest":null,"stdout_digest":null,"task_id":"n6-encoded","verify_cmd":""}
```

运行目录核对：上述 6 个 task_id 在 `target\debug\runs` 下均不存在（拒绝发生在占号前）。

> 说明：为控制篇幅，正例回执只保留本夹具下逐字不变的状态字段；完整字段（`artifacts[]` 的 sha256、`finishedAt`、`duration_ms` 等）在同一次 stdio 输出中一并返回，未作改写。反例回执为完整原文。

### 27.6 未覆盖与限制（如实标注）

- 未在 POSIX 上跑本轮真跑；`is_windows_switch` 仅在 Windows 生效，POSIX 下 `/C` 会按根路径处理（契约已写明该规则是 Windows 专属）。
- `file://` URL、含正则元字符的真实路径、`%NAME%` 展开后的真实目标属于**明确盲区**：前者不校验，后两者按 unknown 放行；契约 §2.6.8 已逐条列出，不声称已覆盖。
- `argv_path_scan.checked[].resolved` 对尚不存在的目标只给到最深已存在祖先（示例 `p7` 为 cwd 本身），不代表最终文件位置。
- 未验证"参数路径指向 cwd 内符号链接、链接目标在 cwd 内再跳回界外"的多跳组合；当前实现逐组件 canonicalize 会按最终目标判断，但未构造该夹具实测。

## 28. v1.29：standalone 并发与重入实证

### 28.1 结论（按回执字段判定，不按阅读代码判定）

| 场景 | 实际发生 | 判定字段 |
|---|---|---|
| 同一 `task_id`、两个独立进程并发 | **一个执行、另一个等待回放**（不是"两个都 ok"） | `replayed=false`（执行方） vs `replayed=true` + `replay_kind="waited"`（回放方）；两方 `finishedAt` / `duration_ms` / `stdout_digest` / `verdict_locator` 完全相同；目录只有 `attempt-1` |
| 不同 `task_id` 4 条并发，其中 1 条超时 | **互不影响**：3 条 `ok` + 1 条 `blocked`，3 份产出文件均在超时清理后落盘 | 3 条 `status=ok` / `exit_code=0` / `replayed=false`；超时条 `status=blocked` / `exit_code=null` / `verdict_conclusion=undetermined`，`blocked_reason` 含"timeout"与"已清理受控集合" |
| 两个 bridge 进程共享同一 `runs/` | **靠原子 `create_dir(attempt-1)` 抢占 + 败者轮询回放**；无全局锁，不支持跨进程事务 | 同 id 场景的 `replayed`/`replay_kind`；异参/残缺目录的 `task_id_conflict` |
| 同一 `task_id` 完成后重入 | **返回既有结果**（不新建 attempt、不重跑、不拒绝） | `replayed=true` + `replay_kind="existing"`；`finishedAt` 不变；`verdict.json` 与 `standalone-result.json` 的 sha256/mtime 前后不变；目录仍只有 `attempt-1` |

### 28.2 先红：HEAD `858b7bf` 上的原始失败（断言未放宽）

做法：`git worktree add --detach D:\Temp\aw-v129-red 858b7bf`，只把本轮 `tests/mcp_contract.rs` 放入工作树，`CARGO_TARGET_DIR` 复用现有 target；跑完 `worktree remove --force`。

命令：`cargo test -p aw-mcp-bridge --test mcp_contract standalone_ -- --nocapture`

```text
running 13 tests
thread 'standalone_replay_returns_final_result_without_new_attempts' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4607:5:
assertion `left == right` failed: 首次执行必须标称非回放：{"argv":["cmd.exe","/C","echo hello>out.txt"],"argv_path_scan":{"checked":[],"checked_total":0,"scope":"\\\\?\\D:\\Temp\\.tmptJKCMl","unknown":[],"unknown_total":0},"artifacts":[{"hash":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","kind":"stdout","locator":"runs/sa-replay/attempt-1/stdout.txt"},{"hash":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","kind":"stderr","locator":"runs/sa-replay/attempt-1/stderr.txt"},{"hash":"sha256:9ab266195d3d826a0ba264b5475abebec9fd8b0157c91511a25a0af7df2d81f8","kind":"verdict","locator":"runs/sa-replay/attempt-1/verdict.json"}],"attempt":1,"attempts_total":1,"blocked_reason":null,"changed_files":["out.txt"],"changed_files_truncated":false,"commit":null,"cwd":"\\\\?\\D:\\Temp\\.tmptJKCMl","duration_ms":2546,"env_keys":[],"exit_code":0,"finishedAt":"2026-11-06T19:46:46Z","max_attempts":1,"mode":"standalone","path_prepend":[],"status":"ok","stderr_digest":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","stdout_digest":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","task_id":"sa-replay","timeout_secs":120,"verdict_conclusion":"passed","verdict_locator":"runs/sa-replay/attempt-1/verdict.json","verify_cmd":"cmd.exe /C echo hello>out.txt"}
  left: Null
 right: false
test standalone_replay_returns_final_result_without_new_attempts ... FAILED
thread 'standalone_duplicate_execute_is_serialized_across_servers' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4704:5:
assertion `left == right` failed: 同一 task_id 只能有一个调用方真正执行：[Object {"argv": Array [String("cmd.exe"), String("/C"), String("ping -n 3 127.0.0.1 >nul & echo once>out.txt")], ...}, Object {...}]  ← 左值为两个回执的 Debug 列表，原文见本轮运行输出
  left: 0
 right: 1
test standalone_duplicate_execute_is_serialized_across_servers ... FAILED
thread 'standalone_concurrent_tasks_are_isolated_from_one_timeout' panicked at apps\mcp-bridge\tests\mcp_contract.rs:4817:13:
assertion `left == right` failed: {"argv":["cmd.exe","/C","ping -n 3 127.0.0.1 >nul & echo ok>out-1.txt"],"argv_path_scan":{"checked":[],"checked_total":0,"scope":"\\\\?\\D:\\Temp\\.tmpoFJeUu","unknown":[],"unknown_total":0},...,"finishedAt":"2026-11-06T19:46:48Z","max_attempts":1,"mode":"standalone","path_prepend":[],"status":"ok","stderr_digest":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","stdout_digest":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","task_id":"sa-iso-1","timeout_secs":120,"verdict_conclusion":"passed","verdict_locator":"runs/sa-iso-1/attempt-1/verdict.json","verify_cmd":"cmd.exe /C ping -n 3 127.0.0.1 >nul & echo ok>out-1.txt"}
  left: Null
 right: false
test standalone_concurrent_tasks_are_isolated_from_one_timeout ... FAILED
test result: FAILED. 10 passed; 3 failed; 0 ignored; 0 measured; 90 filtered out; finished in 6.27s
```

红的关键点：旧代码根本没有 `replayed` 字段（`left: Null`），并发同 id 时两个调用方都把自己算作"执行方"（`executed=0`，期望 1）；不同 id 的 3 条成功回执同样缺该字段。`standalone_concurrent_conflicting_signature_is_named_conflict` 在旧代码上已绿——异参 `task_id_conflict` 是 v1.1 起就有的行为，本轮只补"同参并发"与"重入"的字段证据。

### 28.3 转绿与回归

`cargo test -p aw-mcp-bridge`：`10 passed; 0 failed`（lib）+ `103 passed; 0 failed`（integration，含本轮新增 4 条 standalone 并发/重入用例）。新增用例名：

- `standalone_replay_returns_final_result_without_new_attempts`（重入=existing）
- `standalone_duplicate_execute_is_serialized_across_servers`（同 id 两实例=一个执行一个 waited）
- `standalone_concurrent_tasks_are_isolated_from_one_timeout`（4 条含超时互不影响）
- `standalone_concurrent_conflicting_signature_is_named_conflict`（异参并发=明确拒绝）

### 28.4 真跑夹具与方法

- 可执行文件：`E:\软件开发\云集编程工作站\4.AgentWork\target\debug\aw-mcp-bridge.exe`（v1.29 代码重建于 `2026-11-06T19:48:59Z`）。
- 共享运行目录：该 exe 同级的 `target\debug\runs`（两个/四个进程都用同一路径，符合"独立进程操作同一 runs/"）。
- 工作目录：`D:\Temp\aw-v129-live\work`。
- 客户端：临时 Python stdio 脚本（不入库），每个场景启动 N 个独立 bridge 进程，各自完成 `initialize` + `notifications/initialized`，用 `threading.Barrier` 同时放行 `tools/call task.execute`；回执按 JSON-RPC 原始行解析，以下 `text` 即 `result.content[0].text` 原文。

### 28.5 同一 `task_id` 两进程并发（`v129dup`）

`process-a`（执行方）：

```text
{"argv":["cmd.exe","/C","ping -n 3 127.0.0.1 >nul & echo hello>out.txt"],"argv_path_scan":{"checked":[],"checked_total":0,"scope":"\\\\?\\D:\\Temp\\aw-v129-live\\work","unknown":[],"unknown_total":0},"artifacts":[{"hash":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","kind":"stdout","locator":"runs/v129dup/attempt-1/stdout.txt"},{"hash":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","kind":"stderr","locator":"runs/v129dup/attempt-1/stderr.txt"},{"hash":"sha256:ec9b5431513f251d09e34f9c6885762a6646b872d7ceb81dbed222be9c8a8926","kind":"verdict","locator":"runs/v129dup/attempt-1/verdict.json"}],"attempt":1,"attempts_total":1,"blocked_reason":null,"changed_files":[],"changed_files_truncated":false,"commit":null,"cwd":"\\\\?\\D:\\Temp\\aw-v129-live\\work","duration_ms":2183,"env_keys":[],"exit_code":0,"finishedAt":"2026-11-06T19:49:30Z","max_attempts":1,"mode":"standalone","path_prepend":[],"replayed":false,"status":"ok","stderr_digest":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","stdout_digest":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","task_id":"v129dup","timeout_secs":120,"verdict_conclusion":"passed","verdict_locator":"runs/v129dup/attempt-1/verdict.json","verify_cmd":"cmd.exe /C ping -n 3 127.0.0.1 >nul & echo hello>out.txt"}
```

`process-b`（等待回放方，注意 `replay_kind` 与 `replayed`）：

```text
{"argv":["cmd.exe","/C","ping -n 3 127.0.0.1 >nul & echo hello>out.txt"],"argv_path_scan":{"checked":[],"checked_total":0,"scope":"\\\\?\\D:\\Temp\\aw-v129-live\\work","unknown":[],"unknown_total":0},"artifacts":[{"hash":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","kind":"stdout","locator":"runs/v129dup/attempt-1/stdout.txt"},{"hash":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","kind":"stderr","locator":"runs/v129dup/attempt-1/stderr.txt"},{"hash":"sha256:ec9b5431513f251d09e34f9c6885762a6646b872d7ceb81dbed222be9c8a8926","kind":"verdict","locator":"runs/v129dup/attempt-1/verdict.json"}],"attempt":1,"attempts_total":1,"blocked_reason":null,"changed_files":[],"changed_files_truncated":false,"commit":null,"cwd":"\\\\?\\D:\\Temp\\aw-v129-live\\work","duration_ms":2183,"env_keys":[],"exit_code":0,"finishedAt":"2026-11-06T19:49:30Z","max_attempts":1,"mode":"standalone","path_prepend":[],"replay_kind":"waited","replayed":true,"status":"ok","stderr_digest":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","stdout_digest":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","task_id":"v129dup","timeout_secs":120,"verdict_conclusion":"passed","verdict_locator":"runs/v129dup/attempt-1/verdict.json","verify_cmd":"cmd.exe /C ping -n 3 127.0.0.1 >nul & echo hello>out.txt"}
```

落盘核对：`runs/v129dup/` 下只有 `attempt-1/` 与 `standalone-result.json`；`verdict.json` sha256 为 `ec9b5431513f251d09e34f9c6885762a6646b872d7ceb81dbed222be9c8a8926`，与两方回执 `artifacts[verdict].hash` 一致，只有一份。

### 28.6 不同 `task_id` 4 条并发，其中 1 条超时（`v129iso-*`）

3 条慢任务（`ping -n 3`，约 2.2s）全部成功；超时任务 `timeout_secs=1` 先返回。

`v129iso-1`（`v129iso-2`/`v129iso-3` 同形，仅 `task_id`/`finishedAt`/`duration_ms`/verdict sha256 不同）：

```text
{"attempt":1,"attempts_total":1,"duration_ms":2210,"exit_code":0,"finishedAt":"2026-11-06T19:49:32Z","replayed":false,"status":"ok","task_id":"v129iso-1","verdict_conclusion":"passed","verdict_locator":"runs/v129iso-1/attempt-1/verdict.json"}
{"attempt":1,"attempts_total":1,"duration_ms":2199,"exit_code":0,"finishedAt":"2026-11-06T19:49:32Z","replayed":false,"status":"ok","task_id":"v129iso-2","verdict_conclusion":"passed","verdict_locator":"runs/v129iso-2/attempt-1/verdict.json"}
{"attempt":1,"attempts_total":1,"duration_ms":2210,"exit_code":0,"finishedAt":"2026-11-06T19:49:32Z","replayed":false,"status":"ok","task_id":"v129iso-3","verdict_conclusion":"passed","verdict_locator":"runs/v129iso-3/attempt-1/verdict.json"}
```

超时任务 `v129iso-timeout`（完整原文）：

```text
{"argv":["cmd.exe","/C","ping -n 20 127.0.0.1 >nul & echo late>out-t.txt"],"argv_path_scan":{"checked":[],"checked_total":0,"scope":"\\\\?\\D:\\Temp\\aw-v129-live\\work","unknown":[],"unknown_total":0},"artifacts":[{"hash":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","kind":"stdout","locator":"runs/v129iso-timeout/attempt-1/stdout.txt"},{"hash":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","kind":"stderr","locator":"runs/v129iso-timeout/attempt-1/stderr.txt"},{"hash":"sha256:93b3a709566fd4230281b6cd0a6bf7b5e09ba25f20439d82efde7ed0bf3bf745","kind":"verdict","locator":"runs/v129iso-timeout/attempt-1/verdict.json"}],"attempt":1,"attempts_total":1,"blocked_reason":"timeout：超过 1s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证；Windows 下子进程以挂起态创建并先加入作业再恢复执行（恢复前不会派生子进程）；BREAKAWAY 或加入作业失败的作业外进程仍不可观测","changed_files":[],"changed_files_truncated":false,"commit":null,"cwd":"\\\\?\\D:\\Temp\\aw-v129-live\\work","duration_ms":1018,"env_keys":[],"exit_code":null,"finishedAt":"2026-11-06T19:49:31Z","max_attempts":1,"mode":"standalone","path_prepend":[],"replayed":false,"status":"blocked","stderr_digest":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","stdout_digest":"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","task_id":"v129iso-timeout","timeout_secs":1,"verdict_conclusion":"undetermined","verdict_locator":"runs/v129iso-timeout/attempt-1/verdict.json","verify_cmd":"cmd.exe /C ping -n 20 127.0.0.1 >nul & echo late>out-t.txt"}
```

互不影响的证据：超时任务 `19:49:31Z` 返回并清理本 attempt 的 Job，3 条慢任务在 `19:49:32Z` 各自正常结束，`exit_code=0`；工作目录里 `out-1.txt` / `out-2.txt` / `out-3.txt` 均为 4 字节，`out-t.txt` 不存在（超时被清理、未产出）。4 个任务目录均只有 `attempt-1`，无交叉 attempt 目录或额外 attempt。

### 28.7 重入：`v129dup` 已有完成记录后再次调用

新起一个 bridge 进程，用**完全相同**参数再次 `task.execute`：

```text
{"attempt":1,"attempts_total":1,"duration_ms":2183,"finishedAt":"2026-11-06T19:49:30Z","replay_kind":"existing","replayed":true,"status":"ok","task_id":"v129dup","verdict_conclusion":"passed","verdict_locator":"runs/v129dup/attempt-1/verdict.json"}
```

前后落盘对比（调用前后各取一次）：

| 文件 | 前 | 后 |
|---|---|---|
| `attempt-1/verdict.json` sha256 | `ec9b5431513f251d09e34f9c6885762a6646b872d7ceb81dbed222be9c8a8926` | 同左 |
| `attempt-1/verdict.json` mtime (ns) | `1793994570099226100` | 同左 |
| `standalone-result.json` sha256 | `876341412c3a475f1f04f5f3a8944f0885f5c62c60bbb7a08b8c5dc09a37717d` | 同左 |
| `standalone-result.json` mtime (ns) | `1793994570099226100` | 同左 |
| `runs/v129dup/` 目录 | `["attempt-1"]` | `["attempt-1"]` |

结论：**当前语义是"返回既有结果"**（`existing`），不是新建 attempt，也不是拒绝；要重新执行必须换新 `task_id`。

### 28.8 同一 `task_id`、不同参数并发（`v129conflict`）

两个独立进程同时发：A 为慢任务（`ping -n 3` + `out-a.txt`），B 为快任务（`echo b>out-b.txt`）。本次 B 抢到 `attempt-1` 并执行，A 得到明确工具错误：

```text
{"error":"task_id_conflict: task_id `v129conflict` 已存在且 standalone 参数不一致（cwd / argv / acceptance / max_attempts / timeout_secs 必须完全一致）；重跑请使用新 task_id"}
```

B 回执关键字段：`"replayed":false`、`"status":"ok"`、`"finishedAt":"2026-11-06T19:49:33Z"`。`runs/v129conflict/` 只有 `attempt-1`，B 的 `out-b.txt` 存在，A 的 `out-a.txt` 不存在——败者没有执行、没有写第二份证据。

### 28.9 残缺目录的可检测症状（`v129orphan`）

手工创建 `runs/v129orphan/attempt-1/`（不写 `dispatch.json`），再调用 `task.execute`。等待约 5s 后返回：

```text
{"error":"task_id_conflict: task_id `v129orphan` 已有运行目录但缺少派单签名（dispatch.json），无法证明参数一致；重跑请使用新 task_id"}
```

这是"胜者在极窄窗口崩溃"的可检测症状：不会被静默当作重放，也不会被覆盖。

### 28.10 未覆盖与限制（如实标注）

- 同 id 并发只跑了 2 轮（首轮因 `target\debug` 二进制未随 v1.29 重建而缺少 `replayed` 字段，重建后重跑；两轮都在执行窗口内命中，均得到"一个执行 + 一个 `waited`"）。这是**采样证据**，不是对所有交错顺序的形式证明；原子性判据仍是 OS 级 `fs::create_dir` 的 `AlreadyExists`，未做高频循环压测。
- 超时"不误伤其它任务"的判据是：其它 3 条任务随后正常完成且产出落盘、各 attempt 只有自己的目录。未观测到清理误杀，但**不能证明任意逃逸进程都不可观测**——`blocked_reason` 已明写 BREAKAWAY / 加入作业失败 / 嵌套作业不可观测。
- `runs.prune` 与执行/回放在跨进程下没有全局锁；本轮**未构造**"边 prune 边重放同一任务"的竞争实测，契约 §2.6.9 只把它写成已知限制与症状（`task_id_conflict`）。
- 未在 POSIX 上重跑本组并发场景；Windows Job Object 语义是本轮唯一实测平台。
- 本轮未改 `tools/list` 描述，`tools_sha256` 仍为 `919be60becbc358ae0335b17b7a38d3feb063e7acc4deb4f6312204167c52922`。

## 29. v1.30：`runs.prune` 与 AW 执行的并发防护实证

### 29.1 结论（按文件/字段判定）

| 问题 | 实测结论 | 判据 |
|---|---|---|
| AW 写 attempt 时持路径还是句柄 | stdout/stderr 是**句柄**（交给子进程持续写）；JSON 证据全部是**按路径短写** | 运行中 rename 活跃 attempt 目录 → `PermissionError: [WinError 5] 拒绝访问`，目录原地未动、`.prune-trash` 未产生；AW 回执 `ok` |
| rename 后继续写会怎样 | **Windows 上 rename 先被 OS 拒绝**，不存在"写进 trash"；若目录被删除（允许），句柄仍有效但按路径读证据失败 | 运行中 `shutil.rmtree(attempt-1)` 成功，AW 回执 `isError=true`、`attempt_dir_lost`，`attempt-1` 不存在 |
| 完成文件里有没有"活跃"字段 | **没有**；`handback` / `verdict` / `standalone-result` 都只在结束时写，`dispatch.json` 存在但不含完成态 | 新增 `runs/.inflight/<task_id>.json`：运行时 `seen=true`，`pid=90384`、`startedAt=2026-11-06T20:28:26Z`、`deadlineAt=20:29:56Z`；结束后 `exists_after=false` |
| prune 与执行并发 | `runs.prune` 看到标记即跳过整个 task，**不进候选、不删除** | 6-2 场景（attempt-1 failed、attempt-2 运行中）dry 的 `skipped[0].reason="task_inflight"`、`inflight.stale=false`、`candidates_total=0`；delete 的 `deleted[]` 只有无关的 `r-a/r-b/r-c`，`attempt-1` 保留 |
| 被清理后的重跑 | `replay_basis=rerun_after_records_missing`，可区分于既有结果回放 | 首次 `fresh`；删除 task 目录后再调 → `rerun_after_records_missing` + `prior_attempts=1`；第三次 → `replayed=true` 且保留该标注 |

### 29.2 先红：HEAD `ad98d19` 上的原始失败（断言未放宽）

做法：`git worktree add --detach D:\Temp\aw-v130-red HEAD`，只把本轮 `tests/mcp_contract.rs` 放入工作树，`CARGO_TARGET_DIR` 复用现有 target；跑完 `worktree remove --force`。

```text
thread 'standalone_inflight_marker_is_visible_then_removed' panicked at apps\mcp-bridge\tests\mcp_contract.rs:5685:5:
执行期间必须有 runs/.inflight/sa-inflight.json
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 105 filtered out; finished in 4.37s
```

```text
thread 'prune_skips_task_with_live_inflight_marker' panicked at apps\mcp-bridge\tests\mcp_contract.rs:5749:5:
必须具名记录 task_inflight 跳过：{"batch_candidates_total":1,"candidates":[{"age_days":30,"attempt":1,...,"task_id":"sa-prune-inflight"}],"candidates_total":1,"dry_run":true,...,"skipped":[{"attempt":2,"reason":"latest_attempt_retained","task_id":"sa-prune-inflight"}],...}
```

```text
thread 'standalone_active_attempt_deleted_is_reported_not_silent_ok' panicked at apps\mcp-bridge\tests\mcp_contract.rs:5815:5:
必须具名报告 attempt_dir_lost：{"error":"读取证据失败：系统找不到指定的路径。 (os error 3)"}
```

```text
thread 'standalone_rerun_after_records_removed_is_marked' panicked at apps\mcp-bridge\tests\mcp_contract.rs:5836:5:
assertion `left == right` failed: 首次执行必须是 fresh：{...,"attempt":1,...,"replayed":false,...}
  left: Null
 right: "fresh"
```

红点：旧代码无活跃标记、无台账字段；目录消失时虽然 `isError=true`，但错误文案是通用的"读取证据失败"，外部无法据此判断是并发清理；prune 会把 `sa-prune-inflight/attempt-1` 当成正常候选（`batch_candidates_total=1`）。

### 29.3 转绿与回归

`cargo test -p aw-mcp-bridge`：`10 passed; 0 failed`（lib）+ `107 passed; 0 failed`（integration，含本轮新增 4 条）。新增用例：

- `standalone_inflight_marker_is_visible_then_removed`
- `prune_skips_task_with_live_inflight_marker`
- `standalone_active_attempt_deleted_is_reported_not_silent_ok`
- `standalone_rerun_after_records_removed_is_marked`

### 29.4 真实 stdio 多进程回执

夹具：`target\debug\aw-mcp-bridge.exe`（v1.30 重建），共享 `target\debug\runs`，工作目录 `D:\Temp\aw-v130-live\work`；每个场景独立进程、各自 `initialize`，prune 由第二个进程发起。

**a) 活跃标记（`v130marker`，运行时读取原文）**

```text
{"deadlineAt":"2026-11-06T20:29:56.679097800+00:00","mode":"standalone","pid":90384,"startedAt":"2026-11-06T20:28:26Z","state":"active","task_id":"v130marker","version":"inflight-marker/1"}
```

任务结束后：`exists_after=false`，`runs/v130marker/` 只有 `attempt-1` 与 `standalone-result.json`。

**b) attempt-2 运行中，prune 跳过（`v130pruneactive`）**

dry-run 原文（截取该 task）：

```text
{"attempt":2,"inflight":{"deadlineAt":"2026-11-06T20:30:29.358120300+00:00","mode":"standalone","pid":146380,"stale":false,"startedAt":"2026-11-06T20:28:29Z","state":"active","task_id":"v130pruneactive","version":"inflight-marker/1"},"reason":"task_inflight","task_id":"v130pruneactive"}
```

delete 模式：`deleted[] = [("r-a",1,"deleted"),("r-b",1,"deleted"),("r-c",1,"deleted")]`（无关的旧夹具），`candidates_total=3`，**不含 `v130pruneactive`**；`v130pruneactive/attempt-1` 保留，`attempt-2` 回执：

```text
{"attempt":2,"attempts_total":2,"exit_code":0,"finishedAt":"2026-11-06T20:28:33Z","replay_basis":"fresh","replayed":false,"status":"ok","task_id":"v130pruneactive"}
```

**c) 对照：完成后无标记，同一 attempt 立即成为候选并被删**

把 `attempt-1/handback.json` 的 `finishedAt` 改为 `2026-09-27T18:07:31Z`（40 天前），再跑 prune：

```text
dry candidates: [('v130pruneactive', 1, 40)]
dry skipped(v130): [{'attempt': 2, 'reason': 'latest_attempt_retained'}]
deleted(v130): [('v130pruneactive', 1, 'deleted')]
attempt dirs after: ['attempt-2', 'standalone-result.json']
```

这证明 b) 的拦截来自活跃标记，而不是"该 attempt 本来就不合格"。

**d) 运行中删除活跃 attempt（`v130vanish`）**

```text
{"error":"attempt_dir_lost: attempt 目录在写入过程中消失或被改名（可能被 runs.prune 或外部清理移动）：E:\\软件开发\\云集编程工作站\\4.AgentWork\\target\\debug\\runs\\v130vanish\\attempt-1；本次执行结果不可信，未报告 ok"}
```

`isError=true`，`attempt` 目录不存在，结束后标记也不存在。**不会静默成功**。

**e) 运行中 rename 活跃 attempt（`v130rename`）**

```text
rename_error = PermissionError: [WinError 5] 拒绝访问。: '...\runs\v130rename\attempt-1' -> '...\runs\.prune-trash\manual-v130rename'
attempt_exists = True, trash_exists = False, AW isError = false
```

目录原地未动，AW 正常完成（`ok`）；不存在"rename 成功、句柄继续写入 trash"。

**f) 记录全部被清理后的重跑（`v130rerun`）**

```text
first : replay_basis=fresh,                        prior_attempts=0, replayed=False, attempt=1
second: replay_basis=rerun_after_records_missing,  prior_attempts=1, replayed=False, attempt=1
third : replay_basis=rerun_after_records_missing,  prior_attempts=1, replayed=True,  attempt=1
```

删除 task 目录后再调不会假装是既有结果；台账 `runs/.task-history/v130rerun.json` 保留上次 `attempts=1` / `lastFinishedAt`。

### 29.5 未覆盖与限制（如实标注）

- **POSIX 未实测**：Windows 的"rename 被 OS 拒绝"是平台语义；POSIX 下目录 rename 可能成功，句柄写入会跟到新路径，因此必须依赖标记 + 删除方读标记。
- **TOCTOU 仍存在**：标记检查与 rename/delete 非原子。使用 AW 的 `runs.prune` 已被覆盖；自研删除方若不读 `runs/.inflight/<task_id>.json`，竞争窗口仍在。AW 的写完复核只能保证"不静默 ok"，不能阻止返回后的外部删除。
- **残留标记**：pid 已消失且超过 `startedAt + 5s` 判残留；prune 仍保守跳过并标 `task_inflight_stale`，直到下一次 AW 调用清理。未做"prune 自动清残留"（避免与同时启动的新调用抢占删除）。
- **台账盲区**：删除方若连 `runs/.task-history` 一起清理，或 task 从未被 AW 执行过（在 AW 外伪造 `runs/`），无法判定为重跑。
- 同 id 并发与 prune 的组合只做了 b) 这一种时序（attempt-2 运行窗口约 2s）；未做高频压力循环。
- 本轮未改 `tools/list` 描述，`tools_sha256` 仍为 `919be60becbc358ae0335b17b7a38d3feb063e7acc4deb4f6312204167c52922`。

## 30. `#3` 为什么是 `NOT_PROVEN`：证据链与关闭条件（v1.31，2026-11-07 复核）

### 30.1 `#3` 指哪个能力项

`#3` 不是某一条用例编号，而是契约 §5 第 15 行 / 门禁 G9 的能力项：**超时后终止"整棵进程树"**（对应 `task.execute` 的超时分支）。设计评估与窄定义实现见 [`TIMEOUT-PROCESS-TREE-DESIGN.md`](./TIMEOUT-PROCESS-TREE-DESIGN.md)。当前只宣称"已清理**受控集合**（POSIX 进程组 / Windows Job Object）；逃逸进程是否仍在运行未验证"。

### 30.2 证据链（代码 → 用例 → 缺什么）

| 层 | 位置 | 已证明什么 | 不能证明什么 |
|---|---|---|---|
| 实现 | [`server.rs`](../apps/mcp-bridge/src/server.rs) `process_scope`（约 L1956-L2035 Unix、L2246-L2336 Windows） | POSIX 在 exec 前建组、SIGTERM→SIGKILL→轮询 `kill(-pgid,0)`；Windows `CREATE_SUSPENDED → Assign → Resume`，`TerminateJobObject` 后轮询 `ActiveProcesses`；查询失败按 `Unavailable` | 作用域外的后代（`setsid`/`setpgid`、`BREAKAWAY`、加入作业失败、嵌套作业）不在任何判据里 |
| 实现 | `execute_task` 超时分支（约 L1263）、`timeout_reason`（约 L2641） | 三态分流；只有 `Cleaned` 才写"已清理受控集合"；任何分支不写"已终止进程树 / 无残留" | 文案保守不等于能力已具备 |
| 用例（Windows 单元，实跑） | `job_counts_grandchild_and_empties_after_terminate` / `alive_but_idle_process_is_counted_in_job` / `exited_process_leaves_job_empty` / `suspended_start_defers_execution_until_job_assign` / `single_terminate_suffices_under_spawn_pressure` | 作业内孙进程可清空；活着但不写盘的进程被判"存在"；自然退出判"不存在"；恢复执行前不派生子进程；一次终止后作业计数归零 | 作业外后代枚举；`BREAKAWAY` / 加入失败 |
| 用例（POSIX 单元，WSL 实跑） | `setsid_escape_is_outside_the_observed_group` 等 | `setsid sleep` 逃出原进程组后，组判据返回 `ESRCH` 而 `kill(已知PID,0)` 为 0——**证明 C1 是窄判据** | 整棵树；逃逸进程的持续追踪 |
| 用例（集成，两平台） | `timeout_cleans_controlled_set_including_grandchild` / `timeout_reason_states_action_not_tree_death` / `legacy_timeout_reason_is_returned_verbatim_and_not_rewritten` / `timeout_description_matches_controlled_set_semantics` | 真实超时任务走 `Cleaned` 分支；文案禁语门禁 | 文案之外的能力 |

### 30.3 缺的是"真跑"还是"实现"

缺的是**能枚举全部后代的观测点**，不是窄判据的实现：

1. Windows 只有作业成员计数/列表（`ActiveProcesses` / `JobObjectBasicProcessIdList`），没有"作业外候选"枚举；`BREAKAWAY`、加入作业失败、嵌套作业策略都可能留下作业外后代。
2. POSIX 没有可移植的后代枚举；`/proc` 扫描是 TOCTOU，`setsid` 后无句柄可追；cgroup v2 是唯一接近整棵树的机制，但要求委派权限且进程可自行移出，本环境未验证。
3. "某一时刻看不到"不等于"以后不会出现"：逃逸父进程仍可在观测后继续派生。

判定门槛（四条缺一即保留"未验证"）固定在 [`TIMEOUT-PROCESS-TREE-DESIGN.md`](./TIMEOUT-PROCESS-TREE-DESIGN.md) §6.8：完整枚举、全时段覆盖、三态可分、平台闭合。

### 30.4 本轮本机补跑（2026-11-07）

- Windows：`cargo test -p aw-mcp-bridge --lib -- --nocapture` → `10 passed`，证据原文：

```text
[evidence] grandchild attached=true ActiveProcesses_before=Some(1)
[evidence] grandchild terminate=Cleaned ActiveProcesses_after=Some(0)
[evidence] alive_but_idle attached=true ActiveProcesses_before=Some(1)
[evidence] alive_but_idle terminate=Cleaned ActiveProcesses_after=Some(0)
[evidence] suspended_start marker_before_assign=false marker_after_resume=true attached=true ActiveProcesses_before_terminate=Some(2)
[evidence] suspended_start terminate=Cleaned ActiveProcesses_after=Some(0)
[evidence] exited_process ActiveProcesses_final=Some(0)
```

- Windows 集成（超时组）：`7 passed`，证据原文：

```text
[evidence] timeout_reason=timeout：超过 1s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证；Windows 下子进程以挂起态创建并先加入作业再恢复执行（恢复前不会派生子进程）；BREAKAWAY 或加入作业失败的作业外进程仍不可观测
[evidence] grandchild_reason=timeout：超过 8s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证；Windows 下子进程以挂起态创建并先加入作业再恢复执行（恢复前不会派生子进程）；BREAKAWAY 或加入作业失败的作业外进程仍不可观测
[evidence] grandchild pid=126076 process_exists_after=false
```

- POSIX（WSL `HostGoTest`，Rust 1.88.0，`verify.sh run` 离线）：四类 `#3` 证据行齐全（`timeout_cleans_controlled_set_including_grandchild` / `timeout_reason_states_action_not_tree_death` 均通过）。同次运行出现 **1 条与本项无关的失败**：`standalone_argv_scope_rejects_escapes_before_execution` 的 `argv-backslash` 用例（`..\..\escape.txt` 在 POSIX 是合法文件名、不是分隔符，扫描判 `in_scope` 后执行成功）。该用例属 v1.28；v1.31.1 已按平台分隔符修正夹具并重跑，POSIX `9 lib + 108 integration` 全绿，原始红/绿见 §32。该失败与 `#3` 无关，不计入 `#3` 证据。

### 30.5 结论（不改状态）

**`#3` 继续保持 `NOT_PROVEN`**：本机可补跑的窄判据证据已补齐（Windows 单元 + 集成、POSIX WSL 复核），但仍缺整棵树观测点，属**环境/平台能力阻塞 + 既有窄定义实现**，不是"再写一段代码就能关"。关闭条件与 §6.8 完全一致；四条中任何一条满足前，回执不得出现"已终止进程树 / 无残留"。

## 31. `additionalProperties: false` 仍只声明不强制；新增单次调用严格模式（v1.31）

### 31.1 结论

- **v1.25 之后仍然成立**：六个工具的 `inputSchema` 都声明 `additionalProperties: false`，服务端默认**只读已知键、忽略未知键**，v1.25 只是把"忽略"变成可观测（回执追加 `arguments_diagnostics`）。
- **默认不改为全局强制**（理由与契约 §10.1 一致）：会打断携带 `_meta`、camelCase 别名、自定义元数据的既有调用；且契约已明确不提供"同一请求因进程/环境不同而行为不同"的开关。这不是实现代价问题，而是兼容与可复现性决策。
- **新增单次调用可选的严格模式**：`arguments._meta.strict_arguments = true` 时，服务端按 schema 拒绝 `kind=unknown` / `kind=alias`，返回具名错误 `invalid_arguments`；`_meta` 自身（`client_metadata`）与显式 `null` 可选字段不算违规。默认行为、schema 与 `tools_sha256` 均不变。

### 31.2 先红（HEAD `c5af9cf` 上同一批用例原样失败）

```text
thread 'strict_arguments_meta_rejects_unknown_with_readable_code' panicked at apps\mcp-bridge\tests\mcp_contract.rs:5868:5:
  left: Bool(false)
 right: true
thread 'strict_arguments_meta_rejects_alias_with_suggestion' panicked at apps\mcp-bridge\tests\mcp_contract.rs:5909:5:
  left: Bool(false)
 right: true
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 107 filtered out
```

红点：旧实现即使看到 `_meta.strict_arguments=true` 也照常执行（`isError=false`），声明与强制完全脱节。

### 31.3 转绿 + 真实 stdio 多进程回执

`cargo test -p aw-mcp-bridge --test mcp_contract strict_arguments_meta` → `3 passed; 0 failed`。真实 stdio（独立 bridge 进程，`initialize` 后逐条 `tools/call`）原文：

默认（无 `strict_arguments`，未知键仍被执行并附带诊断）：

```text
default_unknown: isError=False {"arguments_diagnostics":{...,"unknown_arguments":[{"kind":"unknown","name":"offest"...},{"kind":"unknown","name":"unexpected"...}]},...,"total_tasks":30}
```

严格（未声明参数被明确拒绝，错误码与已声明参数可读）：

```text
strict_unknown: isError=True {"alias_suggestions":[],"code":"invalid_arguments","declared_arguments":["limit","offset"],"error":"invalid_arguments: 未声明参数 offest；已声明参数：limit, offset；strict_arguments=true 时按 inputSchema.additionalProperties=false 拒绝",...}
```

严格 + 别名（拒绝并给出更正建议，不自动改名）：

```text
strict_alias: isError=True {"alias_suggestions":["dryRun -> dry_run"],"code":"invalid_arguments","declared_arguments":["confirm","dry_run","exclude","limit","older_than_days","status"],"error":"invalid_arguments: 未声明参数 dryRun；已声明参数：confirm, dry_run, exclude, limit, older_than_days, status；..."}
```

严格 + 合法可选参数缺省（不误拒）：

```text
strict_legal: isError=False {"arguments_diagnostics":{"unknown_arguments":[{"kind":"client_metadata","name":"_meta"...}]},...,"total_tasks":30}
```

`tools/list` 原文：`runs.list additionalProperties=false properties=['limit', 'offset']`；六个工具均 `additionalProperties=false`。

### 31.4 若改为全局强制：哪些客户端会失败（评估）

| 调用方行为 | 默认（现状） | 全局强制后 | 本次选择 |
|---|---|---|---|
| 携带 `_meta` 元数据 | 正常执行（诊断为 `client_metadata`） | 被拒绝 | 严格模式放行 `_meta`，只拒绝真正未声明键 |
| 使用 camelCase 别名（`dryRun`） | 被忽略且**不生效**（诊断为 `alias`） | 被拒绝 | 严格模式拒绝并给 `dryRun -> dry_run` 建议，避免"静默不生效" |
| SDK 序列化出显式 `null` 的可选字段 | 正常执行（诊断为 `null_optional`） | 仍正常（字段已声明） | 与全局强制一致 |
| 自定义元数据 / 拼错的参数名 | 被忽略（诊断为 `unknown`） | 被拒绝 | 默认保持忽略；需要时由调用方按单次请求开启严格 |

仓库内目前**没有 PM 侧真实调用样本**，上表是按 MCP 客户端通用行为推导的类别；默认宽松仍是对既有调用方最安全的选择。

### 31.5 边界

- 严格模式只覆盖 `kind=unknown|alias`；`null_optional` 与 `_meta` 不拒绝。
- 不新增环境变量/进程级开关（避免契约 §10.1 第 2 条"同一请求不同进程行为不同"）；严格与否由**单次请求**决定，回执可审计。
- `tools/list` 的 `description` / `inputSchema` 未改，`tools_sha256` 仍为 `919be60becbc358ae0335b17b7a38d3feb063e7acc4deb4f6312204167c52922`。

## 32. POSIX argv 夹具修正（v1.31.1）

### 32.1 红（HEAD `97e76b3`，WSL `HostGoTest`）

```text
thread 'standalone_argv_scope_rejects_escapes_before_execution' panicked at apps/mcp-bridge/tests/mcp_contract.rs:5592:9:
assertion `left == right` failed: argv-backslash 参数携带的越界路径必须在执行前拒绝：{"argv":["sh","-c","echo ok","--out=..\\..\\escape.txt"],"argv_path_scan":{"checked":[{"class":"relative_path","decision":"in_scope","encoded":false,"index":4,"raw":"--out=..\\..\\escape.txt","resolved":"/tmp/.tmpb6oV28/..\\..\\escape.txt","value":"..\\..\\escape.txt","via":"flag_equals"}],...},"status":"ok",...}
  left: String("ok")
 right: "blocked"
test result: FAILED. 107 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

原因：夹具把 Windows 分隔符 `\` 硬编码进 POSIX 用例；POSIX 下它是文件名字符，不是路径分隔，扫描判 `in_scope` 是正确行为，错的是夹具而不是实现。

### 32.2 修改（只动夹具，不动断言/实现）

```rust
(
    "argv-backslash",
    vec![format!(
        "--out=..{sep}..{sep}escape.txt",
        sep = std::path::MAIN_SEPARATOR
    )],
),
```

- Windows：`MAIN_SEPARATOR` 为 `\`，与旧夹具逐字相同。
- POSIX：构造真实的 `../../escape.txt`。
- 无 `cfg` 豁免分支、无断言放宽：仍断言 `status=blocked`、`blocked_reason` 以 `standalone_argv_path_escape` 开头、原因包含具体参数、拒绝发生在占号前（无运行目录）。

### 32.3 绿

Windows（单项回归）：

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 109 filtered out
```

POSIX（WSL 全量 `verify.sh run`，日志 `/root/.aw-posix-verify/logs/posix-verify-20261107-053436.log`）：

```text
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 108 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
[INFO] 四类 POSIX 用例全部通过，证据行齐全
```

（上一批 POSIX 为 `104 passed; 1 failed`；本批多出的 3 条是 v1.31 新增的严格参数用例，故全绿后为 `108 passed`。）





## 33. v1.32：能力/限制机器可读 + inflight 残留可观察可清（红→绿 + 真实 stdio 回执）

### 33.1 要关的两件事

1. 文档里的边界（`#3` 窄判据、POSIX 未覆盖、TOCTOU、残留不自动清）此前只有文字，调用方读不到。新增只读工具 `capabilities`，把每条边界做成 claim：`id` / `holds`（`true|false|null`）/ `status` / `scope` / `reason_code` / `platforms_verified` / `platforms_unverified` / `automatic` / `evidence`。判定只看字段，`holds=false` 必带非空 `reason_code`。
2. 残留 `.inflight` 标记此前只能靠人工删文件。新增 `runs.inflight`（`action=list|clean`）：`list` 只读给出 `stale`/`stale_reason`/`pid_alive`/时间窗与机器可读 `stale_rule`；`clean` 默认 `dry_run=true`，真删需 `dry_run=false`+`confirm=true`，只删 `stale=true`，活跃标记（pid 存活）永不删除。

### 33.2 红原文（HEAD 未实现时，原样）

`cargo test -p aw-mcp-bridge --test mcp_contract capabilities_declares_machine_readable_boundaries`：

```text
thread 'capabilities_declares_machine_readable_boundaries' panicked at apps\mcp-bridge\tests\mcp_contract.rs:5996:5:
assertion `left == right` failed: 能力声明必须可读取：{"error":"未知工具：capabilities"}
  left: Bool(true)
 right: false
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 114 filtered out
```

`cargo test -p aw-mcp-bridge --test mcp_contract runs_inflight_`：

```text
thread 'runs_inflight_clean_requires_explicit_confirmation' panicked at apps\mcp-bridge\tests\mcp_contract.rs:6272:5:
真删必须显式 confirm=true：未知工具：runs.inflight
thread 'runs_inflight_keeps_unreadable_marker' panicked at apps\mcp-bridge\tests\mcp_contract.rs:6306:5:
assertion `left == right` failed: {"error":"未知工具：runs.inflight"}
  left: Null
 right: 0
thread 'runs_inflight_lists_and_cleans_only_stale_markers' panicked at apps\mcp-bridge\tests\mcp_contract.rs:6161:5:
assertion `left == right` failed: {"error":"未知工具：runs.inflight"}
  left: Null
 right: "inflight-report/1"
thread 'runs_inflight_real_stdio_observes_and_cleans_stale_marker' panicked at apps\mcp-bridge\tests\mcp_contract.rs:6382:5:
assertion `left == right` failed: {"error":"未知工具：capabilities"}
  left: Null
 right: "capabilities/1"
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 111 filtered out
```

### 33.3 绿

`cargo test -p aw-mcp-bridge`（Windows，本机）：

```text
running 10 tests
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 115 tests
test result: ok. 115 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

其中新增 6 条用例：`capabilities_declares_machine_readable_boundaries`、`runs_inflight_lists_and_cleans_only_stale_markers`、`runs_inflight_clean_requires_explicit_confirmation`、`runs_inflight_keeps_unreadable_marker`、`runs_inflight_real_stdio_observes_and_cleans_stale_marker`，以及既有 `tools_list_exposes_read_and_execution_tools` 的名单扩到 8 个工具（属加强断言，未放宽任何旧断言）。

### 33.4 真实 stdio 回执（独立进程，非进程内调用）

夹具：`target\debug\runs\.inflight\stdio-stale.json`（`pid=999999999`、`startedAt` 两天前、`deadlineAt` 一天前）与 `stdio-active.json`（`pid` 为当前 PowerShell 进程、`deadlineAt` 一天前）；先真实执行 `task.execute`（`echo done`），再查询/清理。

正常结束后查询（`action=list`，`result.content[0].text` 解码后原样）：

```json
{"action":"list","active_total":1,"directory":"runs/.inflight","markers":[{"age_secs":3601,"deadlineAt":"2026-11-05T22:26:46Z","deadline_overdue_secs":86401,"file":"stdio-active.json","mode":"dispatched","pid":197288,"pid_alive":true,"removable":false,"stale":false,"stale_reason":"pid_alive","startedAt":"2026-11-06T21:26:46Z","state":"active","task_id":"stdio-active"},{"age_secs":172801,"deadlineAt":"2026-11-05T22:26:46Z","deadline_overdue_secs":86401,"file":"stdio-stale.json","mode":"dispatched","pid":999999999,"pid_alive":false,"removable":true,"stale":true,"stale_reason":"pid_missing","startedAt":"2026-11-04T22:26:46Z","state":"active","task_id":"stdio-stale"}],"stale_rule":{"cleanable_only_when":"stale=true","deadline_grace_secs":120,"live_pid_wins_over_deadline":true,"no_pid":"stale_only_after_deadlineAt_plus_deadline_grace_secs","pid_alive":"never_stale","pid_missing":"stale","pid_missing_grace_secs":5,"unreadable":"kept_never_deleted"},"stale_total":1,"state":"ready","total":2,"unreadable":[],"unreadable_total":0,"version":"inflight-report/1"}
```

说明：正常执行刚结束时先查过一次 `total=0`（无残留）；随后才写入两个夹具标记。

dry-run（`action=clean`，默认）：

```json
{"action":"clean","active_total":1,"directory":"runs/.inflight","dry_run":true,"failed":[],"failed_total":0,"kept":[{"file":"stdio-active.json","pid":197288,"reason":"not_stale","stale_reason":"pid_alive","task_id":"stdio-active"}],"kept_total":1,"removed":[],"removed_total":0,"stale_rule":{"cleanable_only_when":"stale=true","deadline_grace_secs":120,"live_pid_wins_over_deadline":true,"no_pid":"stale_only_after_deadlineAt_plus_deadline_grace_secs","pid_alive":"never_stale","pid_missing":"stale","pid_missing_grace_secs":5,"unreadable":"kept_never_deleted"},"stale_total":1,"state":"ready","total":2,"unreadable_total":0,"version":"inflight-report/1","would_remove":[{"file":"stdio-stale.json","path":"runs/.inflight/stdio-stale.json","pid":999999999,"stale_reason":"pid_missing","task_id":"stdio-stale"}],"would_remove_total":1}
```

真清（`action=clean` + `dry_run=false` + `confirm=true`）：

```json
{"action":"clean","active_total":1,"directory":"runs/.inflight","dry_run":false,"failed":[],"failed_total":0,"kept":[{"file":"stdio-active.json","pid":197288,"reason":"not_stale","stale_reason":"pid_alive","task_id":"stdio-active"}],"kept_total":1,"removed":[{"file":"stdio-stale.json","path":"runs/.inflight/stdio-stale.json","pid":999999999,"stale_reason":"pid_missing","task_id":"stdio-stale"}],"removed_total":1,"stale_rule":{"cleanable_only_when":"stale=true","deadline_grace_secs":120,"live_pid_wins_over_deadline":true,"no_pid":"stale_only_after_deadlineAt_plus_deadline_grace_secs","pid_alive":"never_stale","pid_missing":"stale","pid_missing_grace_secs":5,"unreadable":"kept_never_deleted"},"stale_total":1,"state":"ready","total":2,"unreadable_total":0,"version":"inflight-report/1","would_remove":[],"would_remove_total":0}
```

文件核对：`stale_exists=False active_exists=True`。即残留被真实删除，活跃标记（pid 存活、deadline 已过）保留。

能力声明的关键条目（从同一次 stdio 原始回执提取，未改写值）：

```json
[
 {"id":"timeout.terminates_controlled_set","holds":true,"status":"proven","reason_code":null,"platforms_verified":["windows","linux"]},
 {"id":"timeout.cleans_entire_process_tree","holds":false,"status":"not_proven","reason_code":"no_complete_descendant_enumeration"},
 {"id":"timeout.enumerates_escaped_descendants","holds":false,"status":"not_proven","reason_code":"no_complete_descendant_enumeration"},
 {"id":"platform.posix_inflight_semantics_verified","holds":false,"status":"partial","reason_code":"posix_unverified","platforms_unverified":["linux","macos"]},
 {"id":"concurrency.check_then_delete_atomic","holds":false,"status":"not_proven","reason_code":"check_then_delete_not_atomic"},
 {"id":"inflight.stale_marker_observable","holds":true,"status":"supported","automatic":true},
 {"id":"inflight.stale_marker_cleanable","holds":true,"status":"supported","automatic":false},
 {"id":"inflight.stale_markers_auto_swept","holds":false,"status":"not_proven","reason_code":"explicit_cleanup_required"},
 {"id":"inflight.clean_preserves_active_markers","holds":true,"status":"proven","automatic":false}
]
```

顶层为 `entries_total=13`、`not_proven_total=5`、`platform={"arch":"x86_64","os":"windows"}`、`version="capabilities/1"`。

### 33.5 判定与仍然存在的限制

- `#3` 窄判据：`timeout.terminates_controlled_set=true`（受控集合），`timeout.cleans_entire_process_tree=false`、`timeout.enumerates_escaped_descendants=false`（原因码 `no_complete_descendant_enumeration`）。整树仍 `NOT_PROVEN`，没有因为新增声明而放宽。
- POSIX：`platform.posix_inflight_semantics_verified=false`、`status=partial`、`reason_code=posix_unverified`，未写成已覆盖。
- TOCTOU：`concurrency.check_then_delete_atomic=false`，原因码 `check_then_delete_not_atomic`；`runs.inflight` 只能清理 AW 自己的残留标记，不能消除外部删除方的检查-删除窗口。
- 自动清扫：`inflight.stale_markers_auto_swept=false`、`explicit_cleanup_required`；清理必须显式调用，不是后台定时。
- 残留判定的盲区（pid 复用、僵死/挂起、时间回拨）未在本机穷举，见 §21.5；用例锁定的是"存活 pid 永不判残留"与"消失 pid + 超宽限必判残留"两个方向。
- `tools/list` 描述新增 G17–G19 行，`tools_sha256` 更新为 `37f7f9b0ee7d121e3f588bb65459fdedb7266d17ed744028204f915499dbcd5b`。
- **平台覆盖如实标注**：v1.32 的 5 条新用例只在 Windows 实跑；WSL 当前找不到 cargo，未复跑 POSIX，因此 `inflight.stale_marker_observable` / `inflight.stale_marker_cleanable` / `inflight.stale_markers_auto_swept` / `inflight.clean_preserves_active_markers` 的 `platforms_verified` 只写 `["windows"]`，`platforms_unverified` 写 `["linux","macos"]`；`platform.posix_inflight_semantics_verified` 保持 `false/partial/posix_unverified`，没有因为新增查询工具而上调。（该状态已由 §34 更新：四条的 `platforms_verified` 现为 `["windows","linux"]`，`platforms_unverified` 仅剩 `["macos"]`。）

## 34. v1.32.1：POSIX 双侧复跑 + PM 判活口径（红→绿）

### 34.1 WSL `cargo` 缺失的诊断（原始输出）

WSL 侧唯一发行版为 `HostGoTest`（`wsl -l -v` 只有这一条，STATE 为 Stopped），`/etc/os-release`：

```text
PRETTY_NAME="Ubuntu 24.04 LTS"
NAME="Ubuntu"
VERSION="24.04 LTS (Noble Numbat)"
```

诊断脚本由 Windows 侧写入、经 `/mnt/d` 在 WSL 执行（避免 PowerShell 提前展开 `$?`），原始输出：

```text
which cargo
which_rc=1

ls -la /root/.cargo/bin
ls: cannot access '/root/.cargo/bin': No such file or directory
ls_rc=2

command -v rustc
rustc_rc=1
```

`command -v cargo` 的输出为 `CARGO_NOT_FOUND`，`command -v rustup` 为 `RUSTUP_NOT_FOUND`。PATH 中与 cargo 有关的只有 Windows 侧挂载（节选）：`/mnt/d/AppData/cargo/bin`、`/mnt/c/Users/Administrator/.cargo/bin`。结论：**不是 PATH 拼接问题**，而是 Linux 侧的 rustup shim（`~/.cargo/bin`）从未安装；Windows 的 `cargo.exe` 不会在 Linux 上解析为 `cargo`。

已有可直接使用的工具链（即使没有 rustup shim）：

```text
ls /root/.rustup/toolchains
1.88.0-x86_64-unknown-linux-gnu
1.88.0-x86_64-unknown-linux-musl

/root/.rustup/toolchains/1.88.0-x86_64-unknown-linux-gnu/bin/cargo --version
cargo 1.88.0 (873a06493 2025-05-10)
```

### 34.2 rustup 最小安装：卡在下载，原始报错

（1）用 `curl` 直接取官方 `rustup-init`：

```text
curl: (28) Operation timed out after 120001 milliseconds with 1136884 out of 21347088 bytes received
```

（2）改走 rsproxy 镜像取同一个二进制：

```text
curl: (28) Operation timed out after 90000 milliseconds with 707396 bytes received
```

（3）此前三次 `rustup-init -y --profile minimal ...` 安装均在 `rustup-init` 下载阶段超时，由工具层 10 分钟超时终止（进程退出码 15）；其中包含 `--default-toolchain none --no-modify-path` 与设置 `RUSTUP_DIST_SERVER=https://rsproxy.cn` / `RUSTUP_UPDATE_ROOT=https://rsproxy.cn/rustup` 的两次，结果相同。端点本身可达（探测返回 `HTTP/2 200`），但有效带宽约 6–9 KB/s，21 MB 的 `rustup-init` 无法在可接受时间内完成。

结论：**卡在 rustup-init 二进制下载的带宽上，不是“环境不支持”**；`/root/.cargo/bin` 从未生成，因此没有 `cargo` shim。复跑改用上述已存在的 1.88.0 工具链，以下测试结果均由 `cargo 1.88.0` 实跑。

### 34.3 复跑环境与一次真实的红灯（文档快照）

精简工作区：`/root/.aw-posix-verify/src`（只含 `apps/mcp-bridge` 成员）+ vendored 依赖 `/root/vendor`；`server.rs` 与仓库工作区逐字节同步（`sha256=18f4616ad113c56e15755d6ad6980182f672482f85d0a92ec48632b6c813c95a`）。

首次全量在 WSL 上跑时，`description_gate_snapshot_matches_audit` 真实红（验证工作区里的审计快照仍是 v1.32 的旧摘要）：

```text
assertion `left == right` failed: tools/list 描述或 schema 已改动：请同步 TOOLS-DESCRIPTION-AUDIT.md §21（行断言 + tools_sha256）。实际=a3591dbfe339e2ddb61400bd810ca58e17faf7b098756b044f186cb4cce88084
  left: "a3591dbfe339e2ddb61400bd810ca58e17faf7b098756b044f186cb4cce88084"
 right: "37f7f9b0ee7d121e3f588bb65459fdedb7266d17ed744028204f915499dbcd5b"
test result: FAILED. 8 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

把审计快照同步为 `a3591db...` 后全量：

```text
running 9 tests
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 115 tests
test result: ok. 115 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 34.4 红→绿：平台覆盖（HEAD v1.32 旧代码 + v1.32.1 新断言，WSL）

用 `git show HEAD:4.AgentWork/apps/mcp-bridge/src/server.rs` 覆盖验证工作区副本（保留新断言），只跑新用例：

```text
test capabilities_inflight_entries_cover_current_platform ... FAILED
thread '...' panicked at apps/mcp-bridge/tests/mcp_contract.rs:6156:9:
inflight.stale_marker_observable 已在 linux 实跑，platforms_verified 必须包含 linux：{...,"platforms_unverified":["linux","macos"],"platforms_verified":["windows"],...}
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 114 filtered out
```

换回当前实现（四条 `inflight.*` 与 `platform.posix_inflight_semantics_verified` 的 `platforms_verified=["windows","linux"]`、`platforms_unverified=["macos"]`）：

```text
test capabilities_inflight_entries_cover_current_platform ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 114 filtered out
```

### 34.5 红→绿：PM 判活约定（HEAD 旧代码 + v1.32.1 新断言，WSL）

```text
test runs_inflight_list_exposes_pm_liveness_contract ... FAILED
assertion `left == right` failed: {...,"stale_rule":{...},"total":1,...}
  left: Null
 right: "inflight-liveness/1"
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 114 filtered out
```

实现 `liveness_rule` 后：

```text
test runs_inflight_list_exposes_pm_liveness_contract ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 114 filtered out
```

约定本体写在契约 §2.8；`liveness_rule` 只出现在 `action=list`（`clean` 不加），字段取值与禁止项以 §2.8 表格为准。

### 34.6 全量回归与描述门禁

Windows（本机）：

```text
description_gate: 3 passed (lib) + 4 passed (integration)
running 10 tests -> test result: ok. 10 passed; 0 failed
running 117 tests -> test result: ok. 117 passed; 0 failed
```

WSL：见 §34.3（`9 lib + 115 integration` 全绿）。`tools_sha256` 因 `runs.inflight` 描述追加判活约定而更新为 `a3591dbfe339e2ddb61400bd810ca58e17faf7b098756b044f186cb4cce88084`（§33.5 的 `37f7f9b0...` 为 v1.32 当时值，已被本节取代）；§21.4 增补“PM 判活口径统一（v1.32.1）”门禁行，§21.5 增补 POSIX 平台差异行。

### 34.7 仍然存在的限制

- macOS 未验证：五条能力的 `platforms_unverified` 仍含 `macos`。
- `platform.posix_inflight_semantics_verified` 保持 `holds=false / status=partial / reason_code=posix_unverified`：POSIX 上 rename 活跃 attempt 目录的语义未单独构造复核，不因功能用例跑通而上调为 `proven`。
- rustup shim 未安装：WSL 复跑依赖已存在的 `/root/.rustup/toolchains/1.88.0-x86_64-unknown-linux-gnu`；换一台没有预置工具链、且下载带宽同样受限的机器，仍会卡在 §34.2 的同一步骤。
- 自动清扫仍不成立（`inflight.stale_markers_auto_swept=false`），TOCTOU 未消除（`concurrency.check_then_delete_atomic=false`）。

## 35. v1.33：残留 inflight 标记顺带清扫（无后台线程）

### 35.1 红

新增 5 条集成用例，实现前 4 条红、1 条守护性绿（不可读标记“保留”本来就是既有语义）：

```text
inflight_sweep_on_task_start_removes_only_target_stale_marker ... FAILED
  重启后同 task_id 残标仍在，runs/inflight-sweep.jsonl 不存在

inflight_sweep_on_prune_removes_stale_marker_and_unblocks_candidate ... FAILED
  回执无 inflight_sweep（left: Null），残标仍在，候选仍被 task_inflight_stale 跳过

inflight_clean_writes_audit_and_is_idempotent ... FAILED
  runs/inflight-sweep.jsonl 不存在：clean 只删不记账，无法回答“谁触发、依据哪个 stale_reason”

runs_inflight_list_exposes_sweep_contract ... FAILED
  assertion `left == right` failed: left: Null, right: "inflight-sweep/1"

inflight_sweep_on_task_start_preserves_unreadable_marker ... ok（红阶段即绿，守护既有语义）
```

### 35.2 绿

实现后 5 条用例在 Windows 上逐条绿，随后全量：

```text
running 10 tests  -> test result: ok. 10 passed; 0 failed
running 122 tests -> test result: ok. 122 passed; 0 failed
```

WSL 同步 `server.rs` 后全量：

```text
running 9 tests   -> test result: ok. 9 passed; 0 failed
running 120 tests -> test result: ok. 120 passed; 0 failed
```

### 35.3 实现口径

- 共用一个 `sweep_inflight_markers(trigger, filter, dry_run)`：`clean` 真删改为走它；`runs.prune` 在枚举候选前调用（`dry_run` 只报 `would_remove`）；`task.execute` 在 `create_new` 返回 `AlreadyExists` 时按同一 `task_id` 调用，清掉后重试占位，无残标或不可读则按既有语义返回 `None`。
- 判据完全复用 `assess_inflight_marker`：`pid` 存活优先于 `deadline`；只有 `stale=true` 进入删除集合；`unreadable` 保留并在 `kept[]`/`unreadable_total` 报告。
- 审计 `runs/inflight-sweep.jsonl`：`phase=plan` 先落盘，写失败则零删除；删完写 `phase=result`（`removed[]` 带 `task_id`/`pid`/`stale_reason`/`path`，`trigger` 标明 `task_start|prune|clean`）；无残标时零删除且不追加审计，重复触发幂等。
- 删除沿用既有 TOCTOU 防护：原子 rename 到 `.tmp` 后复核 `task_id`+`pid`+`startedAt`，不一致还原并记 `marker_changed`，消失记 `marker_disappeared`，rename/删除失败进 `failed[]`。
- 能力位改为 `inflight.stale_markers_auto_swept=false / partial / path_triggered_only / automatic=false`：只做到特定路径顺带清扫，不宣称“自动定期清扫”；`capabilities` 的 `not_proven_total` 由 5 降为 4。
- `runs.inflight action=list` 增加 `sweep_rule`（触发条件、审计文件、永不删除项、原子删除与三条跨进程边界）；工具描述同步追加该行为，`tools_sha256` 更新为 `36dbcfc9c9ac83eede1a4ef581801b0b4dcb9b955761d5e3fcb085ea093af993`。

### 35.4 仍然存在的边界

- 跨进程无事务（`concurrency.check_then_delete_atomic=false`）：扫描与 rename 之间仍可能插入新进程；最终由 `create_new` 决出唯一持有者，但两侧动作不是同一事务。
- 判活与删除之间仍可能有新进程加入：清扫只依据扫描时的 `stale`，不排他锁住目录。
- 坏标记不自愈：`unreadable` 永久保留，只能人工删除；没有任何后台线程会重试。
- 触发点有限：没有调用方操作（不重启同 `task_id`、不 prune、不 clean）时，残标仍不会自行消失。

## 36. v1.34：后代进程观测点（`#3` 按平台分裂）

### 36.1 红

先加观测点 API（占位返回 `descendant_snapshot_not_implemented`）与用例，再跑：

- WSL Ubuntu 24.04（`cargo 1.88.0`，日志 `/root/.aw-posix-verify/logs/posix-verify-20261107-101000.log`）：

```
thread 'server::process_scope::imp::tests::descendant_snapshot_cross_checks_process_group' panicked at apps/mcp-bridge/src/server.rs:2778:39:
必须能观测到进程组内的后代
test server::process_scope::imp::tests::descendant_snapshot_cross_checks_process_group ... FAILED

thread 'server::process_scope::imp::tests::setsid_escape_is_flagged_then_unobservable_after_reparent' panicked at apps/mcp-bridge/src/server.rs:2877:38:
根存活期间，逃逸后代必须能被父链快照标出
test result: FAILED. 9 passed; 2 failed
```

- Windows 变异红灯（临时停用 `job_pid_list` 观测点，运行后已逐字还原）：

```
thread 'server::process_scope::imp::tests::descendant_snapshot_cross_checks_job_accounting' panicked at apps\mcp-bridge\src\server.rs:3607:39:
必须能观测到作业内的后代
thread 'server::process_scope::imp::tests::descendant_snapshot_flags_descendants_outside_controlled_set' panicked at apps\mcp-bridge\src\server.rs:3661:39:
未加入作业的进程树也必须可枚举
test result: FAILED. 0 passed; 2 failed
```

### 36.2 绿

- Windows：`12 lib + 122 integration` 全绿。
- WSL：`11 lib + 120 integration` 全绿（日志 `posix-verify-20261107-101517.log`）。

关键证据行（真实进程）：

```
[evidence] descendant_snapshot before root=124800 descendants=[DescendantProcess { pid: 113168, in_controlled_set: true }] controlled=Some([124800, 113168])
[evidence] descendant_snapshot after terminate=Cleaned descendants=[] controlled=Some([]) unobservable=None
[evidence] descendant_snapshot outside_controlled root=172680 descendants=[DescendantProcess { pid: 187288, in_controlled_set: false }] outside=[187288] controlled=Some([])
[evidence] posix_descendant_snapshot before root=701 descendants=[DescendantProcess { pid: 702, in_controlled_set: true }, DescendantProcess { pid: 703, in_controlled_set: true }] controlled=Some([701, 702, 703])
[evidence] posix_descendant_snapshot after terminate=Cleaned descendants=[] controlled=Some([]) unobservable=None
[evidence] setsid_snapshot before escaped=874 descendants=[DescendantProcess { pid: 874, in_controlled_set: false }] outside=[874]
[evidence] setsid_snapshot after outcome=Cleaned escaped_alive=true descendants=[] unobservable=None
```

### 36.3 观测点口径

- 统一结构 `DescendantSnapshot { root, descendants[{pid,in_controlled_set}], controlled_pids, unobservable }`；`unobservable` 非空时 `is_empty()` 恒为 false，禁止把"查不到"读成"没有"。
- Windows：`CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)` 父链传递闭包 × `JobObjectBasicProcessIdList`；两者集合相等才说明"枚举到的后代都在作业内"。父链在 Windows 上不因父进程退出而改写，这是整树可证的平台前提。
- Linux：`/proc/<pid>/stat` 的 `pid (comm) state ppid pgrp`（以最后一个 `)` 为界），父链闭包 × 同 `pgrp` 成员集合；`NotFound` 跳过，其余读失败（权限/hidepid）一律 `unobservable`。
- 生产终止路径在原有判据后追加复核：快照不可观测 → `Unavailable`；仍有可枚举后代 → `StillAlive`；两观测点都空才 `Cleaned`。属于收紧，不放宽原有三态。`StillAlive` 回执措辞同步扩为"受控集合内仍有存活进程，或父链快照仍枚举到存活后代"，避免把逃逸后代误报成集合内成员（`tools/list` 描述未改，`tools_sha256` 不变）。
- `capabilities` 新增 `timeout.descendant_snapshot_cross_check=true/proven/automatic=true`；`timeout.cleans_entire_process_tree` 与 `timeout.enumerates_escaped_descendants` 由 `not_proven` 改为 `holds=false/partial/posix_escaped_after_reparent_unobservable`；条目 13→14，`not_proven_total` 仍 4。

### 36.4 结论与仍然缺的那一段

- Windows：作业覆盖全部后代（`CREATE_SUSPENDED` 先入作业、恢复前不派生、本作业未设 `BREAKAWAY_OK`、嵌套作业仍在外层作业内），父链快照可枚举全部存活后代，终止后快照与 `ActiveProcesses` 同时为空 → **整树清理在 Windows 侧成立**。
- POSIX：`setsid` / 双 fork 换父后被 init/子收割者收养，父链断开；`setsid_escape_is_flagged_then_unobservable_after_reparent` 实测 `escaped_alive=true` 而快照 `descendants=[]`，即"快照为空 ≠ 无逃逸"。因此 POSIX **不能**宣称整树，`cleans_entire_process_tree` 只能保持 `holds=false`。
- 仍缺的边界：Linux 收养后的后代枚举（需 cgroup v2 委派或 subreaper + 启动时间全量扫描，均未实现）；macOS 无 `/proc`；Windows PID 复用可能多报后代（保守方向）；观测是终止后采样而非内核级订阅。
