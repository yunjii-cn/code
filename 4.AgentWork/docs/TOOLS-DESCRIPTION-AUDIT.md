# `tools/list` 描述与实际行为全量对照（v1.14）

- 对照对象：`apps/mcp-bridge/src/server.rs` 中 `tools()` 返回的**全部工具**的 `description` 与 `inputSchema`（含每个字段的 `description`、`type`、范围、`required`、`additionalProperties`）。
- 权威契约：[`PM-AW-DISPATCH-CONTRACT.md`](./PM-AW-DISPATCH-CONTRACT.md)（v1.14）。
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
| 72 | schema `older_than_days`："不提供默认阈值" | 缺失即报错，不落任何默认值 | 与实现一致 |
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
| 78 | 六个工具的 `inputSchema.type` 均为 `object` | `call_tool` 要求 `params.arguments` 为对象（缺失时按 `{}` 处理） | 与实现一致 |
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

1. **`#3` 进程树杀灭未实现**：超时只杀直接子进程，孙进程存活未验证（`NOT_PROVEN`）。`timeout:` 文案与契约 §4 已如实说明。
2. **`additionalProperties: false` 服务端不强制**：见 §7.2，属声明性约束。
3. **清理的阈值/分档/`task_id` 退役纪律未由 PM 定**：`runs.prune` 入口已落地，但契约 §10 仍标"待定"。
4. **认证/准入/并发上限不存在**：`clientInfo` 仅审计、不参与准入；`initialize` 多份记录不等于多会话、不构成身份。
