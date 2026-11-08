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

顶层：`deleted_total` / `failed_total` / `trashed_total` / `lost_race_total` 四集合计数，且 `deleted_total + failed_total + trashed_total + lost_race_total = 本次实际处理的候选数`；`status` 仅在 `failed_total + trashed_total == 0` 时为 `ok`，否则为 `partial`（`lost_race_total` 不参与，v1.44）。

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
| G10 | `receipt.get` | `description` | 默认返回最新一次；cleanup_status；cleaned；still_alive；unavailable；not_started；只有 cleaned | 一定已清理 | `description_gate_snapshot_matches_audit` |
| G11 | `status` | `description` | 连接级计数 connections；unit=connection；进程内无会话概念；active/total/peak 恒为 1；initialize_total 随 initialize 次数累加；多份 initialize 不等于多会话；仅观测，不做准入或上限 | 活跃会话数；会话上限；准入拦截 | `description_gate_behavior_connection_counters_real_stdio` |
| G12 | `task.execute` | `description` | mode=standalone；verdict.json；undetermined；无法判定绝不默认通过；attempt < max_attempts；AW 不定义判据、不做验收 | 自动重试；会终止整棵进程树；已终止进程树；无残留 | `description_gate_standalone_mode_boundary` |
| G13 | `task.execute` | `inputSchema.properties.mode.description` | 缺省 dispatched；AW 只执行不验收；拒绝 cmd/env/path_prepend；同一 task_id 不得跨模式复用；mode_conflict | 自动推断模式 | `description_gate_standalone_mode_boundary` |
| G14 | `task.execute` | `inputSchema.properties.acceptance.description` | standalone 必填；file_exists；file_content_match；git_diff_non_empty；standalone_path_escape；未声明验收不得默认通过 | 视为通过 | `description_gate_standalone_mode_boundary` |
| G15 | `runs.prune` | `inputSchema.properties.exclude.description` | 按条目精确排除；task_id；attempt；不得重复；命名校验；invalid_exclusion；not_in_candidates；枚举入口生效 | 默认排除；自动排除 | `description_gate_snapshot_matches_audit`、`description_gate_rows_hold_on_tools` |
| G16 | `task.execute` | `inputSchema.properties.argv.description` | argv[1..]；携带路径；按规范化后的真实位置校验；standalone_argv_path_escape；standalone_argv_path_unknown；argv_path_scan；不按路径校验；占号前 | 不做作用域校验；忽略参数中的路径 | `description_gate_snapshot_matches_audit`、`description_gate_rows_hold_on_tools` |
| G17 | `capabilities` | `description` | 机器可读；holds（true/false/null）；status；scope；reason_code；platforms_verified；platforms_unverified；automatic；不依赖描述散文 | 仅见文档；无法机器判定 | `description_gate_snapshot_matches_audit`、`description_gate_rows_hold_on_tools`、`capabilities_declares_machine_readable_boundaries` |
| G18 | `runs.inflight` | `description` | runs/.inflight/<task_id>.json；action=list；stale_reason；pid_alive；pid_missing_grace；deadline_expired；unreadable；dry_run=true；confirm=true；活跃标记永不删除；pid+startedAt；would_remove；不进入 runs.prune 候选；服务启动时一次；默认每 3600 秒一次；AW_INFLIGHT_SWEEP_SECS；trigger=startup\|periodic；审计 plan 行写不进去则本轮零删除 | 自动删除 | `description_gate_snapshot_matches_audit`、`description_gate_rows_hold_on_tools`、`runs_inflight_lists_and_cleans_only_stale_markers` |
| G19 | `runs.inflight` | `inputSchema.properties.action.description` | list；clean；stale=true | 默认 clean；隐式删除 | `description_gate_snapshot_matches_audit`、`description_gate_rows_hold_on_tools` |
| G20 | `task.cancel` | `description` | 按 task_id 显式取消；跨进程可见；runs/.cancel/<task_id>.json；canceled；completed_before_cancel；cancel_requested；not_running；未发送信号；不会谎报已清理；request_state；pending；acknowledged；受控集合；cleanup_status；cleanup_claimed；handback | 一定已清理；保证终止整棵进程树；跨进程生效；跨进程不可见 | `description_gate_snapshot_matches_audit`、`description_gate_rows_hold_on_tools` |
| G21 | `task.cancel` | `inputSchema.properties.task_id.description` | 命名规则；派单标识 | 任意字符串 | `description_gate_snapshot_matches_audit`、`description_gate_rows_hold_on_tools` |
| G22 | `task.cancel` | `inputSchema.properties.requested_by.description` | 发起者；写入取消请求文件；缺省为 pid: | 必填 | `description_gate_snapshot_matches_audit`、`description_gate_rows_hold_on_tools` |

机器可读块（供用例解析；修改请连同 `tools_sha256` 一起重算，并保留 `description-gate` 标记）：

<!-- description-gate:begin -->
```json
{
  "version": "description-gate/1",
  "tools_sha256": "95062dad5bc83f4c19e2abb0adfb7010269975f3277445cfaf50d18bba78e316",
  "command": "cargo test -p aw-mcp-bridge description_gate",
  "rows": [
    {
      "id": "G1",
      "tool": "task.execute",
      "path": "description",
      "contains": ["受控集合（POSIX 进程组 / Windows Job Object）", "终止只尝试一次（不重试）", "仅当观测到集合内无存活进程才写「已清理受控集合」", "逃逸进程是否仍在运行未验证", "不承诺终止整棵进程树", "挂起态创建、先加入作业再恢复执行", "task.cancel", "status=canceled"],
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
      "contains": ["默认返回最新一次", "cleanup_status", "cleaned", "still_alive", "unavailable", "not_started", "只有 cleaned"],
      "not_contains": ["一定已清理"]
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
      "contains": ["runs/.inflight/<task_id>.json", "action=list", "stale_reason", "pid_alive", "pid_missing_grace", "deadline_expired", "unreadable", "dry_run=true", "confirm=true", "活跃标记永不删除", "pid+startedAt", "would_remove", "不进入 runs.prune 候选", "服务启动时一次", "默认每 3600 秒一次", "AW_INFLIGHT_SWEEP_SECS", "trigger=startup|periodic", "审计 plan 行写不进去则本轮零删除"],
      "not_contains": ["自动删除"]
    },
    {
      "id": "G19",
      "tool": "runs.inflight",
      "path": "inputSchema.properties.action.description",
      "contains": ["list", "clean", "stale=true"],
      "not_contains": ["默认 clean", "隐式删除"]
    },
    {
      "id": "G20",
      "tool": "task.cancel",
      "path": "description",
      "contains": ["按 task_id 显式取消", "跨进程可见", "runs/.cancel/<task_id>.json", "canceled", "completed_before_cancel", "cancel_requested", "not_running", "未发送信号", "不会谎报已清理", "request_state", "pending", "acknowledged", "受控集合", "cleanup_status", "cleanup_claimed", "handback"],
      "not_contains": ["一定已清理", "保证终止整棵进程树", "跨进程生效", "跨进程不可见"]
    },
    {
      "id": "G21",
      "tool": "task.cancel",
      "path": "inputSchema.properties.task_id.description",
      "contains": ["命名规则", "派单标识"],
      "not_contains": ["任意字符串"]
    },
    {
      "id": "G22",
      "tool": "task.cancel",
      "path": "inputSchema.properties.requested_by.description",
      "contains": ["发起者", "写入取消请求文件", "缺省为 pid:"],
      "not_contains": ["必填"]
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

三次都满足：`deleted_total + failed_total + trashed_total + lost_race_total + excluded_total == batch_candidates_total`，且 `candidates_total == batch_candidates_total + not_processed_total`（v1.44 起含 `lost_race`）。

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
- `capabilities` 新增 `timeout.descendant_snapshot_cross_check=true/proven/automatic=true`；`timeout.cleans_entire_process_tree` 与 `timeout.enumerates_escaped_descendants` 由 `not_proven` 改为 `holds=false/partial/posix_escaped_after_reparent_unobservable`；条目 13→14，`not_proven_total` 实数为 5（本节早期写的 4 为笔误）。

### 36.4 结论与仍然缺的那一段

- Windows：作业覆盖全部后代（`CREATE_SUSPENDED` 先入作业、恢复前不派生、本作业未设 `BREAKAWAY_OK`、嵌套作业仍在外层作业内），父链快照可枚举全部存活后代，终止后快照与 `ActiveProcesses` 同时为空 → **整树清理在 Windows 侧成立**。
- POSIX：`setsid` / 双 fork 换父后被 init/子收割者收养，父链断开；`setsid_escape_is_flagged_then_unobservable_after_reparent` 实测 `escaped_alive=true` 而快照 `descendants=[]`，即"快照为空 ≠ 无逃逸"。因此 POSIX **不能**宣称整树，`cleans_entire_process_tree` 只能保持 `holds=false`。
- 仍缺的边界：Linux 收养后的后代枚举（需 cgroup v2 委派或 subreaper + 启动时间全量扫描，均未实现——**cgroup v2 一条已在 v1.35 落地，见 §37**）；macOS 无 `/proc`；Windows PID 复用可能多报后代（保守方向）；观测是终止后采样而非内核级订阅。

## 37. v1.35：Linux cgroup v2 作用域观测（被收养的逃逸后代可见）

### 37.1 前提探测（先确认本机能不能走这条路）

```
$ stat -fc %T /sys/fs/cgroup
cgroup2fs
$ cat /sys/fs/cgroup/cgroup.controllers
cpuset cpu io memory hugetlb pids rdma misc dmem
$ id -u
0
$ mkdir /sys/fs/cgroup/aw-probe; sleep 5 & echo $! > /sys/fs/cgroup/aw-probe/cgroup.procs
$ cat /sys/fs/cgroup/aw-probe/cgroup.procs
<写入的 pid>
$ rmdir /sys/fs/cgroup/aw-probe
rmdir: failed to remove '/sys/fs/cgroup/aw-probe': Device or resource busy
```

结论：WSL Ubuntu 24.04 是 cgroup v2 单挂载、root 可建子 cgroup 并迁移进程；`rmdir` 失败仅因成员仍在（预期）。

### 37.2 红

临时让 `CgroupScope::prepare()` 恒返回 `Err("cgroup_v2_unavailable")`（模拟"建不起来"，运行后已逐字还原）：

```
thread 'server::process_scope::imp::tests::cgroup_scope_enumerates_adopted_escapee' panicked at apps/mcp-bridge/src/server.rs:
本机未能建立 cgroup v2 观测作用域：Some("cgroup_v2_unavailable")
[left: ProcessGroup / right: CgroupV2]
test result: FAILED. 0 passed; 1 failed
```

另有一条既有用例在实现过程中暴露为不稳定（`setsid_escape_is_outside_the_observed_group` 的 `original_group_empty=false`）：`sh` 写完 PID 就退出，`setsid` 尚未切到新会话。已改为轮询等待其真正离组（测试夹具修正，不改判据）。

### 37.3 绿

- Windows：`12 lib + 122 integration` 全绿。
- WSL Ubuntu 24.04（`cargo 1.88.0`）：`13 lib + 120 integration` 全绿；跑完 `ls -d /sys/fs/cgroup/aw-mcp-bridge-*` 计数为 0。

证据行（真实进程）：

```
[evidence] cgroup_scope opened root=936 scope=CgroupV2 escaped_scope_unobservable=None controlled=Some([936])
[evidence] cgroup_scope before escaped=945 scope=CgroupV2 descendants=[DescendantProcess { pid: 945, in_controlled_set: false }] outside=[945]
[evidence] cgroup_scope after outcome=StillAlive escaped_alive=true escaped_ppid=Some(740) root=936 descendants=[DescendantProcess { pid: 945, in_controlled_set: false }] controlled=Some([]) scope=CgroupV2
[evidence] setsid_snapshot after outcome=StillAlive escaped_alive=true scope=CgroupV2 escaped_scope_unobservable=None descendants=[DescendantProcess { pid: 943, in_controlled_set: false }]
[evidence] cgroup_unavailable reason=Some("cgroup_v2_unavailable")
[evidence] cgroup_degraded scope=ProcessGroup escaped_scope_unobservable=Some("cgroup_v2_unavailable") unobservable=None
```

读法：945 在根退出后被收养（`ppid=740 ≠ root=936`），父链看不到，但 cgroup 仍列出它；终止结果由 v1.34 的 `Cleaned` 修正为 `StillAlive`——修掉了一个假通过。

### 37.4 实现口径

- `DescendantSnapshot` 增 `scope: SnapshotScope{JobObject,CgroupV2,ProcessGroup}` 与 `escaped_scope_unobservable`。
- Linux 建 cgroup 在 `configure`，入组在 `pre_exec`（fork 后、exec 前写自身 PID，仅 `write(2)`，不分配不加锁）；`attach` 核对成员关系，失败补写一次，仍失败记 `cgroup_membership_not_established`。
- 后代集合 = 父链闭包 ∪ cgroup 成员 − 根；`controlled_pids` 恒为**终止作用域**（`pgrp`），`in_controlled_set=false` 即"可见但杀不到"。
- 降级不阻断执行，也不把整份快照置为不可用；`escaped_scope_unobservable` 必须带原因，`covers_escaped_descendants()` 为 false。
- 能力位：新增 `timeout.cgroup_v2_scope_observation=true/proven`（`platforms_verified=["linux"]`、`platforms_unverified=["windows","macos"]`）；`cleans_entire_process_tree` → `posix_escaped_descendant_observable_not_terminated`，`enumerates_escaped_descendants` → `posix_escaped_scope_requires_writable_cgroup_v2`，两者仍 `holds=false/partial`；条目 14→15，`not_proven_total` 仍为 5。
- `StillAlive` 回执删掉"换父被收养的逃逸进程是否仍在运行不可观测"这一定论（cgroup 下已可观测），改为"取决于观测作用域"，`tools/list` 描述未动，`tools_sha256` 不变。

### 37.5 仍然缺的那一段

- **终止未闭合**：`kill(-pgid)` 打不到 `setsid` 逃逸者，cgroup 只用来看。要闭合需 `cgroup.kill`（内核 ≥5.14，写 `1` 即整组 SIGKILL，成员关系保证不漏）或对成员逐个 SIGKILL；本版明确不做，故 `cleans_entire_process_tree` 保持 `holds=false`。
- cgroup 建不起来时（容器/非 root/cgroup v1）观测退回进程组，逃逸不可枚举，只能靠 `escaped_scope_unobservable` 告警。
- 外部可把进程移出 cgroup，移出后不可见。
- 成员未退出时 Drop 删不掉目录，会留空目录（不杀进程）。
- macOS 无 `/proc`、无 cgroup，恒不可观测。

## 38. v1.36：inflight 清扫的「检查后删除」并发边界（可观测判据 + 红→绿）

改动位置：`apps/mcp-bridge/src/server.rs` 的 `sweep_inflight_markers` 与新增 `restore_without_clobber`；不新增工具、不改 `tools/list` 描述、`tools_sha256` 不变。

### 38.1 先把可观测量说清（不先动实现）

清扫者对每个被判 `stale` 的标记执行：read（判定）→ `rename` 到 `.tmp`（认领）→ 读回复核 `task_id`+`pid`+`startedAt` → 一致则删、不一致则还原。

窗口内允许发生的两跳并发：

1. **W1** 在「检查」与「认领」之间用 `create_new`（用前一步的守卫删除后腾出的名字）替换同名文件 → 被 `rename` 带走的是 W1，而不是判定时的 R0；
2. **W2** 在「认领」与「还原」之间再次创建同名标记 → 原名位置被 W2 占住。

可观测量（唯一判据）：清扫结束后，`runs/.inflight/<task>.json` 的内容必须是 **W2 的完整记录**（`pid`+`startedAt` 与 W2 一致），`removed_total=0`，`kept[]` 含 `marker_changed` 或 `marker_superseded`，目录中不留 `.tmp`。

**不可证**：检查到认领之间没有窗口。文件系统没有 compare-and-rename，认领必然短暂空出文件名。故 `concurrency.check_then_delete_atomic` 不改判据，继续 `false/not_proven/check_then_delete_not_atomic`。

### 38.2 红（在改实现之前，HEAD 代码原样失败）

用例 `sweep_restore_never_clobbers_concurrent_new_marker` 通过 `#[cfg(test)]` 的 `SWEEP_AFTER_CLAIM_HOOK` 在认领与复核之间注入真实文件状态（副本=W1 的 `2098-…`、原名=W2 的 `2099-…`），避免靠竞态碰运气：

```
thread 'server::inflight_sweep_race_tests::sweep_restore_never_clobbers_concurrent_new_marker' panicked at apps\mcp-bridge\src\server.rs:6600:9:
assertion `left == right` failed: 还原不得覆盖并发写入的 W2（覆盖后会是 W1 的时间戳）：{"deadlineAt":"2099-01-01T01:00:00+00:00","mode":"dispatched","pid":194200,"startedAt":"2098-01-01T00:00:00+00:00","state":"active","task_id":"race","version":"inflight-marker/1"}
  left: String("2098-01-01T00:00:00+00:00")
 right: String("2099-01-01T00:00:00+00:00")
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.01s
```

读法：`fs::rename(tmp, path)` 把 W2 换成了 W1——一条活跃标记被改写成过期记录，等于给后来者发了「可以开跑」的错误信号。

第一次尝试的插入点错误（hook 放在认领之后、只写 W2）不会失败，因为那条路径删的是「判定时那一份」、不触碰新文件；红用例因此改成两跳注入，判据不变。

### 38.3 绿

- Windows：`13 lib + 122 integration` 全绿（含新 lib 用例）。
- WSL Ubuntu 24.04（`cargo 1.88.0`）：`14 lib + 120 integration` 全绿，日志 `/root/.aw-posix-verify/logs/posix-verify-20261107-195601.log`。
- `description_gate` 3 条全绿；`tools_sha256` 未变（未改 `tools/list`）。

### 38.4 实现口径

- `restore_without_clobber(source=tmp, target=原名)`：`fs::read` 取得被认领内容 → `OpenOptions::create_new(true)` 打开原名（O_EXCL）→ 写入并 `sync_all` → 删除 tmp；`AlreadyExists` 时**保留原名上的新标记**、丢弃手上副本，返回 `Superseded`（记 `marker_superseded`）；其他错误返回 `Err`（记 `marker_changed_restore_failed`），写失败时回滚自己刚创建的原名文件。
- `marker_changed` 语义收窄为「原名仍为空，已把副本放回」；不再出现「用 rename 覆盖」这一分支。
- `sweep_rule.atomic_delete` 文案写实；`sweep_rule.limits` 增加 `no_compare_and_rename_primitive`。
- 能力位：`concurrency.check_then_delete_atomic` 保持 `false/not_proven`；新增 `concurrency.sweep_restore_never_clobbers=true/proven`（`platforms_verified=["windows","linux"]`、`platforms_unverified=["macos"]`、`automatic=true`、`evidence=["sweep_restore_never_clobbers_concurrent_new_marker"]`）；条目 15→16，`not_proven_total` 仍 5。

### 38.5 仍然存在的边界

- **认领窗口**：`rename` 成功到还原/删除之间文件名是空的，第三方 `create_new` 可以成功；本版只能保证不覆盖它，不能保证它不出现。
- **`removed[]` 的读法**：`removed[].path` 只表示「本次删掉了这个文件名下的那份记录」，同一路径在回执写出时可能已有新标记（此时回调方应以 `runs.inflight action=list` 为准），不能据此断言该 task 当前无标记。
- **快照字段有限**：复核只比 `task_id`+`pid`+`startedAt`；三个字段全同的另一条记录无法与判定时区分（例如同进程、同时间戳的重复写入），仍按「一致」处理。
- macOS 未验证（`platforms_unverified=["macos"]`）。


## 39. v1.37：Linux `cgroup.kill` 闭合「超时终止整棵作用域」（可观测判据 + 红→绿）

改动位置：`apps/mcp-bridge/src/server.rs` 的 `process_scope::CgroupScope`（新增 `kill_supported` / `kill_members` / `populated`）与 `Supervisor::terminate`；不新增工具、不改 `tools/list` 描述、`tools_sha256` 不变。

### 39.1 先把可观测量说清（不先动实现）

v1.35 已能**观测** cgroup v2 作用域内被 init 收养的逃逸后代，但终止仍只发 `kill(-pgid)`，逃逸者存活（`outcome=StillAlive escaped_alive=true`）。v1.37 要回答的是「作用域是否真的空了」，判据定为四个观测量**同时**成立：

1. `cgroup.procs` 读为空；
2. `cgroup.events` 为 `populated 0`（与 (1) 互为独立观测点）；
3. 枚举到的逃逸 PID 已不存在（`/proc/<pid>` 消失）；
4. `DescendantSnapshot` 不再含后代，且 `unobservable=None`。

只有 (1)-(4) 全成立才允许返回 `CleanupOutcome::Cleaned`；成员仍在 → `StillAlive`；`cgroup.procs` / `cgroup.events` 读不出 → `Unavailable`。**降级口径不变**：cgroup 作用域建不起来（容器 / 非 root / cgroup v1）或内核没有 `cgroup.kill`（<5.14）时，退回进程组窄判据，逃逸者存活就必须报 `StillAlive`，绝不因为「看不到」而报已清理。

前提探测（WSL Ubuntu 24.04，内核 `6.18.33.2-microsoft-standard-WSL2`）：root cgroup 没有 `cgroup.kill`（`ls: cannot access ...: No such file or directory`），新建子 cgroup 后出现 `--w------- cgroup.kill`，`cgroup.events` 为 `populated 0 / frozen 0`。功能探针：`setsid sleep 300` 写入子作用域（`populated 1`）后写 `cgroup.kill`，`cgroup.procs` 变空、`populated 0`、`/proc/<pid>` 消失。fork 压力探针（13 个成员 + 持续 `sleep 60 &`）：重复写返回 0（幂等），写入后 `procs=0`、`populated 0`。

### 39.2 红（在改实现之前，HEAD 代码原样失败）

用例 `cgroup_kill_terminates_adopted_escapee` 复用 v1.35 的 `setsid` fixture（`pre_exec` 在 exec 前把自身 PID 写进 `cgroup.procs`，无「先派生后入组」窗口），先断言本机确实建起 `CgroupV2` 且 `cgroup.kill` 存在，再在终止后核对四个观测量：

```
[evidence] cgroup_kill after outcome=StillAlive escaped_alive=true members=Ok([1135]) populated=Ok(1) descendants=[DescendantProcess { pid: 1135, in_controlled_set: false }] unobservable=None
thread 'server::process_scope::imp::tests::cgroup_kill_terminates_adopted_escapee' panicked at apps/mcp-bridge/src/server.rs:3658:17:
逃逸后代必须在作用域终止后不再存活
test result: FAILED. 14 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

读法：逃逸者仍被 cgroup 枚举（`members=[1135]`、`populated 1`），但旧终止只打进程组，因此「看得见却杀不掉」——红用例把这一点固化成可执行判据。

### 39.3 绿

- WSL Ubuntu 24.04（`cargo 1.88.0`）：`15 lib + 120 integration` 全绿，日志 `/root/.aw-posix-verify/logs/posix-verify-20261107-204702.log`；证据行 `[evidence] cgroup_kill after outcome=Cleaned escaped_alive=false members=Ok([]) populated=Ok(0) descendants=[] unobservable=None`。
- Windows：`13 lib + 122 integration` 全绿。首次全量时 `timeout_cleans_controlled_set_including_grandchild` 在并发负载下因 8s 窗口抖动失败（`StillAlive`），单跑与二次全量均通过；该用例走 Windows Job Object 路径，与本轮 Linux-only 改动无关。
- `description_gate` 3 条全绿；`tools_sha256` 未变（未改 `tools/list`）。

### 39.4 实现口径

- `CgroupScope::kill_supported()`：判断作用域内是否存在 `cgroup.kill`（root cgroup 永远没有）。
- `CgroupScope::kill_members()`：写 `1` 到 `cgroup.kill`；只负责发信号，是否清空由 `cgroup.procs` / `cgroup.events` 独立复核。
- `CgroupScope::populated()`：解析 `cgroup.events` 的 `populated N`；解析失败 / 文件缺失返回 `Err`，调用方转 `Unavailable`。
- `Supervisor::terminate()`：进程组 SIGTERM → 宽限 → SIGKILL → `child.wait()` 之后，若作用域 `Active` 且 `kill_supported()`，每轮轮询重试写 `cgroup.kill`（fork 压力下首轮可能漏掉正在派生的后代；重复写幂等），直至进程组空 + 作用域空 + `populated 0` + 快照空，才 `Cleaned`。
- 能力位：新增 `timeout.cgroup_kill_terminates_scope=true/proven`（`platforms_verified=["linux"]`、`platforms_unverified=["windows","macos"]`、`automatic=true`、`evidence=["cgroup_kill_terminates_adopted_escapee"]`）；`timeout.cleans_entire_process_tree` **不上调**，仍 `false/partial`，reason 改为 `posix_escaped_scope_requires_cgroup_v2_and_kill`，并增加 `platforms_unverified=["macos"]`；`timeout.cgroup_v2_scope_observation` 保持 `proven`，detail 改为「只声明观测能力」。条目 16→17，`not_proven_total` 仍 5。
- 两条 v1.35 旧用例按新事实分支（未放宽）：`cgroup.kill` 可用 → 断言逃逸者已死 + `Cleaned`；不可用 → 断言仍存活 + `StillAlive` 且可枚举。`cgroup_scope_enumerates_adopted_escapee` 先只杀进程组以保留「换父后仍可枚举」的观测证据，再跑完整 `terminate` 验证作用域终止。

### 39.5 仍然存在的边界

- **内核/权限依赖**：`cgroup.kill` 需要内核 ≥5.14 且非 root cgroup 可写；容器内常用只读或未委派，此时终止仍只打进程组，逃逸者存活并如实报 `StillAlive`。
- **外部迁移**：有权限的外部进程仍可把成员移出 cgroup（无委派锁），移出后不在作用域内。
- **fork 竞态**：`cgroup.kill` 只杀写入时的当前成员；本版靠轮询重试兜底，但「杀完瞬间再派生」的极端场景仍可能拖到 `StillAlive`，不会误报 `Cleaned`。
- **Drop 残留**：作用域目录在成员仍存活时删不掉（不杀进程），会留下空目录。
- macOS 无 cgroup、无 `/proc`，恒不可观测（`platforms_unverified=["macos"]`）。

## 40. v1.38：POSIX rename 活跃 attempt 目录语义闭合（可观测判据 + 红→绿）

改动位置：`apps/mcp-bridge/src/server.rs` 的 Linux 单测 `process_scope::imp::tests::posix_rename_active_attempt_semantics` 与 `capabilities_payload` 的 `platform.posix_inflight_semantics_verified` 条目；`apps/mcp-bridge/tests/mcp_contract.rs` 的 `capabilities_declares_machine_readable_boundaries` 断言；`scripts/posix-timeout-verify/verify.sh` 的退出码取值。不新增工具、不改 `tools/list` 描述、`tools_sha256` 不变。

### 40.1 先把可观测量说清（不先动实现）

`runs.prune` 的 `delete_attempt_dir` 是「`fs::rename(attempt-<N> → .prune-trash/<batch>/<task>-attempt-N)` 成功后 `remove_dir_all`」。要证的是 POSIX 上这一步在「目录里还有活跃进程」时仍然成立，而不是凭 v1.30 的 Windows 观测外推。判据 5 条同时成立：

1. 进程 `cwd` 指向 attempt 目录、且持有目录内文件 fd 时，`rename` 返回 `Ok`；
2. `rename` 后 `/proc/<pid>/cwd` 指向 trash 新路径；
3. 该进程已打开的 fd 仍能读到原内容（换路径不打断已打开的 fd）；
4. 旧路径不存在、新路径存在；
5. 随后对 trash 路径 `remove_dir_all` 返回 `Ok`，进程仍存活，且 fd 仍能读到原内容（unlink 后已打开 fd 仍有效）。

任一不成立就不允许把 claim 上调。

### 40.2 红（HEAD 契约断言原样失败）

用例本身是对 OS 语义的断言（新代码，不存在"先红"）；红由能力位断言承担——先改 `mcp_contract.rs` 期望 `holds=true/proven/null`，在未改 `server.rs` 的 HEAD 上跑 WSL：

```
thread 'capabilities_declares_machine_readable_boundaries' panicked at apps/mcp-bridge/tests/mcp_contract.rs:6391:5:
assertion `left == right` failed: {"automatic":false,"detail":"POSIX 与 Windows 均跑通 inflight 标记、stale 判定、prune 跳过与清理保护用例；POSIX 上 rename 活跃 attempt 目录的语义未单独构造复核，macOS 未验证。","evidence":["standalone_inflight_marker_is_visible_then_removed","prune_skips_task_with_live_inflight_marker","runs_inflight_lists_and_cleans_only_stale_markers"],"holds":false,"id":"platform.posix_inflight_semantics_verified","platforms_unverified":["macos"],"platforms_verified":["windows","linux"],"reason_code":"posix_unverified","scope":"POSIX 上的 inflight create_new、stale 判定、prune 跳过与标记删除保护","status":"partial"}
  left: Bool(false)
 right: true
test capabilities_declares_machine_readable_boundaries ... FAILED
test result: FAILED. 119 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.30s
```

同一轮里新用例已通过（说明 POSIX 语义本身成立，红的只是"未宣称"）：

```
[evidence] posix_rename before cwd="/tmp/aw-posix-rename-1142-1794058543881886029/t-task/attempt-1" fd3=".../attempt-1/keep.txt"
[evidence] posix_rename rename result=Ok(()) old_exists=false new_exists=true
[evidence] posix_rename after cwd=".../.prune-trash/b-1/t-task-attempt-1" fd3=".../.prune-trash/b-1/t-task-attempt-1/keep.txt" content="original" old_exists=false new_exists=true
[evidence] posix_rename remove result=Ok(()) alive=true fd_content_after_delete=Some("original")
test server::process_scope::imp::tests::posix_rename_active_attempt_semantics ... ok
```

### 40.3 绿

- 能力位改为 `holds=true/status=proven/reason_code=null`，`evidence` 增加 `posix_rename_active_attempt_semantics`，detail 写实；`platforms_verified=["windows","linux"]`、`platforms_unverified=["macos"]` 均不变；条目仍 17，`not_proven_total` 5→4。
- WSL Ubuntu 24.04（`cargo 1.88.0`）：`16 lib + 120 integration` 全绿，日志 `/root/.aw-posix-verify/logs/posix-verify-20261107-213638.log`；契约断言通过。
- Windows：`13 lib + 122 integration` 全绿。首次全量时 `descendant_snapshot_cross_checks_job_accounting` 报 `StillAlive`（`after terminate=StillAlive descendants=[] controlled=Some([])`），当时按并发抖动记录；**v1.38.1 专项复核证明该判断错误**——不是窗口太紧，而是父链闭包在 PID 复用下把无关进程树误判为后代（复现证据与结论见 §41）。本轮 Linux-only 改动路径与此无关，但「抖动」这一旧口径已作废。
- `cargo fmt -p aw-mcp-bridge -- --check` 对本轮改动无差异（`build.rs` / `main.rs` 的既有差异未触碰）。

### 40.4 实现口径

- 新用例用 `sh -c 'cd <attempt> && exec 3<keep.txt && echo ready > <ready> && sleep 30; :'` 固定「cwd 在目录内 + 持有文件 fd」两个条件，`ready` 文件只用于同步，随后逐条断言 40.1 的 (1)-(5)。
- 能力位只上调 `platform.posix_inflight_semantics_verified` 一条；`timeout.cleans_entire_process_tree`、`timeout.enumerates_escaped_descendants`、`concurrency.check_then_delete_atomic`、`inflight.stale_markers_auto_swept` 全部保持原 `holds=false`，reason 不变。
- `verify.sh`：`local rc=${PIPESTATUS[0]}`。原写法先执行 `local rc` 再读 `PIPESTATUS`，读到的是 `local` 自身的 0，导致本轮红阶段 integration 已 `1 failed` 而脚本仍输出 `[INFO] 四类 POSIX 用例全部通过`。

### 40.5 仍然存在的边界

- 该用例证明的是 POSIX 文件系统语义与 `delete_attempt_dir` 的 rename-then-delete 假设，不证明 macOS（`platforms_unverified=["macos"]` 保留）。
- 仍不等于「prune 与执行无并发问题」：`concurrency.check_then_delete_atomic` 的 TOCTOU 仍 `holds=false`；本条只说明「rename/删除不会被活跃进程的 cwd/fd 阻止」。
- Windows 仍走 `RetainedAtSource`（v1.30 实测 rename 被拒 `WinError 5`），本条不上调 Windows 相关结论。

## 41. v1.38.1：修复脚本后的 v1.33–v1.38 复跑（无假绿）与 Windows 父链快照 PID 复用缺陷

### 41.1 复跑方法与结果

`verify.sh` 原缺陷：`local rc` 单独一行执行后，下一行 `rc=${PIPESTATUS[0]}` 读到的是 `local` 自身的 0，测试失败仍打印「四类 POSIX 用例全部通过」。修复为 `local rc=${PIPESTATUS[0]}`（赋值展开发生在 `local` 执行之前）。

用**同一份修复版脚本**在六个干净 worktree 上逐个重跑（WSL Ubuntu 24.04，`AW_VENDOR=/root/vendor`，共享 `AW_WORK` 复用依赖缓存）：

| 版本 | commit | 退出码 | lib | integration | 关键用例 | 当时结论 |
|---|---|---|---|---|---|---|
| v1.33 | `6598d09` | 0 | 9 | 120 | `inflight_sweep_on_task_start_removes_only_target_stale_marker`、`inflight_sweep_on_prune_removes_stale_marker_and_unblocks_candidate` 均 ok | 成立 |
| v1.34 | `52b03b4` | 0 | 11 | 120 | `descendant_snapshot_cross_checks_process_group`、`timeout_cleans_controlled_set_including_grandchild` 均 ok | 成立（范围见 §41.3） |
| v1.35 | `6cb5e69` | 0 | 13 | 120 | `cgroup_scope_enumerates_adopted_escapee` ok | 成立 |
| v1.36 | `37dad57` | 0 | 14 | 120 | `sweep_restore_never_clobbers_concurrent_new_marker` ok | 成立 |
| v1.37 | `94aca63` | 0 | 15 | 120 | `cgroup_kill_terminates_adopted_escapee` ok | 成立 |
| v1.38 | `07cf253` | 0 | 16 | 120 | `posix_rename_active_attempt_semantics` ok | 成立 |

结论：**这六版没有发现假绿**。旧脚本只在「测试已失败但四类证据行仍齐全」时才可能误报成功；复跑中没有任何 `FAILED`/`panicked` 行，证据行也齐全。该复跑只覆盖 Linux（脚本定位即 POSIX 超时清理）；Windows 结论不在其覆盖内，macOS 仍未验证。

### 41.2 Windows `descendant_snapshot_cross_checks_job_accounting` 的 `StillAlive` 定性

先定判据，避免把「窗口紧」和「观测错」混为一谈：

- 若 3s `CLEANUP_DEADLINE` 太紧 → `terminate` 耗时应接近 3000ms，且届时 `active_processes()>0`（作业里确实还有活进程）；
- 若观测层误判 → 耗时应远小于 3s，`active_processes()==Some(0)`，但父链快照仍非空。

复现：24 路 CPU 负载下重复全量 lib（第 9 次命中，`FAILED. 13 passed; 1 failed`）。临时探针现场（已还原，未入库）：

```
[probe] race round=102 root=165548 active=Some(0) snapshot=[DescendantProcess { pid: 127140, in_controlled_set: false }, ... 共 20 个 ...]
[probe]   pid=127140 ppid=Some(165548) exe=WXWork.exe
[probe]   pid=28132  ppid=Some(127140) exe=WeChatAppEx.exe
[probe]   pid=...    ppid=Some(127140) exe=crashpad_handler.exe / conhost.exe
[probe] root_rows=[(127140, 165548), (28132, 127140), ...]
[probe] slowest_ms=1392
```

判定：**不是测试窗口太紧，是父链闭包的真实缺陷**。

1. `active=Some(0)` 说明作业成员已全部终止；`slowest_ms=1392 < 3000`，排除 deadline。
2. 快照把这 20 个进程算成 `165548` 的后代，而 `165548` 是本轮新建 `cmd` 的 PID；`WXWork.exe` 的 `ParentProcessID=165548` 是它**创建时**的父进程，那个父进程早已退出，Windows 随后把同号 PID 复用给了本轮 `cmd`。
3. 因此 `descendants_of` 按 `ppid` 匹配时把无关进程树整棵连了进来；另一轮复现是单进程 `pid=92344 ppid=Some(137288)`（root），机制相同。

影响：
- `terminate` 会误报 `StillAlive`（保守，但不是真实存活后代）；
- `in_controlled_set=false` 会把无关进程误标为「逃逸后代」，即 `timeout.enumerates_escaped_descendants` 的 Windows 枚举只能作旁证；
- `timeout.cleans_entire_process_tree` 的 Windows 侧只能证「作业记账归零」，不能证「父链快照为空」。

### 41.3 能力位处置

`timeout.descendant_snapshot_cross_check` 的 detail 原本就写明「不覆盖换父后被收养、**PID 复用**与集合未能建立的情形」，本轮实测落在其声明的排除项内，故**不新增降级**；但把契约 §2.7 的边界栏补上「PID 复用不在覆盖内（实测会误连）」，并把 v1.38 里「并发抖动」的旧口径作废。测试本身仍按确定性 `Cleaned` 断言，未放宽；v1.39 已在实现侧修复该缺陷（见 §42），测试判据保持不变。

### 41.4 修复方向（v1.39 已实现，见 §42）

父链闭包加边校验：只接受「子进程创建时间 ≥ 父进程创建时间」的 `ppid` 边（`OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `GetProcessTimes`）。PID 复用时「旧父链的孩子」创建时间早于新持有者，该边会被拒绝，从而消除这类误连。v1.39 已实现并补可注入进程表的确定性单测，见 §42。



## §42 v1.39：Windows 父链闭包的 PID 复用边校验（2026-11-07）

### 42.1 判据

只回答一件事：`snapshot_descendants` 枚举出的 PID 是否**真的**属于本次任务。为此进程表改为可注入的 `ProcEntry { pid, parent_pid, created_at, alive }`，闭包加三道过滤：

1. **创建时间下界**：每条边要求「子进程创建时间 ≥ 父进程创建时间」。旧父进程退出后同号 PID 被新进程持有，旧子进程残留的 `ParentProcessID` 会把无关树挂到新根上；时间顺序能拒绝这条边。
2. **根退出时间上界**：只对「根 → 直接子进程」生效，要求「子创建时间 ≤ 根退出时间」。根 PID 被后来者复用后，后来者的子进程创建时间晚于根，仅靠下界无法拒绝；退出时间从子进程句柄读 `GetProcessTimes`（不经 PID），不受复用影响。
3. **存活/存在性剔除**：`GetExitCodeProcess` 判为已终止，或 `OpenProcess` 返回 `ERROR_INVALID_PARAMETER`（PID 已不存在，Toolhelp 枚举与逐个打开之间的残行）的行直接剔除。

任一时间/存活状态读不到（权限不足等）时保留该行，维持旧行为——宁可保守报仍有后代，也不把无法鉴定的进程当成已清理。

### 42.2 红→绿

红（临时把 `edge_respects_creation_order` 改成恒 `true`）：

```
[evidence] pid_reuse rows=6 result=[900, 901, 902, 903, 905, 906]
thread 'server::process_scope::imp::tests::pid_reuse_parent_chain_edge_is_rejected' panicked:
  left: [900, 901, 902, 903, 905, 906]
 right: [902, 903, 906]
test result: FAILED. 0 passed; 1 failed
```

红（临时把 `child_created_before_root_exit` 改成恒 `true`）：新用例红出 `result=[902, 903, 906, 907]`，`907` 是根退出后复用者的子进程。

绿：`[evidence] pid_reuse rows=7 result=[902, 903, 906]`、`[evidence] terminated_rows rows=4 result=[912, 913]`，`test result: ok. 15 passed`；补 `create_evidence_error` 单测后 `17 passed`。

### 42.3 24 路负载复跑（判据未改）

- **lib 单测最终版：连跑 20 轮全绿**（含 `descendant_snapshot_cross_checks_job_accounting`、`suspended_start_defers_execution_until_job_assign`），未再出现 `StillAlive`。
- **全量 4 轮**（`--no-fail-fast`）：每轮 `15 lib` 全绿 + integration `120 passed / 2 failed`，两个失败均与父链快照无关：
  1. `timeout_cleans_controlled_set_including_grandchild`：同一负载下实测 PowerShell 从启动到写出 `pids.txt` 需 **10.7–16.2 s**，用例窗口 8 s，属夹具启动延迟；不放宽。
  2. `standalone_active_attempt_deleted_is_reported_not_silent_ok`：目录在创建 `stdout.txt` 前已被删掉，报出裸错误 `创建 stdout 失败：系统找不到指定的路径。 (os error 3)`，而非具名 `attempt_dir_lost`。本轮补 `create_evidence_error`（`NotFound` 且目录不存在 → `attempt_dir_lost`），并加确定性单测。

中途取证（修复前、负载下）确认误报不是 3 s 窗口太紧：

```
[probe] cross_check root=26468 elapsed_ms=396 active_after=Some(0) recorded_root_created=Some(134385355652414131) current_root_row=None edges=[]
[probe] suspended_start root=39580 elapsed_ms=384 active_after=Some(0) edges=[29456@Some(134385358588524392)]
```

`elapsed_ms` 远小于 3000，说明走的是「记账归零 → 看快照」分支；`edges` 在探测瞬间又为空，指向 Toolhelp 枚举与逐个打开之间的残行（`OpenProcess` 失败被当成「时间不可读 → 接受」）。这就是第 3 道过滤要解决的形态。

最终回归：Windows `17 lib + 122 integration` 全绿；WSL Ubuntu 24.04 `18 lib + 120 integration` 全绿（日志 `/root/.aw-posix-verify/logs/posix-verify-20261107-231315.log`，`standalone_active_attempt_deleted_is_reported_not_silent_ok` 在 WSL 与 Windows 均为 `ok`）。

### 42.4 `cleans_entire_process_tree` 在 Windows 侧能证/证不到

**能证**：

- 终止后作业记账 `ActiveProcesses` 归零（`JobObjectBasicAccountingInformation`）；
- 父链快照在创建时间与存活状态可读时，不再把 PID 复用树或枚举残行算作后代（§42.1 三道过滤，确定性单测覆盖六类边）。

**证不到**：

- 作业外进程（`BREAKAWAY`、`AssignProcessToJobObject` 失败）——例如由 csrss 创建、不进作业的 `conhost.exe`；它若在终止瞬间仍存活，快照报 `StillAlive` 是**真实观测**而非误报；
- 换父后被收养的进程是否仍存活（Windows 无 cgroup 等价的作用域观测）；
- 创建时间或存活状态读不到时的同号 PID 复用。

因此该条仍 `holds=false/partial`，父链快照只作旁证，不因 Windows 单侧修复上调。

### 42.5 未改动的边界

Linux `descendants_of` 仍用 `/proc/<pid>/stat` 的 `ppid` 闭环，未加创建时间边校验（本轮用户明确限定 Windows 侧）——**这条已在 v1.40 补上，见 §43**；macOS 仍未验证。

## §43 v1.40：Linux 父链边界校验 + Windows 孙进程超时夹具改造（2026-11-07）

### 43.1 Linux 判据与实现

只回答一件事：`/proc` 枚举出的 PID 是否真的属于本次任务。

1. `ProcRow` 增 `start_time: u64`，取自 `/proc/<pid>/stat` 第 22 字段 `starttime`（单位 clock tick）。`comm` 可含空格与括号，按**最后一个 `)`** 切分后再数字段：`state ppid pgrp ...`，跳过 `session..itrealvalue`（字段 6..=21）后取 `starttime`。
2. `descendants_of(root, root_start_time, rows)` 对闭包里的**每条边**要求「子进程启动时刻 ≥ 父进程启动时刻」。旧父进程退出后同号 PID 被新进程持有，旧子进程残留的 `ppid` 边（子创建早于新父）被拒绝；祖先一旦被拒，其后代因不可达而不计入。
3. `Supervisor::attach` 记录 `root_start = proc_start_time(child.id())`，作为「根 → 直接子进程」这条边的下界；调用点把 `self.root_start` 传给 `descendants_of`。
4. **逐边放宽**：只有「这条边的父进程启动时刻读不到」时才退回旧行为（接受该边）；不因一个未知时刻放开整棵树。与 v1.39 Windows 侧「时间读不到则保守保留该行」同一口径。

确定性单测 `pid_reuse_parent_chain_edge_is_rejected`（Linux，注入 `ProcRow` 表，不依赖真机碰运气）覆盖四类边：

- 根级复用：旧树残留的 `ppid=900` 边早于新根启动时刻 → 拒绝；
- 祖先被拒后其后代不可达（`904` 未出现在结果里）；
- 中间层复用：`902→905`（905 早于 902）→ 拒绝，但 `902` 自身合法（根边成立）；
- 根启动时刻不可读（`None`）：只放宽根出发的边，`902→905` 仍被拒。

```rust
assert_eq!(descendants_of(root, Some(5000), &rows), vec![902, 903, 907]);
assert_eq!(descendants_of(root, None, &rows), vec![900, 901, 902, 903, 907]);
assert!(!edge_respects_start_order(Some(5000), 1000));
assert!(edge_respects_start_order(Some(5000), 6000));
assert!(edge_respects_start_order(None, 1));
```

### 43.2 红→绿原文

红（临时停用 `edge_respects_start_order` 守卫，WSL Ubuntu 24.04）：

```
[evidence] linux_pid_reuse rows=6 result=[900, 901, 902, 903, 905, 907]
thread 'server::process_scope::imp::tests::pid_reuse_parent_chain_edge_is_rejected' panicked at
apps/mcp-bridge/src/server.rs:3205:17:
assertion `left == right` failed
  left: [900, 901, 902, 903, 905, 907]
 right: [902, 903, 907]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 18 filtered out
```

绿：WSL `19 lib + 120 integration` 全绿（`/root/.aw-posix-verify/logs/posix-verify-20261108-004812.log`），Windows `17 lib + 122 integration` 全绿。

首次绿灯时断言写错（期望 `[900,901,902,903,905,907]`），实际 `[900,901,902,903,907]`。原因是把「根启动时刻读不到」理解成「整棵树退回旧行为」，而实现是逐边放宽——根未知只放宽根出发的边，深层 `902→905` 仍按已知时刻校验。修的是断言与注释，判据与实现未动。

### 43.3 Linux 侧能力位：**没有变化**

- `timeout.cleans_entire_process_tree`：仍 `holds=false` / `partial` / `posix_escaped_scope_requires_cgroup_v2_and_kill`。边校验只回答「枚举到的 PID 是否属于本任务」，不解决「逃逸到本作用域外的成员杀不到」。
- `timeout.descendant_snapshot_cross_check`：仍 `proven`。
- `timeout.enumerates_escaped_descendants`：仍 `holds=false` / `partial`，枚举范围仍受 cgroup 是否可写限制。
- 条目仍 17，`not_proven_total` 仍 4。只在三条的 detail 补边校验说明（§2.7），`holds`/`status`/`reason` 与 `evidence` 字段均未改。

### 43.4 Windows 孙进程超时夹具改造（`timeout_cleans_controlled_set_including_grandchild`）

旧夹具用 PowerShell 起进程并写 `pids.txt` 作为观测前提；24 路负载下「PowerShell 启动 → 写出文件」实测 10.7–16.2 s，与「进程树是否被清理」是两件事，故把观测前提改成不经 PowerShell、不落盘：

1. 把 `System32\PING.EXE` 复制为工作目录内的唯一命名 `aw-timeout-grandchild.exe`（`fs::hard_link` 被安全策略拒绝，改 `fs::copy`），窗口外先按相同形态跑一次短命版本预热。
2. 被测命令 `aw-timeout-grandchild.exe -n 30 127.0.0.1 > NUL`（`ping -n 30` 实测 29.36 s 不会自行退出）。
3. 孙进程 PID 由**测试进程自己的 Toolhelp 轮询线程**观测（每 20 ms 一次 `CreateToolhelp32Snapshot`，按镜像名匹配），不写文件、不经解释器；并记录 `first_poll_ms`、`polls`、`snapshot_failures`、`max_rows`、PID 集合变化时间线（`grandchild_timeline`）。
4. 判据不放宽：`status=="blocked"`、`reason` 含「已清理受控集合」、**必须观测到孙进程**（否则视为空跑失败）、调用返回后孙进程零存活。

24 路 CPU 负载（16 逻辑核，24 个 `while($true){}` 燃烧进程 + 循环跑 `cargo test`）实测：

| 场景 | 结果 | 证据 |
| --- | --- | --- |
| 空载 | 绿 | `pids=[181756] first_seen_ms=624 payload_duration_ms=8061 after_cleanup=Some([])`；`grandchild_timeline [(623,"[181756]"),(8397,"[]")]` |
| 24 路负载，只跑该用例 ×9 轮 | 9 绿 | 窗口内均观测到孙进程，调用返回时 `after_cleanup=Some([])` |
| 24 路负载，全量 3 轮 | 2 绿 1 红 | 绿轮 `first_seen_ms=14676 / 13740`、`payload_duration_ms=8324 / 8412`、`after_cleanup=Some([])` |

红轮原文（第 3 轮）：

```
[evidence] grandchild_watch pids=[] first_seen_ms=0 first_poll_ms=32 call_ms=40082
payload_duration_ms=9292 polls=442 snapshot_failures=0 max_rows=682 file_exists=true
after_cleanup=Some([]) process_exists=[]
[evidence] grandchild_timeline []
thread '...timeout_cleans_controlled_set_including_grandchild' panicked at
apps/mcp-bridge/tests/mcp_contract.rs:2851:
超时窗口内必须观测到孙进程 aw-timeout-grandchild.exe 启动，否则用例是空跑
```

判读（为什么这条红不是终止逻辑）：

- 红轮里**窗口内一个孙进程都没出现**（`timeline` 为空、`first_seen_ms=0`），清理仍然成功：`after_cleanup=Some([])`、零存活、`reason` 含「已清理受控集合」。没有「杀不掉/杀漏」的现象。
- 同一轮工具自身在窗口外的前/后置工作（两次 `git` 子进程 + 记录写入）合计约 30.8 s（`call_ms=40082 − payload_duration_ms=9292`），即当轮**创建一次进程**就要 ~10–15 s；8 s 窗口装不进「`cmd` → 孙进程」两级创建。
- 与之对照，24 路负载下单独量测「挂起创建 → 入作业 → 恢复执行」（`spawn_ms=347/444/624`、`attach_ms=404/634/297`，临时探针，已删除）只需 0.3–0.6 s，说明窗口不是被 `attach` 吃掉的常态，而是被整机进程创建延迟吃掉的偶发态。

结论：这条红是「前提（窗口内完成两级进程创建）在极端负载下无法建立」，**不是**夹具的 PowerShell 冷启动（已移除），也**不是**终止逻辑缺陷；未放宽窗口、未改判据、未改实现。

据此选择**方案 A（窗口保持 8s）**，依据就是上面的原始数据：红轮 `grandchild_timeline []` 说明窗口内没有任何孙进程可供观测；`call_ms=40082` 对 `payload_duration_ms=9292` 说明窗口外光前置/后置工作就花了 30.8s，当轮单次进程创建已达 10–15s；而 `spawn+attach` 只要 0.3–0.6s，排除「窗口被挂起创建/恢复吃掉」。三者合起来说明问题在「整机进程创建速度」，不在超时终止本身。

#### 43.4.1 日志里的判定结果（与断言失败区分）

用例在断言前先打印一行 `[verdict]`，把四种结局显式区分开，红了第一眼就能分清是产品坏了还是机器太慢：

| `grandchild_case=` | 含义 | 是否产品判据 |
| --- | --- | --- |
| `pass` | 窗口内观测到孙进程，调用返回后零存活 | 通过 |
| `terminate_defect` | 观测到孙进程，但调用返回后仍有存活 | **是**（产品缺陷） |
| `premise_not_established` | 窗口内没观测到孙进程，清理本身成功（零存活） | 否（环境/负载，该轮既不能证明也不能否证） |
| `unobservable` | 连进程表快照都拿不到，无法判断 | 否（观测失败） |

对应的断言文本也分开了：`terminate_defect` 的断言前缀是「终止逻辑缺陷（产品判据）」；`premise_not_established` 的断言前缀是「前提未建立（环境判定，不是终止逻辑判据）」，并指向 `[verdict]` 行。

判决链路做过实测：临时把观测镜像名换成不存在的名字（命令不变，模拟"窗口内没有孙进程"），原文输出

```
前提未建立（环境判定，不是终止逻辑判据）：超时窗口内没有观测到孙进程 aw-timeout-grandchild.exe，
见 [verdict] grandchild_case=premise_not_established；该轮既不能证明也不能否证整树清理能力
[verdict] grandchild_case=premise_not_established observed=0 survivors=Some([]) call_ms=1420 ...
test result: FAILED. 0 passed; 1 failed
```

改回后 `[verdict] grandchild_case=pass observed=1 survivors=Some([]) call_ms=8431 payload_duration_ms=8048`。另把窗口临时调到 1s 仍是 `pass`（孙进程 0.5s 内出现），说明判决只看「窗口内有没有孙进程」，与窗口长短无关。

### 43.5 覆盖边界（裁决 A，明确写出）

**未被验证的场景**：`全量并发 × 24 路 burner`。在该场景下「孙进程能否在 8s 超时窗口内创建」这一前提无法稳定建立（实测当轮单次进程创建 10–15s），因此**该场景下「超时终止是否清掉整棵树」的能力未被验证**——红轮既不能证明也不能否证它。这不是「已验证通过」，也不是「产品缺陷」。

**已成立、可作为结论的两条**：

1. `单用例 × 24 路 burner`：9 轮全绿，孙进程 0.6–2.4s 出现在窗口内，调用返回时 `after_cleanup=Some([])`、零存活；
2. `空载全量`：Windows `17 lib + 122 integration` 全绿，该用例窗口内观测到孙进程并零存活。

该用例**不进**「全量并发 × 24 路 burner」矩阵；窗口、判据、实现均不放宽。能力位口径不变（§43.6）。

### 43.6 未改动的边界

- `tools/list` 描述未改，`tools_sha256` 不变（`docs/TOOLS-DESCRIPTION-AUDIT.md` 的 `include_str!` 门禁通过）。
- macOS 侧父链边校验仍未验证，能力位不因 Linux 单侧修复上调。

## §44 v1.41：Linux 整树清理的证成范围与降级路径（2026-11-08）

只回答两件事：(1) 满足条件的环境里，「超时终止清掉整个受控作用域」到底能不能证成；(2) `cgroup.kill` 缺失或作用域建不起来时，行为、报告与「有没有可能悄悄漏杀」。

### 44.1 结论先行

- **能证成，但只在满足条件的分支**：cgroup v2 可写且内核提供 `cgroup.kill`（>= 5.14）时，Linux 侧终止的是**整个作用域**，且用真实 `setsid` 逃逸进程（被收养、进程组打不到）实测：终止后 `cgroup.procs` 为空、`cgroup.events populated 0`、逃逸进程已不存在、父链快照无后代，四项同时成立才报 `Cleaned`。这是「整作用域」，不是「整棵任意逃逸树」——逃出作用域（主动迁移 cgroup、无权限写作用域）的进程不在判据内。
- **跨平台能力位不上调**：`timeout.cleans_entire_process_tree` 保持 `holds=false/partial`。同一字段还要覆盖 Windows（作业外进程结构性证不到）与 macOS（未验证）；Linux 侧的条件性能力已由 `timeout.cgroup_kill_terminates_scope=true/proven` 单独表达。把跨平台三态上调会把 Windows 的结构性缺口一起说成「已证」。
- **reason 不再笼统**：由 `posix_escaped_scope_requires_cgroup_v2_and_kill` 改为 `processes_outside_controlled_scope_may_survive`，点明所有平台共同的残余卡点——**受控作用域之外的进程可能存活**；`statement`/`detail` 拆成「有 kill / 无 kill / 作用域建不起」三分支。

### 44.2 WSL 真实进程证据

环境：WSL Ubuntu 24.04，root，cgroup v2，内核 `6.18.33.2-microsoft-standard-WSL2`；日志 `/root/.aw-posix-verify/logs/posix-verify-20261108-013424.log`。用例 `cgroup_kill_terminates_adopted_escapee` 原文：

```
[evidence] cgroup_kill opened root=2122 scope=CgroupV2 escaped_scope_unobservable=None cgroup_kill_supported=true
[evidence] cgroup_kill before escaped=2127 scope=CgroupV2 descendants=[DescendantProcess { pid: 2127, in_controlled_set: false }]
[evidence] cgroup_kill after outcome=Cleaned escaped_alive=false members=Ok([]) populated=Ok(0) descendants=[] unobservable=None
```

配套的枚举用例 `cgroup_scope_enumerates_adopted_escapee` 同时证明「受控集合为空但作用域仍看得见」：

```
[evidence] cgroup_scope adopted escaped_alive=true escaped_ppid=Some(1504) root=2126 descendants=[DescendantProcess { pid: 2134, in_controlled_set: false }] controlled=Some([]) scope=CgroupV2
[evidence] cgroup_scope after outcome=Cleaned escaped_alive=false kill_supported=true descendants=[] controlled=Some([]) scope=CgroupV2
```

判读：逃逸者已被收养（`escaped_ppid` 不是根），**受控进程组为空**（`controlled=Some([])`，`kill(-pgid)` 打不到它），cgroup 仍枚举到它；写 `cgroup.kill` 后三个观测量加快照同时为空，才写 `Cleaned`。因此「满足条件时整树清理」在 Linux 侧有真实进程证据，不是纸面推断。

### 44.3 能力位改动（仅此一条，其他能力位零改动）

| 字段 | v1.40 | v1.41 |
| --- | --- | --- |
| `holds` / `status` | `false` / `partial` | **不变** |
| `reason_code` | `posix_escaped_scope_requires_cgroup_v2_and_kill` | `processes_outside_controlled_scope_may_survive` |
| `statement` | 「Windows 覆盖全部后代；Linux 仅在…时终止整个作用域，否则逃逸成员仍存活并如实报…」 | 拆三分支，明写 Linux 满足条件时已用真实 setsid 逃逸进程证成 |
| `evidence` | 2 条 Linux 用例 | 增 `cgroup_without_kill_reports_still_alive`、`cgroup_degraded_terminate_is_group_only` |
| `platforms_verified` / `unverified` | `["windows","linux"]` / `["macos"]` | 不变 |

契约测试同步断言新 reason，并要求 evidence 同时包含三条 Linux 用例。条目仍 17，`not_proven_total` 仍 4，`tools_sha256` 不变（改的是 capabilities 资源，不是 `tools/list` 描述）。

### 44.4 降级路径：行为、报告与「悄悄漏杀」判定

**分支 A：内核没有 `cgroup.kill`。** `terminate` 不写 `cgroup.kill`，作用域成员仍在；轮询到 3s 截止报 `StillAlive`，快照仍枚举到成员。用例 `cgroup_without_kill_reports_still_alive`（临时目录伪造 `cgroup.procs`/`cgroup.events`，无 `cgroup.kill`，配合真实 `setsid` 逃逸进程）：

```
[evidence] cgroup_no_kill root=2128 escaped=2118 kill_supported=false outcome=StillAlive escaped_alive=true members=Some(Ok([2118])) populated=Some(Ok(1)) descendants=[DescendantProcess { pid: 2118, in_controlled_set: false }]
```

结论：**不会谎报 `Cleaned`**。实现里有三道互相独立的拦截——不调用 `kill_members`、`members()/populated()` 非空即不返回 `Cleaned`、父链快照仍把作用域成员并进来复核。为证明用例真能抓住「悄悄漏杀」，在 WSL 副本里把三道判据**全部**临时停用（不碰工作树），用例原文变红：

```
[evidence] cgroup_no_kill root=1242 escaped=1241 kill_supported=false outcome=Cleaned escaped_alive=true members=Some(Ok([1241])) populated=Some(Ok(1)) descendants=[]
thread '...cgroup_without_kill_reports_still_alive' panicked at apps/mcp-bridge/src/server.rs:4015:17:
assertion `left == right` failed: 作用域内仍有存活成员时不得报告 Cleaned
  left: Cleaned
 right: StillAlive
test result: FAILED. 0 passed; 1 failed
```

只停用其中一道（例如只跳过 `kill_members`）时用例仍是绿的——另外两道接住了，说明这不是「单点守卫」式的假绿。恢复副本后复跑全绿。

**分支 B：作用域建不起来（`Degraded`，cgroup v1 / 非 root / 容器未委派）。** `terminate` 只按进程组判据；被收养的逃逸进程既收不到信号，也不在父链快照里，结果是受控集合口径的 `Cleaned`。用例 `cgroup_degraded_terminate_is_group_only`：

```
[evidence] cgroup_degraded before root=2121 escaped=2116 scope=ProcessGroup escaped_scope_unobservable=Some("cgroup_v2_unavailable") descendants=[]
[evidence] cgroup_degraded after outcome=Cleaned escaped_alive=true descendants=[] scope=ProcessGroup unobservable=None escaped_scope_unobservable=Some("cgroup_v2_unavailable")
```

结论：这是**残余缺口**——`Cleaned` 只表示「受控集合已清」，不等于「无残留」。但它**不是静默漏杀**：降级原因始终写在快照 `escaped_scope_unobservable` 和 capabilities 的 reason 里，`Cleaned` 回执固定写「逃逸进程是否仍在运行未验证」。为证明用例能抓住静默降级，把 `cgroup_degradation()` 对 `Degraded` 的返回临时改成 `None`，用例原文变红：

```
[evidence] cgroup_degraded before root=1499 escaped=1498 scope=ProcessGroup escaped_scope_unobservable=None descendants=[]
thread '...cgroup_degraded_terminate_is_group_only' panicked at apps/mcp-bridge/src/server.rs:4080:17:
assertion `left == right` failed: 降级必须带原因
  left: None
 right: Some("cgroup_v2_unavailable")
test result: FAILED. 0 passed; 1 failed
```

**读法边界**：这两个分支意味着调用方**不能只看结果枚举**。`Cleaned` + POSIX 时必须同时读回执尾句「逃逸进程是否仍在运行未验证」与快照 `escaped_scope_unobservable`；只认 `Cleaned` 就会在「作用域建不起来」时把逃逸进程的存活当成已清理。能力位保持 `partial` 正是为了在机器可读层保留这个区别。

### 44.5 新增用例与回归

- `cgroup_without_kill_reports_still_alive`（Linux）：覆盖 no-kill 分支，断言 `StillAlive` + 逃逸进程存活 + 快照仍枚举到成员。
- `cgroup_degraded_terminate_is_group_only`（Linux）：覆盖 `Degraded` 分支，断言只按进程组判定、`Cleaned` 与逃逸进程存活并存、降级原因始终带在快照里。
- 两个用例都先起真实 `setsid` 进程；本机没有 `setsid` 时显式跳过而不是当成通过。
- 回归：WSL `21 lib + 120 integration` 全绿（日志 `/root/.aw-posix-verify/logs/posix-verify-20261108-013424.log`）；Windows `17 lib + 122 integration` 全绿（平台门禁，两个新用例不在 Windows 编译）。

### 44.6 未改动的边界

- 其他能力位（`timeout.cgroup_kill_terminates_scope`、`timeout.enumerates_escaped_descendants`、`timeout.descendant_snapshot_cross_check` 等）的 `holds`/`status`/`reason`/`evidence` 均未改。
- `tools/list` 描述未改，`tools_sha256` 不变。
- macOS 仍未验证；Linux 的结论只在「cgroup v2 可写 + 内核提供 `cgroup.kill`」分支成立，不外推到容器/非 root/cgroup v1。

## §45 v1.42：inflight 残标的自动清扫（2026-11-08）

目标：把「被遗弃的标记只要没人复用 task_id、没人 prune 就一直躺着」补成真正的自动清扫，同时守住「判据只此一套、审计写不进去就零删除」。

### 45.1 触发时机与默认开关

| 触发 | 行为 | 默认 |
| --- | --- | --- |
| `startup` | `main` 创建 `Server` 后、进入 stdio 循环前调用 `run_startup_sweep()` | **开启**（`AW_INFLIGHT_SWEEP_SECS=off` 时整体关闭） |
| `periodic` | `start_periodic_sweep()` 起后台线程，每 N 秒调用同一清扫函数 | **开启，N=3600 秒**；`0`=仅启动、`off`=关闭、正整数=周期秒数 |

周期默认开启的理由：一次清扫只是只读扫描 `runs/.inflight`，无可清残标时**连审计都不写**，成本近似零；而只在启动时清扫会让长驻 bridge 进程里的残标一直躺着，正是本条目要修的洞。`off` 保留给需要完全手动的部署；测试里两个共用 `target/debug/runs` 的真实 stdio 用例显式设 `off`，避免互相清扫。

### 45.2 实现与审计口径

- 自动路径**只调用** `sweep_inflight_markers(trigger, None, false)`，与 `task_start` / `runs.prune` / `runs.inflight.clean` 是同一个函数、同一套 `stale_rule`（`pid` 存活优先于 `deadline`，`unreadable` 永不删除），没有第二份判断。
- 审计仍是 `runs/inflight-sweep.jsonl`，`plan` 行先写、删除后写 `result` 行；`trigger` 变为 `startup|periodic|task_start|prune|clean`，自动与显式可区分。
- 审计闸门不变并覆盖自动路径：`plan` 行打不开/写不进 → 本轮**零删除**，错误只写 stderr，不阻断服务启动。
- `list` 的 `sweep_rule` 新增机器可读的 `automatic`（启动/周期、环境变量语义、opt-out）与 `audit_gate`。

### 45.3 真实进程证据（非进程内桩）

把 bridge 可执行文件复制到独立临时目录（`current_exe` 旁的 `runs/` 因此与其它用例隔离），造两个标记后**重启服务**：

```
[evidence] inflight_startup trigger="startup" removed_total=1 kept_total=1 removed_task="startup-stale-179632" active_task="startup-active-179632" active_marker_kept=true
```

- 残标：`pid=999999999`，`startedAt/deadlineAt` 都在过去 → `stale_reason=pid_missing`，启动后被清掉。
- 活跃标记：`pid` 为当前测试进程（存活）→ 即使 `deadlineAt` 已过，`stale=false`，**启动清扫后文件仍在**，并出现在 `kept` 里（`kept_total=1`）。
- 审计行：`trigger="startup"`、`phase="result"`、`removed_total=1`、`removed[0].task_id="startup-stale-…"`。

周期触发单独验证：子进程设 `AW_INFLIGHT_SWEEP_SECS=1`，等 `initialize` 返回（此时启动清扫已结束）**之后**才造残标，只有后台周期能清它：

```
[evidence] inflight_periodic trigger="periodic" removed_total=1 removed_task="periodic-stale-179632"
```

审计里该 task 只出现在 `trigger="periodic"` 的 result 行，`trigger="startup"` 的 result 行没有它。

审计失败闸门在自动路径上的实测（用目录占住 `runs/inflight-sweep.jsonl`，让 append 必定失败）：

```
[inflight-sweep] trigger=startup error=inflight_sweep_audit_failed: 打开 runs/inflight-sweep.jsonl 失败：拒绝访问。 (os error 5)；本轮未删除任何标记
[evidence] inflight_auto_audit_failure marker_exists=true audit_path_is_dir=true
```

Linux（WSL）同样通过：`audit_path` 占位导致 `Is a directory (os error 21)`，`marker_exists=true`；启动证据 `removed_task="startup-stale-1320" kept_total=1`，周期证据 `removed_task="periodic-stale-1320"`。

### 45.4 先红：用例能抓住「另写一套判据 / 全删」

在 WSL 副本里把 `sweep_inflight_markers` 中 4 处 `marker.assessment["stale"] == json!(true)` 全部临时改成 `true`（即忽略 `stale_rule`、见标记就删，不碰工作树），启动用例原文变红：

```
thread 'inflight_startup_sweep_removes_stale_and_keeps_active' panicked at apps/mcp-bridge/tests/mcp_contract.rs:7407:5:
assertion `left == right` failed: 活跃标记必须出现在 kept：{... "kept":[], "kept_total":0, "removed_total":1, "trigger":"startup" ...}
  left: Number(0)
 right: 1
test result: FAILED. 0 passed; 1 failed
```

说明用例确实在验证「活跃标记不会被自动清掉」，而不是只看残标消失。恢复副本后 16 条 inflight 用例全绿。

### 45.5 能力位与描述门禁

- `inflight.stale_markers_auto_swept`：`false/partial/path_triggered_only/automatic=false` → **`true/proven/reason_code=null/automatic=true`**；evidence 增加 `inflight_startup_sweep_removes_stale_and_keeps_active`、`inflight_periodic_sweep_removes_stale_marker`、`inflight_auto_sweep_audit_failure_deletes_nothing`。`not_proven_total` 4→3。
- `inflight.stale_marker_cleanable` 的 `automatic=false` 不变：显式 `clean` 仍需调用方 `dry_run=false+confirm=true`。
- `runs.inflight` / `runs.prune` 的 tools/list 描述删除「没有启动/定时自动清理」的旧说法，改为写明启动一次 + 默认每小时、`AW_INFLIGHT_SWEEP_SECS` 语义与审计闸门；`tools_sha256` 从 `36dbcfc9…` 更新为 `915974d2…`，描述门禁 G18 同步（`contains` 增加自动清扫字段，删除对「自动清扫」的禁止项）。

### 45.6 未改动的边界

- 跨进程仍无事务：清扫与其它进程的创建/删除之间仍是 check-then-delete，`limits` 里的四条限制不变。
- `unreadable` 标记仍不会自愈，只会被保留并报告。
- 自动清扫只发生在 bridge 进程存活时；进程退出后不再有周期触发。`off` 是显式退出开关，不属于默认行为。
- 回归：Windows `17 lib + 125 integration` 全绿；WSL `21 lib + 123 integration` 全绿（差异为两个 Linux 独有 cgroup 用例）。

## §46 v1.43：`check_then_delete_atomic` 做成真原子（rename 认领 + 重跑 stale_rule）（2026-11-08）

目标：把 `concurrency.check_then_delete_atomic=false/not_proven` 从「先判断再删除、中间有窗口」改成真原子语义，并证明两件事：并发清扫同一残标只有一方成功；正在被刷新的标记零误删。

### 46.1 实现口径

- 删除判定不再看「认领副本与扫描快照的 `task_id`+`pid`+`startedAt` 三元组是否一致」，而是**对认领副本重读并重跑同一份 `stale_rule`**（`assess_inflight_marker`）：
  - 仍 `stale` → `remove_file` → `outcome=deleted`；
  - 已不 `stale` → `restore_without_clobber`（`create_new`/O_EXCL，绝不覆盖）→ `outcome=not_mine`，子原因 `marker_changed` / `marker_superseded`；
  - 读不出内容 → 按 unreadable 处理，同样还原保留 → `outcome=not_mine` / `recheck_unreadable`；
  - 认领副本已不存在 → `outcome=lost_race`，不算错误。
- 判据只有一套：清扫入口仍先按 `marker.assessment["stale"]` 过滤；认领后的复核调用同一个 `assess_inflight_marker`，没有第二份「身份规则」。
- 审计沿用 `runs/inflight-sweep.jsonl`：`removed[]` / `kept[]` 逐条带 `outcome`，result 增加 `deleted_total` / `lost_race_total` / `not_mine_total`（保留 `removed_total` 兼容）；plan 行写不进去仍零删除。
- 认领临时名增加进程内序号（`INFLIGHT_CLAIM_SEQ`），避免同进程多清扫线程在纳秒时钟上撞名。

### 46.2 Windows 实测：`rename` 返回 Ok 不等于「这份归我」

在同一个卷上并发对同一源文件执行 4 次 `fs::rename`，实测 4 个调用**都返回 Ok**，但目录里只有一个目的文件。因此实现里「认领成功」的判定不是 `rename` 的返回值，而是认领副本是否真的可读：读不到就是输掉认领（`lost_race`）。这一点若不按平台实测，会把失败归错类，并可能在 `restore_without_clobber` 阶段报出 `marker_changed_restore_failed`。

### 46.3 先红：旧实现下两条并发用例的原文

红测试（新增于 `mod inflight_sweep_race_tests`）：

- `sweep_concurrent_claimers_delete_once_others_lost_race`：4 个线程各自 `Server::with_runs_dir`（同一 `runs`），用测试观测点 `SWEEP_BEFORE_CLAIM_HOOK` + `Barrier` 在「扫描判定之后、rename 认领之前」对齐；断言 `deleted_total=1`、`lost_race_total=3`、标记消失、无 `.tmp`、审计同时含 `"outcome":"deleted"` 与 `"outcome":"lost_race"`。
- `sweep_concurrent_refresher_never_deletes_fresh_marker`：3 个清扫线程 + 1 个刷新线程；刷新线程在两个 Barrier 之间用「同目录临时文件 + rename」原子替换成非残标（`task_id/pid/startedAt` 不变，只把 `deadlineAt` 从 2000 延到 2099）；断言 `deleted_total=0`、`not_mine_total>=1`、原位仍是刷新后的 `deadlineAt`、无 `.tmp`、审计含 `"outcome":"not_mine"`。

把实现临时回退到 v1.42（身份三元组复核）后原文（断言与失败文本逐字保留；`[]` 内结果 JSON 过长，仅省略重复字段）：

```
thread 'server::inflight_sweep_race_tests::sweep_concurrent_claimers_delete_once_others_lost_race' panicked at apps\mcp-bridge\src\server.rs:8083:9:
assertion `left == right` failed: 同一个残标只允许一方真正删除，实际 0：[Object {..., "deleted_total": Null, "failed": Array [Object {"error": String("marker_changed_restore_failed: 系统找不到指定的文件。 (os error 2)"), ...}], "failed_total": Number(1), "kept": Array [], "kept_total": Number(0), "lost_race_total": Null, "not_mine_total": Null, "removed": Array [], "removed_total": Number(0), ...}, ... Object {..., "removed": Array [Object {..., "path": String("runs/.inflight/race.json"), ...}], "removed_total": Number(1), ...}]
  left: 0
 right: 1
thread 'server::inflight_sweep_race_tests::sweep_concurrent_refresher_never_deletes_fresh_marker' panicked at apps\mcp-bridge\src\server.rs:8167:9:
认领后必须能识别出「已不是我的」，实际 0：[Object {..., "failed_total": Number(1), "kept": Array [], "lost_race_total": Null, "not_mine_total": Null, "removed": Array [], "removed_total": Number(0), ...}, ... Object {..., "removed": Array [Object {..., "path": String("runs/.inflight/race.json"), ...}], "removed_total": Number(1), ...}]
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 16 filtered out; finished in 0.08s
```

红证据说明两件事：(a) 旧实现没有 `deleted_total` / `lost_race_total` / `not_mine_total`，输掉认领的 3 个清扫者落到 `marker_changed_restore_failed: ... (os error 2)`，把「输掉竞态」误报成失败；(b) 刷新场景里刷新后的标记被**真的删掉**（某一线程 `removed_total=1`，`path=runs/.inflight/race.json`），且 `not_mine_total=Null`——这正是要消灭的误删。

### 46.4 绿：并发语义

恢复实现后两条用例连续 20 轮全绿（`--test-threads=1`）：

- 抢同一残标：`deleted_total=1`、`lost_race_total=3`（4 清扫者），标记不存在，`.tmp` 残留 0，审计里 `outcome=deleted` 与 `outcome=lost_race` 同时出现。
- 并发清扫 + 刷新：`deleted_total=0`、`not_mine_total>=1`，原位 `deadlineAt=2099-01-01T01:00:00+00:00`、`startedAt` 未变，`.tmp` 残留 0，审计里 `outcome=not_mine`。

正向用例也仍绿：`sweep_restore_never_clobbers_concurrent_new_marker`（复核期间原名位置已被新标记占住 → `Restored/Superseded` 都不覆盖、`removed_total=0`）。

### 46.5 能力位与描述

- `concurrency.check_then_delete_atomic`：`false/not_proven/check_then_delete_not_atomic` → **`true/proven/reason_code=null/automatic=true`**；evidence 为上述两条并发用例 + `sweep_restore_never_clobbers_concurrent_new_marker`。`not_proven_total` 3→2。
- `concurrency.sweep_restore_never_clobbers` 保持 `true/proven`；`inflight.clean_preserves_active_markers` 的 detail 同步为「认领后重跑 stale_rule」。
- `runs.inflight` 的 `sweep_rule` 同步：`atomic_delete` 改写、新增 `outcome_totals`、`limits` 换为 `same_volume_rename_required` / `transient_name_absence_during_claim` / `audit_append_not_transactional` / `unreadable_not_self_healing`。
- 未改 `tools/list` 的工具描述与 schema，`tools_sha256` 不变（仍 `915974d2…`）；描述门禁 G18 不需调整。

### 46.6 残余边界（如实保留）

- **同卷前提**：`rename` 跨卷返回 `EXDEV`，认领失败 → 该条记 `failed`，零删除；不把跨卷伪装成成功。
- **原名短暂空缺**：认领与还原之间，`runs/.inflight/<task>.json` 这个文件名短暂不存在；第三方读者（不读审计、不持标记）可能看到这个瞬间。
- **审计非事务**：`plan` 先写、删除后写 `result`；进程在两次 append 之间崩溃会缺 `result` 行，但不会出现「没 plan 就删」。
- **坏标记不自愈**：`unreadable` 永不删除，只能人工处理。
- 回归：Windows `19 lib + 125 integration` 全绿；WSL Ubuntu 24.04 `23 lib + 123 integration` 全绿（WSL 日志 `/root/.aw-posix-verify/logs/posix-verify-20261108-025030.log`）。

## §47 v1.44：`rename` 返回值作为归属/互斥依据的全库核查

### 47.1 判据

只有两种归属判据可以接受：**事后读目的路径**（存在/可读才算拿到），或**独占创建**（`create_new` / O_EXCL）。`rename` 的 `Ok` 只说明"这次调用没报错"，在 Windows 上不说明目的文件归你（§46.2 已实测：4 个并发 rename 同一源全部 `Ok`，只有一个目的文件）。本次据此逐处判定，不按文件名列表交差。

### 47.2 逐处结论

| 位置 | 现状 | 判定 | 处理 |
|---|---|---|---|
| `server.rs` session 审计 tmp→正式名 | 目的名含 pid+毫秒+序号，天然唯一；rename 失败只清 tmp | 不靠返回值判归属：没有竞争同一目的名 | 不改 |
| `server.rs` `delete_attempt_dir` attempt 目录换名 | rename `Ok` 后直接 `remove_dir_all(staged)` | **靠返回值判搬成功**：并发下双方 `Ok`、只有一个目的目录；败者删暂存报 `NotFound`，被归成 `trashed_not_deleted`（Windows）或 `retained_at_source`（POSIX），把竞态报成故障 | **改**：rename `Ok` 后用 `symlink_metadata(staged)` 复核；源 `NotFound` / 目的不存在 / 目的删除前被清走 → `lost_race` |
| `server.rs` inflight 认领 | rename `Ok` 后重读认领副本，读不到即 `lost_race` | 已按目的可读判归属（v1.43） | 不改 |
| `server.rs` inflight 删除失败回滚 `rename(tmp → 原名)` | 认领副本删除失败时用 rename 还原 | 不判归属，但 rename 会覆盖原位新标记 | **改**：复用 `restore_without_clobber`（`create_new`），`Restored`/`Superseded` 分别写实 |
| `server.rs` `posix_rename_active_attempt_semantics` | rename 返回后另断言 `!attempt.exists()` / `staged.exists()` | 已以目的存在为准；返回值只作语义探针 | 不改 |
| `server.rs` 测试刷新者 `rename(staging → marker)` | 测试夹具的原子替换，不宣称归属 | 非生产归属路径 | 不改 |
| `updater.rs` `promote_verified_download` | 校验 sha 后 tmp→正式文件 | 不判归属/互斥（单实例发布步骤） | 不改 |
| `launcher_stub.py` 下载落盘 | 固定 `.tmp` 后 rename 到目标 | 不判归属；只用异常表示是否落盘 | 不改（非派单并发域） |
| `build-version.py` / `backend.py` / `env_service.py` 的 rename / `shutil.move` | 安装器备份或 venv 迁移 | 不判归属/互斥 | 不改 |
| inflight / prune 的 `phase="plan"` | JSONL append + fsync | 不是"plan 文件原子替换"，不涉及 rename 归属 | 不改 |

### 47.3 `lost_race` 的语义与计数归属

- prune 结果新增独立 `lost_race[]` / `lost_race_total`，逐条带 `task_id`/`attempt`/`source_path`/`error`/`rule`/`fingerprint`，`state="lost_race"`、`trash_path=null`。
- **不计入 `failed_total`**；`status` 只在 `failed_total + trashed_total > 0` 时为 `partial`，只有 `lost_race` 时仍 `ok`、`isError=false`。
- 不变式：`deleted_total + failed_total + trashed_total + lost_race_total + excluded_total == batch_candidates_total`（代码注释、`runs.prune` 描述、契约 §8.11 三处一致）。
- 与清扫侧同名同义：**另一方先赢、本进程零删除**。区别只在对象与观测点——清扫侧是 `.inflight` 标记的认领副本读不到；prune 侧是 attempt 目录的源已消失或目的目录不存在。

### 47.4 红：旧实现的实测输入/输出

窗口内两条用例（`mod prune_claim_race_tests`）：

- `prune_claim_lost_is_named_not_trashed`：观测点 `PRUNE_AFTER_RENAME_HOOK` 在 rename 后删掉 `staged`，精确模拟 Windows「rename 返回 Ok 但目的目录不存在」。旧实现断言文本与实测值：

```text
目的目录不存在不是「删除暂存失败」：trashed_not_deleted
```

- `prune_concurrent_claimers_delete_once_others_lost_race`：两个线程各持同一 `runs` 的 `Server`，`PRUNE_BEFORE_RENAME_HOOK` + `Barrier` 在 rename 前对齐，连跑 20 轮。旧实现第 0 轮失败文本与实测状态：

```text
第 0 轮败者不得被报成暂存删除失败：[deleted, trashed_not_deleted]        # Windows
第 0 轮败者不得被报成原样未动：[deleted, retained_at_source]              # WSL Linux
```

红证据说明：Windows 的败者被误报成"暂存删除失败（数据未丢）"，但它其实是被另一方搬走并删掉了；POSIX 的败者被误报成"原样未动"，源其实已经不在。两者都不是 `deleted`，也都不是可核对的竞态终态。

### 47.5 绿：真并发 20 轮

修后同一用例（断言加强为"败者必须是 `lost_race`"）：

```text
[evidence] prune_claim_race rounds=20 deleted=20 lost_race=20 other=0
```

每轮同时满足：恰好一方 `deleted`；另一方 `lost_race`；源目录不存在；两方 `.prune-trash/<batch>` 都不留半截；两条 entry 合计 2、无重复。`prune_claim_lost_is_named_not_trashed` 同样绿（`state="lost_race"`）。

### 47.6 能力位与描述

- 本次不新增/不上调能力位；`concurrency.check_then_delete_atomic` 保持 v1.43 的 `true/proven`。
- `runs.prune` 工具描述的不变式改为含 `lost_race`，`tools_sha256` 由 `915974d2…` → `85da0303…`，审计门禁块同步；G 行只断言"含计数不变式/四集合字段"，不因新增 `lost_race` 失效。

## §48 v1.44：孙进程握手夹具（Windows）

### 48.1 旧夹具为什么前提不成立

旧夹具直接 `cmd /C "<sentinel> -n 30 127.0.0.1 > NUL"` + `timeout_secs=8`，孙进程是否在窗口内完全取决于冷启动耗时。上一轮实测调用 30.8s、`grandchild_timeline=[]`，裁决只能是"前提不成立"——这不是终止逻辑的判据，而是夹具不可控。

### 48.2 握手夹具

- `handshake.cmd`：`echo started > started.txt` → `start "" /b <sentinel> -n %1` → `echo ready > ready.txt` → `ping -n %2` 保持存活。
- 窗口外先跑一次 `handshake.cmd 2 3` 测量 `spawn → ready` 的真实耗时 `handshake_ms`；等探针孙进程完全退出（`process_image_pids(SENTINEL)` 为空）后清掉 started/ready，避免污染窗口内观测。
- 窗口内 `timeout_secs = max(10, handshake_ms/1000 + 6)`，命令 `handshake.cmd 60 120`；`task.execute` 放独立线程，主线程等待 `ready.txt` 并记录 `ready_seen` / `ready_delay_ms`。
- 判据、三条断言、`blocked_reason` 断言、四态均未放宽。

### 48.3 verdict 增加握手字段

```text
[verdict] grandchild_case=... ready_seen=<bool> ready_delay_ms=<n> timeout_ms=<n> \
handshake=<before_timeout|after_timeout|not_completed> observed=<n> survivors=[...] ...
```

- `ready_seen=false` → 断言文本明确写「**握手未完成，本条不构成裁决依据**」，不把它当成"整树清理不可证"的结论。
- `ready_seen=true` 且 `ready_delay_ms >= timeout_ms` → `handshake=after_timeout`，同样注明本条不构成裁决依据。
- `ready_seen=true` 且窗口内未观测到 → `premise_not_established`，子原因 `ready_seen_but_not_observed`。
- 四态仍为 `pass` / `terminate_defect` / `unobservable` / `premise_not_established`。

### 48.4 实测（Windows）

```text
[evidence] grandchild_handshake warm_ms=412 timeout_ms=10000
[evidence] grandchild_first_seen ms=418 pid=...
[evidence] grandchild_gone after_ms=10215
[evidence] grandchild_survivors_after_cleanup=[]
[verdict] grandchild_case=pass ready_seen=true ready_delay_ms=412 timeout_ms=10000 \
handshake=before_timeout observed=1 survivors=[] ...
```

`ready_seen=true`、`handshake=before_timeout`，孙进程确实落在观测窗口内，清理后零存活；本条从"前提不成立"推进为可裁决的 `pass`。POSIX 分支判据与断言不变，WSL 实跑通过。

### 48.5 能力位与剩余 `not_proven`

- `timeout.cleans_entire_process_tree` 的 evidence 增加 `timeout_cleans_controlled_set_including_grandchild`，但 **v1.44 当时 holds 仍 `false/partial`**：本次只证成 Windows 受控集合内的孙进程，证不到作业外进程。**v1.45 已按平台拆分，见 §49。**
- v1.44 当时的剩余两条 `not_proven` 的可证路径已在契约 §2.7 后逐条写明（`timeout.cleans_entire_process_tree`、`timeout.enumerates_escaped_descendants`）：Linux 分支在本机可证（cgroup v2 + `cgroup.kill`），Windows 的"整树/逃逸后代权威枚举"在本机无 OS 原语，结构性不可证。v1.45 已按平台拆分并证掉 Linux 侧，见 §49。

## §49 v1.45：逃出 `cgroup.procs` 的整树清理与平台拆分（2026-11-08）

### 49.1 判据

修前只证到「`setsid` 逃逸但仍在本 cgroup」：`cgroup.kill` 能打到它。真正离开本作用域 `cgroup.procs`（把自己写进旁路 cgroup）的进程，既不在 `cgroup.kill` 的打击范围，`setsid` 后也不在进程组内；根退出被收养后父链也断了。终止必须仍覆盖它，四个观测量同时成立才允许 `Cleaned`：

1. `cgroup.procs` 为空；
2. `cgroup.events` 为 `populated 0`；
3. 逃逸 PID 不存在；
4. 快照无后代。

### 49.2 夹具与红证据

`cgroup_kill_terminates_escapee_that_left_cgroup_procs`（Linux-only，WSL）：

- 父 `sh root.sh` → 子 `sh child.sh` → 孙 `setsid sh grandchild.sh`；
- 孙脚本 `echo $$ > grand.pid`，随后 `echo $$ > <sink>/cgroup.procs`（旁路 cgroup 与作用域同父目录，委派权限一致），`exec sleep 30`；
- 测试断言孙进程已不在 `scope.members()`、`sink_members=[pid]`、`move_status=moved`，且父链快照仍能看到它（`in_controlled_set=false`）。

HEAD 实现红（原样）：

```text
[evidence] cgroup_left before root=1895 child=Some("1906") escaped=1913 scope=CgroupV2 sink_members=[1913] move_status=Some("moved\n") descendants=[DescendantProcess { pid: 1906, in_controlled_set: true }, DescendantProcess { pid: 1913, in_controlled_set: false }] outside=[1913]
[evidence] cgroup_left after outcome=Cleaned escaped_alive=true members=Ok([]) populated=Ok(0) descendants=[] unobservable=None
thread 'server::process_scope::imp::tests::cgroup_kill_terminates_escapee_that_left_cgroup_procs' panicked at apps/mcp-bridge/src/server.rs:4266:17:
逃出 cgroup.procs 的后代在终止后必须不再存活（cgroup.kill 打不到它，必须另行收口）
test result: FAILED. 25 passed; 1 failed
```

这是真缺陷：`cgroup.kill` 只作用于成员，逃逸进程存活而结果误报 `Cleaned`，且快照 `descendants=[]` 让读方以为无残留。

### 49.3 修复

`Supervisor::terminate`（POSIX）：

- 动手前 `tracked_descendants(root)` = `/proc` 父链闭包 ∪ `cgroup.procs` 成员，每项附 `starttime`（`/proc/<pid>/stat` 第 22 字段）。
- `SIGKILL` 进程组前再抓一次（根仍存活，父链还在）；`child.wait()` 后父链断，不再抓。
- 每轮循环：先 `expand_tracked`（存活名单成员在 `/proc` 里的新后代并入），再对 `tracked_pid_alive(pid, start)` 为真者逐个 `SIGKILL`；`starttime` 不一致（PID 复用）绝不发信号。
- 判定：`cgroup.procs` 空、`populated 0`、名单全部消失、快照空，四者同时成立才 `Cleaned`；deadline 到仍有名单存活 → `StillAlive`。

### 49.4 绿证据

WSL Ubuntu 24.04 LTS（root，cgroup2fs，`cgroup.kill` 存在，内核 `6.18.33.2-microsoft-standard-WSL2`，日志 `/root/.aw-posix-verify/logs/posix-verify-20261108-033331.log`）：

```text
[evidence] cgroup_left before root=33075 child=Some("33082") escaped=33087 scope=CgroupV2 sink_members=[33087] move_status=Some("moved\n") descendants=[DescendantProcess { pid: 33082, in_controlled_set: true }, DescendantProcess { pid: 33087, in_controlled_set: false }] outside=[33087]
[evidence] cgroup_left after outcome=Cleaned escaped_alive=false members=Ok([]) populated=Ok(0) descendants=[] unobservable=None
test server::process_scope::imp::tests::cgroup_kill_terminates_escapee_that_left_cgroup_procs ... ok
test result: ok. 27 passed; 0 failed
```

### 49.5 枚举权威性与退化条件

- 权威：`cgroup_scope_enumerates_adopted_escapee` 增断言——根退出、逃逸者被收养（`escaped_ppid` 不再是根）后，`members == vec![escaped]`。成员集合本身点名，不依赖父链。证据 `[evidence] cgroup_scope adopted escaped_alive=true escaped_ppid=Some(32364) root=33029 members=Some(Ok([33071])) descendants=[DescendantProcess { pid: 33071, in_controlled_set: false }] controlled=Some([]) scope=CgroupV2`。
- 退化：
  1. 无 cgroup 写权限 / 非 root / 容器未委派 / cgroup v1 → 作用域建不起来，`escaped_scope_unobservable`，退回进程组旁证；
  2. Active 但 `cgroup.procs` 读不出 → 整份快照 `unobservable`，绝不当空（`cgroup_members_unreadable_is_not_empty`：`[evidence] cgroup_unreadable scope=ProcessGroup unobservable=Some("cgroup_procs_unreadable:No such file or directory (os error 2)") is_empty=false`）；
  3. 进程已离开 `cgroup.procs` → 作用域不含它，只在脱离前靠父链可见（旁证），根退出被收养后不可见；
  4. 无 `cgroup.kill` 不影响枚举权威性，只影响终止；v1.45 由终止前名单补足已知逃逸者。

### 49.6 无 `cgroup.kill` 用例口径同步

`cgroup_without_kill_reports_still_alive`：静态伪造作用域（无 `cgroup.kill`、`cgroup.procs` 非空、`populated 1`、记账不更新）。v1.45 起 `terminate` 会按名单杀掉已知逃逸者（`escaped_alive=false`），但独立观测点仍显示成员 → 如实 `StillAlive`，**不因自己的名单放行**。

### 49.7 能力位（平台拆分）

| 条目 | holds | status | reason_code | 平台 |
| --- | --- | --- | --- | --- |
| `timeout.cleans_entire_process_tree` | true | proven | null | `platforms_verified=[linux]`，unverified `[windows, macos]` |
| `timeout.cleans_entire_process_tree_windows` | false | not_proven | `requires_out_of_job_enumerable_terminable_os_primitive` | `[windows]` |
| `timeout.enumerates_escaped_descendants` | true | proven | null | `[linux]`，unverified `[windows, macos]` |
| `timeout.enumerates_escaped_descendants_windows` | false | not_proven | `requires_out_of_job_enumerable_terminable_os_primitive` | `[windows]` |

- Linux 两条的 scope/detail 明写 `platform=linux`、cgroup v2 可写、`cgroup.kill`、内核 `6.18.33.2-microsoft-standard-WSL2` 实测与残余边界。
- Windows 两条 detail 明写「需要作业外进程可枚举可终止的 OS 原语，本机不存在」；受控集合内孙进程结论只覆盖作业内，不上调整树。
- 条目 17→19，`not_proven_total` 仍 2（均为 Windows 结构性条目）。
- `mcp_contract` 平台断言同步：Linux 两条 `holds=true/proven/reason_code=null`，Windows 两条 `false/not_proven` + 上述 reason。
- 未改 `tools/list` 描述，`tools_sha256` 不变（`85da030305024ac405c9fdecb6cfc0e9d4c46fdc7f2dd4aa000314d346e6a3bc`），门禁通过。

## 50. v1.46 真实链路端到端（MCP stdio，不使用夹具）

### 50.1 方法与不可伪造边界

驱动脚本 `scripts/real-chain-verify/verify.py`（仅标准库）：把 `aw-mcp-bridge` 可执行文件复制到独立 sandbox 的 `bin/`，`cwd` 指向 sandbox 的 `work/`，`APPDATA=/HOME` 指向 sandbox，`AW_INFLIGHT_SWEEP_SECS=0`（只保留 startup 档清扫，去掉时序不确定）；然后以 stdio 逐行 JSON-RPC 调用，请求/响应原文落 `wire.jsonl`。所有断言都同时比对「响应字段」与「磁盘事实」（`runs/` 树、`dispatch.json`、`handback.json`、`inflight-sweep.jsonl`、产物文件与 sha256）。

不可伪造、只能如实标注的部分：

- **没有 cancel 工具**：真实 `tools/list` 返回 `['capabilities', 'health', 'receipt.get', 'runs.inflight', 'runs.list', 'runs.prune', 'status', 'task.execute']`，无 cancel。所以「执行中取消」在产品语义里不存在，唯一真实入口是**杀死 bridge 进程**（客户端断连/崩溃）。本文的“取消”一律指后者。
- **`Supervisor::terminate`、`assess_inflight_marker` 等内部函数不可从 MCP 直达**；它们的效果只能通过 `task.execute` 的 `status`/`blocked_reason`、`runs.inflight`、重启后的审计行观察。内部状态的最终判定仍由 §49 的夹具单测覆盖。
- **`_session/` 与目录布局**：脚本读盘核对，不调用内部函数；`runs.list` 只用于交叉验证（契约明说它不是执行状态权威）。

环境：Windows `10.0.26100`（24H2）、`cargo 1.97.0`、Python 3.12.10；WSL `Ubuntu 24.04 LTS`、内核 `6.18.33.2-microsoft-standard-WSL2`、Python 3.12.3。

### 50.2 场景 A：真实小任务全链路（两平台都过）

请求原文（Windows，`wire.jsonl`）：

```json
{"dir": "send", "payload": {"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "task.execute", "arguments": {"task_id": "real-a-normal", "cmd": "echo hello-real-chain-A>artifact-a.txt", "cwd": "D:\\Temp\\aw-real-windows-a-ren4bazr\\work", "timeout_secs": 30}}}}
```

响应原文（节选）：`{\"attempt\":1,\"cwd\":\"\\\\\\\\?\\\\D:...\",\"duration_ms\":343,\"exit_code\":0,\"finishedAt\":\"2026-11-07T19:43:12Z\",\"status\":\"ok\",\"stdout_digest\":\"sha256:e3b0c442...b855\",...}`。`stdout/stderr` 均为空（命令把内容写进产物），故摘要等于空文件 sha256。

盘上事实（Windows）：

```text
runs\real-a-normal\attempt-1\dispatch.json
runs\real-a-normal\attempt-1\handback.json
runs\real-a-normal\attempt-1\stderr.txt
runs\real-a-normal\attempt-1\stdout.txt
runs\.inflight            (空)
runs\_session\initialize-100300-1794080591426-0.json
```

`dispatch.json` 与请求逐字段一致（`task_id`、`cmd`、`cwd`、`timeout_secs=30`、`dispatchedAt`）；`handback.json` 的 `artifacts[].hash` 与盘上 `sha256` 相同、`locator=runs/real-a-normal/attempt-1/stdout.txt` 从 `runs/` 父目录可解析；`receipt.get` 返回同一个 `finishedAt`/`status`；`runs.inflight` `total=0`；`*.tmp` 0 个。两平台都给出 `[evidence] A label=... status=ok attempt=1 inflight_total=0 tmp_files=0 artifacts_match=true`。

### 50.3 场景 B1：执行中超时（两平台都过）

命令先写心跳再持续运行，`timeout_secs=2`。Windows 回执原文：

```json
"blocked_reason": "timeout：超过 2s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证；Windows 下子进程以挂起态创建并先加入作业再恢复执行（恢复前不会派生子进程）；BREAKAWAY 或加入作业失败的作业外进程仍不可观测",
"duration_ms": 2090, "exit_code": null, "status": "blocked"
```

Linux 同义但无 Windows 子句：`"timeout：超过 2s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证"`，`duration_ms=2509`。超时返回后连续 3 秒心跳文件大小不变（两平台 `controlled_set_silent=true`），`inflight` 无残留、`*.tmp` 0。即：超时不是“只发个信号”，受控集合内确实静默了；但回执措辞没有把它夸成“整棵树已死”。

### 50.4 场景 B2/B3：执行中取消 + 重启 startup 清扫（两平台都过）

执行到 ~3s 直接杀 bridge。杀之前 `runs/.inflight/real-b-cancel.json` 存在（`pid` 是 **bridge 自己的 pid**）。重启（`AW_INFLIGHT_SWEEP_SECS=0`）后审计原文（Windows，Linux 同构，仅 pid 不同）：

```json
{"at":"2026-11-07T19:43:25Z","candidates":[{"file":"real-b-cancel.json","path":"runs/.inflight/real-b-cancel.json","pid":43212,"stale_reason":"pid_missing","task_id":"real-b-cancel"}],"phase":"plan","stale_total":1,"task_id_filter":null,"trigger":"startup","unreadable_total":0,"version":"inflight-sweep/1"}
{"at":"2026-11-07T19:43:25Z","deleted_total":1,"failed":[],"failed_total":0,"kept":[],"kept_total":0,"lost_race_total":0,"not_mine_total":0,"phase":"result","removed":[{"file":"real-b-cancel.json","outcome":"deleted","path":"runs/.inflight/real-b-cancel.json","pid":43212,"reason":"stale","stale_reason":"pid_missing","task_id":"real-b-cancel"}],"removed_total":1,"task_id_filter":null,"trigger":"startup","unreadable_total":0,"version":"inflight-sweep/1"}
```

attempt 目录去向：`runs/real-b-cancel/attempt-1/` 仍在原位，只有 `dispatch.json`、`stdout.txt`、`stderr.txt`，**没有 `handback.json`**；`receipt.get` 明确回“回执不存在”。所以不会出现“status 说 completed 但产物不在”——没有回执就没有完成声明，attempt 目录是“开工了但没交卷”的可解释形态。`inflight-sweep` 只删标记，不动 attempt、不进 `.prune-trash`（`trash` 只属于 `runs.prune`）。

B3 是同一路径上的时间边界：杀进程后**立刻**重启（距 `startedAt` <5s），标记被保留、`stale=false`、`stale_reason=pid_missing_grace`，且 startup 清扫**不追加审计行**（契约 §2.9：无可清残标不写）；等过 5s 再重启，`removed_total=1`、标记消失。日志：`[evidence] B3 label=linux early_restart kept=true stale=False reason=pid_missing_grace pid_alive=False`、`[evidence] B3 label=linux late_restart removed_total=1 marker_gone=true`。

### 50.5 场景 C：并发（两平台都过）

- **C1 同 id 顺序连派**：第二次响应与第一次**逐字段相同**（dispatched 契约是回放既有回执，不引入 standalone 的 `replayed`/`replay_kind` 字段），`once-c1.txt` 只有 1 行，只有 `attempt-1`。随后用同 id + 不同 `cmd` 再派：工具错误里具名 `task_id_conflict`，不新增 attempt，被拒请求零产物。`[evidence] C1 replay_identical=true replayed_field_absent=true attempts=1 conflict_named=true conflict_new_attempt=false`
- **C2 同 id 两进程同时派**：两个 bridge 共用一个 `runs/`，同时发 `task.execute`。两方都拿到 `status=ok` 且回执**逐字段相同**，只有 `attempt-1`，`once-c2.txt` 只有 1 行——没有两个进程写同一 attempt，也没有重复执行。`[evidence] C2 identical_receipt=true attempts=1 once_lines=1 same_finishedAt=true`
- **C3 两个不同 id 并行**：第三个 bridge 在运行中观察 `runs.inflight` 看到 2 条互不相同的标记 `['real-c-par-x','real-c-par-y']`；结束后盘上是两个独立任务目录，各自 `attempt-1/{dispatch,handback,stdout,stderr}`，产物 `out-x.txt` / `out-y.txt` 各 5 字节；x 的回执不含 y 的痕迹。`[evidence] C3 observed_markers=['real-c-par-x', 'real-c-par-y'] ... final_inflight=0`

### 50.6 Windows 与 Linux 的真实差异（单列）

| 观察点 | Windows 24H2 | Ubuntu 24.04 (WSL2) | 说明 |
| --- | --- | --- | --- |
| bridge 被杀后子进程 | `child_survived=False`（心跳停在 20 字节） | `child_survived=True`（心跳 20→35 字节） | Windows 作业对象 `KILL_ON_JOB_CLOSE` 在进程死亡时连带杀作业内进程；Linux 没有等价兜底，子进程成孤儿继续跑（用例命令自封顶 40s，未长期残留）。这是**未承诺路径**：产品没有 cancel，契约不承诺 bridge 死后的清场，故不算回归；若要补，需要 `PR_SET_PDEATHSIG` 或常驻守护。 |
| `cwd` 形态 | `\\?\D:\...\work`（canonicalize verbatim 前缀） | `/root/.../work` | 同一段规范化逻辑，Windows 多 `\\?\` 前缀；比对时必须按路径语义归一。 |
| 超时文案 | 额外附挂起态创建 + BREAKAWAY 子句 | 只到“逃逸进程未验证” | 平台能力不同，如实写在 `blocked_reason`。 |
| 2s 预算实测耗时 | 2090ms | 2509ms | 定时轮询粒度与清理开销差异，未放宽判据。 |
| shell 语义 | `cmd /C`，`echo` 写 CRLF | `sh -c`，`printf` 写 LF | 产物字节不同属预期，摘要按各自盘上文件核算。 |

### 50.7 本轮发现项

1. **已修：测试基建的 `ETXTBSY` 竞态（真红）**。WSL 全量跑出现 2 个失败，原文：

```text
thread 'inflight_periodic_sweep_removes_stale_marker' panicked at apps/mcp-bridge/tests/mcp_contract.rs:7605:10:
启动真实 bridge 进程: Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }
thread 'inflight_startup_sweep_removes_stale_and_keeps_active' panicked at apps/mcp-bridge/tests/mcp_contract.rs:7510:43:
bridge 复制锁: PoisonError { .. }
test result: FAILED. 121 passed; 2 failed
```

根因：`fs::copy` 复制可执行文件时写 fd 未关，其它测试线程并发 `fork` 会把该 fd 带进子进程，本进程随后 `exec` 自己的副本就撞上 `ETXTBSY`（`Text file busy`）；复制锁只串行化了“复制+spawn”，锁不住别的用例的 fork。修复：新增 `spawn_bridge()`，仅对 `ETXTBSY` 做有界重试（100 × 20ms），重试成功时打印 `[evidence] spawn_retry attempts=... reason=ETXTBSY`，其它错误照旧 panic（不吞错）。WSL 连跑两轮 `123 passed; 0 failed`。这不是产品行为，是测试进程内 fork/exec 的固有竞态。

2. **未修，待决策：`_session/` 不进入回收**。每次 `initialize` 落一个 `runs/_session/initialize-<pid>-<ts>-0.json`；`runs.prune` 只枚举任务目录（`_session` 等 `_`/`.` 前缀一律跳过），因此这些连接审计文件**永不回收**。取消现场重启后仍能看到被杀进程的 session 文件。它是连接审计、不是 inflight 残标，本轮不擅自改保留策略；若要与 `runs/` 同寿命管理，需要单独定义保留口径。

3. **已知差异（未承诺）：Linux bridge 死亡遗留子进程**，见 §50.6。契约没有 cancel，也没有“supervisor 死亡后清场”的承诺；此处只记录事实，不当作缺陷修补。

### 50.8 真实链路 vs 夹具覆盖

| 链路 | 真实链路（MCP stdio） | 夹具/契约测试 | 覆盖结论 |
| --- | --- | --- | --- |
| 正常派单 → 回执 → 产物 → 审计自洽 | A（win+linux） | `execute_returns_auditable_handback` | 双证 |
| 超时 `blocked` + 受控集合静默 | B1（win+linux） | `execute_timeout_is_blocked_with_reason`、`timeout_cleans_controlled_set_including_grandchild` | 双证 |
| 执行中取消 | B2（win+linux，唯一真实入口是杀进程） | 无（产品无 cancel） | **仅真实链路** |
| 重启 startup 清扫（`trigger=startup`） | B2（win+linux） | `inflight_startup_sweep_removes_stale_and_keeps_active` | 双证 |
| 5s 容差不误删 / 过容差才删 | B3 | 同上、`runs_inflight_lists_and_cleans_only_stale_markers` | 双证 |
| 周期清扫 | 未跑（脚本固定 `AW_INFLIGHT_SWEEP_SECS=0`） | `inflight_periodic_sweep_removes_stale_marker` | 仅夹具 |
| 同 id 顺序重投（回放） | C1 | 无同名 dispatched 用例（契约 §3.1 口径） | **仅真实链路** |
| 同 id 异参拒绝 | C1 | `same_task_id_with_different_params_is_rejected` | 双证 |
| 同 id 两进程并发（dispatched） | C2 | `standalone_duplicate_execute_is_serialized_across_servers`（standalone） | 真实链路新增 dispatched 证据 |
| 不同 id 并行（dispatched） | C3 | `standalone_concurrent_tasks_are_isolated_from_one_timeout`（standalone） | 真实链路新增 dispatched 证据 |
| 逃逸进程整树清理（cgroup v2 + `cgroup.kill`） | 未跑（本轮无逃逸构造） | v1.45 `cgroup_kill_terminates_escapee_that_left_cgroup_procs` | 仅夹具（WSL 实跑，属单测而非 MCP 链路） |
| 无 `cgroup.kill` / `cgroup.procs` 读不出 | 未跑 | `cgroup_without_kill_reports_still_alive`、`cgroup_members_unreadable_is_not_empty` | 仅夹具 |
| `runs.prune` dry-run / trash / 竞态 | 未跑（不在本轮场景） | `prune_status_whitelist_selection` 等 | 仅夹具 |
| standalone 模式全链 | 未跑（本轮只 dispatched） | `standalone_*` 共 20+ 条 | 仅夹具 |
| `_session/` 回收 | 观测到不回收（§50.7） | 无 | 真实链路发现 |

复现命令（两平台同一脚本）：

```powershell
python scripts/real-chain-verify/verify.py --binary target/debug/aw-mcp-bridge.exe --label windows
```

```bash
python3 scripts/real-chain-verify/verify.py --binary /root/.aw-posix-verify/target/debug/aw-mcp-bridge --label linux
```

## 51. v1.47：Linux 孤儿兜底 / `_session` 保留 / 显式取消 + 真实链路补跑

本轮按 PM 指定顺序落地三项，并补齐 v1.46 真实链路还空着的两项（周期清扫、逃逸进程整树清理）。**红证据先贴**，再给绿。

### 51.1 红证据（改动前，真实链路原文）

| 项 | 红原文 / 事实 | 性质 |
|---|---|---|
| Linux 孤儿 | v1.46 B2 WSL：`child_survived=True`，心跳 `20 → 35` 字节；Windows 同为场景 `child_survived=False` | 平台差异造成的真缺口 |
| 取消 | v1.46 `tools/list` 实测 8 项，无 `task.cancel`；唯一「取消」入口是杀 bridge | 产品语义缺失 |
| `_session` | v1.46 §50.7：`initialize-<pid>-<ts>-<seq>.json` 永不回收（`runs.prune` 只枚举任务目录） | 无界增长 |
| 周期清扫 | v1.46 脚本固定 `AW_INFLIGHT_SWEEP_SECS=0`，周期档**未跑**（仅夹具） | 证据缺口 |
| 逃逸整树清理 | v1.46 无逃逸构造，仅 v1.45 单测 | 证据缺口 |
| 并发 prune | Windows 两条 race 用例偶发红，根因 `batch_id` 同毫秒同进程冲突共用 `.prune-trash/<batch>` | 测试基建缺陷 |

### 51.2 三项实现与边界（写进契约，不夸大）

1. **Linux `PR_SET_PDEATHSIG`**：`process_scope::configure` 在 `process_group(0)` 之后安装 `pre_exec`——`prctl(PR_SET_PDEATHSIG, SIGKILL)` 成功后立即复查 `getppid()`，与预期父 pid 不一致（父已死/已换父）则 `_exit(128)`，堵住 fork→prctl 竞态。
   - **只保直接子进程**：`PDEATHSIG` 不遗传，孙进程在 bridge 猝死后仍存活（B5 实测 `grandchild_survived=True`）。
   - 契约 §2.7 因此分成两段：Windows 作业对象整树；Linux 直接子进程 + **运维兜底**（`runs/.inflight` 标记 + cgroup 清单巡检，实测 `populated 1` pid 9428 → `kill -9` → `populated 0` → `rmdir ok`）。
   - 正常退出不受影响：正常收尾仍由既有 `Supervisor` 流程；已完成任务在 bridge 被杀前后产物与 `handback.json` **字节不变**（B2 新增断言）。
2. **`_session` 按天保留**：`AW_SESSION_RETENTION_DAYS`（1..=3650，默认 30），`sweep_session_records` 在 `startup`/`periodic` 档执行；候选要求「文件名毫秒时间戳 **且** `mtime` 都过期」，旧格式/不可解析/未来时间/仅一边过期 → 保留；先写 `runs/session-retention.jsonl`（`session-retention/1`，plan+result）再删，写入失败零删除；不删 `initialize.json`、`.tmp`、目录、符号链接。阈值不拍脑袋：D3 实测 `106.7–106.8 B/条`（Windows）/ `107.0 B/条`（WSL），1000 次/天、30 天约 `3.05/3.06 MiB`。
3. **`task.cancel`**：入参 `task_id` / `reason`（`maxLength=1024`，超长 → `invalid_reason`，零副作用）。只对**本进程**持有子进程的执行为生效对象；置位后由执行线程走与超时同一终止流程，落盘 `status=canceled`（非 blocked/completed），`blocked_reason` 前缀 `canceled：`；四态 `canceled` / `completed_before_cancel` / `cancel_requested` / `not_running`，只有 `canceled` 才 `cleanup_claimed=true`。审计三源可区分：`timeout：` / `canceled：` / `status=failed`。

### 51.3 工具描述与门禁

- 工具数 **8→9**，新增 `task.cancel`；`task.execute` 描述补取消语义。
- `tools_sha256` 更新为 `744da2614a3730f08d309033b7ca86c956edfab82d369443f06fd4ceecaba9da`。
- 新增门禁行：**G20**（`task.cancel` 描述必须含四种返回状态、`未发送信号`、`不得认为已清理`、`不会谎报已清理`、`跨进程不可见`、`受控集合`；禁止 `一定已清理` / `保证终止整棵进程树` / `跨进程生效`）、**G21**（`task_id.description` 含 `命名规则`、`派单标识`）。
- G1（`task.execute`）新增 `task.cancel`、`status=canceled` 两个 `contains`；`runs.prune.status` 白名单扩为四值，契约与用例同步。

### 51.4 绿：真实链路（MCP stdio，两平台）

| 场景 | Windows 24H2 | Ubuntu 24.04 (WSL2) |
|---|---|---|
| B2 bridge 被 `kill -9` 后直接子进程 | 心跳 `18 → 18`，`child_survived=False` | 心跳 `20 → 20`，`child_survived=False`（v1.46 为 `True`） |
| B2 已完成任务产物/回执 | 字节不变 | 字节不变 |
| B4 同连接执行中取消 | `status=canceled`、前缀 `canceled：`、请求方理由回显、心跳停止、标记清理、盘上 `handback.status=canceled` | 同左 |
| B4 未知 id 取消 | `not_running`，`cleanup_claimed=false`，不发信号 | 同左 |
| B5 `setsid` 孙进程 | 按能力位 skip（无 POSIX 会话） | `grandchild_survived=True`（心跳 `10 → 25`），**如实记录边界** |
| D1 周期清扫 | bridge1 猝死 → bridge2 `startup` 容差内不删，`periodic` 审计点名删除 | 同左 |
| D2 逃逸整树清理 | 按能力位 skip | `escaped_pid=10592 moved_out_of_cgroup=true escaped_gone=true sink_populated=0` |
| D3 `_session` 保留 | 45 天前删除、1 天前保留、坏名保留；plan/result 点名 | 同左 |

两平台各 `evidence_lines=17 / failures=0`。

### 51.5 回归

- Windows：`21 lib + 128 integration` 全绿（并发 prune/sweep race 用例连跑 3 轮）。
- WSL Ubuntu 24.04：`27 lib + 126 integration` 全绿；`scripts/posix-timeout-verify/verify.sh run` 四类 POSIX 用例全通过。

### 51.6 未证 / 边界（不得被本轮描述掩盖）

- Linux bridge 猝死后的**孙进程**仍存活（`PDEATHSIG` 不遗传）：不是「已解决」，是「直接子进程已收口 + 孙进程给出运维兜底」。（v1.48 已改为启动自动回收，见 §52；本行保留 v1.47 时的原始结论）
- `task.cancel` **跨进程不可见**：另一个 bridge 上的同 `task_id` 不能被本工具取消；此时只能 `runs.inflight` 判残标。（v1.48 已修，见 §52）
- `cancel_requested` 是「已请求、未观测到回执」，**不是**已清理；`not_running` 不发信号、不做清理。
- Windows 侧整树清理能力位不变（`requires_out_of_job_enumerable_terminable_os_primitive`）。

## 52. v1.48：启动回收遗留 cgroup 作用域 / `task.cancel` 跨进程可见 / B5 自动收敛

v1.47 把 Linux 孙进程残留在「已知限制 + 人工巡检」；本轮把它改成启动时自动收敛，并让取消请求跨进程可见。**红证据先贴，再给绿。**

### 52.1 红证据（改动前，真实链路原文）

| 项 | 红原文 / 事实 | 性质 |
|---|---|---|
| B5 Linux 孙进程 | `grandchild_survived=True`，心跳 `10 → 25`；`aw-mcp-bridge-<pid>-0` 作用域残留、无归属记录（sidecar），重启后孙进程仍存活且作用域未被回收 | 只能靠人工 `kill -9` + `rmdir` 巡检 |
| B6 跨进程取消 | 进程 B 对进程 A 正在执行的同 `task_id` 调 `task.cancel` → `status=not_running`、`inflight_marker` 可见但取消无效、`runs/.cancel` 目录不存在、A 侧继续执行到自然结束 | 多进程部署下取消失效 |
| 结果 | 两个平台同红；B5 仅在 Linux 触发（Windows 作业对象已在 v1.47 覆盖整树） | — |

### 52.2 实现（判据不放宽）

1. **归属 sidecar + 三条证据**：`process_scope::publish_owner` 在 `attach` 成功（`CgroupStatus::Active`）后写 `runs/.cgroup-scopes/<scope>.json`（`cgroup-scope/1`：`scope`/`cgroup`/`bridge_pid`/`bridge_start`/`task_id`/`attempt`/`root_pid`/`root_start`/`createdAt`）；作用域目录删除成功才删 sidecar。
2. **启动回收**：`reclaim_stale_cgroup_scopes_at(runs_dir, "startup")` 在 `auto_sweep("startup")` 里执行。清理条件 = sidecar 自洽 + `runs/<task_id>/attempt-<n>/dispatch.json` 存在 + owner bridge 已死（`/proc/<pid>/stat` starttime 与 `bridge_start` 不一致才算死，防 PID 复用误杀）。任一不满足 → `phase=kept` / `outcome=kept` / `action=none` / `reason`，零动作。空作用域只删目录；非空先写 plan 审计再 `cgroup.kill`（无则逐成员 SIGKILL、每轮重读 `cgroup.procs`），3s 内 `procs` 空且 `populated 0` 才删目录与 sidecar。
3. **审计**：`runs/cgroup-reclaim.jsonl`（`cgroup-reclaim/1`），**每个扫描到的作用域都写一行**（keep 也写），字段含 `trigger`、`scope`、`members_before`、`members_after`（读不出写 `null`）、`action`、`outcome`、`reason`。
4. **跨进程取消**：`runs/.cancel/<task_id>.json`（`cancel-request/1`，`requested_by`/`requestedAt`/`target{pid,startedAt}`，原子写）；`CancelHeartbeat` 在 spawn 前与执行循环每 200ms 读取，`pending` → 原子改写 `acknowledged`（写成功后才请求取消）→ 收尾写 `done`；target 不符写 `ignored`。`task.cancel` 响应带 `cross_process`/`request_state` 与三个时间戳；无本地 token 时每 100ms 读文件，10s 内等不到终态回 `cancel_requested`（`cleanup_claimed=false`）。
5. **请求清扫**：`runs/cancel-requests.jsonl`（`cancel-requests/1`），只删「`pending`/`acknowledged` 且 target pid 已死超过 5s」与「`done`/`ignored` 超过 30 天」，先写 plan 行、写不进则零删除。

### 52.3 工具描述与门禁

- `task.cancel` 描述改为跨进程语义（`runs/.cancel/<task_id>.json`、`request_state` 三态、`pending`/`acknowledged` 时不宣称已清理），schema 新增 `requested_by`（`maxLength=128`）。
- `tools_sha256`：`744da2614a3730f08d309033b7ca86c956edfab82d369443f06fd4ceecaba9da` → `9f1ee7443846d6c0e46e40e7c8f9f0ff206105a13444c1e191dc261b65c0203c`。
- 门禁 **G20** 更新：`contains` 去掉 `跨进程不可见`，加入 `跨进程可见`、`runs/.cancel/<task_id>.json`、`request_state`、`pending`、`acknowledged`；`not_contains` 保留 `一定已清理`/`保证终止整棵进程树`/`跨进程生效` 并新增 `跨进程不可见`。
- 门禁 **G22** 新增：`inputSchema.properties.requested_by.description` 必须含 `发起者`、`写入取消请求文件`、`缺省为 pid:`，禁止 `必填`。

### 52.4 绿：真实链路（MCP stdio，两平台）

| 场景 | Windows 24H2 | Ubuntu 24.04 (WSL2) |
|---|---|---|
| B5 `kill -9` 后孙进程 | 按能力位 skip（作业对象已覆盖） | 红时 `grandchild_survived=True`；绿 `grandchild_gone=True`、心跳停止、作用域目录与 sidecar 删除、`attempt-1`/`dispatch.json` 保留 |
| B5 审计 | — | plan/result `trigger=startup`、`members_before` 非空、`members_after` 空、`outcome=reclaimed` |
| B5 负向 decoy | — | 伪造 `aw-mcp-bridge-decoy-<pid>`（无 sidecar）→ 目录保留、审计 `outcome=kept` / `reason=owner_record_missing` |
| B6 三态 | `pending → acknowledged → done`，三个时间戳 | `pending → acknowledged → done`，三个时间戳 |
| B6 取消结果 | `status=canceled`、`cleanup_claimed=true`、target 与 inflight 标记一致、A 侧 `blocked_reason` 前缀 `canceled：`、handback `canceled` | 同左 |
| B6 再次取消 | `not_running`，已落盘的 `done` 请求保留、不被覆盖 | 同左 |

### 52.5 回归

- Windows：`23 lib + 129 integration` 全绿（新增 `v148_tests` 的 2 条跨进程用例 + 1 条跨实例集成测试）。
- WSL Ubuntu 24.04：`33 lib + 127 integration` 全绿（`scripts/posix-timeout-verify/verify.sh run`，含 4 条 Linux cgroup 回收单测）。
- 真实链路：Windows `18 evidence / 0 failures`、WSL `19 evidence / 0 failures`。
- 两次跨平台红都与本次改动无关：①`descendant_snapshot_cross_checks_process_group` 在并行高负载下出现 `outcome=Unavailable`（隔离连跑 5 轮全绿、整套重跑全绿）；②既有 `sweep_concurrent_claimers_delete_once_others_lost_race` 在并发高负载下偶发 `restore_failed`（重跑全绿，未改该路径代码）。

### 52.6 未证 / 边界（不得被本轮描述掩盖）

- 回收**只在启动时**做，不做周期；bridge 长期运行期间出现的新孤儿要等下一次启动。
- 无 cgroup v2 / `/sys/fs/cgroup` 只读 → `state=unsupported`，不扫描、不写审计；无 `cgroup.kill` 时逐 pid 终止存在 PID 复用窗口（每轮重读成员表收紧，不承诺零窗口）。
- 跨机器不可见：sidecar、`.cancel` 请求都只在同一 `runs/` 内有效。
- `pending` 在真实链路两个平台都观测到了（20ms 轮询），但窗口随执行侧调度变化，可能短于观测间隔；`requestedAt`/`acknowledgedAt`/`finishedAt` 三个时间戳始终可区分三阶段，不以「没轮询到 pending」当作未实现。

## 53. v1.49：两条不稳定用例的诊断 / 定性 / quarantine，以及 B7 纯文件跨进程取消

### 53.1 红证据（改动前原文，不是转述）

**用例 B：`server::inflight_sweep_race_tests::sweep_concurrent_claimers_delete_once_others_lost_race`（Windows）**

`cargo test -p aw-mcp-bridge --lib inflight_sweep_race_tests -- --test-threads=8` 连跑 30 轮，第 30 轮红，4 个清扫者里 1 个记 `failed`：

```
assertion `left == right` failed: 其余清扫者必须记 lost_race，实际 2
failed[0].error = "系统找不到指定的文件。 (os error 2)；还原失败：系统找不到指定的文件。 (os error 2)"
failed_total = 1, deleted_total = 1, lost_race_total = 2
```

即：赢家删除成功（`deleted_total=1`），两个清扫者读到 `Gone` 记 `lost_race`，但有一个清扫者的**认领副本在「复核为 stale」之后、`remove_file` 之前消失**，被记成 `failed`。

**用例 A：`server::process_scope::imp::tests::descendant_snapshot_cross_checks_process_group`（POSIX）**

用户全量 POSIX 并行跑（`test-threads>=16`）出现 1 次 `outcome=Unavailable`（终止返回不可观测，断言要求 `Cleaned`）。本地改动后 8 轮全量并行（32 条用例 / `--test-threads=16`，日志 `/root/.aw-posix-verify/logs/v149-stress.txt`）0 失败；改动前没有有效的本地复现样本——早先 16 轮 WSL 循环因 `cargo` 不在 PATH 实际未运行，**不计入观测频率**。

### 53.2 定性（两者分开写，不混）

- **用例 B 是产品在高并发下的缺陷（分类错误），不是测试自身不稳。** 产品注释已写明「Windows 上多个并发 rename 可能都返回成功，但只有一个目的地真的产生文件」，但 `rename Ok` 之后只处理了「副本读不到 → lost_race」和「复核后不是 stale → not_mine」，漏了「复核为 stale、删除/还原时副本已消失」这一窗口，把它记成 `failed` 并写进审计。修法：删除与还原两处遇到 `NotFound` 一律记 `lost_race: claim_copy_gone`（谁先处置谁赢，不产生假失败）；`restore_without_clobber` 写回原位后 `source` 消失也不再算失败。
- **用例 A 归为「测试自身不稳」（不是产品在高并发下谎报清理）。** `Unavailable` 是安全侧降级（观测不到就不宣称清空），方向正确；不稳的是「高负载下仍要求必然 `Cleaned`」这条断言。同时暴露了观测健壮性缺口：`proc_table()` 原先「任意一条 `/proc/<pid>/stat` 读失败 / 解析失败即放弃整份扫描」过于脆弱。已收紧为：只有确认 `/proc/<pid>` 已不存在（读失败、解析失败、空内容）才跳过；进程仍存在时的读失败/解析失败仍然整份返回不可观测——判据没有放宽。根因未能证实，因此**仍 quarantine**，下次红时由 53.3 的诊断直接给出具体 `unobservable` 原因，再决定是否升级为产品缺陷；不以「重跑全绿」结案。

### 53.3 诊断信息（下次红能直接定位）

- 清扫并发用例：按标记路径隔离的时序记录 `sweep_trace`，每条事件带「相对毫秒 / 线程 id / 动作 / 现场」，动作覆盖 `claim_attempt` / `claim_failed` / `reread_claim_copy`（含 `exists` 与 `len`）/ `claim_copy_removed|vanished|remove_failed`；失败断言一次性附上 `.inflight` 目录清单（名字、长度、mtime）、审计原文、各清扫者结果、CPU 数。
- 终止/快照用例：线程内时序记录 `terminate 开始 / 终止前名单 / 清扫循环开始 / 每轮 group_exists / 各 Unavailable 的具体原因（cgroup_scope_lost / cgroup_procs_unreadable:<原因> / cgroup_events:<原因> / snapshot:<原因>）/ StillAlive 原因`；`Supervisor::diagnostics` 再 dump `pgid / root_start / cgroup.status / cgroup.procs / cgroup.events / members() / populated() / /proc 行数 / 终止后快照 / 观测并发 active+peak`。实测并行时 `peak=11`，可直接看到失败当时的并发度。
- 确定性回归：新增 `sweep_claim_copy_vanishing_after_reread_is_lost_race_not_failure`，用 `set_sweep_after_reread_hook` 在「复核后、删除前」删掉认领副本，强制走该窗口，断言 `failed_total=0 / deleted_total=0 / lost_race_total=1 / reason=claim_copy_gone`。

### 53.4 quarantine：默认门禁按 WARN 呈现，不静默通过

- 清单：`scripts/test-gate/quarantine.txt`（测试全名 | 平台 | 负责人 | 自版本 | 触发条件 | 观测频率）。
- 分类器：`scripts/test-gate/gate.sh`（POSIX）/ `gate.ps1`（Windows）。语义固定：
  - 清单用例**照常运行**，不 skip、不 `#[ignore]`；
  - 运行结果一律打印 `[WARN] quarantined <name> result=ok|FAILED|ignored`——**通过也只是 WARN，不记成 PASS**；
  - 失败不阻断门禁（退出码 0），但汇总行为 `[GATE] warn=N fail=0 — …不代表全绿结论`；非清单用例失败退出码 2；
  - 清单条目没被观测到（被过滤/改名）也打印 WARN，避免清单悄悄失效；
  - **没有证据不判绿**：日志不存在/不可读、日志里一条结果行都没有（编译/链接失败、被过滤）、cargo 退出码非 0 且没有任何 FAILED 行，一律打印 `[GATE] fail=1 …` 并以 2 退出。quarantine 只豁免清单内用例的 FAILED，不豁免「跑都没跑起来」。
  - 实测负向对照：`gate.sh --log /tmp/aw-no-such-log.log` → `[GATE] fail=1 — 日志不存在或不可读`，退出 2；只含编译输出的日志 → `[GATE] fail=1 — 未解析到任何测试结果行`，退出 2；`gate.ps1 -LogFile` 指向不存在文件同样是 2。
- `scripts/posix-timeout-verify/verify.sh` 已接入：全量测试后用 `gate.sh --log` 分类，裁剪 workspace 时同步复制 `scripts/test-gate/`。

### 53.5 B7：真实派单链路的纯文件跨进程取消（两平台）

| 角色 | 进程 | 做什么 |
| --- | --- | --- |
| 派单方 | 验证进程 | 真实 `task.execute`，落盘 `runs/<task>/attempt-1/dispatch.json` + `runs/.inflight/<task>.json` |
| 执行侧 | bridge 进程 | 200ms 心跳读 `.cancel/<task>.json`，收敛后写 acknowledged / done |
| 取消方 | **独立 OS 进程，不是 MCP 客户端** | 按 `cancel-request/1` 直接写文件（`O_EXCL` + fsync），自己轮询三态 |

断言：`canceller.pid != executor.pid`；`requested_by=b7-canceller-pid-<canceller.pid>`；`handledBy.pid=executor.pid`；`target{pid,startedAt}` = 活跃标记；`dispatch.json` 属于本次派单；取消进程观测到 `pending→acknowledged→done`；执行侧 `canceled`、`blocked_reason` 前缀 `canceled：`；盘上 handback `canceled`；标记删除；心跳停止。整条取消链路只依赖文件。

红→绿：首跑 B7 红——取消方进程未创建 `.cancel/` 父目录，`O_CREAT` 直接失败（`written=null`，执行侧没收到请求）；补 `os.makedirs(dirname, exist_ok=True)` 后两平台绿。这条红本身证明「取消只走文件」不是靠测试客户端代劳。

### 53.6 回归（本轮实测）

- Windows：`gate.ps1` → lib 24 + integration 129 全绿，`[GATE] warn=0 fail=0`；清扫并发用例改后连跑 40 轮全绿。
- WSL Ubuntu 24.04（cgroup2fs，root）：`verify.sh run` → lib 34 + integration 127 全绿，四类 POSIX 证据齐全；门禁对 quarantine 用例打印 `[WARN] … result=ok`。
- 真实链路（MCP stdio + 独立取消进程）：Windows `19 evidence / 0 failures`，Linux `20 evidence / 0 failures`（均含新增 B7；两平台各自实测三进程与三态）。
- 已知不稳定观测频率：A 1 次（2026-11-08 全量 POSIX）；改后 8 轮全量并行未复现。B 1 次 / 30 轮（改前）；改后 40 轮 0 次。

### 53.7 未证 / 边界

- A 的根因未证实（未能抓到失败瞬间的 `unobservable` 原因）；quarantine 移除条件：连续 30 次默认门禁 + 10 次隔离 soak（`--test-threads=1` 与 16）全绿，并写下根因结论。不满足前它始终以 WARN 呈现。
- `proc_table` 只在「条目已退出」时跳过；hidepid / 权限导致的读失败仍让整份观测不可用（不放松「没有证据不宣称清空」）。
- B7 的 `pending` 窗口仍可能短于观测间隔；三阶段区分以盘上 `requestedAt/acknowledgedAt/finishedAt` 为准。

## 54. v1.50：A 的复现驱动 / 根因定向 / quarantine 临时化，门禁负向对照常驻，PM 接入须知

### 54.1 复现驱动与频率表（实跑）

驱动：`unavailable_frequency_under_controlled_load`（`#[ignore]`，Linux）+ `scripts/posix-timeout-verify/unavailable-repro.sh`。
每个 observation worker 独立「建作用域 → spawn sh + N 个 sleep 后代 → 等快照可观测 → terminate」，同时可挂 fork/exit churn 与 `/proc` 狂扫负载线程。
记录：CPU 核数、worker 数、负载线程数、观测并发峰值（生产同款 `ObservationGuard` 计数）、系统 loadavg、容器/裸机、Unavailable 次数与 `原因=` 原文。

WSL Ubuntu 24.04 / 16 核 / bare-metal(VM) / cgroup=Active，每 worker 100 轮：

| workers | load_threads | 总轮数 | cleaned | unavailable | still_alive | observe_fail | rate | peak_obs | max_loadavg |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 0 | 100 | 100 | 0 | 0 | 0 | 0.0000 | 2 | 1.73 |
| 4 | 0 | 400 | 400 | 0 | 0 | 0 | 0.0000 | 8 | 0.77 |
| 8 | 16 | 800 | 800 | 0 | 0 | 0 | 0.0000 | 14 | 8.40 |
| 16 | 64 | 1600 | 1600 | 0 | 0 | 0 | 0.0000 | 21 | 41.19 |

合计 2900 轮，`Unavailable=0`，`observe_fail=0`；峰值 21 已高于原失败现场的 11。
另做全量 POSIX lib 用例 `--test-threads=16` 重复 12 轮，全部通过（加上 v1.49 的 8 轮，改后累计 20 轮全量并行 0 失败）。

### 54.2 判定：哪一侧

**结论：A 属于「观测器把单个进程退出的 `/proc` 读竞态升级为整表不可用」的健壮性缺口，是测试/观测侧问题，不是产品在高并发下真有问题。**

- **历史样本不能直接归因**：那次失败发生在诊断字段上线前，没有留下 `原因=`，所以不能从旧日志读出侧别。
- **按代码路径排除 cgroup 侧**：`terminate` 在作用域仍 Active 且由本线程持有时才会读到 `cgroup.procs` / `cgroup.events`；同一线程内作用域不可能在终止前被 Drop，`cgroup_scope_lost` 也不是负载相关路径。原症状只可能来自 `procfs_unavailable`。
- **确定性注入把两个分支钉死**（`proc_fault`，仅测试编译）：
  - `proc_read_failure_after_process_exit_is_skipped_not_unavailable`：读失败 + 进程已退出 → 跳过该条目，整表仍可用；
  - `proc_read_failure_for_live_process_is_unavailable`：读失败 + 进程仍存活 → 整表判 `Unavailable`（安全侧不变量不放宽）。
- **为什么改后抓不到**：v1.49 的 `proc_entry_gone` 跳过恰好移除了「进程在 readdir 与 read 之间退出」这条最常见的触发路径，失败率从偶发降到本轮 0/2900。因此 quarantine 保留，但性质从「原因不明」变为「机制已定向、样本不再出现」。

### 54.3 quarantine 已按判据解除（2026-11-08）

- **清单条目已移出**：`server::process_scope::imp::tests::descendant_snapshot_cross_checks_process_group`。`scripts/test-gate/quarantine.txt` 现在为空（只保留移出记录与恢复条件），该用例改按普通用例判绿/判红；再红时门禁直接 exit 2，不再降级成 WARN。
- **解除判据**（与 §53.7 一致，写死在这里）：连续 30 次默认门禁 + 10 次隔离 soak（`--test-threads=1` 与 16 各半）全绿，且本文的根因结论未被新证据推翻。
- **达标记录**：30/30 次默认门禁 + 10/10 次隔离 soak（5 次 `--test-threads=1`、5 次 `--test-threads=16`），全部 `rc=0`、`test result: ok`，逐次记录 `原因=` 全为「无」，`cgroup` 列全为「无」；任一次红即 streak 归零重计，本次全程未触发重计。跑批期间未出现 `cgroup_*`，§54.2 的「观测器把 /proc 读竞态升级为整表不可用」结论未被推翻；机制仍由两个确定性注入用例覆盖。
- **原始记录**：
  - 默认门禁 30 次：`/root/.aw-posix-verify/logs/quarantine-batch-20261108-080540/summary.tsv`（该目录另含 10 次默认线程 soak，作为附加证据，不计入判据）。
  - 隔离 soak 10 次：`/root/.aw-posix-verify/logs/quarantine-soak-20261108-081408/summary.tsv`。
  - 两份 `summary.tsv` 原文：

```tsv
# quarantine-batch-20261108-080540/summary.tsv（Linux / 2026-11-08）
date	platform	kind	run	rc	ok	reason	cgroup
2026-11-08	Linux	gate	1	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	2	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	3	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	4	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	5	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	6	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	7	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	8	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	9	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	10	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	11	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	12	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	13	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	14	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	15	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	16	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	17	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	18	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	19	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	20	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	21	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	22	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	23	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	24	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	25	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	26	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	27	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	28	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	29	rc=0	ok=1	原因=无	无
2026-11-08	Linux	gate	30	rc=0	ok=1	原因=无	无
# quarantine-soak-20261108-081408/summary.tsv（Linux / 2026-11-08）
date	platform	kind	run	threads	rc	ok	reason	cgroup
2026-11-08	Linux	soak	1	1	rc=0	ok=1	原因=无	无
2026-11-08	Linux	soak	2	1	rc=0	ok=1	原因=无	无
2026-11-08	Linux	soak	3	1	rc=0	ok=1	原因=无	无
2026-11-08	Linux	soak	4	1	rc=0	ok=1	原因=无	无
2026-11-08	Linux	soak	5	1	rc=0	ok=1	原因=无	无
2026-11-08	Linux	soak	6	16	rc=0	ok=1	原因=无	无
2026-11-08	Linux	soak	7	16	rc=0	ok=1	原因=无	无
2026-11-08	Linux	soak	8	16	rc=0	ok=1	原因=无	无
2026-11-08	Linux	soak	9	16	rc=0	ok=1	原因=无	无
2026-11-08	Linux	soak	10	16	rc=0	ok=1	原因=无	无
```

- **恢复条件**：下一次红时先看诊断里的 `原因=`：`procfs_unavailable` → 按原格式把条目加回清单；`cgroup_*` → 「观测器降级」结论作废，需重开根因分析，不能沿用 §54.2。
- **跑批脚本的两个坑（已修，记录在此避免复发）**：
  1. WSL 非登录 shell 的 PATH 不含 Rust，`/root/.cargo/bin/cargo` 也不存在；必须显式 `export PATH="/root/.rustup/toolchains/1.88.0-x86_64-unknown-linux-gnu/bin:$PATH"`，否则每次都是 `cargo: command not found`（rc=2）。
  2. soak 带 `--nocapture` 时 `[evidence]` 会打在 `test <name> ...` 同一行，`grep 'test <name> ... ok'` 永远不匹配（首版 37 次全被误判为红）。判绿必须用 cargo 汇总行 `test result: ok. 1 passed; 0 failed`。

### 54.4 门禁负向对照：从「验过一次」变成常驻用例

- `scripts/test-gate/selftest.sh` / `selftest.ps1`：7 条固定对照——
  1. 日志不存在 → exit 2；
  2. 编译失败（零结果行）→ exit 2；
  3. 非 quarantine FAILED → exit 2 且打印 `[FAIL]`；
  4. quarantine FAILED → exit 0 且 `[WARN] quarantined … result=FAILED`；
  5. quarantine ok → exit 0 且 `[WARN] … result=ok`；
  6. cargo 非零且无 FAILED 行（有结果行也不能判绿）→ exit 2；
  7. 干净日志 → exit 0 `[GATE] warn=0 fail=0`。
- 接线：`verify.sh run` 在跑测试前先执行 `selftest.sh`，失败即 `reason=gate_selftest_failed` 终止；`gate.ps1` 默认先跑 `selftest.ps1`（`selftest.ps1` 内部用 `-SkipSelfTest` 防止递归）。两平台各 7/7 通过。
- 第 6 条用 PATH 前缀注入假 `cargo`（打印一行 ok 后退出 1）验证，不需要真的制造编译失败。
- **第 4/5 条改用固定 fixture**（v1.51）：生产清单合法地可能为空（§54.3 已把唯一条目移出），负向对照不能因此失去 WARN 分支。`selftest.sh` / `selftest.ps1` 各自在临时目录生成 `aw_gate_selftest::synthetic_quarantined_case`，分别经 `AW_GATE_QUARANTINE_FILE` 与 `-QuarantineFile` 传给门禁；门禁默认仍读 `scripts/test-gate/quarantine.txt`。改后两平台复跑仍 7/7（原始输出见 §55.8）。

### 54.4.1 竞态连跑：可选档（默认关闭）

- **参数**：Windows `-RaceRepeat N`（`gate.ps1`，默认 `1`）；POSIX `--race-repeat N` 或 `AW_RACE_REPEAT=N`（`gate.sh`）；`verify.sh run` 读 `AW_RACE_REPEAT`（默认 `1`）。
- **语义**：`N=1` 与旧行为完全一致——主套件里本来就跑 1 次该用例，不额外耗时。`N>1` 时在主套件之后额外连跑 `N` 次单测 `server::prune_claim_race_tests::prune_concurrent_claimers_delete_once_others_lost_race`（每次内部 20 轮）。
- **判定**：必须同时满足「退出码 0」和「输出出现 `test result: ok. 1 passed`」。只看退出码会假绿——`--exact` 名字写错时 cargo 以 0 个用例、退出码 0 返回。
- **可观测**：每次打印 `[RACE] run i/N ok`，结尾打印 `[RACE] repeat=N ok=… bad=…`；默认档也打印 `[RACE] repeat=1（默认，未启用连跑）`，避免「以为跑了 12 次其实跑了 1 次」。
- **实测（v1.51）**：

  | 平台 | 默认档 | `N=12` |
  | --- | --- | --- |
  | Windows（`gate.ps1`，含 selftest） | 49.2s，`[RACE] repeat=1` | 56.8s，`ok=12 bad=0` |
  | WSL（`verify.sh run`） | 12.03s，`repeat=1（默认，未启用）` | 26.86s，`ok=12 bad=0` |

  两次 `N=12` 的 `[GATE] warn=0 fail=0` 均未变化；`quarantine.txt` 仍为空、`tools_sha256` 不变。
- **建议用法**：日常提交跑默认档；夜间或发版前跑 `-RaceRepeat 12`（WSL 侧 `AW_RACE_REPEAT=12 … verify.sh run`）。成本约 +7.6s（Windows）/ +14.8s（WSL），不是数量级增长。
- **不默认开启的原因**：单次门禁已经覆盖该用例 1 次；连跑的价值是压低偶发窗口，不该让所有人的每次提交都付这份成本。

### 54.5 PM 接入须知与自测入口

- 文档：`docs/PM-INTEGRATION-GUIDE.md`。只依赖公开 MCP 接口与 `runs/` 目录约定，覆盖：初始化、派单参数/幂等、超时的三档清理措辞、MCP 取消与纯文件取消、`pending/acknowledged/done` 三态、「已清理」必须同时满足 `status=canceled && cleanup_claimed=true && handback_status=canceled`、回执字段与产物哈希、错误矩阵、多进程部署、验收清单。
- 自测：`scripts/pm-integration-selftest/pm_selftest.py`（仅标准库）。同一连接上模拟三种 PM 行为：正常派单（含产物哈希与 `receipt.get` 回读）、超时（`timeout：` 前缀与 `duration_ms`）、取消（长任务执行中发 `task.cancel`，校验三态与执行侧回执）。
- 实测：Windows `pass=32 fail=0`，Linux `pass=32 fail=0`。首跑红一次：断言把 `artifacts[].locator` 当成了相对 `runs/`，实际相对可执行文件目录且含 `runs/` 前缀；修正后全绿，文档同步改正。

### 54.6 回归（本轮实测）

- Windows：`gate.ps1`（含自测）→ selftest 7/7；lib 24 + integration 129；`[GATE] warn=0 fail=0`。
- WSL：`verify.sh run` → selftest 7/7；lib 36（新增 2 条注入用例，另有 1 条复现驱动 marked ignored）+ integration 127；quarantine 用例 `[WARN] … result=ok`。
- 复现驱动：2900 轮 0 Unavailable；全量并行 12 轮 0 失败。
- PM 自测：两平台各 32/32。

### 54.7 未证 / 边界

- 未拿到改前同款失败在新诊断下的样本：根因是「代码路径排除 + 注入机制」定向，不是从旧日志直接读出。
- 注入只证明分类分支（跳过 vs 不可观测），不证明内核在那一刻一定返回该错误；真实 `hidepid`/权限导致的读失败仍走 `Unavailable`。
- 复现驱动的负载是合成负载；容器（docker/k8s）与裸机对比只在当前 WSL VM 上做过，容器内结果未采集。

---

## 55. v1.51：PM 联调缺陷收口（B1 / B2 / A3 + 其余 12 条）

输入：PM 侧 `E:\软件开发\云集项目管家\docs\aw-pm-integration-verification-2026-11-08.md`（只读）。本轮遵循「以实测行为为准，不为迁就文档改行为，除非行为本身会把调用方挂住」。

### 55.1 B1：执行中重复派发（结论：改文档，不改行为）

**选择**：保留「阻塞到原执行结束再回放回执」，文档按实测写清；不改成「立即返回 `task_in_progress`」。**理由**：`task.execute` 本身就阻塞到回执；重复派发绝大多数是传输层重试，立即报错会把合法重试变成假失败；阻塞有明确上限 `timeout_secs + 5s`（`timeout_secs ≤ 600s`，故最长 605s）；等待 `attempt-1/dispatch.json` 最多 5s，仍缺失才 `task_id_conflict:`；N 个并发重投各自阻塞回放，`task.cancel` 仍由主循环处理。

**可复现**（真实 MCP stdio 会话，原执行的命令 `ping -n 20 127.0.0.1`，重复派发后分别在 1.5s / 4s 探测）：

```text
B1 duplicate @1.5s responded=False
B1 duplicate @4s responded=False
B1 duplicate total_elapsed_ms=4322
B1 duplicate isError= False status= canceled attempt= 1 finishedAt= 2026-11-07T22:36:56Z
B1 first finishedAt= 2026-11-07T22:36:56Z
```

原执行被取消收尾后，重复派发才返回，且 `finishedAt`、`attempt` 与首次回执一致（原样回放）。**落点**：`PM-AW-DISPATCH-CONTRACT.md` §2.6.11（新增）、§2.6.4 / §2.6.9（更正）；`PM-INTEGRATION-GUIDE.md` §2.4（重写）。

### 55.2 B2：`task.cancel` 的实测响应（结论：改文档）

**实测**（同一探针，最终代码）：`task.cancel` 阻塞 108ms 返回，直接 `status=canceled`，并带未文档化的 `handback`（18 键完整 canceled 回执）与 `message`：

```text
B2 cancel elapsed_ms= 108
B2 cancel keys= ['acknowledged_at', 'canceled', 'cleanup_claimed', 'cleanup_detail', 'cleanup_status',
 'cross_process', 'finished_at', 'finished_observed', 'handback', 'handback_status', 'message',
 'request_file', 'request_state', 'requested_at', 'requested_by', 'requested_reason', 'status', 'task_id']
B2 cancel status= canceled cleanup_claimed= True handback_status= canceled request_state= done
B2 message= 取消已生效：执行线程完成终止并落盘 canceled 回执
B2 handback keys= ['artifacts', 'attempt', 'blocked_reason', 'changed_files', 'changed_files_truncated',
 'cleanup_status', 'commit', 'cwd', 'duration_ms', 'env_keys', 'exit_code', 'finishedAt', 'path_prepend',
 'status', 'stderr_digest', 'stdout_digest', 'task_id', 'timeout_secs', 'verify_cmd']
B2 handback.status= canceled
```

只有等不到回执（`CANCEL_WAIT_SECS=10s` 超时）才返回 `cancel_requested`；示例里的 `cleanup_claimed=false` 与 `acknowledged` 是误解。**落点**：`PM-INTEGRATION-GUIDE.md` §4.3（用实测返回替换 + 字段说明表）、§5；`PM-AW-DISPATCH-CONTRACT.md` §2.12 / §3.6。

### 55.3 A3：文件通道「已清理」判据（结论：改代码 + 文档）

**问题**：旧 `cleanup_claimed` 在文件通道终态里不存在，且旧判据把「`canceled=true`」当成「已清理」——但 `canceled` 只说明终止动作执行过，受控集合仍可能 `StillAlive` / `Unavailable`，会误报「已清理」。

**改法**：新增机器可读 `cleanup_status`（`cleaned` / `still_alive` / `unavailable` / `not_started` / `null`），写入 handback、`task.cancel` 响应、取消请求 `done` 终态三处；`cleanup_claimed` 收紧为 `handback_status="canceled" && cleanup_status="cleaned"`。文件通道等效判据：`status=done && handback_status=canceled && cleanup_status=cleaned`。`not_started`（取消在 spawn 前到达、从未启动进程）单独说明为「必然无存活、无需清理」，但 `cleanup_claimed=false`。

**修复前**（文件终态键，无 `cleanup_status` / `cleanup_claimed`）：

```text
A3 terminal keys=['acknowledgedAt','finishedAt','handback_status','handledBy','reason','requestedAt',
 'requested_by','status','target','task_id','version']
```

**修复后**（同一探针，真实取消）：

```text
A3 states_seen= ['pending', 'acknowledged', 'done']
A3 terminal= {"status": "done", "handback_status": "canceled", "finishedAt": "2026-11-07T22:36:57Z",
 "cleanup_claimed": true, "cleanup_status": "cleaned"}
A3 execute keys= [..., 'cleanup_status', ...]
```

**PM 不会把「部分条件」误判成已清理**：回归用例 `cancel_request_terminal_state_never_overclaims_cleanup` 先对真实取消记录断言 `claims_cleaned=true` 且机器写入的 `cleanup_claimed=true`，再对同一记录做五种变异——`cleanup_status=still_alive`、`cleanup_status=unavailable`、删除 `cleanup_status`、`status=acknowledged`、`handback_status=ok`——判据全部为 false。另有单测 `cleanup_claim_requires_canceled_and_cleaned` 直接锁 `cleanup_claim_allowed`。

**落点**：`server.rs`（`cleanup_status_label` / `cleanup_claim_allowed` / `CancelHeartbeat::finish`）、`tests/mcp_contract.rs`、契约 §2.12 / §2.13 / §3.6、指南 §0 / §4.2 / §4.4 / §5。

### 55.4 反向验证：防退化断言真的会报红

**门禁（描述指纹 + 行断言）**：临时把 `status` 描述末尾的「；不回显任何配置值」删掉，`cargo test -p aw-mcp-bridge description_gate` 原文：

```text
test server::description_gate::description_gate_rows_hold_on_tools ... FAILED
test server::description_gate::description_gate_snapshot_matches_audit ... FAILED
G9：status 的 description 缺少必需措辞「不回显任何配置值」：...
assertion `left == right` failed: tools/list 描述或 schema 已改动：请同步 TOOLS-DESCRIPTION-AUDIT.md §21（行断言 + tools_sha256）。
实际=dc8e141ffa4ac725c6b145b0348db25ae8fe027ceb7b1232c6e37c481c9abb16
  left: "dc8e141ffa4ac725c6b145b0348db25ae8fe027ceb7b1232c6e37c481c9abb16"
 right: "95062dad5bc83f4c19e2abb0adfb7010269975f3277445cfaf50d18bba78e316"
test result: FAILED. 1 passed; 2 failed
```

**自测器（PM 行为断言）**：临时把 `task.cancel` 响应退回旧行为（无 `cleanup_status`、`cleanup_claimed=status==canceled`），`pm_selftest.py` 原文：

```text
[FAIL] cancel: cleanup_status=cleaned — None
[SELFTEST] pass=39 fail=1
```

**单测（清理判据）**：同一临时回退下，`cargo test -p aw-mcp-bridge --lib cleanup_claim_requires_canceled_and_cleaned` 原文：

```text
thread '...cleanup_claim_requires_canceled_and_cleaned' panicked at apps\mcp-bridge\src\server.rs:11483:9:
assertion failed: !cleanup_claim_allowed(Some("canceled"), Some("still_alive"))
test result: FAILED. 0 passed; 1 failed
```

恢复后全部转绿：`description_gate` 3 passed；`pm_selftest.py` `pass=40 fail=0`；`--lib cleanup_claim` 2 passed。门禁指纹最终为 `95062dad5bc83f4c19e2abb0adfb7010269975f3277445cfaf50d18bba78e316`。

### 55.5 两平台实跑（原始输出）

全部在 §55.10 修复**之后**的代码上重跑（v1.51 回归确认，含新增的 `rename_failure_treats_missing_source_as_lost_race`）。

```text
# Windows：powershell -File scripts/test-gate/gate.ps1（含 selftest），墙钟 51.3s
[SELFTEST] pass=7 fail=0
running 27 tests
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.51s
running 130 tests
test result: ok. 130 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 25.51s
[GATE] warn=0 fail=0 — 非 quarantine 用例全部通过；已知不稳定按 WARN 呈现，不代表全绿结论

# Windows：cargo check -p aw-mcp-bridge
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.50s

# WSL（POSIX）：AW_VENDOR=/root/vendor bash scripts/posix-timeout-verify/verify.sh run，墙钟 11.7s
[INFO] 运行 POSIX 四类用例（离线，日志 /root/.aw-posix-verify/logs/posix-verify-20261108-200449.log）
running 40 tests
test result: ok. 39 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.71s
running 128 tests
test result: ok. 128 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.47s
[GATE] warn=0 fail=0 — 非 quarantine 用例全部通过；已知不稳定按 WARN 呈现，不代表全绿结论
[INFO] 四类 POSIX 用例：非 quarantine 全部通过，证据行齐全；已知不稳定按 WARN 呈现
```

- `1 ignored` 是既有复现驱动（`#[ignore]`，手动触发），不是 quarantine；清单已清空后 WSL 门禁的 `warn=0` 证明该用例已按普通用例判绿。
- `--nocapture` 下 evidence 行（`[evidence] timeout_reason=…`、`cgroup_*`、`setsid escaped_pid=…` 等）全部命中，`verify.sh` 未报 `missing_evidence`。

```text
# PM 行为自测（scripts/pm-integration-selftest/pm_selftest.py --binary …，两平台各 40 项断言）
Windows: [SELFTEST] pass=40 fail=0
WSL:     [SELFTEST] pass=40 fail=0     （含 status.contract_version=1.51 / cleanup_status / 请求文件 cleanup_claimed 断言）
```

### 55.6 其余 12 条逐条处理状态

| 编号 | 级别 | 结论 | 落点 | 依据（实测/动作） |
| --- | --- | --- | --- | --- |
| A1 | S1 | 改文档 | 指南 §10（9 工具进参/出参/读写性质表） | 以 `tools/list` 实际 schema 抄录，PM 无需再翻源码 |
| A2 | S1 | 改文档 | 指南 §4.2（`.inflight` 样例 + 出现时机 + 半截读重试 5×20ms） | 实测键 `version/task_id/mode/state/pid/startedAt/deadlineAt`；`File::create+write+sync` 非原子可见 |
| A4 | S3 | 改文档 | 指南 §1.1 | 实测关 stdin 后 29.1s 等执行收尾才退出、退出码 0、无残留进程；要立刻停必须先 cancel |
| A5 | S3 | 改文档 | 指南 §5 / §10 | 实测 `isError=true`、`text={"error":"回执不存在：<task_id>"}` |
| B3 | S2 | 改文档 | 指南 §4.2 / 契约 §2.12 | 一个 200ms 心跳内可直接 `pending→done`，`acknowledgedAt=null`；不能把 ack 当稳定证据 |
| B4 | S2 | **改代码 + 文档** | `status.contract_version="1.51"`；指南开头版本识别 | 实测 `serverInfo.version=0.1.0` 只是二进制版本；契约版本需要机器可读字段 |
| C1 | S3 | 改文档 | 指南 §4.3 / §5 | 实测 `cleanup_detail` = 回执 `blocked_reason` 原文，`string|null` |
| C2 | S3 | 改文档 | 指南 §5 前缀表 | 完整列出 `timeout：` / `canceled：` / `command_not_found:` / `cwd_not_found:` / `standalone_*`，标注全角/半角冒号 |
| C3 | S3 | 改文档 | 指南 §5 | 任意终态都保证 stdout/stderr 两个产物，spawn 前创建 |
| C4 | S3 | 改文档 | 指南 §4.3 / §5 | 实测响应含 `handback`（18 键）与 `message`，可直接用 handback 免二次 `receipt.get` |
| 接入注意 1 | — | 改文档 | 指南 §10 注 | `inputSchema.required` 只有 `task_id`，schema 不能当前置校验器 |
| 接入注意 2 | — | 改文档 | 指南 §2.1 / §10 注 | `argv`/`acceptance`/`max_attempts` 仅 standalone，dispatched 传入被拒 |

### 55.7 明确不做

- **不改 `task.execute` 的阻塞语义**（B1）：见 55.1 理由。
- **不把 `task.execute` 的 schema 改成按 mode 动态 required**：JSON Schema 无法表达 dispatched/standalone 互斥，改为代码内校验并以 `blocked` 回执表达，文档已说明。
- **不实现分档、不重开 W2c、`#3` 维持 `NOT_PROVEN`**：与本次缺陷无关。
- **不改 PM 侧仓库**：只改 AW 侧文档与代码。

### 55.8 门禁负向对照 7/7（两平台）

`scripts/test-gate/selftest.ps1`（Windows）与 `selftest.sh`（POSIX）各跑一遍，逐条证明门禁会红：

```text
[SELFTEST][PASS] missing-log (rc=2)
[SELFTEST][PASS] compile-failure-no-results (rc=2)
[SELFTEST][PASS] non-quarantined-failure (rc=2)
[SELFTEST][PASS] quarantined-failure-is-warn (rc=0)
[SELFTEST][PASS] quarantined-ok-is-warn (rc=0)
[SELFTEST][PASS] cargo-nonzero-without-failed (rc=2)
[SELFTEST][PASS] clean-log-is-green (rc=0)
[SELFTEST] pass=7 fail=0
```

两平台输出逐行一致，均为 `pass=7 fail=0`。该输出是在**生产清单已清空**、selftest 改用固定 fixture（§54.4）之后复跑的，第 4/5 条 WARN 分支仍被真实命中。

### 55.9 Windows 上 Linux 专属能力如实标注 `not_proven`

`capabilities` 实跑（Windows，x86_64，最终二进制），21 条里 2 条 `not_proven`，均带同一原因码字段 `reason_code`：

```text
entries_total=21 platform={"arch": "x86_64", "os": "windows"}
timeout.cleans_entire_process_tree | proven | verified=['linux'] unverified=['windows', 'macos']
timeout.cleans_entire_process_tree_windows | not_proven | verified=['windows'] unverified=[] | reason_code=requires_out_of_job_enumerable_terminable_os_primitive
timeout.enumerates_escaped_descendants | proven | verified=['linux'] unverified=['windows', 'macos']
timeout.enumerates_escaped_descendants_windows | not_proven | verified=['windows'] unverified=[] | reason_code=requires_out_of_job_enumerable_terminable_os_primitive
timeout.cgroup_v2_scope_observation | proven | verified=['linux'] unverified=['windows', 'macos']
timeout.cgroup_kill_terminates_scope | proven | verified=['linux'] unverified=['windows', 'macos']
lifecycle.startup_reclaims_abandoned_cgroup_scopes | proven | verified=['linux'] unverified=['windows', 'macos']
not_proven_total=2
```

- 两条 Linux 专属能力（`proven`）在 Windows 上带 `unverified=['windows','macos']`，不会被读成 Windows 已证；cgroup 逃逸终止同此。
- 两条 `*_windows` 条目保留在列表里，状态 `not_proven`，原因码 `requires_out_of_job_enumerable_terminable_os_primitive`；**没有删除、也没有标成通过**。作业内成员由握手夹具证成，作业外进程仍只是旁证。
- 同脚本在 WSL 上实跑：`entries_total=21 platform={"arch": "x86_64", "os": "linux"} not_proven_total=2`——同样两条 Windows 作用域条目仍未证，未因「在 Linux 上跑不到」改写状态。两条为什么没有 Windows 等价原语、替代路径与调用方口径，见 §56.1；cgroup 三条为什么不升级到 Windows，见 §56.2。

### 55.10 收尾实跑新发现：prune 并发败者被误判为 `retained_at_source`（已修）

- **现象**：Windows 全量门禁 `gate.ps1` 首次跑到 `server::prune_claim_race_tests::prune_concurrent_claimers_delete_once_others_lost_race` 红：
  `第 16 轮败者必须具名记为 lost_race：["retained_at_source", "deleted"]`，`left="retained_at_source" right="lost_race"`。
- **根因**：`delete_attempt_dir` 只在 `fs::rename` 返回 `ErrorKind::NotFound` 时判 `LostRace`。Windows 上并发 rename 同一源时，败者可能拿到 `AccessDenied` / `SharingViolation`（非 NotFound），于是走进 `RetainedAtSource`（语义是「attempt 原样未动」）——与事实相反：源已被胜者搬走。
- **修复**（`apps/mcp-bridge/src/server.rs`，`delete_attempt_dir`）：判据从「错误类型」改为「事实」——`NotFound` **或** 源已不存在（`!source.exists()`）都记 `LostRace`；源仍在才记 `RetainedAtSource`。归属仍以目的目录是否存在复核（既有 `symlink_metadata(&staged)` 分支不变）。
- **验证**：
  - 确定性用例 `prune_claim_race_tests::rename_failure_treats_missing_source_as_lost_race`：直接对判定函数喂 `NotFound` / `AccessDenied` / `SharingViolation` 三类错误，断言「非 NotFound + 源已不在 → lost_race」「源仍在 → 不是 lost_race」。不依赖并发时序，每次必触发。
  - 并发用例 `prune_claim_race_tests::prune_concurrent_claimers_delete_once_others_lost_race`（单跑一次内部 20 轮）：Windows 12 次 × 20 轮 = 240 轮、WSL 12 次 × 20 轮 = 240 轮，全部 `ok=12 bad=0`。
  - 两次判定都要求输出里出现 `test result: ok. 1 passed` 才算通过——`--exact` 名字写错时 cargo 会以 0 个用例退出码 0，只看退出码会得到假绿（本轮实测踩到过一次，已改判定方式）。
  - 全量门禁复跑结果见 §55.5。
- **为什么不加进 quarantine**：这不是「已知不稳定」，是分类错误——败者本就没有任何东西留在源目录，把它记成「原样未动」会让 PM/审计看到假状态，属于必须修的行为。

## 56. Windows 不受控进程与 cgroup 等价性：调查结论（v1.51）

本轮把两个此前含糊的地方做实：两条 `not_proven` 到底缺什么原语、Windows 上有没有等价物；三条 cgroup 能力能不能按「作业对象配额/内存限制」升级到 Windows。结论是**不升级、不删除**：两条继续 `not_proven`，三条继续 `verified=['linux']`，但 `capabilities` 的 `detail` 已写清「为什么不可得 / 替代路径 / 调用方口径 / 为什么不冒充」，不再停留在三个字。

### 56.1 两条 `not_proven`：Windows 没有「作业外进程可枚举 + 可终止 + 存活有权威来源」的三合一原语

两条条目（`timeout.cleans_entire_process_tree_windows`、`timeout.enumerates_escaped_descendants_windows`）的 `reason_code` 是 `requires_out_of_job_enumerable_terminable_os_primitive`。逐 API 核查：

| Windows API / 机制 | 覆盖范围 | 枚举作业外进程 | 终止作业外进程 | 存活有权威来源 | 结论 |
|---|---|---|---|---|---|
| `QueryInformationJobObject(JobObjectBasicProcessIdList)` | 只列**作业内**成员 PID | 否 | — | 只证明作业内成员是否还在 | 不满足 |
| `TerminateJobObject` | 只杀与作业关联的成员（含嵌套子作业） | — | 否 | — | 不满足 |
| 作业完成端口（`JOBOBJECT_MSG_*`） | 只在成员入作业 / 退出时投递事件 | 否（作业外进程不产生事件） | 否 | 事件不是存活快照 | 不满足 |
| `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)` / `EnumProcesses` / WMI `Win32_Process` / `NtQuerySystemInformation` | 可列全系统进程 | 能列「进程」，不能列「归属」 | 否 | 归属靠 ppid；ppid 不携带所有权、退出后不归零、PID 可复用 | 只是旁证 |
| `JOB_OBJECT_LIMIT_BREAKAWAY_OK` / `SILENT_BREAKAWAY` | 允许进程**离开**作业 | 离开后立即不可见 | 否 | — | 反向能力，不是追回能力 |
| 容器 / Windows 服务边界 | 宿主级隔离，不是进程级原语 | 否 | 否 | 由宿主保证 | 不是同一层语义 |

为什么必须三合一：整树结论要求同时满足（1）枚举全（含 BREAKAWAY、加入作业失败、不进作业的 `conhost.exe` 等）；（2）可终止；（3）存活判定有权威来源、能排除 PID 复用。任一环缺失都会把「没看到」误读成「不存在」，因此不能拿单项拼成整树证明。

**替代路径与调用方口径**（已写入两条条目的 `detail`）：

1. **受控集合清理**：`timeout.terminates_controlled_set` 两平台 `proven`，保证作业内成员终止并观测集合内无存活。`cleaned` 只读成「受控集合已清理」，**不要**用它推导「系统无残留」。
2. **独立旁证**：`timeout.descendant_snapshot_cross_check` 两平台 `proven`，用父链快照与作业成员交叉核对，能报「集合外仍有枚举到的后代」；v1.39 起父链边带创建时间/PID 复用过滤（审计 §42），但仍是旁证，不作权威存活结论。
3. **强隔离**：需要「一定无残留」的场景请把任务放进容器或 Windows 服务边界，由宿主保证；AW 不宣称自己提供该保证。
4. **调用方动作**：读 `capabilities` 时按 `id` + `platforms_verified` 判定，不要按平台名猜；需要更强证据时读 `receipt.get` 的后代快照并自行在作业外核查。

**可复跑命令**（无需额外环境，Windows / WSL 均可）：

```text
cargo test --offline -p aw-mcp-bridge --test mcp_contract capabilities_declares_machine_readable_boundaries
cargo test --offline -p aw-mcp-bridge --test mcp_contract timeout_cleans_controlled_set_including_grandchild
```

第一条钉死两条条目仍为 `holds=false` / `status=not_proven` / 同一 `reason_code` / `platforms_verified=["windows"]`，防止「顺手升级」；第二条证明受控集合内的孙进程确实被终止，同时不越界宣称整树。实测（2026-11-08）：前者 `1 passed`；后者在 Windows 门禁里输出 `grandchild_case=pass ready_seen=true ready_delay_ms=1045 survivors=Some([])` 并 `ok`。

### 56.2 三条 cgroup 条目：Windows 无同类语义，不升级、不冒充

对 `timeout.cgroup_v2_scope_observation`、`timeout.cgroup_kill_terminates_scope`、`lifecycle.startup_reclaims_abandoned_cgroup_scopes` 逐条判定：

| 条目 | Linux 语义（判据） | Windows 侧候选 | 判定 |
|---|---|---|---|
| `timeout.cgroup_v2_scope_observation` | 进程属于且仅属于一个作用域；成员集合独立于父链，换父被收养后仍可枚举 | 作业成员枚举只覆盖作业内；BREAKAWAY 出去的进程立刻不可见，没有「作用域成员集合」 | 不够格 |
| `timeout.cgroup_kill_terminates_scope` | 写 `cgroup.kill` 后作用域内全部进程（含终止过程中派生）被杀，子作用域传播 | `TerminateJobObject` 只杀与作业关联的成员（嵌套子作业可连带），对 BREAKAWAY / 未加入作业的进程无效；AW 也未建立嵌套作业 | 不够格 |
| `lifecycle.startup_reclaims_abandoned_cgroup_scopes` | 可持久化的作用域目录 + sidecar 记录 owner/成员，启动时可枚举并回收孤儿 | 作业对象随句柄消失，没有可枚举的持久作用域目录，也没有「孤儿回收」语义；`KILL_ON_JOB_CLOSE` 只是句柄生命周期副作用 | 不够格 |

**作业对象配额/内存限制是否够格**：`JOBOBJECT_EXTENDED_LIMIT_INFORMATION` 的 `ProcessMemoryLimit` / `JobMemoryLimit` / `ActiveProcessLimit` 是**资源上限**，不是**成员集合与生命周期**语义：

- 只限制分配或阻止新进程加入作业，**不枚举当前成员**，也**不终止已有进程**；
- 不覆盖已经 BREAKAWAY 出去的进程；
- 超配额的表现是分配失败，不是「作用域已清空」。

因此不能拿配额当 cgroup 的同类判据。三条保持 `status=proven` 仅对 Linux，`platforms_verified=["linux"]`、`platforms_unverified=["windows","macos"]`；`detail` 明写「Windows 无 cgroup v2 / 无 `cgroup.kill` 等价物 / 不升级」，**不冒充**。

**可复跑命令**：

```text
# Linux（WSL）：跑出 cgroup 证据行
AW_VENDOR=/root/vendor bash scripts/posix-timeout-verify/verify.sh run
# 两平台：钉死平台覆盖字段
cargo test --offline -p aw-mcp-bridge --test mcp_contract capabilities_declares_machine_readable_boundaries
```

WSL 实测（2026-11-08）证据行含 `cgroup_scope ... members=Some(Ok([66569]))`（被收养逃逸后代仍被成员集合点名）与 `cgroup_kill ... after outcome=Cleaned members=Ok([]) populated=Ok(0)`；Windows 上同一条目仍只声明 Linux，`unverified` 含 `windows`。

### 56.3 门禁与指纹未变（本轮不放松任何门禁）

本轮只改 `capabilities` 的 `detail` 文本与本审计章节；`detail` 不在 `tools/list` 描述与 schema 里，因此指纹不变：

```text
tools_sha256 = 95062dad5bc83f4c19e2abb0adfb7010269975f3277445cfaf50d18bba78e316
cargo test --offline -p aw-mcp-bridge --lib description_gate
  test result: ok. 3 passed; 0 failed
```

两平台门禁（默认档，`RaceRepeat=1`）：

```text
Windows  gate.ps1   rc=0 elapsed=54.3s  SELFTEST pass=7 fail=0 | lib 27 passed | integration 130 passed | warn=0 fail=0 | [RACE] repeat=1（默认，未启用连跑）
WSL      verify.sh  rc=0 elapsed=14s    lib 39 passed + 1 ignored | integration 128 passed | warn=0 fail=0 | [RACE] repeat=1（默认）
quarantine.txt 非注释行 = 0
```

`capabilities` 实跑（同一份二进制）两平台均为 `entries_total=21 not_proven_total=2`；三条 cgroup 仍 `verified=['linux']`，两条 `*_windows` 仍 `not_proven` + 原 `reason_code`——**没有删除、没有标通过、没有新增 verified**。

### 56.4 顺带修复：setsid 夹具读空文件的竞态（未用 quarantine）

本轮复跑 WSL 门禁时，`setsid_escape_is_flagged_then_unobservable_after_reparent` 红一次，报 `setsid 子进程 PID: ParseIntError { kind: Empty }`。不是产品缺陷，也不是「已知不稳定」：夹具脚本 `setsid sleep 30 & echo $! > pid.txt` 的**重定向先创建空文件**，`echo` 稍后才写入；测试轮询只要文件存在就 break，读到空内容后直接 `parse()`。首次运行靠时序侥幸通过，整机并发高时暴露。

- **修复**（`apps/mcp-bridge/src/server.rs`，两处同样的 `pid_file` 轮询夹具）：读到空内容视为「尚未就绪」继续轮询，非空才取值；不缩短超时、不放宽断言。
- **验证**（WSL，最终代码）：精确连跑 12 次 `ok=12 bad=0`；全量 lib 套件并行连跑 5 次，每次均 `39 passed; 0 failed; 1 ignored`，无 `FAILED`。
- **为什么不进 quarantine**：这是夹具的竞态读，修一行就消除；用 quarantine 会把它变成长期 WARN，掩盖真实信号。

## 57. 未支持/未验证平台的 fail-open / fail-closed 审计（v1.51）

本节回答一个问题：在 `verified` 没覆盖的平台上，清理逻辑会不会**照常动手并宣称已清理**。结论：Windows 侧全部 fail-closed（不适用或查询失败即 Unavailable）；macOS 此前存在一条 **fail-open**（组记账为空就报 Cleaned，从不检查后代快照，而 macOS 的 `snapshot_descendants` 恒为 unobservable）；Linux 降级的 cgroup 路径也有同类问题（`is_empty()` 忽略 `escaped_scope_unobservable`）。已修成 fail-closed，并给出红/绿证据。

### 57.1 逐条结论

| 能力 | Windows 运行时行为 | macOS 运行时行为 | 判定 |
|---|---|---|---|
| `timeout.cgroup_v2_scope_observation` | cgroup 代码全在 `#[cfg(target_os = "linux")]` 内，Windows 根本不编译、不执行；能力目录标 `unverified` | 同样不编译；unix 通用 `snapshot_descendants` 返回 `unobservable("procfs_unavailable")`，不作为 cgroup 观测 | fail-closed（拒绝执行/不适用），不冒充 |
| `timeout.cgroup_kill_terminates_scope` | Windows 只用 `TerminateJobObject` 走独立 imp，未被当作 `cgroup.kill` 使用；能力目录标 `unverified` | 无 `cgroup.kill` 调用点 | fail-closed（拒绝执行/不适用） |
| `lifecycle.startup_reclaims_abandoned_cgroup_scopes` | `reclaim_stale_cgroup_scopes` 非 Linux 实现直接返回 `state=unsupported, reason=platform_has_no_cgroup_v2`，`reclaimed_total=0`，不删任何目录 | 同 Windows 的 stub | fail-closed（拒绝执行） |
| `timeout.cleans_entire_process_tree_windows`（`not_proven`） | 作业对象清理，动手前查作业记账与快照；查询失败/快照不可观测→`Unavailable`；作业外进程不可观测，运行时文案明写「未验证」 | 进程组清理；修复前组空即 `Cleaned`（见 57.2），修复后快照不可观测→`Unavailable` | Windows：显式降级（只声明受控集合）；macOS：已修为 fail-closed |
| `timeout.enumerates_escaped_descendants_windows`（`not_proven`） | 只能枚举作业内成员；作业外不做权威枚举，能力目录 `not_proven` | 无 `/proc`，快照恒 `unobservable`；修复后不得据此宣称 Cleaned | Windows：显式降级；macOS：fail-closed |

代码位置：cgroup 结构 `#[cfg(target_os = "linux")]` 见 [server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L4671-L4683)；非 Linux 回收 stub 见 [server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L2412-L2419)；macOS 的不可观测快照见 [server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L4398-L4401)；Windows terminate 的查询失败分支见 [server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L7141-L7172)；能力目录三处 cgroup 条目见 [server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L10491-L10520)。

### 57.2 红证据（修复前确实 fail-open）

用 Linux 的 cgroup 降级路径复现（这是 macOS「无作用域可观测」的可执行等价物），修复前实跑原文：

```text
[evidence] cgroup_degraded before root=132099 escaped=132098 scope=ProcessGroup escaped_scope_unobservable=Some("cgroup_v2_unavailable") descendants=[]
[evidence] cgroup_degraded after outcome=Cleaned escaped_alive=true descendants=[] scope=ProcessGroup unobservable=None escaped_scope_unobservable=Some("cgroup_v2_unavailable")
test result: ok. 1 passed
```

即：作用域明确不可观测（`escaped_scope_unobservable` 非空）、逃逸进程 `escaped_alive=true`，但终止结果仍是 `Cleaned` → `cleanup_status=cleaned` → PM 侧 `cleanup_claimed=true`。根因两处：

1. 判据用了 `snapshot.is_empty()`（只检查 `unobservable`，[server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L4078-L4082)），而结构注释已写明 `escaped_scope_unobservable` 非空时「**不代表**没有逃逸后代」（[server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L4059-L4061)）——字段被声明为不可读成空，却在实际判定里被读成空。
2. 全部快照复核都在 `#[cfg(target_os = "linux")]` 块内，macOS 分支只查「进程组是否为空」就返回 `Cleaned`；而 macOS 的 `snapshot_descendants` 恒返回 `unobservable("procfs_unavailable")`，等于「从不观测就宣称清空」。

### 57.3 修复：不可观测作用域 → 不动进程组，只杀直接子进程

- **终止前闸门**（[server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L4488-L4506)）：Linux `cgroup_degradation()` 非空、或 Linux `proc_table()` 不可读、或非 Linux POSIX（macOS 无 `/proc`）时，**不向进程组发信号**，只 `child.kill()` + `wait()`，返回 `Unavailable`。与契约措辞「仅向直接子进程发送终止信号」逐字一致——宁可少清理，也不盲杀不可观测的作用域。
- **终止后复核保底**（[server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L4633-L4661)）：所有 POSIX 平台共同检查快照；`unobservable` 或 `escaped_scope_unobservable` 非空 → `Unavailable`，快照非空 → `StillAlive`，三者皆无才 `Cleaned`。
- macOS 因此恒为 `Unavailable`（`cleanup_status=unavailable`、`cleanup_claimed=false`），不再出现「看不见后代却宣称 cleaned」。
- Linux cgroup 降级同理：不再动进程组，只杀直接子进程并报 `Unavailable`；能力 `timeout.cleans_entire_process_tree` 的 detail 与 evidence 已同步改为 `cgroup_degraded_terminate_reports_unavailable`（[server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L10419-L10447)）。
- 未新增 `verified`、未新增 `cleanup_status` 取值；`Unavailable` 本来就映射 `cleanup_status="unavailable"` 且 `cleanup_claimed=false`（[server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L8037-L8051)），PM 判定模块无需改动。
- 用例改名与断言：`cgroup_degraded_terminate_reports_unavailable` 固定「逃逸仍存活 + outcome=Unavailable」（[server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L6774-L6854)）。

### 57.4 修复后证据

```text
[evidence] cgroup_degraded before root=153635 escaped=153621 scope=ProcessGroup escaped_scope_unobservable=Some("cgroup_v2_unavailable") descendants=[]
[evidence] cgroup_degraded after outcome=Unavailable escaped_alive=true descendants=[] scope=ProcessGroup unobservable=None escaped_scope_unobservable=Some("cgroup_v2_unavailable")
test result: ok. 1 passed
```

两平台门禁（默认档，最终代码）：

```text
Windows gate.ps1 : rc=0 52.5s | SELFTEST pass=7 fail=0 | lib 27 | integration 130 | warn=0 fail=0
WSL verify.sh    : rc=0 14s   | lib 39 passed + 1 ignored | integration 128 | warn=0 fail=0
description_gate : test result: ok. 3 passed; 0 failed（24 filtered out）
tools_sha256     : 95062dad5bc83f4c19e2abb0adfb7010269975f3277445cfaf50d18bba78e316（未变）
quarantine.txt   : 非注释行 = 0
```

### 57.5 macOS 全程未验证（可检索）

**macOS 全程未验证**：本仓库没有 macOS 构建、没有 macOS 用例、没有 macOS CI 或实跑记录；`capabilities` 中 macOS **只**出现在 `platforms_unverified`，不出现于任何 `platforms_verified`。任何 macOS 行为结论（包括本节 fail-closed 的推断）都只是代码路径分析，**不构成「已验证」**；首次在 macOS 实跑前，按未验证平台对待。

### 57.6 门禁复跑暴露的并发改名归属问题（顺手修，未进 quarantine）

本轮复跑 Windows 门禁时，`prune_concurrent_claimers_delete_once_others_lost_race` 间歇红两次，两次都不是产品数据问题，而是**归属分类**错：

1. 败者 rename 拿到 `AccessDenied` 时，胜者的 `MoveFile` 还没收尾，`source.exists()` 仍为 true → 被记成 `retained_at_source`。修复：非 `NotFound` 错误给胜者 250ms 收尾窗口，源消失即 `lost_race`（[server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L1213-L1241)）。
2. Windows 并发改名可能让两方都返回 Ok；胜者先删共享暂存目录后，败者的 `remove_dir_all` 报 `AccessDenied`，但目录随即消失 → 被记成 `trashed_not_deleted`。修复：删除失败后再给 250ms 窗口，暂存消失即 `lost_race`（零删除）；仍存在才报 `RetainedInTrash`（数据确实保留在暂存）（[server.rs](file:///E:/软件开发/云集编程工作站/4.AgentWork/apps/mcp-bridge/src/server.rs#L1265-L1299)）。
3. 两处都只改「失败后的归属判定」，不放宽 `delete_attempt_dir` 的前置闸门，也不动 `quarantine.txt`。

验证（Windows，最终代码）：精确连跑 **50 次 × 每次 20 轮 = 1000 轮** `failures=0/50`；门禁默认档再跑 20 轮全绿（`[GATE] warn=0 fail=0`）。

