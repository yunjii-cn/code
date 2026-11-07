# PM → AW 派单与回执契约 v1.51

状态：v1.51。§10 的默认决策已落地，不再等待 PM 逐项拍板；PM 可在派单侧显式覆盖（标注"必须显式"者除外）
适用范围：PM 向 AW 的 `mcp-bridge` 派单、AW 回执、失败处置
实现状态：G1 / G2 / G3 / G4（仅审计）已落地（见 §11）；v1.3 起回执补充执行上下文；v1.4 起支持 `env` 透传；v1.5 起提供 `runs.list` 只读枚举；v1.6 起派单与回执带 UTC 时间戳；v1.7 起 `env.PATH` fail-closed（不许整体覆盖）；v1.8 起 `runs.list` 的 `total` 与 `attempts` 恒等；v1.9 起 `path_prepend` 已实现；v1.10 起 `path_prepend: []` 与缺省等价；v1.11 起超时文案只写"已发送终止信号、孙进程未验证"；v1.12 起提供显式两段式证据清理 `runs.prune`（**默认不删、无任何自动清理**，见 §8）；v1.13 起 `initialize` 审计改为逐份文件留存、`status.session` 聚合（并发不丢）；v1.14 起 `tools/list` 描述与契约单一措辞（env fail-closed + path_prepend 声明）；v1.15 起新增 `#3` 进程树杀灭的**设计评估**（仅文档，**未实现**，`NOT_PROVEN` 不变）；v1.16 起按窄定义实现 `#3` 的**受控集合**清理（POSIX 进程组 / Windows Job；Windows 采用 `spawn → Assign`，窗口存在），判据只用 `kill(-pgid, 0) == ESRCH` 与 `ActiveProcesses == 0`；v1.16.1 曾因状态门未满足退回旧文案；**v1.17 起状态门已满足（POSIX 四类用例实跑通过 + Windows 三类通过 + 版本递增），启用"已清理受控集合；逃逸进程是否仍在运行未验证"三态文案**，Windows 逃逸负例仍为 `UNPROVEN`；**整棵进程树仍不可判定，`#3` 保持 `NOT_PROVEN`**；认证与并发上限仍未实现；**v1.18 起把原"待 PM 确认"项转为有依据的默认决策（§10），`path_prepend` 的 16/4096 限制写入 schema；v1.19 按 PM 口径把 `runs.prune` 的推荐阈值校正为 365 天、`limit` 缺省 100、明确先 dry-run、不分档（§8.9）；v1.20 收窄 Windows `spawn → Assign` 窗口：子进程以挂起态创建、先加入作业再恢复执行，恢复前不会派生子进程；v1.21 实现 `runs.prune.status` 多值白名单（显式才生效、与年龄 AND、`limit` 后置、无默认状态/无分档，§8.10），分档仍未实现；**v1.22 起删除回执改为自包含可核对（纯加法）：`deleted[]` 新增 `state`/`source_path`/`trash_path`/`deleted_at`/`rule`/`fingerprint`，`failed[]` 新增 `retained_at_source` 状态与同形字段，新增 `trashed[]`/`trashed_total`；`deleted[]` 是唯一表示已移除的集合，指纹为删除前算出的真实 sha256 与字节数（缺文件写 `null`）**；**v1.23 起 `status` 增加连接级计数 `connections`（纯加法、只观测）：实测确认进程内没有"会话"概念，只有一条 stdio 连接，故 `active`/`total`/`peak` 恒为 1、`initialize_total` 随 `initialize` 次数累加（多份 `initialize` 不等于多会话）；不做准入、不做上限**；**v1.24 起把超时的"单次终止尝试"做实：POSIX 68 轮 + Windows 20 轮实测确认一次 `kill(-pgid,SIGKILL)` / 一次 `TerminateJobObject` 后集合内无存活，故不新增第二次尝试；描述明写"终止只尝试一次（不重试）"，判据与回执语义不变（只有观测到无存活才写 `Cleaned`，逃逸仍"未验证"，`#3` 仍 `NOT_PROVEN`）**；**v1.25 起把"未知/多余参数被静默忽略"变为可观测诊断（§2.5）：回执按需追加 `arguments_diagnostics`（未知参数名、camelCase/snake_case 别名指向、显式 `null` 的可选字段，值只给类型/长度），同一份 JSON 同步写 stderr；诊断不改变任何调用的成功/失败结果，`additionalProperties:false` 仍只声明不强制**；**v1.26 起 `task.execute` 显式区分 `dispatched` / `standalone` 两种运行模式（§2.6）：dispatched 下 AW 仍只执行、不验收、不判停；standalone 下由 AW 按任务声明的 `acceptance` 自行判定，并把三态结论（`passed` / `failed` / `undetermined`）落盘到 `runs/<task_id>/attempt-N/verdict.json`，失败有界重试、无法判定绝不默认通过**；**v1.27 起 `runs.prune` 增加可选按条目排除 `exclude`（§8.11）：按 `task_id`、可精确到 `attempt`，在**候选枚举入口**生效，被排除条目只进 `excluded[]`、不进 `candidates[]`；回执新增 `excluded[]` / `excluded_total` / `not_in_candidates_total` / `exclude_requests_total` / `batch_candidates_total` / `not_processed_total`，并写死计数不变式 `deleted + failed + trashed + excluded_total == batch_candidates_total`、`candidates_total == batch_candidates_total + not_processed_total`；越界 id 与任务目录同一命名校验，不存在/不在候选集记 `not_in_candidates`（不报错、不静默忽略）**；**v1.28 起 standalone 的 `argv[1..]` 中携带路径的参数也作作用域校验（§2.6.8）：占号前按解析后的真实位置判断，越界 `standalone_argv_path_escape`；明确盲区（URL/正则/普通标量/未展开变量等）逐条写死，无法判定放行并在回执与 verdict 的 `argv_path_scan.unknown[]` 记 `standalone_argv_path_unknown`（不 fail-closed、不写成已验证安全）；dispatched 不变**；**v1.29 起实证 standalone 并发与重入（§2.6.9）：同 id 两进程=一个执行 + 一个等待回放（`replayed` / `replay_kind`），败者等待窗口覆盖 `timeout_secs * max_attempts + 5s`，重入=回放既有结果（`existing`）**；**v1.30 起补 `runs.prune` 与执行之间的并发防护（§2.6.10）：每次 `task.execute` 在 `runs/.inflight/<task_id>.json` 原子占位（`create_new`，含 pid/时间窗），正常/失败/阻断/异常返回由 RAII 删除，崩溃残留靠 pid+时间判定；`runs.prune` 看到活跃标记一律跳过并具名记录 `task_inflight`；活跃 attempt 目录被外部移走/删除时回执报 `attempt_dir_lost`（绝不静默 ok）；全部记录被清理后的重跑凭 `runs/.task-history/<task_id>.json` 台账标注 `replay_basis=rerun_after_records_missing`（`fresh` 为首次）**；**v1.31 起复核 `additionalProperties:false` 仍只声明不强制（默认行为/schema 不变），并新增**单次请求可选**的严格模式（§10.2）：`arguments._meta.strict_arguments=true` 时按 schema 拒绝 `unknown`/`alias`，返回具名错误 `invalid_arguments`（含未声明参数、已声明参数、别名更正建议），`_meta` 与 `null` 可选字段不误拒；不提供进程级开关；`#3` 整棵树仍不可判定，`NOT_PROVEN` 不变，证据链见 `docs/TOOLS-DESCRIPTION-AUDIT.md` §30/§31**；**v1.32 起把"边界只写在文档里"改为机器可读（§2.7）：新增只读工具 `capabilities`，逐条 claim 给出 `id` / `holds`（`true`/`false`/`null`）/ `status` / `scope` / `reason_code` / `platforms_verified` / `platforms_unverified` / `automatic` / `evidence`，`#3` 窄判据（`timeout.cleans_entire_process_tree`、`timeout.enumerates_escaped_descendants`）、POSIX 未覆盖（`posix_unverified`）、TOCTOU（`check_then_delete_not_atomic`）、残留不自动清（`explicit_cleanup_required`）均为可判定条目；新增 `runs.inflight`（`action=list|clean`）只读列出 `runs/.inflight/*.json` 的 `stale`/`stale_reason`/`pid_alive` 与机器可读 `stale_rule`，`clean` 默认 `dry_run=true`、真删需 `dry_run=false`+`confirm=true`，只删 `stale=true`（pid 已消失且超过宽限，或无 pid 且超过 `deadlineAt+120s`），活跃标记永不删除，删除前原子改名并复核 `pid+startedAt`；正常结束无残留，构造的 pid 消失+超时残留可被识别并清掉（真实 stdio 实跑见审计 §33）**；**v1.33–v1.46 的变更逐条记录在 §11 表内；v1.47 起新增显式取消 `task.cancel`（§3.6）、`_session` 按天保留（§2.10）、Linux 直接子进程 `PDEATHSIG` 兜底（§2.7）；v1.48 起新增启动回收遗留 cgroup 作用域（§2.11）、`task.cancel` 跨进程请求（§2.12），Linux 孙进程不再依赖人工巡检**；**v1.49 起两条不稳定用例完成诊断/定性/quarantine，并实证 B7 纯文件跨进程取消；v1.50 起补齐 A 类缺陷的复现驱动、门禁负向对照常驻与 PM 接入须知/自测入口（§2.13）；v1.51 起按 PM 联调实测收口：B1 执行中重复派发改为文档写明「阻塞回放到原执行结束（上限 `timeout_secs+5s`）」、B2 `task.cancel` 示例按实测返回（`canceled` + `cleanup_claimed=true` + `handback` + `message`）、A3 新增机器可读 `cleanup_status` 并把 `cleanup_claimed` 收紧为「`handback_status=canceled` 且 `cleanup_status=cleaned`」（文件通道等效判据同源）、B4 `status` 顶层新增 `contract_version="1.51"`**

## 0. 结论

- 复用 MCP 已有的 `initialize.params.clientInfo` 与 stdio 进程归属，**不新造 token/凭据格式**；`clientInfo` 只作**审计弱识别**，不构成身份、不构成准入。
- `task_id` 是唯一幂等键。**重投 = 返回既有回执、复用既有 attempt 编号，零执行、零新目录**；有意重跑必须换新 `task_id`。
- 回执完全映射到已实现字段，不新增第二套字段；失败原因只借用既有的 `blocked_reason` 字符串前缀做分类。
- 一句话边界（按模式）：**dispatched（PM 派单）下 AW 只执行，不验收、不判停、不定义判据、不写 L0/事件，PM 是唯一写权威；standalone（无外部派单方）下 AW 自行闭环——按任务声明的 `acceptance` 判定并把三态结论落盘到 attempt 目录（`verdict.json`），但仍不写 L0/事件、不成为项目状态的写权威。**

## 1. 术语

| 术语 | 含义 |
|---|---|
| 派单 | PM 调用 `task.execute` 的一次逻辑任务，由 `task_id` 唯一标识 |
| 重投 | 同一次派单因传输/重试被再次发送（`task_id`、`cmd`、`cwd`、`timeout_secs` 完全一致） |
| 重跑 | PM 有意重新执行（判据不变或变更），须使用**新的** `task_id` |
| attempt | 同一 `task_id` 下的一次执行记录，落盘于 `runs/<task_id>/attempt-<N>/` |
| 回执 | `task.execute` 的返回值，持久化为 `handback.json`，可由 `receipt.get` 读回 |

## 2. 调用方标识（审计弱识别）

### 2.1 来源与传递形式

| 项 | 定义 |
|---|---|
| 调用方自称 | MCP `initialize` 的 `params.clientInfo{name, version, ...}`（协议既有字段）。**仅作审计弱识别，不是身份、不构成准入** |
| 执行方标识 | AW。以 `health` 返回的 `server.name` / `server.version` / `server.buildTime` 标识 |
| 通道 | MCP stdio，一问一答；一个 bridge 进程同一时刻只服务一个启动方（1:1） |
| 进程归属 | 谁拉起 bridge 进程即谁是调用方；这是当前的**事实准入边界** |

**不新造凭据**：不使用自定义 header、token、签名或独立握手字段。

**审计落点与边界（写死）**：`initialize` 收到的 `clientInfo` 原样落盘为 `runs/_session/initialize-<pid>-<毫秒时间戳>-<序号>.json`（含 `receivedAt`），**每次 initialize 一份独立文件、互不覆盖**；`status.session` 只读聚合全部记录（含旧版单文件 `initialize.json`）。这些记录：

- **不构成身份**，不产生任何凭据或会话标识；**多份记录不等于多个可信会话**，也不等于"已认证"；
- 调用方可任意自称/伪造，内容与真实调用方无关；
- **任何拒绝/放行（准入）逻辑都不得读取它**；它只用于事后审计与问题追溯。

**为什么改为逐份留存（v1.13，写死取舍）**：多 bridge 共享同一 `runs/` 后，旧的单文件 `initialize.json` 是覆盖写——两个 bridge 同时启动会**静默丢掉**先写的那条，且双方都不报错。覆盖语义天生只能留"最后一次"，**不是完整审计**。v1.13 起每次 `initialize` 写一份新文件（文件名含 pid、毫秒时间戳与进程内序号，`create_new` 语义避免覆盖，tmp + rename 保证可见时内容完整），因此并发与先后写入都**不丢**。旧 `initialize.json` 保留可读并纳入聚合，**不回填、不改写**。

### 2.2 当前强度声明

- v1 不做密码学认证，不做准入拦截；`clientInfo` 已记录（`runs/_session/initialize*.json` 逐份留存，`status.session` 聚合回显），但**仅用于审计追溯，不参与任何准入判断**。
- 真正的准入边界是“本地进程由谁启动 + 本地配置由谁可写”，本轮不在契约内展开。
- 引入认证时只允许在**会话层**（`initialize` 之后、`tools/call` 之前）加一次性凭据，且必须复用 PM/MH 已有通道；不得在每条请求体里新造字段。

### 2.3 准入默认不启用；若要让身份参与准入需要哪些字段（v1.18）

**默认（写死）：`clientInfo` 不参与准入。** 依据：`clientInfo.name` / `version` / `title` 均为客户端自报，bridge 未做签名、未绑定进程/管道、无会话票据；不同调用方可任意自称。按 `clientInfo` 放行等于把自报字符串当身份，提供的是**虚假安全**。多份 `initialize` 记录也不等于多会话（§2.1）。

**若未来要让身份参与准入，至少需要以下字段（当前均不存在）**：

| 字段 | 来源 | 作用 | 当前状态 |
|---|---|---|---|
| 传输/进程锚点 | bridge 自身观测（stdio 父进程/管道归属；或 socket peer） | 证明"谁拉起了这个 bridge" | 只有 §2 的 1:1 进程归属事实，未落盘为字段 |
| 会话票据 | PM/MH 在 `initialize` 前签发（一次性 nonce 或 HMAC/签名） | 防冒充、防重放 | 不存在；v1 明确不新造凭据 |
| 主体标识 | 票据内的 `subject`（PM 调用方 id）+ OS 用户/进程信息 | 授权主体 | 不存在 |
| 授权范围 | `project_id` / `task_id` 白名单 / capability 列表 | 限定可执行范围 | 不存在 |
| `clientInfo` | 客户端自报 `name` / `version` / `title` | **仅展示/审计**，不得单独作为准入依据 | 已落盘审计 |

结论：在拿到"传输/进程锚点 + 会话票据 + 授权范围"之前，任何基于 `clientInfo` 的准入判断都不可信；因此默认不做，且拒绝逻辑不得读取它（§2.1）。

### 2.4 会话 / 连接计数（v1.23，只观测）

实测（真实 bridge 进程、stdio）：同一进程内连续两次 `initialize` 都返回协议版本，随后 `status.connections` 为 `active=1`、`total=1`、`peak=1`、`initialize_total=2`。结论：**bridge 进程内没有"会话"概念，只有一条 stdio 连接**；多份 `initialize` 不是多个会话，也不产生多条连接。因此计数明确为**连接级**（`unit="connection"`、`scope="process"`），不使用"会话数"措辞：

| 字段 | 含义 | stdio 下的取值 |
|---|---|---|
| `unit` | 计数单位 | 恒为 `"connection"` |
| `scope` | 计数作用域 | 恒为 `"process"` |
| `active` | 当前活跃连接数 | `1`（进程存活期间只有一条 stdio 连接） |
| `total` | 累计连接数 | `1` |
| `peak` | 并发连接峰值 | `1` |
| `initialize_total` | 本连接累计 `initialize` 次数 | 随调用次数累加（多份 ≠ 多会话） |
| `note` | 口径说明 | 固定文案，明写"无会话概念、只观测" |

PM 侧接法：只把它当运行时可观测口径，**不得用于准入、限流或判定"第几个会话"**；`initialize_total > 1` 只说明同一连接重复握手，不代表新的调用方。本版仍**不做准入、不做连接上限**。

### 2.5 参数诊断 `arguments_diagnostics`（v1.25，只观测、不拦截）

`tools/call` 命中某个工具后，若请求里存在**未知参数**或**显式传 `null` 的可选字段**，工具回执的 JSON 文本内容会追加 `arguments_diagnostics`；没有可报告项时**不出现该字段**（不产生噪声）。诊断与调用结果完全解耦：`isError`、`error`、正常 payload 字段一律不变。

| 字段 | 含义 |
|---|---|
| `tool` | 本次调用的工具名 |
| `declared_arguments` | 该工具 schema 已声明的参数名列表（调用方无需读源码即可对照） |
| `unknown_arguments[]` | 请求里出现、但 schema 未声明的参数 |
| `unknown_arguments[].name` | 参数名（只报名字） |
| `unknown_arguments[].kind` | `unknown` = 对不上任何已声明参数（拼写错误一类）；`alias` = 能对上已声明参数的别名；`client_metadata` = MCP 客户端元数据（如 `_meta`） |
| `unknown_arguments[].alias_of` | 仅 `kind=alias` 时出现，指向已声明参数（如 `dryRun` → `dry_run`） |
| `unknown_arguments[].value` | `{type, length}`：仅类型与长度（字符串字符数 / 数组项数 / 对象键数），**绝不回显值** |
| `null_optional_arguments[]` | schema 声明为可选、调用方显式传 `null` 的字段 |
| `null_optional_arguments[].kind` | 恒为 `null_optional`（不是错误，调用结果照旧） |
| `null_optional_arguments[].declared_type` | schema 声明的类型（如 `string` / `integer`） |
| `null_optional_arguments[].value` | 恒为 `{type:"null", length:null}` |
| `note` | 固定文案：只记录参数名与值类型/长度、不改变结果 |

规则写死：

1. **别名不生效**：`dryRun` 与 `dry_run` 仍只按 `dry_run` 读取；诊断指出 `alias_of`，服务端不替调用方改名。
2. **不拒绝**：`additionalProperties:false` 仍是声明性约束（§10 第 5 行）；未知参数被忽略，但现在是**可观测的忽略**。
3. **同源两侧**：同一份 JSON 同时写入回执字段与 stderr 单行日志（前缀 `[aw-mcp-bridge] arguments_diagnostics `），不是只写 stderr 的自由文本。
4. **敏感值**：只报参数名、类型、长度；`_meta` 等对象只报键数，不报嵌套键与值。
5. **未知工具名不做参数诊断**：调用本身返回未知工具错误，行为不变。

### 2.6 运行模式与独立判定（v1.26）

`task.execute` 的 `mode` **必须显式区分**，不靠推断：

| 模式 | 谁负责闭环 | AW 做什么 | AW 不做什么 | 入口 |
|---|---|---|---|---|
| `dispatched`（缺省） | PM | 只执行、交可审计回执；现有契约与行为**逐字不变** | 不验收、不判停、不定义判据、不写 `verdict.json`、不宣称 `verified`；回执**不新增** `mode` / `verdict_*` / `attempts_total` 字段；`dispatch.json` **不写** `mode` | 沿用既有 `task_id` + `cmd` + `cwd` + `timeout_secs` + `env` + `path_prepend` |
| `standalone` | AW 自己 | 按任务声明的 `acceptance` 逐条判定、落盘 `verdict.json`、失败有界重试 | 仍不写 L0/事件、不成为项目状态写权威；不替调用方发明验收标准 | 必须显式 `mode="standalone"` + `cwd` + `argv` + 非空 `acceptance` |

**硬边界：同一 `task_id` 不得跨模式复用。** 已存在记录的模式与本次请求不一致 → 工具错误 `mode_conflict`，**零执行、零新 attempt、零覆盖**。判定依据是 `runs/<task_id>/attempt-1/dispatch.json` 的 `mode` 字段（旧记录缺该字段一律视为 `dispatched`）；同一目录不会既被 PM 当写权威、又被 AW 自行判了。

#### 2.6.1 standalone 输入契约

| 参数 | 要求 | 拒绝码（占号前，不创建 attempt 目录） |
|---|---|---|
| `mode` | 必须为字符串 `"standalone"` | 非法值 `invalid_mode` |
| `cwd` | 必填、可解析为目录；作为**执行与验收的唯一作用域根目录**，以规范化绝对路径落盘 | `standalone_cwd_required` / `standalone_cwd_invalid` |
| `argv` | 必填、非空字符串数组；`argv[0]` 是程序，其余逐项传参，**不经 shell**；`argv[1..]` 中携带路径的参数按 §2.6.8 在占号前校验 | `standalone_argv_required` / `standalone_argv_invalid` / `standalone_argv_path_escape` |
| `acceptance` | 必填、非空数组；见 §2.6.2 | `standalone_acceptance_required` / `standalone_acceptance_invalid` / `standalone_path_escape` |
| `max_attempts` | 可选，整数 `1..=5`，缺省 `3` | `standalone_max_attempts_invalid` |
| `timeout_secs` | 可选，整数 `1..=600`，缺省 `120`；每次尝试独立计时 | `standalone_timeout_invalid` |
| `cmd` | **禁止**（不接受 shell 命令字符串） | `standalone_cmd_rejected` |
| `env` / `path_prepend` | **禁止**（作用域与可复现性考虑） | `standalone_env_rejected` / `standalone_path_prepend_rejected` |

拒绝一律返回 `status=blocked` 的预检回执（带 `mode="standalone"`），**不占号、不落盘、不产生 `verdict.json`**。

#### 2.6.2 验收标准从哪来：任务输入里的 `acceptance[]`

验收标准**只来自任务输入**，AW 不自行发明。四种可核对类型：

| `type` | 必填字段 | 判定 | 观测记录 |
|---|---|---|---|
| `exit_code` | `expected`（整数） | 进程退出码逐字相等 | `observations.exit_code` + `criteria[].observed.exit_code` |
| `file_exists` | `path`（cwd 内相对路径） | 文件存在 | `observed.exists` / `bytes` |
| `file_content_match` | `path` + `contains`（非空字符串） | 文件内容包含子串 | `observed.exists` / `matched` / `bytes` |
| `git_diff_non_empty` | 无 | `git status --porcelain` 输出非空（**含未跟踪文件**） | `observed.empty` / `bytes` / 执行的命令 |

规则：

1. 路径必须是 cwd 内的**相对路径**；绝对路径、`..`、根/前缀组件一律 `standalone_path_escape`（占号前拒绝）。
2. `git_diff_non_empty` 在非 git 仓库或 git 不可用时**不是通过、也不是失败，而是 `undetermined`**（见 §2.6.3）。
3. **没有声明 `acceptance`（缺省或空数组）→ 明确拒绝** `standalone_acceptance_required`；不允许把"没写验收"当通过。

示例（成功任务）：

```json
{
  "task_id": "demo-standalone-ok",
  "mode": "standalone",
  "cwd": "D:\\Temp\\demo",
  "argv": ["cmd.exe", "/C", "echo hello>out.txt"],
  "acceptance": [
    { "type": "file_exists", "path": "out.txt" },
    { "type": "file_content_match", "path": "out.txt", "contains": "hello" }
  ],
  "max_attempts": 1
}
```

#### 2.6.3 判定落盘：`runs/<task_id>/attempt-N/verdict.json`

每次尝试都写一份**独立** `verdict.json`（重试不覆盖上一次）：

```json
{
  "version": "standalone-verdict/1",
  "mode": "standalone",
  "task_id": "demo-standalone-ok",
  "attempt": 1,
  "decided_at": "2026-11-07T00:00:00Z",
  "verdict": "passed",
  "acceptance": [ /* 本次任务声明的验收标准原文 */ ],
  "argv_path_scan": { /* 见 §2.6.8，判定前的参数路径预检 */ },
  "criteria": [
    {
      "index": 1,
      "spec": { "type": "file_exists", "path": "out.txt" },
      "observed": { "path": "out.txt", "exists": true, "bytes": 7 },
      "passed": true
    }
  ],
  "observations": {
    "exit_code": 0,
    "stdout_digest": "sha256:...",
    "stderr_digest": "sha256:...",
    "stdout_preview": "前 200 字符（超出则 stdout_truncated=true）",
    "stderr_preview": "",
    "stdout_bytes": 0,
    "stderr_bytes": 0,
    "stdout_truncated": false,
    "stderr_truncated": false,
    "changed_files": ["out.txt"],
    "changed_files_truncated": false,
    "duration_ms": 123,
    "timed_out": false,
    "spawn_failed": null
  },
  "missing": [],
  "decision": { "action": "terminate", "reason": "acceptance_met", "last_failure": null }
}
```

结论**三态**，没有第四种：

| `verdict` | 含义 | `decision.action` | 回执 `status` |
|---|---|---|---|
| `passed` | 所有标准都有观测且全部通过；且（退出码为 0，或任务显式声明了 `exit_code` 标准并已通过） | `terminate` | `ok` |
| `failed` | 至少一条标准有观测但未通过；或标准全过但命令退出码非 0 且未声明 `exit_code`（`last_failure=command_failed`） | `retry`（未达上限）/ `terminate`（耗尽） | `failed` |
| `undetermined` | 缺少可核对观测（如非 git 仓库的 `git_diff_non_empty`）、spawn 失败、超时 | 一律 `terminate` | `blocked` |

**无法判定绝不默认通过**：`missing[]` 必须逐条写明缺什么（例如 `git_diff_non_empty：无法取得 git status --porcelain…`）；spawn 失败/超时写 `spawn_failed` / `timeout`，且**不重试**。

standalone 回执在既有字段之外**仅追加**：`mode`、`argv`、`max_attempts`、`verdict_conclusion`、`verdict_locator`（如 `runs/<task_id>/attempt-1/verdict.json`）、`attempts_total`（仅终止尝试）；`artifacts[]` 增加 `kind="verdict"` 及其 sha256。dispatched 回执**不出现**这些字段（见 §2.6.5）。

#### 2.6.4 有界重试

- 上限：`max_attempts`（默认 3，范围 1..5）。第 N 次尝试目录为 `attempt-N`，每次都有独立的 `dispatch.json` / `stdout.txt` / `stderr.txt` / `verdict.json` / `handback.json`，**不覆盖、不合并**。
- 仅 `verdict=failed` 且 `attempt < max_attempts` 时重试，`decision.reason="acceptance_not_met"`（或 `command_failed`）。
- 达到上限写 `decision.action="terminate"`、`reason="retry_exhausted"`，`last_failure` 保留最后一次失败原因。
- `undetermined`、spawn 失败、超时**不重试**，直接终止。
- 全部尝试结束后在 `runs/<task_id>/standalone-result.json` 写最终回执；同参重投按它回放，**不追加 attempt**；执行未结束时的重投**阻塞等待** `standalone-result.json`（上限 `timeout_secs * max_attempts + 5s`），等不到才返回工具错误 `task_in_progress`。

#### 2.6.5 dispatched 行为零变化（回归锁）

- 缺省 `mode` 或缺省 `"dispatched"` 走原有分支：`cmd` + 既有参数、既有签名、既有回执字段。
- `dispatch.json` 不写 `mode`；回执不写 `verdict_conclusion` / `verdict_locator` / `attempts_total` / `max_attempts` / `mode`。
- dispatched 下传 `argv` / `acceptance` / `max_attempts` → `dispatched_input_rejected`（避免调用方误以为 AW 会按这些字段验收）；`cmd` 本身不受影响。
- PM 侧无需任何改动；既有 dispatched 契约用例必须全绿。

#### 2.6.6 执行作用域：允许什么、拒绝什么

| 项 | 规则 | 原因码 |
|---|---|---|
| 进程启动 | `Command::new(argv[0]).args(argv[1..]).current_dir(cwd)`，**不经 shell**；`&&`、`>`、`|` 等按字面参数传入，不具 shell 语义 | — |
| 工作目录 | 固定为规范化后的 `cwd`；产出与验收都相对它 | — |
| 显式程序路径 | 含分隔符或绝对路径时，规范化后必须位于 `cwd` 内 | `standalone_program_outside_scope`（越界）/ `standalone_program_unresolved`（无法证明在界内，fail-closed） |
| 裸程序名 | 不含分隔符的名字（如 `git` / `cmd.exe`）按继承 `PATH` 解析；这是解释器/运行时的必要能力 | — |
| 验收路径 | 仅 cwd 内相对路径 | `standalone_path_escape` |
| 参数携带路径 | `argv[1..]` 中可识别的路径（识别范围见 §2.6.8）按规范化后的真实位置判断，越界在占号前拒绝 | `standalone_argv_path_escape`（越界）/ `standalone_argv_path_unknown`（无法判定，放行并记回执） |
| `cmd` / `env` / `path_prepend` | 一律拒绝 | `standalone_cmd_rejected` / `standalone_env_rejected` / `standalone_path_prepend_rejected` |

**明确不做**：不提供任意 shell 入口、不允许环境变量注入、不允许把执行根目录挪到 `cwd` 之外。

#### 2.6.7 真跑证据（v1.26）

真实 stdio 进程（非测试桩）两单，原始 `verdict.json` / `handback.json` 见 [`TOOLS-DESCRIPTION-AUDIT.md`](./TOOLS-DESCRIPTION-AUDIT.md) §25：

- 成功：`standalone-ok-20261107` → `verdict=passed`、`changed_files=["out.txt"]`、`status=ok`。
- 失败：`standalone-fail-20261107`，`max_attempts=2` → attempt-1 `failed/retry/acceptance_not_met`；attempt-2 `failed/terminate/retry_exhausted`；两次记录独立保留。

#### 2.6.8 argv 参数携带路径：识别范围、归一化与盲区（v1.28）

`argv[0]` 按 §2.6.6 的"显式程序路径/裸程序名"规则处理；本节只覆盖 `argv[1..]`。扫描发生在 `cwd`/`argv`/`acceptance` 校验之后、任何 attempt 目录创建与进程启动之前，因此越界一律是**占号前拒绝**（不执行、不落盘、不占号）。

**会被当作路径校验的形式**（命中即按 `cwd` 解析后的真实位置判断）：

| 形式 | 示例 | 校验对象 |
|---|---|---|
| 独立参数值 | `["--out", "../x"]` | 第二个 token 自身 |
| 旗标赋值 | `--flag=value` / `-k=value` | `=` 后的 value（可再带一层引号） |
| `=` 分隔 | `key=value`（key 为字母/下划线开头，仅字母数字 `_ . -`，≤64 字符） | value |
| `:` 分隔 | `key:value`（同上 key 规则；盘符 `C:` 不按 key 拆分） | value |
| 引号包裹 | `"../x"` / `'../x'` / `--out="my dir/out.txt"` | 去一层配对引号后的内容 |
| 相对路径与 `..` | `a/../../b`、`./x`、`..`、`a//b` | 逐组件归一化；`//` 折叠为空组件，`..` 逐级上跳，一旦跳出 `cwd` 即拒绝 |
| 绝对路径 | `/x`、`\x`、`C:\x`、`C:/x`、`D:\cwd\out.txt` | 字面归一化 + 真实位置双重判断 |
| UNC | `\\server\share\x` | 归一化后不属于本地 `cwd` 即拒绝 |
| Windows 逐字前缀 | `\\?\C:\cwd\x` | 按绝对路径判断，`\\?\`/`\\.\` 与普通盘符前缀等价比较 |
| 百分号编码 | `..%2f..%2fescape.txt`、`%2e%2e/x` | 最多两轮 `%HH` 解码后再判断（`encoded=true`） |
| 含空格路径 | `--out=my dir\out.txt`、`"my dir/out.txt"` | 原样逐组件解析，不按空白切分 |

**不会被当作路径**（明确盲区，宁可漏报也不误拒）：

| 形态 | 示例 | 原因 |
|---|---|---|
| URL | `https://example.com/a/b`、`file:///C:/x` | `scheme://` 优先识别为 URL；**不解析 `file://` 本地路径** |
| 正则/通配符 | `--pattern=^hello$`、`./logs/*.txt` | 含 `^ $ ( ) [ ] { } \| * + ?` 任一字符即视为模式，不解析（**含上述字符的真实路径会漏报**） |
| 普通标量 | `--name=foo`、`--count=3`、`out.txt` | 无分隔符、非 `.`/`..` 开头、且不是 `cwd` 下已存在的符号链接；单个名字无法指向界外 |
| 破折号开关名 | `--out` 本身 | 开关名不是路径；其独立值另行判断 |
| Windows 单字母斜杠开关 | `/C`、`/S` | cmd.exe 一类命令的参数，不是路径（`/etc/passwd` 这类多字符根路径仍按路径校验） |
| 未展开的环境变量/波浪号 | `%USERPROFILE%\x`、`$HOME/x`、`~user/x` | 不猜测真实位置，按"无法判定"处理（见下） |

**无法判定时的处理：放行并记未知（不 fail-closed）。** 原因：fail-closed 会把 URL、正则、环境变量这类合法参数一并误拒；而未知形态会在回执 `argv_path_scan.unknown[]` 逐条可见（原始参数、`via`、`class`、`reason_code=standalone_argv_path_unknown`、原因文案），调用方可复核。**不把"未知"写成"已验证安全"**，也不把它当作通过依据。

**归一化算法（按真实位置，不看字面）**：相对路径从 `cwd` 逐组件前进；遇到已存在组件即 `canonicalize`（符号链接按目标真实位置，指向界外立即拒绝；破链记为 unknown）。绝对路径先做字面归一化证明是否可能落在 `cwd` 内，再用 `canonicalize` / 最深的已存在祖先解析真实位置；跨卷、UNC 与本地 `cwd` 前缀不等即越界。Windows 组件比较忽略大小写，`\\?\`/`\\.\` 前缀视同盘符前缀。`a//b` 的空组件被折叠，`a/../../b` 的 `..` 按上跳后位置判定。

**回执字段（仅 standalone）**：

```json
"argv_path_scan": {
  "scope": "D:\\Temp\\demo",
  "checked": [
    { "index": 4, "raw": "--out=../x", "value": "../x", "via": "flag_equals",
      "class": "relative_path", "encoded": false, "decision": "in_scope",
      "resolved": "D:\\Temp\\demo\\x" }
  ],
  "unknown": [
    { "index": 5, "raw": "%USERPROFILE%\\x", "value": "%USERPROFILE%\\x",
      "via": "bare", "class": "unknown_env_expansion", "encoded": false,
      "decision": "allowed", "reason_code": "standalone_argv_path_unknown",
      "reason": "环境变量 %USERPROFILE% 未展开，无法判定真实位置" }
  ],
  "checked_total": 1,
  "unknown_total": 1
}
```

- `index` 是 1-based 的 argv 序号（`argv[0]` 恒为程序，不出现）。
- `class` ∈ `relative_path` / `absolute_path` / `unc_path` / `windows_drive_path` / `unknown_env_expansion`。
- `resolved` 是最深可解析位置（文件尚未创建时可能只到其已存在的祖先目录）。
- 同一份 `argv_path_scan` 也写入每次 attempt 的 `verdict.json`，作为判定前的预检证据。
- 越界时回执 `status=blocked`，`blocked_reason` 形如 `standalone_argv_path_escape: argv 第 N 项（原样参数）解析后越出 working directory：<解析细节>`；预检失败的回执不含 `argv_path_scan`。

**dispatched 不受影响**：该模式下传 `argv` 仍然直接 `dispatched_input_rejected`，不产生 `argv_path_scan`，既有回执字段与行为不变。

真跑证据（6 正例 + 6 反例，原始回执）见 [`TOOLS-DESCRIPTION-AUDIT.md`](./TOOLS-DESCRIPTION-AUDIT.md) §27。

#### 2.6.9 standalone 并发与重入语义（v1.29）

**并发模型（v1.51 更正）**：`task.execute` 在 bridge 内被交给**后台执行线程**，主循环继续读下一条消息；因此同一进程内可以有多个 `task.execute` 并发阻塞，`task.cancel` / `receipt.get` / `status` 等仍在主循环同步处理（`task.cancel` 自身最多阻塞 `CANCEL_WAIT_SECS = 10s`，期间不处理其他消息）。不存在「写完上一条响应前不会读下一条」的限制——这条旧表述只对非 `task.execute` 的工具成立。

**跨进程同 `task_id`（两个独立 bridge 进程、同一 `runs/`）**：

- 抢占锁是 `fs::create_dir(runs/<task_id>/attempt-1)` 的原子性。胜者创建成功并执行；败者得到 `AlreadyExists`，进入回放路径，**不会写第二个 attempt 目录**。
- 败者先读 `attempt-1/dispatch.json`（缺失则最多等 5s），校验 `mode` 与签名（`cwd` / `argv` / `acceptance` / `max_attempts` / `timeout_secs`）；一致后等待 `standalone-result.json`，等待上限为 `timeout_secs * max_attempts + 5s`。
- 等到的回执就是胜者那一份：`finishedAt` / `duration_ms` / `attempt` / `attempts_total` / `artifacts` / `stdout_digest` 完全相同，并追加 `replayed=true`、`replay_kind="waited"`。
- 只有胜者写 `verdict.json` / `handback.json` / `standalone-result.json`；不存在两个调用方互相覆盖同一 attempt 证据的路径。
- 参数不一致：败者得到工具错误 `task_id_conflict`（`isError=true`），不产生第二份证据。
- 等待超窗：得到工具错误 `task_in_progress`（明确拒绝，不自动重跑）；调用方可稍后用相同参数重试。
- 残缺目录（有 `runs/<task_id>/` 但无 `dispatch.json`，例如胜者崩溃在极窄窗口）：5s 后得到 `task_id_conflict`。这是**可检测症状**，不是静默复用。

**重入（同一 `task_id` 在已有完成记录后再次调用）**：当前语义是**回放既有结果**，三条候选路径中选"返回既有结果"——不新建 attempt、不重跑、不改写 `finishedAt` / `verdict.json` / `standalone-result.json`；响应带 `replayed=true`、`replay_kind="existing"`。要重新执行必须换新 `task_id`。

**不同 `task_id` 并发**：任务目录互不共享；每个 attempt 的超时清理独立使用一个 Job Object（`process_scope::Supervisor`），只终止本 attempt 的受控集合，不会按名字/全局杀进程。`blocked_reason` 会明示清理结果与不可观测的 BREAKAWAY/作业外进程；"已清理受控集合"不等于"整棵进程树已死"。

**跨进程已知限制（如实写入）**：

- `runs.prune` 与执行/回放之间**没有全局锁**，但 v1.30 起有跨进程活跃标记：AW 在 `runs/.inflight/<task_id>.json` 用 `create_new` 原子占位，`runs.prune` 看到该文件即跳过整个任务（`skipped[].reason="task_inflight"`，残留标 `task_inflight_stale`），因此"attempt-1 已 failed、attempt-2 正在跑"时不会删除 attempt-1。仍不保证标记检查与删除之间的 TOCTOU：若删除方绕过 AW 的 `runs.prune`（自研实现且不读标记），竞争窗口仍存在；AW 只能在写完后复核目录仍在，否则 `attempt_dir_lost`，不能阻止返回后的外部删除。
- 两个进程以**不同参数**同时抢同一 `task_id` 时只有一个真正执行，另一个必定 `task_id_conflict`；不支持参数协商或后到覆盖。
- 不保证跨进程的"先查后写"原子性之外的任何事务语义（例如 `receipt.get` 与执行并发时可能读到中间态 attempt 记录）。

真跑证据（两进程同 id、4 进程含超时、完成后重入、异参与残缺目录）见 [`TOOLS-DESCRIPTION-AUDIT.md`](./TOOLS-DESCRIPTION-AUDIT.md) §28。

### 2.6.10 与 `runs.prune` 的并发防护（v1.30）

**活跃标记（AW 侧唯一权威）**

- 路径 `runs/.inflight/<task_id>.json`。`.inflight` 以 `.` 开头，`is_task_dir_name` 直接排除，`runs.list` / `runs.prune` 都不会把它当任务目录。
- 内容：`version` / `task_id` / `pid` / `mode` / `state` / `startedAt` / `deadlineAt`。
- 获取：`create_new(true)` 原子占位。已存在且未过期 → 本次调用不持有该标记（同 id 并发继续走 attempt 目录回放）；已存在且判定残留 → 删除后重试（最多 3 次），并发清理由 `create_new` 决出唯一持有者。
- 删除：RAII `Drop`，成功 / 失败 / `blocked` / `?` 提前返回全部覆盖。进程崩溃或被 kill 时文件残留：`pid` 已消失且超过 `startedAt + 5s` 判残留；无 `pid` 时退回 `deadlineAt + 120s`。
- `deadlineAt = now + timeout_secs * max_attempts + 60s`（dispatched 按 1 次），只用于残留判定与展示，不是准入条件。

**`runs.prune` 行为（PM 无需改代码）**

- 看到活跃标记一律跳过整个 task，不进入 `candidates[]` / `candidates_total`。dry-run 的 `skipped[]` 给出 `{task_id, attempt, reason, inflight{stale,...}}`，`reason` 为 `task_inflight`（存活）或 `task_inflight_stale`（残留，仍保守跳过）。
- 实测：attempt-1 已 failed、attempt-2 正在跑时，dry-run 的 `candidates` 不含该 task、`skipped` 为 `task_inflight`；delete 模式 `deleted[]` 不含该 task；attempt-1 原样保留、attempt-2 回执 `ok`。同一任务在完成后（无标记）把 attempt-1 的 `finishedAt` 改成 40 天前，dry-run 立即把它列为候选并删除——证明该 attempt 本来是合格候选，拦截来自标记。

**写路径 vs 句柄（实测结论）**

- stdout/stderr 是创建时打开的**句柄**并交给子进程写；`dispatch.json` / `verdict.json` / `handback.json` / `standalone-result.json` 全部是**按路径的短写**。
- Windows 实测：运行中 rename 活跃 attempt 目录 → `WinError 5 拒绝访问`，目录原地未动、`.prune-trash` 未产生，AW 正常返回 `ok`。这是操作系统对打开句柄的共享语义（rename 被拒），不是 AW 自己加锁。
- 若目录被外部删除/移走（Windows 允许删文件、句柄继续有效），按路径读证据会失败 → 回执具名 `attempt_dir_lost`；写完后再次复核 5 个关键文件与 `standalone-result.json`，缺一即报错，绝不返回 `ok`。
- POSIX 未实测：目录 rename 在打开句柄下可能成功，句柄写入会落到新路径（即"写进 trash"）。因此非 Windows 部署必须依赖标记 + 删除方读取标记，不能只靠 OS 拒绝。

**全部记录被清理后的重跑**

- 台账 `runs/.task-history/<task_id>.json`（不含执行证据，只记 `firstSeenAt` / `attempts` / `lastStatus` / `lastFinishedAt`）。
- 新任务目录不存在且台账存在 → 本次执行与 `verdict.json` / handback 带 `replay_basis="rerun_after_records_missing"`、`prior_attempts`、`prior_last_finished_at`；首次为 `fresh`。同 id 再调回放时字段随落盘记录保留（`replayed=true`、`replay_kind=existing`）。
- 判据：调用方看到 `replay_basis != "fresh"` 且 `attempt=1`，即"此前记录已丢失，本次是重跑"，不会误认为拿到既有结果。
- 限制：若删除方把 `.task-history` 一并清理，则无法判定；在 AW 之外伪造 `runs/` 且从未被 AW 执行过的 task 也判定不了。不写成"覆盖所有删除来源"。

**TOCTOU 边界**：标记检查与 rename/delete 非原子。用 AW 自带 `runs.prune` 已被覆盖；自研删除方必须先读 `runs/.inflight/<task_id>.json`，文件存在且 `stale=false` 就跳过。PM 只需看到该文件（或 prune 回执里的 `task_inflight`），不需要读源码。

#### 2.6.11 dispatched 执行中重投：阻塞回放（v1.51 实测）

- **行为**：同 `task_id`、同签名，在首次执行尚未落盘时再次 `task.execute` → 不启动第二份执行，不立即报 `task_in_progress`；该调用阻塞等待首次回执落盘，然后**原样回放**（`attempt` / `finishedAt` / `status` 与首次一致）。
- **上限**：等待窗口为 `timeout_secs + 5s`（签名一致，两次调用的 `timeout_secs` 相同，故上限 605s）。超窗才返回工具错误 `task_in_progress:`。
- **极窄窗口**：首次刚创建 attempt 目录、尚未写 `dispatch.json` 时到达的重投，先等 `dispatch.json` 最多 5s；仍缺失返回 `task_id_conflict:`，同样不启动第二份。
- **并发性质**：N 个重投各自阻塞、各自回放同一份回执，不排队、不互相取消；它们跑在各自的后台执行线程，bridge 主循环仍处理 `task.cancel` / `receipt.get`。
- **实测**：原任务 `ping -n 20`，0.8s 后重投；重投阻塞 4.3s，直到 `task.cancel` 让首次收尾，随后回放同一份 `canceled` 回执（`finishedAt` 相同）。
- **取舍**：改文档不改行为。`task.execute` 本就是阻塞调用，重投通常是调用方重试，阻塞回放保证「重试拿到同一份结果」；改成立即报错会把合法重试变成假失败。PM 若不愿阻塞，应先 `receipt.get` 或另起连接。

### 2.7 机器可读的能力声明与活跃标记清理（v1.32）

**`capabilities`（只读、无参数）**

返回 `{version:"capabilities/1", platform:{os,arch}, status_enum[], decision_rule, entries_total, not_proven_total, entries[]}`。每条 claim 的字段语义：

| 字段 | 机器判定方式 |
|---|---|
| `id` | 稳定标识；按 id 查找，不依赖顺序与文案 |
| `holds` | `true`=在 `scope` 内成立；`false`=明确不成立或未证明；`null`=本环境未验证 |
| `status` | `proven` / `not_proven` / `partial` / `unverified` / `supported`；`holds=false` 时必带非空 `reason_code` |
| `scope` | 该结论覆盖的边界（不是宣传语）；超出 `scope` 的行为不在 claim 内 |
| `reason_code` | 不成立/未证明的原因码；可直接做 if 分支 |
| `platforms_verified` / `platforms_unverified` | 已实跑验证 / 未逐条验证的平台集合（`windows` / `linux` / `macos`） |
| `automatic` | 是否无需调用方额外操作即自动发生 |
| `evidence` | 对应的用例名（集成或单元），可复核 |

当前 19 条（v1.45 平台拆分：Linux 侧两条可证的改为 `proven`，Windows 侧各立独立 `not_proven` 条目；`not_proven_total=2`，即两条 Windows 条目），关键条目：

| id | holds | status | reason_code | 边界 |
|---|---|---|---|---|
| `timeout.terminates_controlled_set` | true | proven | null | POSIX 进程组 / Windows Job Object |
| `timeout.descendant_snapshot_cross_check` | true | proven | null | 父链快照 × 受控集合成员；cgroup 可建时并入 cgroup 成员，查询失败记 `Unavailable`；v1.39 起 Windows 父链边带「子创建时间 ≥ 父创建时间」校验、根→直接子进程再加「≤ 根退出时间」上界，并剔除已终止或 `OpenProcess` 返回 `ERROR_INVALID_PARAMETER`（PID 已不存在）的行；v1.40 起 Linux 父链边同样按 `/proc/<pid>/stat` 的 `starttime` 做「子启动时刻 ≥ 父启动时刻」校验，只放宽「父边启动时刻读不到」的那条边；创建时间/存活状态不可读时仍按旧行为保留（审计 §42、§43） |
| `timeout.cgroup_v2_scope_observation` | true | proven | null | 仅 Linux；需 cgroup v2 可写委派，建不起来只降级记原因，不阻断执行 |
| `timeout.cgroup_kill_terminates_scope` | true | proven | null | 仅 Linux；需可写 cgroup v2 且内核提供 `cgroup.kill`（>=5.14），可终止被收养的逃逸后代 |
| `timeout.cleans_entire_process_tree` | true | proven | null | **仅 Linux**（`platforms_verified=[linux]`）：需可写 cgroup v2 + `cgroup.kill`。整树 = 终止开始时可归属本任务的全部后代（父链闭包 ∪ `cgroup.procs` 成员）及其终止过程中派生的子进程；逃出 `cgroup.procs` 的进程由终止前名单逐个 SIGKILL 收口（v1.45，`cgroup_kill_terminates_escapee_that_left_cgroup_procs`）。四观测量（`cgroup.procs` 空、`populated 0`、逃逸 PID 不存在、快照无后代）同时成立才报 `Cleaned`。残余边界：进程若在 terminate 开始前已同时离开 `cgroup.procs` 且父链被收养，本机无 OS 原语可指认；快照/回执标 unobservable |
| `timeout.cleans_entire_process_tree_windows` | false | not_proven | `requires_out_of_job_enumerable_terminable_os_primitive` | **仅 Windows**（`platforms_verified=[windows]`）：作业只覆盖加入作业的进程，作业外进程（`BREAKAWAY`、加入失败、不进作业的 `conhost.exe`）可能在终止后存活；需要「作业外进程可枚举 + 可终止 + 存活有权威来源」的 OS 原语，本机不存在。受控集合内孙进程已证成（`timeout_cleans_controlled_set_including_grandchild`），不上调整树 |
| `timeout.enumerates_escaped_descendants` | true | proven | null | **仅 Linux**（`platforms_verified=[linux]`）：cgroup v2 可写时 `cgroup.procs` 成员集合是逃逸后代的权威枚举，不依赖父链，被收养后仍精确点名（`cgroup_scope_enumerates_adopted_escapee` 断言 `members=[escaped]`）。退化：无写权限/非 root/容器/cgroup v1 → `escaped_scope_unobservable` 退回进程组旁证；`cgroup.procs` 读不出 → 整份快照 `unobservable`（`cgroup_members_unreadable_is_not_empty`）；进程已离开 `cgroup.procs` → 作用域不含它，只在脱离前靠父链可见（旁证） |
| `timeout.enumerates_escaped_descendants_windows` | false | not_proven | `requires_out_of_job_enumerable_terminable_os_primitive` | **仅 Windows**（`platforms_verified=[windows]`）：Toolhelp 父链快照是旁证，不是作业外进程的权威枚举；`conhost.exe`、创建时间/存活状态读不到时仍可能误报。需要「作业外进程可枚举 + 可终止 + 存活有权威来源」的 OS 原语，本机不存在 |
| `platform.posix_inflight_semantics_verified` | true | proven | null | POSIX 已跑通标记/stale/prune 用例，并单独复核 rename 活跃 attempt 目录（cwd/打开 fd 仍有效）；macOS 未验证 |
| `concurrency.check_then_delete_atomic` | true | proven | null | 同卷 rename 认领 + 认领后重跑同一份 `stale_rule`：并发清扫同一残标只有一方 `deleted`、其余 `lost_race`；被刷新（身份不变、deadline 延长）的标记零误删，记 `not_mine` 并 create-if-absent 还原。残余边界：跨卷 `EXDEV` 则认领失败、零删除；认领与还原之间原名短暂空缺；审计 append 非事务；Windows 并发 rename 可能都返回成功，以认领副本是否存在判定归属（v1.43） |
| `concurrency.sweep_restore_never_clobbers` | true | proven | null | 复核不一致时还原只做 create-if-absent，不覆盖更新者已占住的原名；不保证窗口内无短暂文件名空缺 |
| `inflight.stale_marker_observable` | true | supported | null | `runs.inflight action=list` |
| `inflight.stale_marker_cleanable` | true | supported | null | `action=clean`，需显式调用（`automatic=false`） |
| `inflight.stale_markers_auto_swept` | true | proven | null | 服务启动时自动清扫一次，默认每 3600 秒再清扫；与显式路径共用 `sweep_inflight_markers` / `stale_rule` / `runs/inflight-sweep.jsonl`（`trigger=startup|periodic`）；`AW_INFLIGHT_SWEEP_SECS` 可调：0=仅启动、off=关闭、正整数=周期；审计写不进去则零删除（v1.42） |

调用方判定规则：`holds=true` 才可依赖；`holds=false` 必须读 `reason_code` 并据此降级；`holds=null` 不得当成已保证。

**v1.45：两条 `not_proven` 按平台拆开（Linux 可证的已证，Windows 结构性不可证单独列）**

v1.44 的结论是「两条都保持 `false/partial`」，把 Windows 的结构性不可证与 Linux 的「可证但当时没证」混在同一格里。v1.45 起改为平台维度：

1. `timeout.cleans_entire_process_tree`（Linux，`true/proven`）：整树清理在 **platform=linux、cgroup v2 可写、内核提供 `cgroup.kill`** 时证成。新增 `cgroup_kill_terminates_escapee_that_left_cgroup_procs`：父（`sh`）→ 子（`sh`）→ 孙（`sh`），孙进程 `setsid` 后**把自己写进旁路 cgroup**（`sink_members=[<pid>]`、`move_status="moved"`），即真正离开本作用域的 `cgroup.procs`；此前的实现在 `cgroup.kill` 后误报 `Cleaned` 而逃逸进程仍存活——**先红后绿**：

   ```
   [evidence] cgroup_left before root=1895 child=Some("1906") escaped=1913 scope=CgroupV2 sink_members=[1913] move_status=Some("moved\n") descendants=[DescendantProcess { pid: 1906, in_controlled_set: true }, DescendantProcess { pid: 1913, in_controlled_set: false }] outside=[1913]
   [evidence] cgroup_left after outcome=Cleaned escaped_alive=true members=Ok([]) populated=Ok(0) descendants=[] unobservable=None
   thread 'server::process_scope::imp::tests::cgroup_kill_terminates_escapee_that_left_cgroup_procs' panicked at apps/mcp-bridge/src/server.rs:4266:17:
   逃出 cgroup.procs 的后代在终止后必须不再存活（cgroup.kill 打不到它，必须另行收口）
   test result: FAILED. 25 passed; 1 failed
   ```

   修复（v1.45，`Supervisor::terminate`）：动手前用「`/proc` 父链闭包 ∪ `cgroup.procs` 成员」建立后代名单，每项附 `/proc/<pid>/stat` 的 `starttime`；`cgroup.kill` 之后对名单里仍存活者逐个 `SIGKILL`，starttime 不一致（PID 复用）绝不发信号；终止过程中每轮把存活名单成员在 `/proc` 里的新后代并入名单。**四个观测量同时成立才允许 `Cleaned`**：`cgroup.procs` 空、`populated 0`、逃逸 PID 不存在、快照无后代。绿（WSL Ubuntu 24.04 LTS，root，cgroup2fs，内核 `6.18.33.2-microsoft-standard-WSL2`，日志 `/root/.aw-posix-verify/logs/posix-verify-20261108-033331.log`）：

   ```
   [evidence] cgroup_left before root=33075 child=Some("33082") escaped=33087 scope=CgroupV2 sink_members=[33087] move_status=Some("moved\n") descendants=[DescendantProcess { pid: 33082, in_controlled_set: true }, DescendantProcess { pid: 33087, in_controlled_set: false }] outside=[33087]
   [evidence] cgroup_left after outcome=Cleaned escaped_alive=false members=Ok([]) populated=Ok(0) descendants=[] unobservable=None
   test server::process_scope::imp::tests::cgroup_kill_terminates_escapee_that_left_cgroup_procs ... ok
   test result: ok. 27 passed; 0 failed
   ```

   残余边界（写进能力位 detail）：进程若在 `terminate` 开始前就已同时离开 `cgroup.procs` 且父链被收养，本机没有 OS 原语可指认它；此情形由快照/回执标 `unobservable`，不读成「无残留」。

2. `timeout.cleans_entire_process_tree_windows`（Windows，`false/not_proven`）：作业外进程（`BREAKAWAY`、加入作业失败、不进作业的 `conhost.exe`）可能在终止后存活。`reason_code=requires_out_of_job_enumerable_terminable_os_primitive`，detail 明写「需要作业外进程可枚举可终止的 OS 原语，本机不存在」。受控集合内孙进程的握手夹具结论保留（`timeout_cleans_controlled_set_including_grandchild`），只覆盖作业内，不上调整树。

3. `timeout.enumerates_escaped_descendants`（Linux，`true/proven`）：cgroup v2 可写时，`cgroup.procs` 成员集合是**权威**枚举，不依赖父链。证据（同一次 WSL 运行）——根退出、逃逸后代被收养（`escaped_ppid` 不再指向根）后，成员集合仍精确点名该 PID（断言 `members == vec![escaped]`）：

   ```
   [evidence] cgroup_scope adopted escaped_alive=true escaped_ppid=Some(32364) root=33029 members=Some(Ok([33071])) descendants=[DescendantProcess { pid: 33071, in_controlled_set: false }] controlled=Some([]) scope=CgroupV2
   ```

   反例条件（逐条实测/文档化）：① **无 cgroup 写权限**（非 root / 容器未委派 / cgroup v1）→ 作用域建不起来，记 `escaped_scope_unobservable`，枚举退回进程组（旁证）；② **`cgroup.procs` 读不出**（权限/卸载/竞态）→ 整份快照 `unobservable`，绝不当空（`cgroup_members_unreadable_is_not_empty`：`[evidence] cgroup_unreadable scope=ProcessGroup unobservable=Some("cgroup_procs_unreadable:No such file or directory (os error 2)") is_empty=false`）；③ **进程已脱离 `cgroup.procs`**（写进别的作用域）→ 作用域枚举不含它，只在脱离前靠父链可见（旁证），根退出被收养后不可见；④ **无 `cgroup.kill`** 不影响枚举权威性，只影响终止（v1.45 由终止前名单补足已知逃逸者，见上条）。

4. `timeout.enumerates_escaped_descendants_windows`（Windows，`false/not_proven`）：Toolhelp 父链快照是旁证，不是作业外进程的权威枚举；`reason_code=requires_out_of_job_enumerable_terminable_os_primitive`，detail 明写「需要作业外进程可枚举可终止的 OS 原语，本机不存在」。

条目 17→19（两条 Linux 条目 + 两条 Windows 条目替换原来的两条跨平台条目），`not_proven_total` 仍为 2，且这 2 条现在都是明确的 Windows 结构性条目。macOS 两条 Linux 条目的 `platforms_unverified` 含 `macos`（未验证）。

平台覆盖（v1.34，v1.38 更新）：`inflight.*` 四条与 `platform.posix_inflight_semantics_verified` 的 `platforms_verified` 已含 `windows`+`linux`（WSL Ubuntu 24.04 实跑），`platforms_unverified` 只剩 `macos`。v1.38 起 `platform.posix_inflight_semantics_verified` 为 `holds=true/proven/reason_code=null`：POSIX 上已单独复核 rename 活跃 attempt 目录——进程 cwd 指向该目录且持有目录内文件 fd 时，`rename` 与随后的 `remove_dir_all` 均成功，cwd/fd 跟随新路径、fd 仍读到原内容（用例 `posix_rename_active_attempt_semantics`）。该上调只限本条，`timeout.*` 与 TOCTOU 条目不受影响。

后代观测（v1.35）：观测结构改为「父链快照 ∪ 作用域成员」，并新增 `scope` 与 `escaped_scope_unobservable` 两个字段。Windows 用 Toolhelp 父链快照 × `JobObjectBasicProcessIdList`；Linux 用 `/proc/<pid>/stat` 父链快照 × `pgrp` 成员，cgroup v2 可建时再并入 `cgroup.procs` 成员（子进程在 exec 前由 `pre_exec` 把自己写入独立 cgroup，无「先派生后入组」窗口）。`controlled_pids` 始终表示**终止作用域**（真正会被 `kill(-pgid)` / 作业终止打到的集合），因此 `in_controlled_set=false` 就是「会被观测到但杀不到」的逃逸后代。

**观测边界（v1.35 实测）**：cgroup 建立成功后，`setsid` 逃逸后代在根退出被收养后仍被枚举，终止结果由 `Cleaned` 修正为 `StillAlive`（此前是假 `Cleaned`）。cgroup 建不起来时（cgroup v1 / 容器 / 非 root）`escaped_scope_unobservable` 记原因，观测退回进程组，`is_empty()` 不能读成「没有逃逸后代」。macOS 无 `/proc`，恒不可观测。
**Windows 侧能力边界（v1.39）**：`timeout.cleans_entire_process_tree` 在 Windows 上**能证**两件事——(a) 终止后作业记账 `ActiveProcesses` 归零（`JobObjectBasicAccountingInformation`）；(b) 父链快照在创建时间与存活状态可读时做三道过滤，把与作业无关的进程排除在「后代」之外：按「子进程创建时间 ≥ 父进程创建时间」校验每条父链边（拒绝同号 PID 复用留下的旧父链）；对「根 → 直接子进程」再加「子创建时间 ≤ 根退出时间」上界（拒绝根 PID 被后来者复用后产生的子进程）；用 `GetExitCodeProcess` 与 `OpenProcess` 的 `ERROR_INVALID_PARAMETER` 剔除「已终止 / PID 已不存在」的行（拒绝 Toolhelp 枚举与逐个打开之间的竞态残行）。确定性用例 `pid_reuse_parent_chain_edge_is_rejected` / `terminated_rows_are_not_counted_as_descendants` 覆盖根级复用、中间层复用、根退出后复用、时间不可读、僵尸行、存活状态不可读六类边。**证不到**：(c) 作业外进程（`BREAKAWAY` 或 `AssignProcessToJobObject` 失败）——例如由 csrss 创建、不进作业的 `conhost.exe`，它若在终止瞬间仍存活，快照报 `StillAlive` 是**真实观测**而非误报；(d) 换父后被收养的进程是否仍存活（Windows 无 cgroup 等价的作用域观测）；(e) 创建时间或存活状态读不到（权限不足）时，边校验退回旧行为（保留），快照只作旁证。因此 v1.39 当时本条保持 `holds=false/partial`（历史）；v1.45 起 Windows 侧独立为 `timeout.cleans_entire_process_tree_windows=false/not_proven`，不再与 Linux 共用一个 `holds`。

**Linux 侧父链边校验（v1.40）**：`/proc` 父链闭包补上同一条判据——每条边要求「子进程启动时刻 ≥ 父进程启动时刻」，启动时刻取 `/proc/<pid>/stat` 第 22 字段 `starttime`（自开机起的时钟滴答，不随 PID 复用变化）。`comm` 可含空格与括号，解析以最后一个 `)` 为界；根进程的启动时刻在 `attach` 时记录，闭包每层都带父边的启动时刻。只有「这条边的父进程启动时刻读不到」时才退回旧行为（接受该边），不因为一个未知时刻把整棵树都放宽。确定性单测 `pid_reuse_parent_chain_edge_is_rejected` 用注入的 `ProcRow` 表覆盖根级复用、祖先被拒后其后代不可达、中间层复用、时刻不可读四种情形，不依赖真机撞 PID 复用。**v1.40 当时能力位没有变化**（历史）：`timeout.cleans_entire_process_tree` 与 `timeout.enumerates_escaped_descendants` 当时仍 `holds=false/partial`，条目 17、`not_proven_total=4`；v1.45 已按平台拆分，见上节（条目 19、`not_proven_total=2`，两条 Linux 条目 `proven`）。

**为什么 Linux 的 `enumerates_escaped_descendants` 能升为 `proven`（v1.45）**：该能力在 **`platforms_verified=[linux]` + cgroup v2 可写** 的限定下成立——`cgroup.procs` 成员集合本身是权威枚举，不依赖父链（被收养后仍精确点名，见上节证据）。容器/非 root/cgroup v1 建不起作用域时不是「无条件成立」而是**如实降级**：`escaped_scope_unobservable` 记原因、退回进程组旁证；运行期读不出 `cgroup.procs` 时整份快照 `unobservable`，绝不当空。窗口/判据未放宽，只把「条件」写进 `scope`、`platforms_verified`、`platforms_unverified` 与 detail；Windows 的旁证性质由独立条目 `timeout.enumerates_escaped_descendants_windows` 表达。

**终止判据（v1.37 实测）**：Linux 可写 cgroup v2 且内核提供 `cgroup.kill`（v5.14+；root cgroup 没有该文件，子 cgroup 才有）时，`terminate` 在原有进程组信号之后向作用域写 `cgroup.kill`，并按**三个独立观测量**判定：`cgroup.procs` 为空、`cgroup.events` 为 `populated 0`、逃逸 PID 已不存在；任一不满足或读不出即报 `StillAlive`/`Unavailable`，只有三者都成立且快照无可枚举后代才写 `Cleaned`。`cgroup.kill` 在 fork 压力下可能漏掉正在派生的后代，因此每轮轮询重试写入（重复写返回 0，幂等；实测 13 个成员 + 持续 fork 的进程在写入后 `procs=0`、`populated 0`）。v1.37 当时只新增条件性窄结论 `timeout.cgroup_kill_terminates_scope=true/proven`，未上调整树条目（**历史**）；v1.45 起见上节：Linux 条目在「cgroup v2 可写 + `cgroup.kill`」限定下 `true/proven`，Windows 侧独立 `false/not_proven`。

**Linux 整树清理的证成范围与残余卡点（v1.41）**：在满足条件的环境（cgroup v2 可写 + 内核提供 `cgroup.kill`）里，Linux 侧整作用域终止**能证成**，不再是「笼统地需要 cgroup」。WSL Ubuntu 24.04（root，cgroup v2，内核 `6.18.33.2-microsoft-standard-WSL2`）的真实进程证据（日志 `/root/.aw-posix-verify/logs/posix-verify-20261108-013052.log`）：

```
[evidence] cgroup_kill before escaped=1191 scope=CgroupV2 descendants=[DescendantProcess { pid: 1191, in_controlled_set: false }]
[evidence] cgroup_scope adopted escaped_alive=true escaped_ppid=Some(432) root=1181 descendants=[DescendantProcess { pid: 1200, in_controlled_set: false }] controlled=Some([]) scope=CgroupV2
[evidence] cgroup_kill after outcome=Cleaned escaped_alive=false members=Ok([]) populated=Ok(0) descendants=[] unobservable=None
[evidence] cgroup_scope after outcome=Cleaned escaped_alive=false kill_supported=true descendants=[] controlled=Some([]) scope=CgroupV2
```

即：`setsid` 逃逸进程被收养（`ppid` 不再是根）、受控进程组为空（`kill(-pgid)` 打不到）、cgroup 仍能枚举；写 `cgroup.kill` 后 `cgroup.procs` 为空、`populated=0`、快照无后代、逃逸进程已死，四项同时成立才报 `Cleaned`。**v1.41 当时不把跨平台条目上调为 `holds=true`**（历史）：同一字段还要覆盖 Windows 与 macOS，Linux 的条件性由 `timeout.cgroup_kill_terminates_scope` 单独表达。v1.45 改为**平台拆分**：Linux 条目本身改为 `true/proven`（`platforms_verified=[linux]`，条件写进 scope/detail），Windows 另立 `false/not_proven` 条目——不再让 Windows 的结构性不可证把 Linux 拖在同一格里。

**降级路径（v1.41 实测，v1.45 更新两个确定性用例）**：

- 内核没有 `cgroup.kill`：`terminate` 不写 `cgroup.kill`；v1.45 起仍按「父链闭包 ∪ `cgroup.procs`」名单把已知逃逸成员逐个 `SIGKILL`（进程会死），但判定只认独立观测点——伪造的静态作用域记账仍显示成员（`cgroup.procs` / `populated` 不更新），轮询到 3s 截止后报 `StillAlive`。实测 `[evidence] cgroup_no_kill root=33056 escaped=33038 kill_supported=false outcome=StillAlive escaped_alive=false members=Some(Ok([33038])) populated=Some(Ok(1)) descendants=[DescendantProcess { pid: 33038, in_controlled_set: false }]`（用例 `cgroup_without_kill_reports_still_alive`）。**不因「自己杀掉了名单」就谎报 `Cleaned`**。
- 作用域建不起来（`Degraded`，如 cgroup v1 / 非 root / 容器未委派）：`terminate` 只按进程组判据，被收养的逃逸进程既收不到信号、也不在父链快照里，结果是受控集合口径的 `Cleaned`。实测 `[evidence] cgroup_degraded after outcome=Cleaned escaped_alive=true descendants=[] scope=ProcessGroup unobservable=None escaped_scope_unobservable=Some("cgroup_v2_unavailable")`（用例 `cgroup_degraded_terminate_is_group_only`）。这是**降级分支**的残余缺口：`Cleaned` 只表示「受控集合已清」，不等于「无残留」。它**不是静默**——降级原因写在快照 `escaped_scope_unobservable`，能力位 `scope`/detail 明写该分支；`Cleaned` 回执固定写「逃逸进程是否仍在运行未验证」，读方必须同时读回执与能力位。

**bridge 非正常退出时的直接子进程回收（v1.47，platform split）**

- **Windows**：`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`。bridge 进程消失时内核关闭作业句柄，作业内所有进程（含孙进程）被终止。真实链路 B2：`child_heartbeat_before=18 after=18 child_survived=False`。
- **Linux**：`pre_exec` 里调用 `prctl(PR_SET_PDEATHSIG, SIGKILL)`，覆盖 fork→exec 窗口，跨越 exec 保留；设完立即复查 `getppid()`，若父进程已在 fork 与 `prctl` 之间死亡则 `_exit(128)`，不执行用户命令。正常退出路径不触发（PDEATHSIG 只在父死亡时投递一次），已完成任务不受影响（B2：完成态产物与 `handback.json` 字节未变）。真实链路 B2（WSL）：`child_heartbeat_before=20 after=20 child_survived=False`。
- **Linux 的边界（必须同时读）**：`PR_SET_PDEATHSIG` **不遗传**。它只收掉 bridge 的**直接子进程**；该子进程再 fork 出来的孙进程在子进程死亡后被 reparent 到 init，继续运行。真实链路 B5（WSL）：`pdeathsig_scope=direct_child_only grandchild_heartbeat_before=10 after=25 grandchild_survived=True`。因此不能把 Linux 写成「整棵树随 bridge 消失」，也不能与 Windows 作业对象混为一谈。
- **启动回收（v1.48，取代人工巡检）**：bridge 启动时扫描 `/sys/fs/cgroup/aw-mcp-bridge-*`，只有同时满足「sidecar 归属记录自洽 + `runs/<task_id>/attempt-<n>/dispatch.json` 存在 + owner bridge 已死（`/proc/<pid>/stat` starttime 防 PID 复用）」才动手；任一不满足只写审计、不动进程。详见 §2.11。v1.47 的人工巡检脚本**不再作为兜底路径**，只保留为排障参考。
- **能力位不变**：本条是崩溃路径的进程回收，不是上面 19 条 timeout 能力位的一部分；Windows 与 Linux 的覆盖范围差异写在这里，§2.7 的 `timeout.*` 判据与 `not_proven_total=2` 不受影响。

**`runs.inflight`（`action=list|clean`）**

- `list`：只读枚举 `runs/.inflight/*.json`（非 `.json` 临时文件、符号链接不参与），每个标记给出 `task_id`、`file`、`pid`、`pid_alive`、`stale`、`stale_reason`（`pid_alive` / `pid_missing_grace` / `pid_missing` / `deadline_in_grace` / `deadline_expired` / `unreadable`）、`startedAt`、`deadlineAt`、`age_secs`、`deadline_overdue_secs`、`removable`；顶层给出 `total` / `stale_total` / `active_total` / `unreadable_total` / `state`（`missing|empty|ready`）与机器可读的 `stale_rule`（`pid_alive=never_stale`、`pid_missing_grace_secs=5`、`deadline_grace_secs=120`、`live_pid_wins_over_deadline=true`、`cleanable_only_when=stale=true`、`unreadable=kept_never_deleted`）及 `sweep_rule`（见 §2.9）。
- `clean`：默认 `dry_run=true`，只回 `would_remove[]`、零删除；真删必须同时 `dry_run=false` 与 `confirm=true`，否则 `confirm_required` 且不删。只处理 `stale=true` 的标记；`pid` 存活（`pid_alive=true`）**永不进入可删集合，即使 deadline 已过**。
- 删除安全：先把标记原子 rename 为 `.inflight/*.clean-*.tmp`，再读回并复核 `task_id`+`pid`+`startedAt` 与判定时一致才删；不一致说明被并发替换，按 create-if-absent 还原（原名已被更新者占住则保留新标记并记 `marker_superseded`，原名仍为空才放回并记 `marker_changed`），**任何分支都不用 rename 覆盖已存在文件**。文件已消失记 `marker_disappeared`；读不懂的 JSON 保留并记 `unreadable`，绝不按残留删除。
- 可选 `task_id`：只处理该标记，命名校验与任务目录同源；非法 id 返回 `invalid_arguments`，零写入。
- 不触碰任何 `runs/<task_id>/attempt-N/` 目录，不进入 `runs.list` / `runs.prune` 的候选枚举（`.inflight` 以 `.` 开头，被 `is_task_dir_name` 排除）。

**PM 只需看到什么**：能力边界看 `capabilities.entries[].id/holds/reason_code`；残留看 `runs.inflight` 的 `markers[].stale/stale_reason`，清理调用 `action=clean` + `dry_run=false` + `confirm=true`。不需要读 AW 源码，也不依赖人工删文件。

**仍然存在的限制**：自动清扫只在「服务进程存活 + 启动/周期触发」时发生（v1.42 起 `inflight.stale_markers_auto_swept=true/proven`；`AW_INFLIGHT_SWEEP_SECS=off` 可退回仅三个显式路径）；v1.43 起 `concurrency.check_then_delete_atomic=true/proven`，认领后重跑 `stale_rule` 已消除「删掉别人刚刷新标记」这一类误删，但跨卷 `EXDEV`、认领与还原之间的短暂文件名空缺、审计 append 非事务仍是边界；POSIX 的标记/rename 语义仍为 `partial`。

**并发判据（v1.36）**：跨进程窗口不再用「一次 rename 就是原子」这种说法。可观测量定为：给定被判 `stale` 的记录 R0，若 W1 在「检查→认领」之间替换同名文件、W2 在「认领→复核」之间再次占住原名，则清扫结束后**原位置必须是 W2 的完整记录**（`pid`+`startedAt` 一致），`removed_total=0`，`kept[]` 具名 `marker_changed`/`marker_superseded`，且目录里不留 `.tmp` 副本。实现上还原改用 `create_new`（O_EXCL）而不是 `rename`。这条只证「不删、不覆盖与判定快照不同的标记」（`concurrency.sweep_restore_never_clobbers=true/proven`）；**不证**「检查到认领之间没有窗口」——文件系统没有 compare-and-rename，认领前后仍可能出现短暂的文件名空缺，第三方仍可能在该空缺里开跑。`removed[]` 的 `path` 只表示「本次删掉了这个文件名下的那份标记」，读方不得据此断言该路径当前不存在标记。

**v1.43 收紧**：认领后不再只比身份三元组，而是对认领副本重跑同一份 `stale_rule`。仍 `stale` 才删（`outcome=deleted`）；已不 `stale`（例如刷新者只延长 `deadlineAt`、`task_id/pid/startedAt` 全不变）就地 create-if-absent 还原并记 `outcome=not_mine`；认领副本已不存在记 `outcome=lost_race`（不算错误）。并发清扫同一残标的可观测量：`deleted_total=1`、`lost_race_total=N-1`；N 个清扫者与被刷新标记并发时 `deleted_total=0` 且 `not_mine_total>=1`，原位必须是刷新后的记录。Windows 上并发 `rename` 可能全部返回成功但只有一个目的地产生文件，故以「认领副本是否存在」判定归属。

### 2.8 PM 判活口径（v1.32.1，唯一约定）

AW 提供 `runs.inflight` 后，PM **只允许**用 `action=list` 的以下字段判活：

| 用途 | 字段 | 判定/含义 |
|---|---|---|
| 判活（唯一权威） | `markers[].task_id` + `markers[].stale` | 命中该 `task_id` 且 `stale=false` → 活跃；无命中，或命中但 `stale=true` → 当前无活跃执行者 |
| 清理（不得用于判活） | `markers[].removable` | 仅表示该标记可清；恒等于 `stale`，不代表任务结果 |
| 诊断（不得用于判活） | `pid` / `pid_alive` / `stale_reason` / `startedAt` / `deadlineAt` / `age_secs` / `deadline_overdue_secs` / `mode` / `state` | 只用于解释原因与时间窗；`pid` 可能复用，年龄不改变判定 |
| 明确禁止 | attempt 目录缺 `handback.json` / 缺 `dispatch.json`、裸看 `.inflight` 文件是否存在、`pid` 数字、标记年龄 | 与 AW 判定不一致，不得作为判活依据 |

`list` 回执里的 `liveness_rule` 是这条约定的机器可读副本：`authoritative_fields` / `cleanup_fields` / `diagnostic_fields` / `not_liveness_evidence` / `fallback.allowed=false`、`decision.task_live|task_not_live|not_live_does_not_prove|result_authority`。PM 可直接按字段断言，**不要自行复算 `stale`**（阈值只用于审计，判定以 `stale` 为准）。

两条语义边界：

- “无标记/残留标记”只证明**当前没有活跃执行者**，不证明任务未执行、未完成或记录不可用；任务结果仍以 `receipt.get` / `verdict.json` / `standalone-result.json` 为准。
- 仅当 `runs.inflight` 不存在（旧版 AW）时，才允许退回“任一 attempt 缺 handback/dispatch 视为可能活跃”的启发式，且必须在输出里标注为降级判定；新版 AW 禁止该回退。

### 2.9 残留 inflight 标记的清扫（v1.33；v1.42 起含自动触发）

v1.42 起在既有动作路径之外增加自动触发（服务启动一次 + 默认每小时周期，后台线程），五条路径全部复用同一 `sweep_inflight_markers` 与 `stale_rule`：

| 触发点 | 触发条件 | 作用范围 |
|---|---|---|
| `startup` | bridge 进程启动、进入 stdio 循环前（`AW_INFLIGHT_SWEEP_SECS=off` 时关闭） | 全部残标 |
| `periodic` | 默认每 3600 秒（后台线程；`0`=仅启动、`off`=关闭、正整数=周期秒数） | 全部残标 |
| `task_start` | 同一 `task_id` 的标记已存在且 `stale=true` | 只清扫该 `task_id` 的标记；清掉后重新 `create_new` 占位 |
| `runs.prune` | `dry_run=false` 且 `confirm=true` | 枚举候选前清扫全部残标，清完后该 task 不再被 `task_inflight_stale` 跳过；`dry_run` 只报 `would_remove`、零删除 |
| `runs.inflight.clean` | `dry_run=false` 且 `confirm=true` | 全部残标（给了 `task_id` 时只清该条） |

- **判据与永不删除项**：只有 `stale=true` 参与清扫；`pid` 存活（`pid_alive=true`）优先于 `deadline`，永不删除；坏 JSON/读不出的标记保留并在 `unreadable` 报告，不参与清扫。自动路径不另写判据，直接调用同一 `sweep_inflight_markers`。
- **审计**：每次实际清扫写 `runs/inflight-sweep.jsonl`（append-only，`inflight-sweep/1`），先写 `phase=plan` 再删、删完写 `phase=result`；字段含 `trigger`（`startup|periodic|task_start|prune|clean`）、`removed[]` 的 `task_id` / `pid` / `stale_reason` / `path` / `outcome=deleted`、`kept[]` 的 `outcome=lost_race|not_mine`、汇总 `deleted_total` / `lost_race_total` / `not_mine_total` / `removed_total` / `kept_total` / `failed_total` / `unreadable_total`。审计写不进去时**一条都不删**，自动触发同样适用。
- **幂等**：没有可清残标时零删除、不追加审计；重复触发返回 `removed_total=0`，不报错。
- **TOCTOU 不退化为盲删（v1.43）**：与 `clean` 同一实现——先同卷 rename 认领为 `.tmp`，再对认领副本重读并重跑同一份 `stale_rule`：仍 `stale` 才删（`outcome=deleted`）；已不 `stale`（刷新者常只延长 `deadlineAt`，身份三元组不变）按 create-if-absent 还原并记 `outcome=not_mine`（`marker_changed` / `marker_superseded` / `recheck_unreadable`）；认领副本已不存在记 `outcome=lost_race`，不算错误。
- **仍然存在的边界**：跨卷 `EXDEV` 时认领失败、零删除；认领与还原之间原名短暂空缺；审计 append 非事务（崩溃可能缺 `result` 行）；坏标记不自愈，只能人工处理。

`list` 的 `sweep_rule` 是本节机器可读副本：`triggers` / `trigger_conditions` / `automatic`（启动/周期开关与 `AW_INFLIGHT_SWEEP_SECS` 语义）/ `audit` / `audit_gate` / `criteria` / `never_removed`（`active_pid_alive`、`unreadable`）/ `atomic_delete` / `outcome_totals`（`deleted_total`、`lost_race_total`、`not_mine_total`）/ `idempotent` / `limits`（`same_volume_rename_required`、`transient_name_absence_during_claim`、`audit_append_not_transactional`、`unreadable_not_self_healing`）。

### 2.10 `runs/_session/initialize-*.json` 按天保留（v1.47，与任务目录分开）

`runs/_session/initialize-<pid>-<毫秒时间戳>-<seq>.json` 是连接审计，不进入 `runs.prune`（它只枚举任务目录），由独立的 `sweep_session_records` 在 `startup` / `periodic` 档执行，和 `.inflight` 清扫共用触发点但**判据与审计文件都分开**：

| 项 | 规则（写死） |
|---|---|
| 保留天数 | `AW_SESSION_RETENTION_DAYS`，取值 `1..=3650`，默认 **30**；未设置/空白用默认；非法值不删除、只写 stderr（不静默回退） |
| 候选 | 文件名严格匹配 `initialize-<pid>-<ts>-<seq>.json` 且能解析出毫秒时间戳；`initialize.json` 旧格式、名字解析不出、时间戳在未来 → **判不准不删** |
| 双时间一致 | 只有「文件名时间」与文件 `mtime` **都**早于阈值才删；只有一边过期 → 保留并记 `mtime_not_older_than_threshold` |
| 删除前审计 | 先向 `runs/session-retention.jsonl`（append-only，`session-retention/1`）写 `phase=plan`，写不进去则本轮零删除；删完写 `phase=result` |
| 审计字段 | `trigger`（`startup|periodic`）、`retention_days`、`candidates[]`（`file`/`createdAt`/`age_days`）、`removed[]`（含 `outcome=deleted`）、`failed[]`、`kept[]`（仅判不准项，含 `reason`）、`within_retention_total`、`removed_total`/`failed_total`/`kept_total` |
| 幂等 | 没有候选时零删除、不追加审计；重复触发不报错 |
| 不做 | 不删除旧格式 `initialize.json`、非 `initialize-` 前缀文件、`.tmp`、目录、符号链接 |

阈值来源（真实链路 D3 实测，不是拍脑袋）：Windows `bytes_per_record=106.7–106.8`、WSL `107.0`；即每天 1000 次 initialize 约 `104 KiB/天`，30 天约 `3.05/3.06 MiB`。保留 30 天足以覆盖连接审计追溯，同时把长期驻留体积限制在个位数 MiB；需要更长审计窗口时用环境变量放大，代价按上表线性增长。真实证据：D3 删除 45 天前记录、保留 1 天前记录与 `initialize-broken-name.json`，审计 plan/result 各行点名 `deleted` 与 `unparsable_name`。

### 2.11 启动回收上一轮遗留 cgroup 作用域（v1.48，仅 Linux）

bridge 被 `kill -9` 后，它创建的 `aw-mcp-bridge-<bridge_pid>-<seq>` 作用域与其中后代进程会留下。v1.48 在 `auto_sweep("startup")` 里自动回收，**不以「看着像孤儿」为判据**：

| 项 | 规则（写死） |
|---|---|
| 归属记录 | `<runs>/.cgroup-scopes/<scope>.json`（`cgroup-scope/1`），字段 `version`/`scope`/`cgroup`/`bridge_pid`/`bridge_start`/`task_id`/`attempt`/`root_pid`/`root_start`/`createdAt`；只在 `attach` 成功（`CgroupStatus::Active`）后写，作用域目录删除成功才删 sidecar，删不掉就保留 |
| 触发 | `trigger=startup`；`AW_INFLIGHT_SWEEP_SECS=off` 时与自动清扫一起关闭；非 Linux 返回 `state=unsupported` / `reason=platform_has_no_cgroup_v2`，不扫描、不写审计 |
| 证据链（缺一即 keep） | ① sidecar 存在且自洽（`version`/`scope`/`cgroup` 三处一致、`task_id` 合法、`attempt>=1`）；② `runs/<task_id>/attempt-<n>/dispatch.json` 存在；③ owner bridge 已死——`/proc/<bridge_pid>/stat` 的 starttime 与 `bridge_start` 不一致视为 PID 已被复用（上一轮已死），一致则是仍在运行的另一个 bridge，**绝不动** |
| 保守项 | 成员表读不出、`cgroup.events` 的 `populated` 读不出、成员含当前 bridge pid、空成员但 `populated!=0` → 一律 keep |
| 动作 | 空作用域只删目录；非空先写 `phase=plan` 审计（写不进去则本轮零动作，记 `audit_unwritable`），再 `cgroup.kill`（内核不支持则逐成员 SIGKILL、每轮重读 `cgroup.procs`），3s 内轮询到 `procs` 空且 `populated 0` 才删目录并删 sidecar；结果 `reclaimed` / `still_alive` / `remove_failed` / `kill_failed`。`members_after` 读不出写 `null`，不宣称清空 |
| 审计 | `runs/cgroup-reclaim.jsonl`（append-only，`cgroup-reclaim/1`）：**每个扫描到的作用域都写一行**；keep 也写，带 `phase=kept` / `outcome=kept` / `reason`（`owner_record_missing`/`owner_record_invalid`/`attempt_evidence_missing`/`owner_alive`/`owner_unverifiable`/`members_unreadable`/`populated_unreadable`/`self_member`/`populated_inconsistent`/`audit_unwritable`）、`action=none`、`members_before`/`members_after` |
| 不做 | 不扫描非 `aw-mcp-bridge-[a-z0-9-]+` 名字的目录；不做周期回收（只有 `startup`）；不清理不属于任何已知 attempt 的作用域——证据不足只报不删 |

边界：无 cgroup v2 / `/sys/fs/cgroup` 只读 → `unsupported`；内核无 `cgroup.kill` 时逐 pid 终止存在 PID 复用窗口（靠每轮重读成员表收紧，不承诺零窗口）；跨机器不可见（sidecar 与被回收进程必须在同一台机器）。

真实证据（B5，WSL）：bridge `kill -9` 后 `grandchild_survived=True`、作用域与 sidecar 残留；重启后 `grandchild_gone=True`、心跳停止、作用域目录与 sidecar 删除、`attempt-1`/`dispatch.json` 保留；审计 plan/result `trigger=startup`、`members_before=[<pid>]`、`members_after` 为空、`outcome=reclaimed`。负向对照：伪造 `aw-mcp-bridge-decoy-<pid>`（无 sidecar）→ 目录保留且审计 `outcome=kept` / `reason=owner_record_missing`。

### 2.12 跨进程取消请求（v1.48）

`task.cancel` 从「只在发起进程内有效」改为**请求落盘、执行侧心跳读取**，多进程部署下取消不再失效：

| 项 | 规则（写死） |
|---|---|
| 请求文件 | `<runs>/.cancel/<task_id>.json`（`cancel-request/1`），字段 `version`/`task_id`/`requested_by`/`requestedAt`/`reason`/`status`/`target{pid,startedAt}`/`acknowledgedAt`/`handledBy{pid,startedAt}`/`finishedAt`/`handback_status`/**`cleanup_claimed`**/**`cleanup_status`**（v1.51） |
| 三态（必须可区分） | `pending` = 已落盘、执行侧未读到；`acknowledged` = 已读到、正在清理（`acknowledgedAt` 由执行侧写入）；`done` = 收尾完成（带 `finishedAt`、`handback_status`、`cleanup_claimed`、`cleanup_status`）；另 `ignored` = target 与本轮执行不符，不误杀。**推进不保证每一步都被观察到**：执行侧可能在一个 200ms 心跳内从 `pending` 直接到 `done`，`acknowledgedAt` 保持 `null`；PM 不得以「见过 acknowledged」为前提 |
| 绑定 | `target` 来自 `runs/.inflight/<task_id>.json` 的 `pid` + `startedAt`；执行侧只认 `target.pid` 与自己相同且 `target.startedAt` 与 inflight 记录相同，否则写 `ignored` 不动 |
| 写入 | `create_new` 落盘；同 target 复用；上一轮的终态残留先删再建；不可读 → `cancel_request_unreadable`，零动作；原子写（同目录 tmp + rename + fsync）保证跨进程读不到半个文件 |
| 执行侧 | `CancelHeartbeat`：spawn 前 + 执行循环每 200ms 读一次；读到 `pending` → 原子改写为 `acknowledged`（写成功后才请求取消），读到 `acknowledged` → 请求取消；收尾后写 `done` |
| 响应 | `cross_process`、`request_state`（`pending|acknowledged|done|ignored`）、`request_file`、`requested_by`、`requested_at`、`acknowledged_at`、`finished_at`、`handback_status`、`handback`（完整 canceled 回执）、`message`、`cleanup_detail`、**`cleanup_status`**；`cleanup_claimed` 自 v1.51 起等价 `status=canceled && cleanup_status=cleaned`（不再是「只要 canceled 就 true」） |
| 清扫 | `runs/cancel-requests.jsonl`（`cancel-requests/1`）：只删「`pending`/`acknowledged` 且 target pid 已死超过 5s」与「`done`/`ignored` 超过 30 天」；判不准保留；先写 plan 行，写不进则零删除 |

边界：请求文件只在同一 `runs/` 目录内可见（跨机器不可见）；执行侧读不到就停在 `pending`（如实返回，不冒充已清理）；执行结束后标记删除，再次取消只能 `not_running`，且不得覆盖已落盘的 `done` 请求。

真实证据（B6）：两个平台都观测到 `pending → acknowledged → done` 三态与三个时间戳（20ms 轮询）；A 侧返回 `canceled`，`blocked_reason` 前缀 `canceled：`，handback `status=canceled`；再次取消 `not_running` 且 `done` 请求保留。

真实证据（B7，v1.49，更接近真实派单链路）：派单方（验证进程）走真实 `task.execute` 落盘 `runs/<task>/attempt-1/dispatch.json` + `runs/.inflight/<task>.json`；取消方是**独立 OS 进程且不是 MCP 客户端**，只按 `cancel-request/1` 格式直接写 `.cancel/<task>.json`，`requested_by=b7-canceller-pid-<其 pid>`；执行侧 bridge 在自己的 200ms 心跳里读到并收敛。两个平台都验证：`handledBy.pid` = 执行进程 pid、`target{pid,startedAt}` = 活跃标记、`requested_by` 保持取消进程身份、harness 与取消进程各自观测到 `pending/acknowledged/done`（harness 的 pending 窗口随调度可能短于 20ms 轮询）、执行侧 `canceled`、盘上 handback `canceled`、标记删除、心跳停止。整条取消链路只依赖文件，不依赖任何进程内状态或第二条 MCP 连接。

### 2.13 PM 侧接入须知与自测入口（v1.50）

- 面向 PM 的独立接入文档：`docs/PM-INTEGRATION-GUIDE.md`（只依赖公开 MCP 接口与 `runs/` 目录约定，不需要读源码）。
- 「已清理」的唯一判据（v1.51）：MCP 通道为 `task.cancel` 返回 `status=canceled` **且** `cleanup_status=cleaned`（等价 `cleanup_claimed=true`）**且** `handback_status=canceled`；文件通道为 `request.status=done` **且** `handback_status=canceled` **且** `cleanup_status=cleaned`。**特例**：`cleanup_status=not_started` 表示取消在启动前到达、从未启动任何进程，受控集合不存在因而**必然无存活**；PM 可表述为「已取消（未启动进程）、无需清理」，但不要写成「已清理」（`cleanup_claimed=false`，因为未执行终止动作）。其余 `request_state=pending/acknowledged`、`still_alive`、`unavailable`、缺 `cleanup_status` 都只能表述为「请求已送达/正在清理/未验证」，**不得宣布已清理**。这样 PM 可把「确定无存活」（`cleaned` / `not_started`）与「可能仍在」（`still_alive` / `unavailable` / 缺字段）可靠分开。
- 版本识别（v1.51）：契约版本读 `status` 返回的顶层 `contract_version`（当前 `"1.51"`，与本文档标题一致）；MCP 握手 `serverInfo.version` 只是二进制版本（当前 `"0.1.0"`），**不得**用它判断契约版本。PM 可在握手后先调一次 `status` 做机器校验，版本不符时按不兼容处理。
- 自测：`python scripts/pm-integration-selftest/pm_selftest.py --binary <aw-mcp-bridge>`，在同一连接上模拟正常派单 / 超时 / 取消三种 PM 行为（两平台各 40 项断言，全绿退出 0）。

## 3. 幂等

### 3.1 规则

1. `task_id` 是唯一幂等键，格式沿用实现校验：字母或数字开头，仅含字母数字 `. _ -`，长度 ≤ 64。
2. **重投**（`task_id` + `cmd` + `cwd` + `timeout_secs` + `env` + `path_prepend` 全等）必须：
   - 直接返回该 `task_id` 的最新回执；
   - 不启动新进程、不创建新 attempt 目录；
   - `attempt` 与被重投的那次完全一致。
3. 同一 `task_id` 但 `cmd`/`cwd`/`timeout_secs`/`env`/`path_prepend` 任一不同 → **拒绝**（视为契约违规），不得覆盖或追加，必须显式报错并要求新 `task_id`。
4. **重跑**一律使用新 `task_id`（建议后缀 `-r2`、`-r3`，由 PM 生成），不得用“同 id 再调一次”表达重跑。
5. 一个 `task_id` 对应 PM 侧一条派单记录；AW 侧 `attempt > 1` 仅允许出现在：崩溃恢复、人工运维重试、或未来由 PM 显式授权的重试语义（v1 不定义"同 id 重试"输入；默认重跑换新 `task_id`，见 §10 第 2 行）。

### 3.2 派单参数 `env` 的语义（v1.4，v1.7 起 PATH fail-closed）

`task.execute` 可选 `env`：字符串 map。规则逐条写死，不留“按常理”：

| 项 | 规则 |
|---|---|
| 类型 | 可选对象；键与值都必须是字符串，否则 `invalid_env` |
| 缺省 | 不传 `env` = 不覆盖任何变量，只继承 bridge 进程环境 |
| 空 map | `env: {}` 与缺省**完全等价**：执行行为相同，且**签名相同**（同 `task_id` 重投命中同一 attempt）。与 §3.5 的 `path_prepend: []` 口径一致：两个"空集合"都表示"不追加/不覆盖"，都归一到同一签名值 |
| 继承 | 未在 `env` 中列出的变量照常继承父进程（bridge）环境 |
| 覆盖 | `env` 中与父进程同名的键**覆盖**继承值；子进程只看到显式值 |
| 空串值 | `env: {"K": ""}` 表示 K 存在且为空串，**不是 unset**；不提供“删除变量”能力 |
| 键校验 | 拒绝空键、含 `=` 的键、含空字节的键；值必须为字符串且不含空字节 → 错误码 `invalid_env`（工具错误，**占号之前**拒绝，不创建目录） |
| 大小写 | 原样透传；Windows 下变量名不区分大小写，不同大小写的同名写法会互相覆盖 |
| 明文 | `env` 值可能承载密钥：`dispatch.json` 只记 `键 -> sha256:<hex>` 摘要，**不落明文**；回执只列 `env_keys`，**不回显值** |
| 幂等 | `env` 参与签名：同 `task_id` 且 `env` 不同 → `task_id_conflict`，不追加 attempt、不返回旧回执 |
| 不做推断 | 不自动注入任何变量（不注入密钥、代理、PATH 前缀）；只透传调用方给的值 |
| PATH（v1.7 fail-closed） | `env` 中出现 `PATH`（**大小写任意变体**：`PATH` / `Path` / `path`，Windows 下同一变量）**一律拒绝**：错误码 `invalid_env`，reason 必须写明"PATH 不可整体覆盖"；在**占号之前**拒绝——不建目录、不写 `dispatch.json`、不产生回执。**继承的 PATH 永远保留** |
| 不静默降级 | 禁止"看见了 `env.PATH` 但悄悄忽略"：那会让调用方误以为自己设的 PATH 生效。**看见即拒绝**，不降级、不警告后放行 |
| 追加目录 | 若确实需要往 PATH 追加目录，必须用独立显式字段 `path_prepend`（名字在此写死），由 AW 显式拼接；实现与顺序见 §3.5（v1.9 起已落地）；`env.PATH` 永远不承担这件事 |
| 工具描述同步（v1.14） | `tools/list` 中 `env` 的 description 与本节**逐句一致**：PATH 任意大小写变体一律拒绝（`invalid_env`）、继承的 PATH 永远保留、不整体覆盖、不静默忽略；追加目录只走 `path_prepend`。**禁止**描述里出现"PATH 会整体覆盖"这类与 fail-closed 相反的措辞 |
| 旧记录 | v1.7 之前的 `dispatch.json` 里可能已有 `env.PATH` 摘要（旧实现允许整体覆盖）。这类记录的覆盖行为属**已知缺陷，不追认**：只读接口原样返回、不改写不补；但同参重投会因入参 `env.PATH` 被拒而不再回放 |
| 升级兼容 | v1.4 之前的 `dispatch.json` 无 `env` 字段，比对时视为 `env: {}`；旧派单同参重投仍可回放，非空 `env` 仍判冲突 |

### 3.3 与“保留历史”的关系

上一轮实现的“同 id 重跑追加 attempt、历史不覆盖”仍然成立，但它保护的是**执行记录**，不是**传输重投**。v1 已把两者分开：重投命中 `dispatch.json` 签名即回放既有回执，不产生新 attempt；有意重跑必须换新 `task_id`，因此 `attempt > 1` 只可能来自历史数据或人工运维。

### 3.4 派单时间戳 `dispatchedAt`（v1.6）

| 项 | 规则 |
|---|---|
| 写入位置 | `runs/<task_id>/attempt-<N>/dispatch.json`，与签名同文件、同一对象 |
| 语义 | 该 attempt **完成取号、派单记录开始落盘**的时刻；不是 `task.execute` 收到调用的时刻，也不是子进程启动时刻 |
| 格式 | RFC3339，**UTC**，秒级精度，`Z` 结尾（例 `2026-11-06T03:21:07Z`）。不写本地时区偏移，避免跨时区比较歧义 |
| 与幂等签名的关系 | **不属于签名**：比对“同 id 是否同参”时必须**剔除** `dispatchedAt`（与“旧记录补 `env: {}`”并列处理）。它是时间事实而非参数；若参与比对，任何一次重投都会因时间不同被误判为 `task_id_conflict` |
| 失败语义 | 不适用：只要 `dispatch.json` 存在该字段就存在；写入失败即该 attempt 不成立（`task.execute` 返回工具错误，不产生回执） |
| 缺省语义 | v1.6 之前的 `dispatch.json` 无此字段，**只读不补**；消费方按“**缺失即未知**”处理，**不得用 mtime 反推**（见 §8） |

### 3.5 追加 PATH 的显式字段 `path_prepend`（v1.9）

| 项 | 规则（已写死） |
|---|---|
| 取值形态 | **字符串数组** `path_prepend: ["dir1", "dir2"]`；**不接受带分隔符的单字符串**——分隔符由 AW 按平台决定，不允许调用方注入 |
| 顺序语义 | 数组即**结果顺序**（字面顺序前插）：结果 `PATH = 数组[0] + sep + 数组[1] + … + sep + 继承 PATH`。不支持"反向压栈"语义 |
| 拼接规则 | prepend 全部在**前**、继承 PATH 全部在**后**；继承部分**原样保留**：不去重、不做路径规范化、不展开 `~` |
| 分隔符 | **按平台**：Windows `;`、POSIX `:`，不得写死一种。元素内部若含分隔符按普通字符处理（不拆分） |
| 与 `env` 的关系 | `env` 中**仍禁止** `PATH`（v1.7 不变）；`path_prepend` 与 `env` 独立生效，同现时互不影响 |
| 幂等签名 | **参与签名**（它是执行输入的一部分）。后果写死：同 id 改 `path_prepend` → `task_id_conflict`；旧记录无此字段 → 视为 `[]`，同参重投仍可回放 |
| 校验（占号之前） | 非数组 / 元素非字符串 / 空字符串元素 / 含 NUL 元素 / 超过 **16** 个元素 / 单个元素超过 **4096** 字符 → `invalid_path_prepend`，不建目录、不落盘、不产生回执 |
| 空数组（v1.10，选 A） | `path_prepend: []` **与缺省完全等价**：执行相同、**签名相同**、同 id 重投命中同一 attempt。理由：与 §3.2 的 `env: {}` 保持一致——"空集合"表达的是"不追加/不覆盖"，不是错误，避免调用方在两个字段上踩相反的方向 |
| 两字段独立归一 | `env` 与 `path_prepend` **各自**归一到空集合后再比对（`{}` / `[]`），因此"都缺省 / 都为空 / 一空一缺 / 一缺一空"**四种组合签名全等** |
| 目录是否存在 | **不校验**：AW 不假设目标机文件系统布局。不存在的目录原样写进 PATH，子进程找不到可执行时的表现与系统一致（不额外报错、不改写） |
| 回执呈现 | 回执新增 `path_prepend` 字段：**只回目录名列表**（原样、不做规范化），与 `env_keys` 并列；不回任何 env 值 |
| 工具描述同步（v1.14） | `tools/list` 中 `path_prepend` 的 schema 与 description 与本节一致：`type=array` / `items=string`、字面顺序前插、平台分隔符、空数组等价缺省、参与幂等签名、上限 16 项/单项 4096 字符、不校验目录存在性 |

### 3.6 显式取消 `task.cancel`（v1.47 起，v1.48 扩为跨进程）

**入口**：`task.cancel`（`tools/list` 第 9 个工具），入参 `task_id`（必填，与 `task.execute` 同一命名规则）、`reason`（可选字符串，`maxLength=1024`）、`requested_by`（可选字符串，`maxLength=128`，写入请求文件供审计；缺省 `pid:<当前进程>`）。`task_id` 非法 → 工具错误；`reason` 超过 1024 字符 → 工具错误 `invalid_reason`；`requested_by` 为空或超长 → 工具错误 `invalid_requested_by`；以上均**零副作用**（不发信号、不落盘）。

**作用域（v1.48）：本进程持有时走本地取消；不持有时通过 `runs/.cancel/<task_id>.json` 跨进程可见（§2.12）。**

| 项 | 规则 |
|---|---|
| 可见性 | 取消请求落盘到 `runs/.cancel/<task_id>.json`，绑定 `target{pid,startedAt}`（取自 `runs/.inflight`）。执行侧在 spawn 前与执行循环中每 200ms 读一次，只认 target 完全一致的那一轮；target 不符写 `ignored`，不误杀另一个进程的同名任务。请求只在同一 `runs/` 目录内可见，**跨机器不可见** |
| 三态可区分 | `request_state` 独立于四态返回：`pending`（请求已落盘、执行侧未读到）、`acknowledged`（执行侧已读、正在清理，带 `acknowledgedAt`）、`done`（收尾完成，带 `finishedAt` + `handback_status` + `cleanup_status`）、`ignored`（target 不符）。三者混在一起视为未实现；`pending`/`acknowledged` 时 `cleanup_claimed=false`。`acknowledged` 可能被跳过（一个心跳内直接 `done`），不是判据 |
| 终止由谁做 | 取消只**置位**标记（本地 token 或跨进程请求文件），不跨线程杀进程；持有子进程的执行线程在下一轮轮询中看到标记，走与超时**完全相同**的 `Supervisor::terminate` 流程（POSIX 进程组 + cgroup v2 作用域 + 终前名单逐个 SIGKILL；Windows 作业对象 + 后代快照复核），并落盘回执 |
| 启动前取消 | 占号后、`spawn` 前检查到取消 → **不创建子进程**，直接落盘 `canceled` 回执（`exit_code=null`） |
| 状态四态 | `canceled`：回执已落盘且确为 `canceled`；`completed_before_cancel`：取消到达前执行已正常结束（**未终止任何进程**）；`cancel_requested`：请求 `pending`/`acknowledged` 或 10s 内未观测到回执，**不得认为已清理**；`not_running`：磁盘上既无活跃标记也无可识别的执行（标记缺失/过期/target 不符），**未发信号、未做清理** |
| `cleanup_claimed` | v1.51 起等价 `status=canceled && cleanup_status=cleaned`；`still_alive` / `unavailable` / `not_started` / 缺字段一律 `false`，防止把「取消成功」写成「已清理」 |
| `cleanup_status` | canceled / timeout 回执的机器可读终止结果：`cleaned` / `still_alive` / `unavailable` / `not_started` / `null`；与三态措辞同源，不从字符串反推 |
| 等待窗口 | `CANCEL_WAIT_SECS = 10`；有本地 token 时轮询请求文件直到 `done`/`ignored`；无 token 时每 100ms 读请求文件。超时即回 `cancel_requested`，附带 `request_state` 供调用方继续观察 |
| 响应字段 | `cross_process`（是否跨进程生效）、`request_state`、`request_file`、`requested_by`、`requested_at`、`acknowledged_at`、`finished_at`、`handback_status`、`handback`、`message`、`cleanup_detail`、`cleanup_status`；`inflight_marker` 仍是磁盘观测，不代表进程状态 |
| 盘上回执 | dispatched：`status=canceled`（**不是** `blocked` / `completed`），`exit_code=null`，`blocked_reason` 固定前缀 `canceled：`，并包含请求方 `reason`（原文回显，不做推断） |
| 审计可区分 | 三种终止原因在盘上互不混淆：超时 `blocked_reason` 前缀 `timeout：`；取消前缀 `canceled：`；失败 `status=failed` 且 `exit_code≠0`。三者都保留 `finishedAt` / `duration_ms` / 摘要。跨进程请求本身另有 `runs/cancel-requests.jsonl` 清扫审计 |
| 清理口径 | 与超时同一条四观测量（`cgroup.procs` 空、`populated 0`、逃逸 PID 不存在、快照无后代）；只有全部成立才写「已清理受控集合」，否则如实写仍存活/不可用；逃逸进程是否仍在运行**未验证**，措辞不含「已终止进程树 / 无残留」 |
| standalone | 取消当前 attempt，`verdict` 结论记 `canceled`、`action=terminate`，**不重试**（取消不是失败，不消耗 `max_attempts`）；已落盘的此前 attempt 记录不动 |
| 幂等与重跑 | 取消**不改变**幂等签名，也不额外占 attempt；取消后同 `task_id` 同参重投 → 回放既有 `canceled` 回执（要重跑必须换新 `task_id`，与 §3.1 一致）。取消后同 `task_id` 异参重投 → 仍 `task_id_conflict`。执行结束、标记删除后再次取消 → `not_running`，且不覆盖已落盘的 `done` 请求 |
| `runs.prune` | `status` 白名单扩为四值 `ok`/`failed`/`blocked`/`canceled`，`canceled` 只作为过滤条件，不改删除判据 |
| 工具描述同步 | `tools/list` 里 `task.cancel` 的 description 与本节一致；`task.execute` 描述补「执行中可被 `task.cancel` 终止，落盘状态为 `canceled`」。**工具数 8→9**，`tools_sha256` 与门禁同步更新（见 `docs/TOOLS-DESCRIPTION-AUDIT.md` §52） |
| 纯文件链路证据（v1.49） | B7 证明取消不依赖第二条 MCP 连接：取消方是独立进程，只写 `.cancel/<task>.json`；执行侧心跳读到后收敛。三态、target 绑定、`handledBy`、handback `canceled` 均在两平台实测（见 §2.12） |

## 4. 回执

字段一律沿用已实现 handback，不新增第二套命名。

| 字段 | 类型 | 语义 | 当前实现 |
|---|---|---|---|
| `task_id` | string | 派单标识 | 已实现 |
| `attempt` | integer | 该 id 的第几次执行，从 1 起连续 | 已实现（原子取号） |
| `status` | `ok` / `failed` / `blocked` / `canceled` | dispatched：仅描述进程结果，**不是验收结论**（`canceled` 表示被 `task.cancel` 显式终止，见 §3.6）；standalone：映射判定结论（`passed→ok` / `failed→failed` / `undetermined→blocked` / 取消→`canceled`），结论细节以 `verdict.json` 为准 | 已实现（standalone 映射 v1.26，`canceled` v1.47） |
| `verify_cmd` | string | 实际执行的命令（回显派单参数，供 PM 逐字比对判据） | 已实现 |
| `exit_code` | integer / null | 进程退出码；`blocked` / `canceled` 时为 null | 已实现 |
| `changed_files` | string[] | 执行前后 `git status --porcelain` 差集，相对路径，最多 20 条 | 已实现（有 20 条上限） |
| `changed_files_truncated` | boolean | 差集是否超过 20 条而被截断；`true` 表示还有未列出的改动 | 已实现 |
| `cwd` | string | 本次实际执行目录（规范化绝对路径；Windows 下可能带 `\\?\` 前缀），供 PM 复算 `changed_files` / `commit` | 已实现 |
| `timeout_secs` | integer | 本次实际生效的超时秒数（含默认值 120），用于解释 `timeout` 类 `blocked_reason` | 已实现 |
| `env_keys` | string[] | 本次实际覆盖/追加的 env **键名**（不含值），让 PM 知道哪些变量被改写 | 已实现（v1.4） |
| `commit` | string / null | 执行时工作目录的短 HEAD | 已实现 |
| `stdout_digest` | `sha256:<hex>` | stdout 原始日志摘要 | 已实现 |
| `artifacts` | object[] | `{kind, locator, hash}`，locator 为相对路径 `runs/<task_id>/attempt-<N>/stdout.txt` 等 | 已实现 |
| `duration_ms` | integer | 执行耗时（契约要求可审计，字段已存在） | 已实现 |
| `stderr_digest` | `sha256:<hex>` | stderr 原始日志摘要 | 已实现（既有附加字段） |
| `blocked_reason` | string / null | `blocked` 与 `canceled` 时非空，见第 6 节；前缀固定区分来源：`timeout：` / `canceled：` | 已实现（自由文本，未分类；`canceled：` v1.47） |
| `finishedAt` | string | 该 attempt **结束并落盘回执**的时刻，RFC3339/UTC/秒级/`Z`；`ok` / `failed` / `blocked` / `canceled` 一律写实际结束时间。**落盘回执中必填、不为 null**；不落盘的预检 `blocked` 回执不含此字段（它没有 attempt） | 已实现（v1.6） |
| `path_prepend` | string[] | 本次实际前插的目录列表（原样，不做规范化）；未提供时为空数组。只给目录名，不含任何 env 值 | 已实现（v1.9） |
| `mode` | string | **仅 standalone 回执**出现，恒为 `"standalone"`；dispatched 回执不出现该字段 | 已实现（v1.26） |
| `argv` | string[] | **仅 standalone**：本次实际启动的 argv（程序 + 参数） | 已实现（v1.26） |
| `argv_path_scan` | object | **仅 standalone**：`argv[1..]` 的路径预检结果（`scope` / `checked[]` / `unknown[]` / 计数）。越界在占号前 blocked，不出现该字段；dispatched 不出现 | 已实现（v1.28，§2.6.8） |
| `replayed` | bool | **仅 standalone**：`false`=本次调用真正执行；`true`=回放既有结果（重入或并发落败方）。落盘的 `standalone-result.json` / `receipt.get` 恒为 `false`，回放标记只在本次响应出现 | 已实现（v1.29，§2.6.9） |
| `replay_kind` | string | **仅 standalone 且 `replayed=true`**：`existing`=调用时最终结果已落盘；`waited`=调用时胜者仍在执行，等待其落盘后回放 | 已实现（v1.29，§2.6.9） |
| `max_attempts` | integer | **仅 standalone**：本次任务声明的重试上限（默认 3） | 已实现（v1.26） |
| `verdict_conclusion` | `passed` / `failed` / `undetermined` | **仅 standalone**：与 `verdict.json` 的 `verdict` 一致；dispatched 不出现 | 已实现（v1.26） |
| `verdict_locator` | string | **仅 standalone**：相对路径 `runs/<task_id>/attempt-<N>/verdict.json` | 已实现（v1.26） |
| `attempts_total` | integer | **仅 standalone**：终止尝试的编号（即总尝试次数）；中间重试回执不带该字段 | 已实现（v1.26） |
| `replay_basis` | string | **仅 standalone**：`fresh`=该 task 首次被 AW 执行；`rerun_after_records_missing`=`runs/.task-history` 显示此前执行过、但 task 目录已不存在（记录被清理/移动），本次是重跑 | 已实现（v1.30，§2.6.10） |
| `prior_attempts` | integer | **仅 standalone**：台账记录的上次尝试次数；`fresh` 时为 0 | 已实现（v1.30） |
| `prior_last_finished_at` | string / null | **仅 standalone**：台账记录的上次结束时间；`fresh` 时为 null | 已实现（v1.30） |

回执使用规则：

- **dispatched**：PM **不采信 `status` 自述**，只按 `exit_code`、`verify_cmd` 与判据的逐字一致性、`stdout_digest` 与 `artifacts` 中原始日志的重算一致性判定。
- **standalone**：`verdict_conclusion` 是 AW 自己的判定，不是 PM 的验收；调用方仍应复核 `verdict.json` 的 `criteria[].spec/observed` 与哈希。`undetermined` 表示"缺观测、判不了"，**不得被当成通过**。
- dispatched 的 `status=ok` 只表示进程退出码为 0，不表示验收通过；standalone 的 `status` 已映射判定结论，见上一行。
- 历史读取：`receipt.get` 默认返回最新一次；`include_history=true` 返回 `attempts / latest / history[]`，用于回答“执行过几次、每次 exit_code 与耗时”。
- `cwd` / `timeout_secs` / `changed_files_truncated`（v1.3）与 `env_keys`（v1.4）属**纯增量字段**：既有字段语义未变；旧 `handback.json` 不含这些字段，`receipt.get` 原样返回、**不回填、不改写**。
- `dispatchedAt`（派单记录，§3.4）与 `finishedAt`（回执）同样**纯增量**：v1.6 起写入；旧记录缺失，`receipt.get` **原样返回、不回填**，消费方按“**缺失即未知**”处理。
- **年龄判定只认这两个时间戳**：`dispatchedAt` / `finishedAt` 之外的一切信号（mtime、目录顺序、文件名、`attempt` 编号）都不得用于推断年龄或作为清理判据。全文档只有这一套说法，§8 与本节一致。
- **`env_keys` 行为不变**：它只列"本次实际生效的 env 键名"。`env` 被拒（含 `env.PATH`）的派单**不占号、不落盘、不产生回执**，因此不会出现在任何回执或 `runs.list` 中。
- `attempt=null` 的 `blocked` 预检回执（缺命令 / cwd 不存在）不落盘，也不带执行上下文（没有实际执行目录与超时）。
- **`blocked_reason` 只描述"已执行的动作与受控集合的可判定结果"，不描述"整棵树的结果"**：`timeout:` 文案的作用域**写死为"受控集合"**（POSIX 进程组 / Windows Job Object），**不是整棵进程树**。v1.17 起三态与逐字文案：
  - 受控集合已清空：`timeout：超过 {timeout_secs}s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证`；Windows 额外写明 `Windows 下子进程以挂起态创建并先加入作业再恢复执行（恢复前不会派生子进程）；BREAKAWAY 或加入作业失败的作业外进程仍不可观测`（v1.20）；
  - 受控集合内仍有存活进程：`timeout：超过 {timeout_secs}s 未结束，已向受控集合发送终止信号，但受控集合内仍有存活进程；逃逸进程是否仍在运行未验证`；
  - 受控集合未能建立或无法查询：`timeout：超过 {timeout_secs}s 未结束，受控集合未能建立或无法查询，仅向直接子进程发送终止信号；其派生的子进程是否仍在运行未验证`。
  - 判据只有两个可判定观测点：POSIX `kill(-pgid, 0)` 返回 `ESRCH`；Windows Job 的 `ActiveProcesses == 0`。**任何分支都不写"已终止进程树 / 无残留"**。`#3` 整棵进程树杀灭仍不可判定、状态为 `NOT_PROVEN`，调用方不得据此认为超时任务已彻底停止，需自行确认受控集合之外的残留进程（含 POSIX `setsid` 逃逸与 Windows `BREAKAWAY` / 加入作业失败导致的作业外进程）。v1.20 的 Windows 挂起态创建只关闭"恢复执行前派生子进程"这一窗口，**不放宽任何判定条件**，残余逃逸仍不可观测。
  - **终止只尝试一次（v1.24 实测确认）**：POSIX 68 轮（高速 fork / 2GB 进程 16ms fork 窗口 / 400 个已存在子进程）与 Windows 20 轮（成员持续拉起后代）均未出现"一次终止后集合内仍有存活"；能构造出的 `StillAlive` 场景（僵尸、冻结）重试也无效，故不新增第二次尝试。无论是否重试，**只有观测到集合内无存活才写 `Cleaned`**；`StillAlive` / `Unavailable` 不因重试被升级。证据见设计文档 §6.9 与审计文档 §23。
- **历史回执不回填**：旧 `handback.json` 里已有的旧文案（含"已终止进程"这类完成式断言）**原样返回、不推断、不改写**；但那些旧文案**本身不准确**，读到时按上一行的真实语义理解。

## 5. 契约 vs 现有实现

| # | 契约项 | 状态 | 说明 / 缺口所在层（本批不补） |
|---|---|---|---|
| 1 | 回执字段映射（第 4 节） | **一致** | 全部字段已实现，含 `attempt`/`duration_ms`/`artifacts` 哈希 |
| 2 | attempt 编号连续无洞、并发不撞号 | **一致** | `create_dir` 原子占位，并发取号已在实现中 |
| 3 | 边界（不验收、不写 L0/事件、不成为写权威） | **一致** | bridge 无任何写权威接口，只落本地 `runs/` |
| 4 | `blocked_reason` 分类（第 6 节） | **一致** | `command_not_found` / `cwd_not_found` / `timeout` 均为 `blocked` 回执 + 前缀码；前两者在占号之前判定 |
| 5 | `clientInfo` 审计记录（第 2 节） | **一致（仅审计）** | `initialize` 落盘 `runs/_session/initialize.json`，`status.session` 只读回显；不参与任何准入 |
| 6 | 重投幂等（第 3 节） | **一致** | 同 `task_id` + 同签名命中 `dispatch.json` 即回放既有 `handback.json`，零执行、零新目录 |
| 7 | 同 id 异参拒绝 | **一致** | 签名不一致返回 `task_id_conflict`，不追加 attempt、不覆盖旧回执 |
| 8 | 超并发上限、上游拒单 | **缺（不在 AW）** | AW 无并发闸门；上游拒单属 PM 侧，AW 不产生该码 |
| 9 | `runs.list` 只读枚举（第 7 节） | **一致** | 零写副作用、排除 `_` 前缀目录、`task_id` 升序 + 确定分页、只回 `env` 键名不回值；明确非执行状态权威 |
| 10 | 证据清理 `runs.prune`（第 8 节） | **一致** | 显式触发、`dry_run` 默认零写入、`confirm` 才真删、阈值必填；只删非最新 attempt 的整目录；`runs/prune.jsonl` 先记账后删除；部分失败如实报 `partial` |
| 11 | 证据时间戳 `dispatchedAt` / `finishedAt`（§3.4 / 第 4 节） | **一致** | v1.6 起随派单与回执落盘（UTC/秒级）；不参与幂等签名；旧记录不回填、缺失即未知 |
| 12 | `env.PATH` fail-closed（§3.2） | **一致** | 大小写任意变体一律 `invalid_env`，占号前拒绝；继承 PATH 永远保留；不静默忽略 |
| 13 | `runs.list` 的 `total` 与 `attempts` 一致性（第 7 节） | **一致** | `total == Σ attempts`（attempt 目录数，非任务数）；分页不改变 `total`；新增 `total_tasks` 供换算页数 |
| 14 | `path_prepend`（§3.5） | **一致** | 目录数组前插、继承 PATH 在后、分隔符按平台；参与幂等签名；占号前校验；回执只给目录名 |
| 15 | `timeout` 文案诚实性（第 4 / 6 节） | **一致** | v1.17：文案只描述**受控集合**（POSIX 进程组 / Windows Job）的清理结果，三态分流（已清理 / 仍存活 / 不可用），明写"逃逸进程是否仍在运行未验证"；v1.20 Windows 改写为"挂起态创建、先入作业再恢复执行"，残余逃逸为 `BREAKAWAY` / 加入作业失败；v1.24 实测确认单次终止足够并明写"终止只尝试一次（不重试）"；任何分支都不出现"已终止进程树 / 无残留"；整树 `#3` 仍 `NOT_PROVEN`；证据见设计文档 §6.5 / §6.9 与审计文档 §23 |
| 16 | 默认策略与 schema 声明（§10） | **一致** | `additionalProperties` 仍为声明性忽略（新增用例锁定）；`runs.prune` 推荐 365 天、`limit` 缺省 100，但 `older_than_days` 缺省仍 `invalid_retention`；`clientInfo` 仅审计、不参与准入；`path_prepend` 的 16 项 / 4096 字符已写入 schema 且实现同源 |
| 17 | `runs.prune.status` 状态过滤（§8.10） | **一致** | v1.21：schema 声明 `array`/`minItems=1`/`uniqueItems`/`enum`；显式才生效、缺省不过滤；与年龄 AND、`limit` 后置；空数组/null/未知值 `invalid_retention` 零删除零审计；`status_unknown`/`status_mismatch` 不删；plan/result 记录 `status_filter`；23 条用例实跑通过（Windows 6 组、POSIX 同套减去 Windows-only 的 case 20） |
| 18 | 运行模式与独立判定（§2.6） | **一致** | v1.26：`mode` 显式（缺省 dispatched）；standalone 要求 `cwd`/`argv`/`acceptance`、拒绝 `cmd`/`env`/`path_prepend`；四类验收标准；`verdict.json` 三态落盘（`passed`/`failed`/`undetermined`），`undetermined` 绝不默认通过；`max_attempts` 1..5 有界重试、逐 attempt 独立记录；跨模式复用 `mode_conflict`；dispatched 回执零新增字段。用例 8 条先红后绿（Windows 7 条 standalone + 1 条 dispatched 回归锁），integration 91 全绿；真实 stdio 双单证据见审计 §25 |
| 19 | 能力/限制机器可读（§2.7） | **一致** | v1.32：`capabilities` 逐条给出 `id`/`holds`/`status`/`scope`/`reason_code`/`platforms_verified`/`platforms_unverified`/`automatic`/`evidence`；`holds=false` 必带非空原因码；`#3` 整树/逃逸枚举、POSIX 覆盖、TOCTOU 均为具名条目；v1.42 起自动清扫 `inflight.stale_markers_auto_swept` 上调为 `true/proven`，`not_proven_total=3` 可核对（v1.38 起 POSIX rename 闭幕后 5→4，v1.42 自动清扫闭幕后 4→3） |
| 20 | 残留标记可观察可清（§2.7） | **一致** | v1.32：`runs.inflight action=list` 给出 `stale`/`stale_reason`/`pid_alive`/时间窗与 `stale_rule`；`clean` 默认 dry-run、真删需 `confirm`；只删 `stale=true`，pid 存活的活跃标记永不删除；删除前原子改名并复核 `pid+startedAt`；坏 JSON 保留记 `unreadable`；不进入 `runs.list`/`runs.prune` 候选 |

统计：**已一致 19 条、缺 1 条**（第 8 条按分工归 PM 侧，AW 不做；认证不在 v1 范围，默认按 §10 第 1 行处理，准入所需字段见 §2.3）。

### 5.1 缺口补齐规划（后续批次）

| 缺口 | 补齐层 | 只管什么 | 状态 |
|---|---|---|---|
| G1 重投幂等 | `mcp-bridge` 的 `tools/call` 入口，先查同 id+同参的签名，命中即回放 | 零执行、零新 attempt | **已落地** |
| G2 同 id 异参拒绝 | 同上，比对签名后返回 `task_id_conflict` | 不改 schema | **已落地** |
| G3 失败码分类 | 参数校验层：`command_not_found`/`cwd_not_found` 转为 `blocked` + 前缀码；`timeout` 复用 | 只约定 `blocked_reason` 前缀，不加字段 | **已落地** |
| G4 身份记录 | MCP `initialize` 会话层，记录 `clientInfo` 供审计 | **仅审计，不做准入**；记录不得被放行逻辑读取；每次 initialize 一份文件，`status.session` 聚合 | **已落地（仅审计；v1.13 起逐份留存、并发不丢）** |
| G5 并发上限 | PM 侧排队产生（AW 不内置闸门） | 拒单时不占用 attempt | 归 PM 侧 |
| G6 上游拒单 | PM 侧，不进入 AW | AW 不感知 | 归 PM 侧 |
| G7 证据清理 | `runs.prune` 显式入口（默认 dry-run、无自动清理） | 只按 `finishedAt` 判龄、只删非最新 attempt 整目录、先记账后删除 | **已落地（v1.12）**；自动清理**明确不做**（§8.8）；v1.19 按 PM 口径统一推荐 365 天、`limit` 缺省 100、先 dry-run、不分档（§8.9） |
| G8 `env.PATH` fail-closed | `task.execute` 参数校验层（占号之前） | `env` 中出现 `PATH`（任意大小写）一律 `invalid_env`；不静默忽略 | **已落地（v1.7）** |
| G9 超时文案诚实 | `task.execute` 回执生成处 | 文案只写**受控集合**（POSIX 进程组 / Windows Job）的清理结果与可判定判据（`kill(-pgid,0)==ESRCH` / `ActiveProcesses==0`），逃逸进程标"未验证"；不写"已终止进程树 / 无残留" | **已落地（v1.11 文案诚实；v1.16 受控集合实现；v1.17 新措辞启用；v1.20 Windows 窗口收窄）**：POSIX 四类用例实跑通过（3 unit + 64 integration）、Windows 四类通过（4 unit + 65 integration）；Windows 逃逸负例仍 `UNPROVEN`；整树杀灭仍不可判定，`#3` 保持 `NOT_PROVEN`；证据见 [`TIMEOUT-PROCESS-TREE-DESIGN.md`](./TIMEOUT-PROCESS-TREE-DESIGN.md) §6.5 |

## 6. `blocked` 失败码分类

分类以 `blocked_reason` 的**前缀**表达（如 `timeout: 超过 120s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证`），不新增字段。**前缀码后面的文案描述对受控集合已执行的动作与可判定结果，作用域不是整棵进程树**。

| 分类 | 产生方 | 触发条件 | 调用方（PM）处置 |
|---|---|---|---|
| `command_not_found` | AW | 命令不存在、不可执行，或缺少 `cmd`（占号之前判定，不产生 attempt） | 修正 `cmd` 或环境；**不得原样重投**，修正后换新 `task_id` 重派 |
| `cwd_not_found` | AW | `cwd` 不存在或不是目录（占号之前判定，不产生 attempt） | 修正工作目录/仓库路径；换新 `task_id` 重派 |
| `timeout` | AW | 超过 `timeout_secs` 未结束；对**受控集合**（POSIX 进程组 / Windows Job）发送 `SIGTERM → SIGKILL` / `TerminateJobObject`，并按 `kill(-pgid, 0) == ESRCH` / `ActiveProcesses == 0` 判定是否清空；逃逸进程未验证（整树 `#3` 仍 `NOT_PROVEN`） | 评估拆小任务或延长超时；**换新 `task_id` + 新参数**，不得假设原任务可续跑；**需自行确认受控集合之外的残留进程**（含 POSIX `setsid` 逃逸与 Windows `BREAKAWAY` / 加入作业失败导致的作业外进程） |
| `process_resume_failed` | AW | Windows 下子进程已挂起创建，但加入作业后无法恢复主线程（Toolhelp/OpenThread/ResumeThread 失败）；AW 终止该进程，**命令从未执行** | 视为环境故障；换新 `task_id` 重派；**不得把该回执当作命令结果** |
| `upstream_rejected` | PM | PM 在派单前拒绝（判据不成立、权限不足、依赖未就绪） | 不派单；记录 blocker；条件解除后再派 |
| `concurrency_limit` | AW（未来） | AW 达到并发上限，**未占号、未执行** | 可安全地以**同 `task_id` 重投**（幂等），按退避重试 |

约束：

- 本节只列 `blocked` 回执。参数非法属**工具错误**（`isError=true`、不落盘、不占号），具名错误码：`task_id_conflict`（同 id 异参）、`invalid_env`（env 键/值非法，**或出现 `PATH` 整体覆盖**）、`invalid_path_prepend`（`path_prepend` 形态/元素/数量/长度非法，**空数组除外**——`[]` 与缺省等价）、`invalid_pagination`（`runs.list` 的 `offset`/`limit` 类型或范围越界）、`timeout_secs` 越界（无前缀，沿用既有报错）。
- `blocked` 必须带 `blocked_reason`，禁止空字符串；空原因视为契约违规。
- `blocked` 的 `exit_code` 为 null；`status` 只描述进程结果，不构成验收。
- 未被识别的 `blocked_reason` 前缀按最严格策略处理：不自动重试，由 PM 人工裁定。

### 6.1 刻意不做：按退出码细分 `command_not_found`（候选 #6）

**结论：不做，本文件长期记录该决定，避免以后被重新提起。**

现状（已核实）：

- `command_not_found:` 只在两种**确定性**场合产生：缺 `cmd`（占号之前），以及 `spawn` 失败（占号之后，例如可执行文件不存在）。
- 命令拼错通常不是 `spawn` 失败，而是 `sh -c` / `cmd /C` 正常启动后返回 127 / 9009 之类的退出码，当前如实记为 `failed` + 原始 `exit_code`。

不按退出码改名的理由：

1. **不可移植**：127/126 是 POSIX shell 约定而非保证；Windows `cmd` 的"找不到命令"码不同；`sh`、`cmd`、直接 spawn 三条路径不一致。
2. **会误判真实失败**：子进程完全可能把 127 当作业务返回码；改名会把"真实业务失败"误标成"命令不存在"，把 PM 引向"改命令"而不是"修问题"。
3. **破坏回执一致性**：§6 已写死 `blocked` 时 `exit_code` 为 null；把"进程已跑完并返回 127"改判成 `blocked` 会造成 `status` 与 `exit_code` 自相矛盾，且 PM 的处置动作（"修正后换新 `task_id`"）会丢掉真实失败证据。

判据：分类只依据**确定性事实**（命令压根没启动），不做启发式猜测。**误判比不判更糟。**

## 7. `runs.list`（只读枚举，非权威；v1.5，v1.8 定死 `total` 口径）

`runs.list` 只读枚举 `runs/` 目录，让 PM 看到“本机有哪些派单痕迹”，不新增任何写路径。

| 项 | 规则 |
|---|---|
| 副作用 | **零写副作用**：不创建目录、不落盘、不改任何文件；runs 目录不存在时也不会创建 |
| 状态区分 | `state`：`missing`（目录不存在）/ `empty`（目录存在但无任务）/ `ready`（有任务） |
| 范围 | 仅列 `task_id` 合法（字母或数字开头、长度 ≤64）的目录；`_session` 等 `_` 开头预留目录一律排除 |
| 排序 | 按 `task_id` 升序，与目录创建顺序无关；同一状态多次调用结果逐字段一致 |
| 分页 | `offset` 默认 0；`limit` 默认 20、范围 1..=100；类型/范围越界 → `invalid_pagination`（工具错误） |
| 条目 | `task_id` / `attempts`（**真实存在的 `attempt-<N>` 目录数**）/ `latest_attempt`（编号最大值，**不等于** `attempts`）/ `handback`（最新 attempt 是否可解析回执）/ `status` / `exit_code` / `duration_ms` / `blocked_reason` / `env_keys` |
| `attempts` 计数口径（v1.8） | 按**目录个数**计，不按最大编号计，也**不看 `handback.json`**：目录存在即"占号成立"，回执缺失或损坏只让 `handback=false`，不影响计数。编号不连续（有 1、3 无 2）时 `attempts=2`、`latest_attempt=3`，不推断缺失编号、不补目录、不视为异常 |
| `total` 口径（v1.8） | **不是任务数**，而是**所有匹配任务的 attempt 目录数总和**：`total == Σ 条目.attempts`（对全体匹配任务，不限于当前页）。恒等式是本接口的硬约束，不成立即视为契约违规 |
| `total` 与分页 | **分页不改变 `total`**：`offset`/`limit` 只影响 `tasks[]`，`total` 在同一次查询的各页之间恒定 |
| `total_tasks`（新增，v1.8） | 匹配的 task 目录数，用于换算页数；恒等式不涉及该字段。`state` 的 `empty`/`ready` 由 `total_tasks` 判定，不由 `total` 判定 |
| 边界：目录存在但回执缺失或损坏 | **计入** `attempts` 与 `total`（占号已成立）；该条 `handback=false`，`status`/`exit_code`/`duration_ms`/`blocked_reason` 为 null；`latest_attempt` 仍指向编号最大的目录 |
| 边界：编号不连续（1、3 无 2） | 计入 **2**（按目录数），`latest_attempt=3`；不推断、不补、不报错 |
| 边界：派单被拒（占号前拒绝，如 `invalid_env` / `invalid_pagination`） | **不产生 attempt 目录 → 不计入任何条目与 `total`**；若 task 目录也未创建，该 task 不出现在列表里 |
| 边界：task 目录存在但 0 个 attempt 目录 | 该 task **仍出现**（记录已建立），`attempts=0`、`latest_attempt=null`、`handback=false`，对 `total` 贡献 0 |
| 敏感信息 | **不回 env 值**：只给 `env_keys`（`dispatch.json` 中的键名），不返回明文，也不返回摘要以外的值 |
| 权威性 | **目录枚举结果不是执行状态的权威来源**：它只镜像磁盘现状，可能滞后或被人工改动；PM 判定某次派单必须用 `receipt.get`，不得以 `runs.list` 的 `status` 作为验收依据。第 8 节的清理已提供显式入口（`runs.prune`）；**被清理的 attempt 是"不出现"，不等于"从未派单"**；`receipt.get` 的"回执不存在"同样只表示本机当前无证据 |

## 8. 证据清理 `runs.prune`（v1.12：显式触发、默认不删、两段式）

**方向写死：默认什么都不删。** 清理一旦写错就是销毁证据，比不清理严重得多，因此本节每条规则都按"宁可不删"取向。**对任务 attempt 不存在任何自动清理**：没有定时任务、没有启动时扫描、没有"超过 N 天自动删除"；唯一入口是调用方显式调用 `runs.prune`。唯一的按时间自动回收对象是连接审计 `runs/_session/initialize-*.json`（不是任务证据），口径单列在 §2.10，且不进入本节候选。

### 8.0 前置事实（已核实）

- 证据只存在于**已占号**的 attempt：`runs/<task_id>/attempt-<N>/`（`dispatch.json`、`handback.json`、`stdout.txt`、`stderr.txt`）。占号前就被拒的 `blocked`（缺 `cmd`、`cwd` 不存在、参数非法）**不落盘**，没有证据可删（见 §4 末条）。
- **时间戳（v1.6 起已落地）**：`dispatch.json` 有 `dispatchedAt`、`handback.json` 有 `finishedAt`（均 RFC3339/UTC/秒级，见 §3.4 与第 4 节）。**v1.6 之前的历史记录没有这两个字段**，只能按"年龄未知"处理（不补、不猜）。
- **mtime 不是判据**：可被复制、解压、备份恢复、`touch` 改写；Windows 上目录 mtime 语义与文件不同。删除判据只认 `finishedAt`，不认 mtime，也不认目录顺序、文件名或 `attempt` 编号（与 §4 的"年龄判定只认这两个时间戳"是同一句话）。
- `task.execute` 自身不追加 attempt（重投走 `replay_existing` 回放）；`attempt-N`（N>1）目前只可能来自手工运维或未来的"显式重试"特性。因此实践中被清理的候选很少，这是预期的。

### 8.1 触发方式：只有显式调用（写死）

| 项 | 规则 |
|---|---|
| 唯一入口 | `runs.prune` 工具（针对任务 attempt）。**不提供任何其他删除任务证据的入口**：无命令行、无启动清理、无定时任务，`runs.list` 绝不隐式删除 |
| 默认行为 | 不调用就永不删除；调用时 `dry_run` 缺省为 `true` |
| 自动清理 | 任务 attempt 上**不存在**：不读策略文件、不在启动时扫描、不按天数自动删。若要自动，必须另立契约重新评审。`runs/_session/initialize-*.json` 是例外且只限连接审计：v1.47 起按天数单独回收（§2.10），它不读 `prune.jsonl`、不影响本节任何删除判据 |
| 删除对象 | 只可能是 `runs/<task_id>/attempt-<N>/` **整个目录**，且 `N` 严格小于该 task 的最大 attempt 编号 |
| 排除 | `_` 前缀目录（`_session` 等）、非任务目录、符号链接、`runs/` 根下任何文件一律不进入候选；不删任务目录之外的任何路径。**`runs/_session/initialize-*.json` 不在本节范围内**（不进入候选、也不被本节删除），它由独立的时间保留策略负责，见 §2.10：按天数、单独审计 `runs/session-retention.jsonl`、判不准不删 |

### 8.2 两段式：先 dry-run，再显式 confirm

| 项 | 规则 |
|---|---|
| `dry_run` | 布尔，缺省 `true`。`true` 时**零写入**：不删目录、不创建/追加 `prune.jsonl`、不建目录、不改任何文件 |
| `confirm` | 布尔，缺省 `false`。**只有 `dry_run=false` 且 `confirm=true` 才真删**；缺一即拒绝 |
| 拒绝语义 | `dry_run=false` 而 `confirm` 不为 `true` → 工具错误 `confirm_required`，**零写入**；反之 `confirm=true` 但未显式 `dry_run=false` → 仍按 dry-run 处理（`confirm` 不隐含真删） |
| 阈值必填 | `older_than_days` 必填（整数，1..=3650）；缺失或越界 → 工具错误 `invalid_retention`，**零写入**。**不提供隐式默认阈值**——"没写阈值"绝不能变成"按某个默认值删"；调用方按 §8.9 的 365 天推荐值显式传入 |
| dry-run 输出（稳定可解析） | `dry_run=true`、`state`（`missing`/`empty`/`ready`）、`older_than_days`、`status_filter`（缺省 `null`，显式时为去重后的数组）、`exclude_filter`（缺省 `null`）、`evaluated_at`（UTC）、`limit`、`candidates_total`（过滤后命中数，含被排除项）、`batch_candidates_total`、`not_processed_total`、`truncated`、`candidates[]`、`skipped_total`、`skipped[]`、`exclude_requests_total`、`excluded_total`、`not_in_candidates_total`、`excluded[]`；`candidates[]` 每项固定字段：`task_id` / `attempt` / `path`（相对路径 `runs/<task_id>/attempt-<N>`）/ `status`（回执状态，缺省 `null`）/ `dispatchedAt` / `finishedAt` / `age_days`；`excluded[]` 见 §8.11 |
| `skipped[]` 原因码 | `latest_attempt_retained`（最新 attempt，永不删）/ `unfinished_or_missing_handback`（无回执或不可解析）/ `missing_dispatch`（缺派单签名，年龄与一致性都无法证明）/ `age_unknown`（回执无 `finishedAt`）/ `invalid_finished_at`（时间戳不可解析）/ `future_timestamp`（晚于当前时刻，按时钟异常跳过）/ `not_old_enough`（未达阈值）/ `unsafe_attempt_path`（符号链接或非真实目录）/ `status_unknown`（传了 `status` 但回执缺状态或值不认识，宁可不删）/ `status_mismatch`（状态不在白名单） |
| 单批上限 | `limit` 缺省 100、范围 1..=100；**在 `status` 过滤与 `exclude` 排除之后**按 `(task_id, attempt)` 升序取前 `limit` 个；`candidates_total` 是过滤后的命中总数（**含被排除项**），`not_processed_total` 是因 `limit` 截断未处理的数量，`not_processed_total > 0` 时 `truncated=true`。**真实删除只删被列出的这一批**，剩余的下一次调用再评估 |
| 两段一致性 | 相同参数、磁盘未变时，dry-run 的 `candidates[]` 与真实删除的 `deleted[]` **逐项一致**（同样的排序、同样的排除与同样的 `limit` 截断）；`excluded[]` 也逐项一致；不一致即契约违规 |

### 8.3 年龄判据：只认 `finishedAt`

- 只认 `handback.json` 的 `finishedAt`。缺回执 → `unfinished_or_missing_handback`；回执缺字段 → `age_unknown`；两者都**不删**。
- 只有 `dispatch.json` 的 `dispatchedAt`、没有可解析回执的 attempt **不删**：未完成 attempt 是"执行发生过但没结果"的唯一线索，`dispatchedAt` 只用于审计记账，不用于放行删除。
- 时间戳晚于当前时刻（时钟回拨或数据被改）→ `future_timestamp`，跳过。
- 年龄按秒级时间戳计算：整日差 `age_days >= older_than_days` 才可能成为候选。

### 8.4 硬性保留（写死）

| 规则 | 内容 |
|---|---|
| 每个 task 至少保留最新 attempt | **最新 attempt（编号最大的目录）永不进入候选**，无论多老。因此删除单位是 `attempt-N`（N < 最新编号），不是整个 task 目录 |
| 只有一次 attempt 的 task | **永远删不掉**。这是有意的：那条记录既是回执又是唯一证据，删掉它等于让同 `task_id` 重投重新执行（见 §8.7） |
| 整条链一致 | 只删**整个 attempt 目录**：先把目录整体移到同卷的 `runs/.prune-trash/<batch_id>/`（重命名一次完成，整链一起离开 attempt 命名空间），再删除该暂存目录。**归属以改名后的目的目录是否存在为准，不以 rename 返回值为准**（Windows 实测并发 rename 同一源可能都返回成功、只有一个目的目录产生）。源已不存在或目的目录不存在 → `lost_race`（另一方先赢，本进程零删除）；其他改名错误 → 原样不动。**禁止**选择性删文件（例如只删 `dispatch.json` 留下孤立 attempt）。任一必要文件缺失或不可解析 → 该 attempt 不进入候选 |
| 删除后的可见变化 | 保留最新 attempt，`runs.list` 的 `attempts` 相应减少，`latest_attempt` 不变；`receipt.get` 仍可读回最新回执 |

### 8.5 删除留痕（先记账，后删除）

- 落点：`runs/prune.jsonl`，append-only JSONL；`runs.list` 只列目录，因此该文件不会出现在任务列表里。
- **真删（`dry_run=false` + `confirm=true`）且候选非空时**，每批追加两行，共享 `batch_id`：
  1. `phase="plan"`：**删除之前**写入并 `fsync`。字段：`at`（UTC）、`batch_id`、`older_than_days`、`candidate_count`、`candidates[]`（`task_id` / `attempt` / `path` / `dispatchedAt` / `finishedAt` / `age_days`）、`exclude_filter`、`exclude_requests_total`、`excluded_total`、`not_in_candidates_total`、`excluded[]`。
  2. `phase="result"`：删除结束后写入。字段：`at`、`batch_id`、`older_than_days`、`status_filter`、`exclude_filter`、`limit`、`deleted[]`、`failed[]`、`trashed[]`、`lost_race[]`、`deleted_total`、`failed_total`、`trashed_total`、`lost_race_total`、`exclude_requests_total`、`excluded_total`、`not_in_candidates_total`、`batch_candidates_total`、`excluded[]`。
- **删除回执（v1.22，纯加法；旧字段全部保留、不改名、不删除）**：
  - `deleted[]`：**唯一**表示"已从 `runs/` 移除"的集合。每条除保留 `task_id`/`attempt`/`path`/`status`/`dispatchedAt`/`finishedAt`/`age_days` 外，新增 `state="deleted"`、`source_path`、`trash_path`、`deleted_at`、`rule`、`fingerprint`。
  - `failed[]`：**仍在原路径**的失败项，`state="retained_at_source"`、`trash_path=null`，同样带 `source_path`/`error`/`rule`/`fingerprint`。
  - `trashed[]`：**已整链移入 `.prune-trash` 但未删掉**的失败项，`state="trashed_not_deleted"`、`trash_path` 指向暂存路径，同样带 `rule`/`fingerprint`。
  - `lost_race[]`（v1.44）：**另一方先赢、本进程零删除**的竞态项，`state="lost_race"`、`trash_path=null`，带 `task_id`/`attempt`/`source_path`/`error`/`rule`/`fingerprint`。触发条件：同一 attempt 的源目录已被另一认领者改名带走（rename 报 `NotFound`），或 rename 返回成功但目的目录不存在（Windows 并发改名只产生一个目的目录），或目的目录在删除前被另一方清走。**不计入 `failed_total`，不把 `status` 拉成 `partial`**（只有 `lost_race` 时仍为 `ok`、`isError=false`），但必须逐条可见、可核对是哪个 attempt。
  - `rule` 逐条自包含：`older_than_days` / `status_filter` / `limit` / `batch_id` 与批次一致，跨批次核对不回连顶层。
  - `fingerprint`：`status` / `finishedAt` / `dispatchedAt` / `handback_sha256` / `dispatch_sha256` / `bytes{handback,dispatch,total}`；sha256 与字节数均在**删除前**读出计算；文件缺失写 `null`（不编造）。
  - 语义由代码保证：`failed[]`/`trashed[]` 的 `state` 只能来自 `RetainedState`（`retained_at_source` / `trashed_not_deleted`），`lost_race[]` 的 `state` 固定为 `lost_race`，构造不出 `deleted`；`.prune-trash` 里的一切都不算删除；四集合互斥，`deleted_total + failed_total + trashed_total + lost_race_total = 本批候选数 - excluded_total`（v1.44 写死为 `deleted_total + failed_total + trashed_total + lost_race_total + excluded_total == batch_candidates_total`，见 §8.11）。`status` 只看 `failed_total + trashed_total`，`lost_race` 不参与。
- **先记账后删除**：`plan` 行写不进去（目录不可写、磁盘满）→ **放弃本轮删除**，返回工具错误，一个目录都不删。禁止"删完再记"。
- 崩溃可对账：只有 `plan` 没有 `result` = "已记录、未完成"，人工按清单核对；两行都在 = 可回答"删了什么、凭什么时候删的"。
- `dry_run=true` 或候选为空时**不写审计**（没有发生删除，也就不该产生删除记录）。
- 审计行**不含 env 值、不含 stdout/stderr 正文**，只含相对路径与时间戳（沿用 v1.4 口径）。
- 审计文件自身不被清理：`runs.prune` 只处理 `attempt-*` 目录，不删根下文件，也不删 `_` 前缀目录。
- 暂存目录 `runs/.prune-trash/`（以 `.` 开头、非合法 `task_id`）**不会出现在 `runs.list`**；删除失败留下的整链暂存**不自动清理**，由人工核对后手工移除（宁可不删）。

### 8.6 失败语义（不许报成功）

| 项 | 规则 |
|---|---|
| 部分失败 | 某个 attempt 的暂存重命名或删除失败 → 继续处理其余候选，返回 `status="partial"`、`deleted_total`、`failed_total`、`trashed_total` 与逐项错误；工具响应 `isError=true`。失败项要么**原样未动**（`state="retained_at_source"`），要么**整链留在 `.prune-trash/`**（`state="trashed_not_deleted"`，数据未丢）。**不许把"删了一半"报成成功**，也不许把两种未删除状态报成 `deleted` |
| 竞态失手（`lost_race`） | 同一 attempt 已被另一方先认领/删除 → 逐条记 `lost_race[]`、计入 `lost_race_total`，**不计入 `failed_total`、不把 `status` 拉成 `partial`**（只有 `lost_race` 时仍为 `ok`、`isError=false`）。它不是本进程的删除，也不是失败；禁止吞掉不记，禁止并进 `failed[]`/`trashed[]`/`deleted[]`。与清扫侧 `lost_race` 同词同义：另一方先赢、本进程零删除（区别只在对象：清扫侧是 `.inflight` 标记认领副本，prune 侧是 attempt 目录） |
| 不重试 | **不自动重试**任何删除；失败项只能由调用方下一次显式调用重新评估 |
| 记账失败 | `plan` 行写失败 → 不删；`result` 行写失败 → 删除已发生，返回工具错误并明写"已删除 N 个、失败 M 个、暂存 K 个、竞态失手 L 个，结果行写入失败"，不得声称成功 |
| 非删除类错误 | 参数非法（`invalid_retention` / `invalid_exclusion` / `confirm_required`）在**任何写入之前**拒绝 |

### 8.7 与权威性的关系（对 §7 的补充）

- `runs.list` 的 `state=missing|empty` 与 `receipt.get` 的"回执不存在"，只表示**本机当前没有证据**，**不表示"从未派单"或"未执行过"**；被清理的 attempt 是"不出现"，不是"不存在"。
- 判定某次派单是否执行过、结果如何，权威顺序是：**PM 自己的任务账本 > 未清理的 `receipt.get` > 审计日志（`prune.jsonl`）> `runs.list`**。
- 清理的副作用（写死）：删掉非最新 attempt **不影响**同 id 重投——`task.execute` 只按最新 attempt 的 `dispatch.json` 比对。但若最新 attempt 被人为删除，同 id 重投会重新执行；**已被清理过的 task 不得再当作可安全重投的键**。

### 8.8 明确不做

- 不做定时/启动自动清理，不读策略文件，不做"超过 N 天自动删"。
- 不做按结果分档的内置阈值：`runs.prune` 只接受调用方给定的统一 `older_than_days`，不按 `ok`/`failed`/`blocked` 自动分档（历史方案见下表，**均未进入代码**）。
- 不做"按总大小/按条数只留最近 N 个"：磁盘告急时会先删最新（最可能仍被引用）的证据。
- 不做回收站/软删除/恢复接口：删除不可恢复，故必须先 dry-run。
- 不清理 `runs/` 根下文件、不清理 `_` 前缀目录、不清理最新 attempt、不清理任务目录之外的任何路径。

历史方案对比（保留决策依据，不代表实现）：

| 方案 | 结论 |
|---|---|
| A 统一期限 | 长期可追溯性最差（成功/失败/超时证据同寿）；但 PM 口径已定为**统一 365 天**（§8.9），v1.19 按此执行 |
| B 按结果分档 | `ok=30` / `failed=180` / `blocked=365` 仍为**分档需求、未实现、需另立契约**；v1.21 只实现**状态多值白名单**（`status`），不含档位到阈值的映射，也不默认生效 |
| C 按引用分档 | 自指（引用表本身成为新证据），且把只读接口变成影响清理结果的状态源，违背 §7 零写原则；不采用 |

### 8.9 默认保留包络（v1.19 校正；v1.21 增加可选状态白名单，仍不自动删）

取值只在 PM 包络内（显式触发、默认不删、只删非最新 attempt、只认 `finishedAt`、缺字段不删；§8.1-§8.7），**不放宽任何删除条件**；下面的默认值只决定"什么时候可以开始删"，不改变"删什么/怎么删"。

| 项 | 默认值 | 依据 |
|---|---|---|
| 统一阈值 | `older_than_days = 365`（由调用方显式传入） | PM 已定口径：统一 365 天；覆盖验收/审计窗口且偏保守，远低于 3650 上限 |
| 批量上限 | `limit = 100`（可覆盖，1..=100） | PM 已定口径：单批最多 100 个候选；取上限减少多次调用，同时保留分批能力 |
| 操作顺序 | 先 `dry_run=true` 预览，再 `dry_run=false` + `confirm=true` 真删；默认 `dry_run=true` | PM 已定口径"必须先 dry-run"；服务端默认不删，真删需显式两段式确认 |
| 是否隐式默认 | **否**：`older_than_days` 缺省仍 `invalid_retention` | "没写阈值"绝不能变成"按默认值删"（§8.3）；365 是推荐值，不是隐式默认 |
| 状态过滤 / 分档 | **`status` 可选多值白名单，显式传入才生效，缺省不过滤；分档仍不做**（PM 口径：不分档） | v1.21 §8.10：无默认状态、无服务端档位；`ok=30` / `failed=180` / `blocked=365` 仍为未实现的历史参考，PM 若要分档须自行展开为 `status` + 显式阈值 |
| `task_id` 退役 | **任一 attempt 被清理过的 `task_id` 不得复用**；重投必须换新 id | §8.7：最新 attempt 可能已被删，复用会重新执行并混淆历史 |

可配置：每次调用显式给 `older_than_days`（1..=3650）、`limit`（1..=100）、`dry_run`、`confirm`；可选 `status`（多值白名单，见 §8.10）、`exclude`（按条目精确排除，见 §8.11）。本节的 365 天只是**推荐值**，不是 API 隐式默认。

### 8.10 状态过滤（v1.21 已实现；分档仍未实现）

**参数**：`status` 可选 `string[]`，元素只能取 `ok` / `failed` / `blocked`（大小写敏感）。语义是**白名单**：只有 `handback.status` 精确等于数组内某值才进入候选。

**必须显式**：`status` 缺省 = 不过滤，候选集合与 v1.20 完全一致；**不存在默认状态、不存在服务端档位**。`ok=30` / `failed=180` / `blocked=365` 仍是"未实现、需另立契约"的分档需求，AW 不提供 `tier` 参数；PM 若要分档，只能自己展开为 `status` + 显式 `older_than_days`。

**组合语义**：`status` 与 `older_than_days` 为 **AND**；`limit` 在过滤之后生效（`candidates_total` 是过滤后的命中总数）；最新 attempt 永不进入候选；`status` 不能替代 `older_than_days`。

**校验（fail-closed）**：非数组、空数组、`null`、含非字符串、含未知值 → 工具错误 `invalid_retention`，**零删除、零审计**；重复值先去重再执行，`status_filter` 回显去重结果。

**缺失/未知状态**：传入 `status` 时，回执缺 `status` 或值不认识 → `skipped` 记 `status_unknown`，**永不进入候选**（宁可不删）；不在白名单 → `status_mismatch`。不传 `status` 时保持 v1.20 兼容行为（不做状态校验）。

**审计**：`runs/prune.jsonl` 的 plan 与 result 两行都必须记录 `status_filter`、`older_than_days`、`limit`、`batch_id`；真删响应回显 `status_filter`，dry-run 响应回显 `status_filter`（缺省 `null`）。

**能力门禁**：`tools/list` 的 `runs.prune.inputSchema.properties.status` 是唯一能力位。PM 只有在 schema 含 `status` 时才允许携带 `status` 并 `dry_run=false + confirm=true`；旧服务端会忽略未知参数，**不得**把"旧服务端忽略 `status`"当成可用降级路径。

### 8.11 按条目精确排除（v1.27 已实现）

**为什么要这个能力位**：PM 分档真删时，同一档里只要有一条"不可自动判定"的条目，整批调用就会把它一起删掉；没有排除参数时，PM 只能把整档所有可判定条目一起暂缓。`exclude` 让 PM 在同一次批量调用里精确留下指定条目，而不是被迫拆成多批或整档放弃。

**能力位**：`tools/list` 的 `runs.prune.inputSchema.properties.exclude` 是否存在，是 PM 判断"能否走批量路径"的唯一能力位。没有该字段的服务端会忽略未知参数并把被标记条目一起删掉，因此 PM **不得**在旧服务端上携带 `exclude` 真删；旧服务端只能整档暂缓。

**参数**：`exclude` 可选 `array`，最小 1 条、最多 100 条，schema 声明 `uniqueItems: true`；每项 `{ "task_id": string, "attempt"?: integer >= 1 }`：

- `task_id` 必填，必须通过与任务目录**同一条**命名校验（`is_task_dir_name`：仅字母数字与 `.` `_` `-`，长度 ≤64，且不以 `_` 或 `.` 开头）。含路径分隔符、`..`、绝对路径、通配符一律 `invalid_exclusion`，**零删除、零审计**；不得借这个参数读取或影响枚举范围之外的任何路径。
- `attempt` 缺省 = 排除该 task 的**全部候选**；给出时只排除该次 attempt。
- 去重约束：同一 `(task_id, attempt)` 不得重复；同一 `task_id` **不得**同时按整任务与单次 attempt 排除（歧义形态直接拒绝）。违者 `invalid_exclusion`。
- 服务端不依赖 schema 的 `uniqueItems`：重复/歧义在解析阶段自行拒绝（fail-closed）。

**生效位置（写死）**：排除在**候选枚举入口**一次完成，不是删除前过滤。命中排除的合格条目只进 `excluded[]`，**不进** `candidates[]`、不占 `limit`、不会被删除；`candidates_total` 仍把它们计入候选总数，因此"候选计数"与"删除口径"一致。

**回执字段（纯加法）**：

| 字段 | 含义 |
|---|---|
| `exclude_filter` | 回显归一化后的排除请求；未传时 `null` |
| `excluded[]` | 逐条 `{task_id, attempt, path, reason}`；`attempt` 缺省为 `null`；`path` 只在 `excluded_by_request` 时给真实相对路径 |
| `excluded_total` | `reason=excluded_by_request` 的条数（从候选集中被请求排除） |
| `not_in_candidates_total` | `reason=not_in_candidates` 的条数 |
| `exclude_requests_total` | 本次排除请求总条数（= `excluded_total + not_in_candidates_total`，可直接核对） |
| `lost_race_total` | 被另一方先认领/删除的条数（本进程零删除，不计入 `failed_total`，不改变 `status`） |
| `batch_candidates_total` | 本批候选总数 = `deleted_total + failed_total + trashed_total + lost_race_total + excluded_total` |
| `not_processed_total` | 因 `limit` 截断未处理的本批候选数 |

**计数不变式（回执可直接核对）**：

1. `deleted_total + failed_total + trashed_total + lost_race_total + excluded_total == batch_candidates_total`
2. `candidates_total == batch_candidates_total + not_processed_total`
3. `status = partial` 当且仅当 `failed_total + trashed_total > 0`；`lost_race_total` 不参与

**边界（择一，写死）**：排除**不存在**或**本就不在候选集**的 id（任务不存在、attempt 不存在、该 attempt 因年龄/状态/最新 attempt/缺 `finishedAt` 等原因未入选）**不报错**，逐条记 `reason="not_in_candidates"`，`path=null`（不编造不存在的路径），`isError=false`，且不影响其余候选正常删除。不得静默忽略、不得计入 `excluded_total`。

**审计**：`runs/prune.jsonl` 的 plan 与 result 两行都记录 `exclude_filter`、`exclude_requests_total`、`excluded_total`、`not_in_candidates_total`、`excluded[]`；result 行另记 `batch_candidates_total`。

**回归**：不传 `exclude` 时，候选集合、删除行为、`candidates_total` / `truncated` 口径与 v1.26 完全一致；新增字段取零值 / 空数组 / `null`。

## 9. 边界（写死，按模式）

**dispatched（PM 派单）：AW 只执行，不验收、不判停、不定义判据、不写 L0/事件；PM 是唯一写权威。**

**standalone（无外部派单方）：AW 自己闭环——按任务声明的 `acceptance` 判定、落盘 `verdict.json`、失败有界重试；但 AW 仍不写 L0/事件、不成为项目状态的写权威，判定结论只存在于 AW 自己的 attempt 目录。**

由此派生：

- 两种模式都：AW 不写项目状态、任务、审批，不改 TK 的 L0/L1/L2，不发 `[task]/[delivery]/[verification]/[blocker]` 标记。
- dispatched：AW 不判定 `ok` 是否代表“完成”，也不提供“验收通过/退回/升级”结论。
- standalone：AW 只对**任务输入里声明的**验收标准给出 `passed/failed/undetermined`；不发明标准、不把 `undetermined` 当通过、不宣称 `verified`。
- 两种模式都：AW 不派单给其他 AI 工具，不做第二个协调者。
- PM 验收后按 TK 既有事件 schema 记账；AW 仅提供回执与（standalone 时的）判定证据。

## 10. 默认决策（v1.18 起；v1.19 校正 prune 默认包络）

2026-11-06 起，原先的开放项按下列默认值/默认假设执行；PM 可在派单侧显式覆盖，标注"必须显式"者除外。AW 不再因等待外部拍板而停摆。

| # | 项 | 默认 | 依据 | 可配置方式 | 若强制改变会打断什么 |
|---|---|---|---|---|---|
| 1 | 通道归属 | PM 直接以 stdio 子进程拉起 bridge；进程归属即事实准入边界 | §2：谁拉起谁是用调用方；bridge 无独立凭据 | 启动方式由 PM 决定；若改由 MH 拉起，MH 必须提供 peer 归属信息（新契约） | 直接把 bridge 挪到 MH 下而不提供归属信息，身份退回弱识别（现状），不会阻断执行 |
| 2 | 重跑表达 | 换新 `task_id`（建议 `-r2`/`-r3`），零新增字段 | §3.1：同 id 是幂等键，不是重跑键 | PM 生成新 id | 若改为"同 id 重试"，需扩展输入契约并定义 attempt 语义 |
| 3 | 失败码归属 | `upstream_rejected` 由 PM 记录；`concurrency_limit` 由 PM 侧排队，AW 不内置闸门 | §5 第 8 条、§6：AW 不做第二个协调者 | PM 侧策略；若 AW 内置限流需新契约 | 在 AW 加闸门会引入"未执行却被计为派单"的语义风险 |
| 4 | 证据清理 | 统一 `older_than_days=365`、`limit=100` 为推荐调用值，**older 仍须显式传入**；先 dry-run 预览再真删；不分档；可选 `status` 多值白名单**显式才生效**；可选 `exclude` 按条目精确排除（v1.27，`task_id` + 可选 `attempt`，枚举入口生效）；清理过的 `task_id` 退役 | §8.9、§8.10、§8.11、§8.7、§8.8 | 每次调用显式传 `older_than_days`/`limit`/`dry_run`/`confirm`，可选 `status`/`exclude` | 若把 365 变成隐式默认，会让"漏传阈值"变成静默删除；若给 `status` 默认值，会让"漏传状态"变成按档位静默删；若用排除参数做"删除前过滤"，会让候选计数与删除口径不一致 |
| 5 | `additionalProperties` | 保持**忽略未知参数**（schema 仍声明 `false`）；v1.25 起在回执按需追加 `arguments_diagnostics`（§2.5），**可观测但不拒绝** | 兼容现有调用方；MCP 客户端可能附加元数据/别名；schema 静态、行为需确定 | 契约版本级；不提供运行时开关 | 强制拒绝会打断携带额外字段的现有调用（§10.1） |
| 6 | `clientInfo` 准入 | 默认**不参与准入**，仅审计 | §2.3：自报字段不可信 | 需新增票据/进程锚点/授权范围（§2.3） | 仅凭 `clientInfo.name` 放行会让冒充直接通过 |
| 7 | `path_prepend` 限制 | 默认**写入 schema**：`maxItems=16`、`items.maxLength=4096` | 与实现校验上限一致；声明先于调用，减少无效派单 | 调用方传 0..16 项、每项 ≤4096；上限是契约常量，改动需升版 | 不写入 schema 会让客户端以为可传超量，直到占号前才被拒 |
| 8 | `task.execute.mode` | 缺省 `dispatched`（PM 派单，行为零变化）；`standalone` **必须显式传入** | §2.6：不靠推断；同一 `task_id` 不得跨模式复用 | 调用方显式传 `mode`；`dispatched` 可省略 | 若让 standalone 靠"有没有 argv"推断，会把 PM 派单与 AW 自判混在同一目录；若给 dispatched 加 verdict 字段，PM 侧契约与消费方会被迫改动 |

### 10.1 `additionalProperties=false` 为什么默认仍是"只声明、不强制"

现状：六个工具的 schema 都声明 `additionalProperties: false`，但 `tools/call` 只读取已知键，未知键被忽略。v1.18 决定**保持忽略**，理由：

1. **兼容**：现有调用方可能带 `_meta`、SDK 序列化的 null 可选字段、camelCase 别名（`taskId`/`dryRun`）或自定义元数据；今天这些调用能正常执行，强制拒绝会把它们直接变成错误。
2. **行为确定**：MCP 工具 schema 是静态声明；若用环境变量切换强制/宽松，同一请求在不同 bridge 进程行为不同，回执与审计难以复现。
3. **声明与强制分离**：schema 的 `false` 已给客户端提示，服务端不据此拒绝；这是 v1.14 起的既有边界，不是本次新增。

仓库内目前**没有 PM 侧实际调用样本**；上面列的是按 MCP 客户端通用行为推导的"会被打断的类别"，不是声称已观测到这些调用存在。一旦 PM 侧出现真实调用样本，按其 payload 复核后再议是否强制。

v1.25 在不改变接受/拒绝的前提下把"忽略"变成**可观测**：多余参数、camelCase/snake_case 别名与显式 `null` 的可选字段都会出现在回执的 `arguments_diagnostics` 中（§2.5），调用结果不变；这回应的是"参数名拼错还是客户端多塞了东西"的排查需要，不是强制模式。

若未来要**全局**强制：必须再升契约版本，新增具名工具错误 `unknown_argument`（列出未知键），并提供兼容模式（默认宽松）与迁移说明。**默认不这么做。** v1.31 提供的是**单次请求显式 opt-in** 的严格模式（§10.2），用于需要"多余参数即失败"的调用方；默认宽松、schema 与 `tools_sha256` 均未改变。

### 10.2 v1.31：单次调用可选的严格模式（默认不变）

复核结论：v1.25 之后 `additionalProperties:false` **仍只声明不强制**；默认行为、schema、`tools_sha256` 均不变（证据见 `docs/TOOLS-DESCRIPTION-AUDIT.md` §31）。

调用方可在**单次** `tools/call` 的 `arguments._meta.strict_arguments = true` 上显式要求按 schema 校验：

- **触发**：`kind=unknown`（拼错/多余）或 `kind=alias`（camelCase 别名）→ 不执行，返回 `isError=true`。
- **回执**：`code="invalid_arguments"`；`error` 以 `invalid_arguments:` 开头并列出"未声明参数 X；已声明参数：..."；同时附 `unknown_arguments[]`、`declared_arguments[]`、`alias_suggestions[]`（如 `dryRun -> dry_run`）与固定 `hint`。只回显参数名，不回显值。
- **不拒绝**：`_meta` 自身（`client_metadata`）、显式 `null` 的可选字段（`null_optional`）、缺省的可选参数。
- **不自动改名**：别名只给建议；要执行请改用已声明参数名。
- **默认**（不带 `_meta.strict_arguments`）仍是 v1.18/v1.25 的忽略 + `arguments_diagnostics`；**不提供环境变量/进程级开关**（理由见 §10.1 第 2 条）。
- **PM 只需看**：回执 `code` 或 `error` 前缀是否为 `invalid_arguments`；若是，按 `declared_arguments` / `alias_suggestions` 修正参数后重试即可，无需读源码。

## 11. 变更记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1 | 2026-11-05 | 首版：身份、幂等、回执映射、失败码分类、边界、实现对照 |
| v1.1 | 2026-11-05 | 落地 G1 / G2 / G3：重投按 `dispatch.json` 签名回放、异参返回 `task_id_conflict`、失败码转 `blocked` 前缀；身份与并发上限未动 |
| v1.2 | 2026-11-06 | 落地 G4：`initialize` 的 `clientInfo` 落盘 `runs/_session/initialize.json` 并由 `status.session` 只读回显；措辞由“身份”改为“审计弱识别”，通道假设不变 |
| v1.3 | 2026-11-06 | 回执补充 `cwd` / `timeout_secs` / `changed_files_truncated`（纯增量，旧记录不回填）；修复 20 条截断静默问题 |
| v1.4 | 2026-11-06 | `task.execute` 增加可选 `env` 透传（覆盖/继承/空串/键校验/不落明文/参与幂等签名）；回执增加 `env_keys` |
| v1.5 | 2026-11-06 | 新增 `runs.list` 只读枚举（零写、排除 `_session` 等预留目录、`task_id` 升序 + 确定分页、`missing`/`empty`/`ready` 三态、只回 `env` 键名）；明确「目录枚举结果不是执行状态的权威来源」 |
| v1.5.1 | 2026-11-06 | **仅文档、未实现代码**：新增 §8 证据保留与清理方案（默认关闭、按结果分档、先记账后删除、清理后"不出现≠不存在"）；新增 §6.1 记录「按退出码细分 `command_not_found`」**刻意不做**及其理由；`runs.list` 权威性行补充清理口径 |
| v1.6 | 2026-11-06 | 派单记录补 `dispatchedAt`、回执补 `finishedAt`（RFC3339/UTC/秒级，纯增量、旧记录不回填、不参与幂等签名）；**清理仍未实现**，年龄判据只认这两个字段、不认 mtime |
| v1.7 | 2026-11-06 | `env.PATH` 改为 fail-closed：大小写任意变体一律 `invalid_env`，占号前拒绝、不落盘；继承 PATH 永远保留，不再允许整体覆盖；`path_prepend` 仅作预留命名（未实现）；旧记录中的 `env.PATH` 覆盖行为记为已知缺陷、不追认 |
| v1.8 | 2026-11-06 | `runs.list` 定死计数口径：`attempts` = 真实 attempt 目录数（不看回执、不按最大编号），`total` = **Σ attempts**（非任务数）且分页无关，新增 `total_tasks`；三种边界（回执缺失/损坏、编号不连续、占号前拒单）各有结论 |
| v1.9 | 2026-11-06 | 实现 `path_prepend`（§3.5）：字符串数组按字面顺序前插、继承 PATH 在后、分隔符按平台（Windows `;` / POSIX `:`）；参与幂等签名；空元素/NUL/超 16 个/单项超 4096 字符 → `invalid_path_prepend`（占号前拒绝；**空数组在 v1.10 改为与缺省等价**）；目录存在性不校验；回执新增 `path_prepend` 目录列表 |
| v1.10 | 2026-11-06 | 消除空集合口径不一致（选 A）：`path_prepend: []` 改为**与缺省完全等价**（执行相同、签名相同、重投命中同一 attempt），与 §3.2 的 `env: {}` 一致；两字段各自独立归一，"都缺省/都为空/一空一缺/一缺一空"四种组合签名全等 |
| v1.11 | 2026-11-06 | 修正超时文案的诚实性：`blocked_reason` 从"已终止进程"改为"已向直接子进程发送终止信号；其派生的子进程是否仍在运行未验证"；契约明写文案只代表已执行动作、不代表进程树已终止，与 `#3` 的 `NOT_PROVEN` 对齐；历史旧文案原样返回、标注为不准确 |
| v1.12 | 2026-11-06 | 实现证据清理 `runs.prune`（§8）：**只有显式调用、无任何自动清理**；`dry_run` 默认零写入、`dry_run=false + confirm=true` 才真删；年龄只认 `finishedAt`（缺字段一律不删）；**最新 attempt 永不进入候选**（单 attempt 的 task 永远删不掉）；只删整个 `attempt-<N>` 目录；`runs/prune.jsonl` 先记账（`plan`+`result` 两行）后删除；部分失败报 `partial` 且 `isError=true`，不自动重试；不删 `_` 前缀目录与任务外路径；本批**未删除任何既有证据** |
| v1.13 | 2026-11-06 | 修正 `initialize` 审计的多进程覆盖缺陷：落盘从单文件覆盖改为**每次一份 `_session/initialize-<pid>-<毫秒>-<序号>.json`**（tmp + rename 原子可见），并发/先后写入都不丢；`status.session` 聚合全部记录（含旧版 `initialize.json`），新增 `total` 与 `records[]`（每条带 `source` 文件名）；**边界不放宽**：多份记录不等于多会话、不构成身份、不参与准入；旧记录只读聚合、不回填 |
| v1.14 | 2026-11-06 | **仅修工具描述与契约措辞，不改任何行为**：`tools/list` 的 `env` 描述从"PATH 会整体覆盖"改为 v1.7 的 fail-closed 口径（任意大小写一律拒绝、继承 PATH 永远保留、不静默忽略）；schema 补上已实现的 `path_prepend`（数组、字面顺序、平台分隔符、空数组等价缺省、参与幂等签名、16 项/4096 字符上限、不校验目录存在性）；`status` 描述补"initialize 记录可能多份、按 receivedAt 聚合"；§3.1 的签名口径补上 `path_prepend`（与 §3.5 单一措辞）、§3.2/§3.5 各加"工具描述同步"行；全量对照见 `docs/TOOLS-DESCRIPTION-AUDIT.md` |
| v1.15 | 2026-11-06 | **仅文档、未实现代码、未改用例**：新增 `docs/TIMEOUT-PROCESS-TREE-DESIGN.md`，评估 `#3` 超时进程树杀灭。写清当前事实（`child.kill()` 只作用于直接子进程；上一轮实测超时返回后进程仍存活、由测试脚本强制清理——单次观测）；否定"两次 `stat` 无变化"的坏判据（会假通过）；给出两个平台的可判定窄判据（POSIX `kill(-pgid, 0)` 为 `ESRCH`、Windows Job `ActiveProcesses == 0`，均只覆盖受控集合）；说明"整棵树"在当前环境**仍不可判定**（逃逸后代无可靠观测点），因此 **`#3` 保持 `NOT_PROVEN`**；依赖事实：`windows-sys 0.61.2` / `libc 0.2.186` 已在 lock、仅 dev 传递，转正常依赖不新增包与版本 |
| v1.16 | 2026-11-06 | **按 PM 接受的窄定义实现 `#3`（受控集合，不是整棵树）**：POSIX 用 `process_group(0)` 在 exec 前建组、超时 `kill(-pgid, SIGTERM → SIGKILL)`；Windows 用 Job Object + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，**采用 W2d（接受 `spawn → Assign` 竞态窗口，未走 W2c）**，窗口已在回执文案与 `TIMEOUT-PROCESS-TREE-DESIGN.md` 明写；判据只用 `kill(-pgid, 0) == ESRCH` 与 `ActiveProcesses == 0`（查询失败按不可用，绝不当 0），**不使用超时后 `stat` 比较**；超时文案三态分流（`已清理受控集合；逃逸进程是否仍在运行未验证` / 仍存活 / 不可用），任何分支都不写"已终止进程树 / 无残留"；旧回执原样返回、不改写；依赖仅将已锁定的 `windows-sys 0.61.2` / `libc 0.2.186` 转为 target-specific 正常依赖（**不新增包、不改版本**）；**整棵树仍不可判定，`#3` 保持 `NOT_PROVEN`**（v1.16.1 已将超时文案退回旧版） |
| v1.16.1 | 2026-11-06 | **只退文案，保留实现与窄定义**：因状态门未满足（POSIX 四类用例未实跑、Windows 逃逸负例 `UNPROVEN`），`timeout_reason` 退回契约 v1.11 的旧文案"已向直接子进程发送终止信号；其派生的子进程是否仍在运行未验证"；受控集合清理实现与判据不动；用例改为断言旧文案且**禁止**出现"已清理受控集合 / 已终止进程树 / 无残留"；契约 §4 / §5 / §6 与设计文档同步标注"新措辞待门禁满足后启用"；`#3` 仍 `NOT_PROVEN` |
| v1.17 | 2026-11-06 | **状态门满足，启用窄定义文案**：POSIX 四类用例**实跑**通过（3 unit + 61 integration，WSL Ubuntu 24.04 / Rust 1.88.0 / 真实进程，证据见设计文档 §6.5：`group_empty=true`、`escaped_alive=true original_group_empty=true`、`kill(-pgid,0)_exists=true` → SIGKILL 后 `group_empty=true`）；Windows 三类通过（3 unit + 62 integration，`ActiveProcesses_before=Some(1)` → `terminate=Cleaned` → `ActiveProcesses_after=Some(0)`）；`timeout_reason` 恢复三态（已清理受控集合 / 仍存活 / 不可用），Windows 额外明写 `spawn→Assign` 窗口；**Windows 逃逸负例仍 `UNPROVEN`**；`#3` 整棵树仍不可判定，**保持 `NOT_PROVEN`** |
| v1.18 | 2026-11-06 | **把"待 PM 确认"转为默认决策（§10），不改执行/超时行为**：`additionalProperties` 保持声明性忽略（新增用例锁定；强制拒绝会打断携带 `_meta`/别名/额外字段的现有调用，见 §10.1）；`runs.prune` 给出统一推荐默认（该值已在 v1.19 按 PM 口径校正为 365 天，见下行；仍须显式传入，缺省仍 `invalid_retention`），分档参考值 `ok=30` / `failed=180` / `blocked=365` 仅入契约、**未实现且不作为推荐**（§8.9）；`clientInfo` 默认不参与准入，并列出参与所需字段（§2.3）；`path_prepend` 的 `maxItems=16` / `items.maxLength=4096` 写入 schema（与实现校验同源）；**`#3` 仍 `NOT_PROVEN`，W2c 不重开，回执仍只写已观测状态** |
| v1.19 | 2026-11-06 | **按 PM 口径校正 `runs.prune` 默认包络（不改任何删除条件）**：推荐阈值校正为 **365 天**（不再使用上一版的临时值），`limit` 缺省 **20→100**；明确"先 `dry_run=true` 预览，再 `dry_run=false` + `confirm=true` 真删"；**不分档**；`ok=30` / `failed=180` / `blocked=365` 保留为"未实现、需另立契约"的历史参考，**不作为推荐**；`older_than_days` 仍必填、缺省仍 `invalid_retention`；其余默认决策（§10）不变；**`#3` 仍 `NOT_PROVEN`，W2c 不重开** |
| v1.20 | 2026-11-06 | **在 W2d 基础上继续收窄 Windows `spawn → Assign` 窗口（落地 W2a，不改判据、不重开 W2c）**：子进程以 `CREATE_SUSPENDED` 创建，先 `AssignProcessToJobObject` 再恢复主线程，恢复前不执行任何代码，因此“窗口内派生作业外后代”对后代不再成立；恢复失败时终止进程并记 `process_resume_failed`，不把“从未执行”当成命令结果；Windows 回执与 `tools/list` 描述同步改为“挂起态创建、先入作业再恢复执行”，残余逃逸为 `BREAKAWAY` / 加入作业失败 / 嵌套作业；判据（`ActiveProcesses == 0`、`kill(-pgid,0)==ESRCH`）与“逃逸未验证”口径**不放宽**；**`#3` 仍 `NOT_PROVEN`**。证据：Windows 新增 `suspended_start_defers_execution_until_job_assign` 实跑（先红后绿，`marker_before_assign=false`、`ActiveProcesses_before_terminate=Some(2)`、`terminate=Cleaned`、`after=Some(0)`），POSIX 3 unit + 64 integration 复跑通过 |
| v1.21 | 2026-11-06 | **实现 `runs.prune.status` 状态多值白名单（按 `RUNS-PRUNE-STATUS-TIER-PROPOSAL.md`，§8.10）**：`status` 为可选字符串数组，元素只能是 `ok` / `failed` / `blocked`，**显式传入才生效**，缺省 = 不过滤、候选集合与 v1.20 完全一致；与 `older_than_days` 为 **AND**，`limit` 在状态过滤**之后**生效；空数组 / `null` / 非数组 / 未知值一律 `invalid_retention`（占号前拒绝，**零删除、零审计落盘**）；**无默认状态、无服务端档位**（不存在 `tier` 参数，`tier` 与其他未知键仍按 `additionalProperties:false` 声明性忽略）；最新 attempt 永不删、两段式确认（`dry_run` 缺省 / `confirm`）与"先写 `runs/prune.jsonl` 再删除"不变；传入 `status` 时，回执缺 `status` 或值不认识 → `skipped: status_unknown`（不删），不在白名单 → `skipped: status_mismatch`（不删）；`tools/list` 的 `runs.prune` 新增 `status` schema（`array` / `minItems=1` / `uniqueItems` / `items.enum`），**PM 必须先确认 schema 含 `status` 才可携带并真删**（旧服务端会静默忽略 `status` 从而多删）；plan / result / audit 行均记录 `status_filter` / `older_than_days` / `limit` / `batch_id`；证据：提案附 23 条用例先红后绿实跑（Windows 6 组全绿，POSIX 同套减去 Windows-only 的 case 20）；**分档（`ok=30` / `failed=180` / `blocked=365`）仍未实现**，`#3` 仍 `NOT_PROVEN` |
| v1.22 | 2026-11-06 | **删除回执自包含可核对（按 §19 建议形状，纯加法：旧字段全部保留、不改名、不删除）**：`deleted[]` 每条新增 `state="deleted"`、`source_path`、`trash_path`、`deleted_at`、`rule`（`older_than_days`/`status_filter`/`limit`/`batch_id`，逐条自包含）、`fingerprint`（`status`/`finishedAt`/`dispatchedAt`/`handback_sha256`/`dispatch_sha256`/`bytes{handback,dispatch,total}`，sha256 与字节数在**删除前**算出，缺文件写 `null`）；`failed[]` 新增 `source_path`/`state="retained_at_source"`/`trash_path=null`/`rule`/`fingerprint`；新增 `trashed[]`（`state="trashed_not_deleted"`，整链已移入 `.prune-trash` 但未删掉）与 `trashed_total`；三集合互斥，`deleted+failed+trashed = 候选数`；**语义由代码保证**：`failed[]`/`trashed[]` 的 `state` 只能来自 `RetainedState` 枚举（构造不出 `deleted`），`.prune-trash` 里的一切都不算删除；删除后重跑无幽灵候选的行为不变。证据：`prune_receipt_deleted_entries_carry_fingerprint_and_rule` 先红后绿（删前 sha256/字节数与回执逐字比对）、Windows partial 用例扩断言（`retained_at_source` + `trash_path=null` + 无 `deleted` 字样）、单元用例锁定 `RetainedState` 与缺文件 `null`；Windows 6 unit + 74 integration、POSIX 5 unit + 72 integration 全绿；**分档仍未实现**，`#3` 仍 `NOT_PROVEN` |
| v1.23 | 2026-11-06 | **把"多份 `initialize` 不等于多会话"变成可观测口径（只观测、不拦截）**：实测真实 stdio 进程连续两次 `initialize` 均成功，`status.connections` 为 `active=1`/`total=1`/`peak=1`/`initialize_total=2`；结论为**进程内无会话概念，只有一条连接**，故字段明确按**连接级**命名（`unit="connection"`、`scope="process"`），不写"会话数"；新增 `connections` 为纯加法输出，准入/上限仍不做，`clientInfo` 仍不参与准入；描述门禁新增 G11 行 + 真实 stdio 行为用例，改描述或改计数都过闸。证据：新用例先红后绿（去掉 `initialize_total` 累加 → 实测量 0≠2），Windows 9 unit + 78 integration、POSIX 8 unit + 76 integration 全绿；**分档仍未实现**，`#3` 仍 `NOT_PROVEN`，W2c 未重开 |
| v1.24 | 2026-11-07 | **把超时的"单次终止尝试"做实（实测结论：一次即足够，不加重试）**：POSIX 68 轮（高速 fork 40 + 2GB 进程 16ms fork 窗口 20 + 400 个已存在子进程 8）与 Windows 20 轮（成员持续 `start /b` 拉起后代）均未出现"一次 `kill(-pgid,SIGKILL)` / 一次 `TerminateJobObject` 后集合内仍有存活"；Linux `copy_process` 在致命信号 pending 时中止 fork，Windows 作业成员在 `CreateProcess` 时原子继承且 `CREATE_SUSPENDED` 关闭恢复前派生窗口；能构造出的 `StillAlive`（僵尸、cgroup 冻结）重试也无效，故**不新增第二次尝试**。判据与回执语义不变：只有观测到 `kill(-pgid,0)==ESRCH` / `ActiveProcesses==0` 才写 `Cleaned`，任何分支不出现"已终止进程树"，逃逸仍"未验证"。`task.execute` 与 `timeout_secs` 描述新增"终止只尝试一次（不重试）"，门禁 G1/G2 同步（`tools_sha256` 更新）；两平台新增压力用例并以变异红灯证明非空转。证据：Windows 10 unit + 78 integration、POSIX 9 unit + 76 integration 全绿；**分档仍未实现**，`#3` 仍 `NOT_PROVEN`，W2c 未重开 |
| v1.25 | 2026-11-07 | **把"未知/多余参数被静默忽略"变成可观测诊断（§2.5，不拒绝、不改结果）**：命中工具后按需在回执 JSON 追加 `arguments_diagnostics`——`unknown_arguments[]`（`kind` 区分 `unknown` 拼写错误 / `alias` 别名 / `client_metadata` 如 `_meta`，别名给 `alias_of`）、`null_optional_arguments[]`（显式 `null` 的可选字段，`kind=null_optional`，调用照旧成功）、`declared_arguments` 与固定 `note`；值只给 `{type,length}`（字符串字符数 / 数组项数 / 对象键数），**绝不回显值**；同一份 JSON 同步写 stderr 单行日志；无可报告项时不出现该字段；`additionalProperties:false` 仍只声明不强制，别名不生效。证据：5 条新用例先红后绿 + 1 条既有"未知参数被忽略"用例改为精确断言（`payload` 无 `error` 字段、诊断存在）；真实 stdio 双探针（正确 `limit` vs 拼错 `lmtt`）原样回执见 `docs/TOOLS-DESCRIPTION-AUDIT.md` §24。`tools/list` 描述与 schema 未改（门禁 `tools_sha256` 不变）；Windows 10 unit + 83 integration 全绿；**分档仍未实现**，`#3` 仍 `NOT_PROVEN` |
| v1.26 | 2026-11-07 | **运行模式显式化：AW 独立运行能力与 PM 协作边界共存（§2.6）**：`task.execute` 新增可选 `mode`（缺省 `dispatched`，PM 派单行为**零变化**）；`standalone` 必须显式传 `mode` + `cwd` + `argv` + 非空 `acceptance`，拒绝 `cmd`/`env`/`path_prepend`，执行不经 shell、作用于 cwd（显式程序路径越界 `standalone_program_outside_scope`，验收路径越界 `standalone_path_escape`）；判定逐 attempt 落盘 `runs/<task_id>/attempt-N/verdict.json`（`version`/`mode`/`criteria[].spec+observed+passed`/`observations`（退出码、stdout/stderr 摘要与预览、changed_files）/`missing[]`/`decision{action,reason,last_failure}`），结论三态 `passed|failed|undetermined`，**`undetermined` 绝不默认通过且不重试**；`max_attempts` 1..5（默认 3）有界重试，仅 `failed` 且未耗尽时重试，每次 attempt 独立目录不覆盖，耗尽写 `retry_exhausted`；同 `task_id` 跨模式复用 → `mode_conflict` 零写入；dispatched 回执**不新增** `mode/verdict_*/attempts_total`，`dispatch.json` 不写 `mode`。证据：8 条新用例先红后绿（Windows 7 条 standalone + 1 条 dispatched 回归锁），Windows 10 unit + 91 integration 全绿；真实 stdio 双单（成功 `passed` + 失败 `retry_exhausted`）原样 `verdict.json` / `handback.json` 见 `docs/TOOLS-DESCRIPTION-AUDIT.md` §25；`tools/list` 的 `task.execute` 描述与 schema 同步更新（`required` 改为 `["task_id"]` + `anyOf(cmd|mode+argv)`），门禁 `tools_sha256` 更新；**分档仍未实现**，`#3` 仍 `NOT_PROVEN` |
| v1.28 | 2026-11-07 | **补齐 standalone 下 argv 参数携带路径的作用域校验（§2.6.8）**：`argv[1..]` 在**占号前**扫描，支持独立值、`--flag=value`、`=`/`:` 分隔、一层引号、相对/`..`/`//`、绝对路径、盘符、UNC、`\\?\` 逐字前缀、`%HH` 编码（最多两轮解码）、含空格路径；按解析后的真实位置判断（逐组件 `canonicalize`、符号链接指向界外即拒绝、跨卷/UNC 前缀不等即拒绝、`..` 上跳越界即拒绝）。越界统一 `standalone_argv_path_escape` 且不创建 attempt、不执行；无法判定（未展开的 `%NAME%`/`$NAME`/`~`、破链等）**放行并在回执/verdict 的 `argv_path_scan.unknown[]` 记 `standalone_argv_path_unknown`**，不 fail-closed、不把未知写成安全。明确不当作路径：URL（含 `file://`）、含 `^$()[]{}|*+?` 的正则/通配符、无分隔符普通标量、破折号开关名、Windows 单字母斜杠开关（`/C`）；盲区逐条写入契约。standalone 回执与每次 `verdict.json` 新增 `argv_path_scan`（`scope`/`checked[]`/`unknown[]`/计数），dispatched 回执与行为零变化。证据：3 条新用例先红后绿（红原文：越界参数 `--out=../escape.txt` 实际执行、`status=ok`、`exit_code=0`），Windows 10 unit + 100 integration 全绿；真实 stdio 6 正例 + 6 反例原样回执见 `docs/TOOLS-DESCRIPTION-AUDIT.md` §27；`tools/list` 的 `argv` 描述同步，门禁新增 G16、`tools_sha256` 更新；**分档仍未实现**，`#3` 仍 `NOT_PROVEN` |
| v1.29 | 2026-11-07 | **实证 standalone 并发与重入（§2.6.9，不改执行/验收语义）**：同一 `task_id` 两独立进程并发时，靠 `fs::create_dir(attempt-1)` 原子抢占——胜者执行、败者等待并回放胜者结果，**不会写第二个 attempt，也不会两个都执行**；回执新增纯增量字段 `replayed`（bool，仅 standalone；真实执行为 `false`）与 `replay_kind`（`existing` / `waited`）。败者等待窗口改为覆盖胜者总上限 `timeout_secs * max_attempts + 5s`（原实现只等一次 timeout，多 attempt 时会过早报 `task_in_progress`）。重入语义实测确认为**回放既有结果**（`replay_kind="existing"`，`finishedAt` / `verdict.json` / `standalone-result.json` 均不变，要重跑必须换新 `task_id`）；异参并发为 `task_id_conflict`，残缺目录（有 `attempt-1` 无 `dispatch.json`）5s 后同样 `task_id_conflict`。不同 `task_id` 并发互不影响，每个 attempt 独立 Job Object 只清理本受控集合；`runs.prune` 与执行的跨进程一致性**不保证**，症状写为 `task_id_conflict`。证据：3 条新用例先红后绿（红原文 `replayed` 字段为 `Null`、同 id 并发执行方计数 `0≠1`），Windows 10 unit + 103 integration 全绿；真实 stdio 多进程回执（同 id 两进程、4 进程含 1 超时、重入、异参、残缺目录）见 `docs/TOOLS-DESCRIPTION-AUDIT.md` §28；未改 `tools/list`，`tools_sha256` 不变 |
| v1.30 | 2026-11-07 | **补齐 `runs.prune` 与 AW 执行的并发防护（§2.6.10，不改执行/验收语义）**：新增跨进程活跃标记 `runs/.inflight/<task_id>.json`（`create_new` 原子占位，含 pid/startedAt/deadlineAt；RAII 全路径删除；崩溃残留按 pid+时间判定，新调用清理后重占）；`runs.prune` 看到标记即跳过整个 task，dry-run 的 `skipped[]` 给出 `task_inflight` / `task_inflight_stale` 与标记快照，不进候选。实测 Windows 下运行中 rename 活跃 attempt 目录被 OS 拒绝（`WinError 5`），不存在"句柄继续写入 trash"；目录被删除时报 `attempt_dir_lost`（写完后复核 5 个关键文件与 result，绝不静默 ok）。新增历史台账 `runs/.task-history/<task_id>.json`，任务目录整体消失后重跑标注 `replay_basis=rerun_after_records_missing` + `prior_attempts` + `prior_last_finished_at`，首次为 `fresh`；同 id 回放保留该标注。红→绿：4 条新用例，HEAD 上原文失败（无标记、`attempt_dir_lost` 缺失、`replay_basis` 为 Null）；Windows 10 lib + 107 integration 全绿；真实多进程回执与对照（标记拦截 vs 无标记删除）见 `docs/TOOLS-DESCRIPTION-AUDIT.md` §29；未改 `tools/list`，`tools_sha256` 不变 |
| v1.31 | 2026-11-07 | **复核 `#3` 的 `NOT_PROVEN` 证据链 + 新增单次请求严格参数模式（默认行为不变）**：`#3`（超时终止整棵进程树）本机补跑 Windows 10 unit + 7 集成、POSIX WSL 复核，四类窄判据证据齐全，但仍缺 §6.8 的完整后代枚举观测点（Windows 作业外候选不可枚举、POSIX 无稳定全后代扫描、逃逸父进程可在观测后继续派生），判定为**环境/平台能力阻塞 + 既有窄定义实现**，状态**保持 `NOT_PROVEN`**，关闭条件见 `docs/TOOLS-DESCRIPTION-AUDIT.md` §30；`additionalProperties:false` 在 v1.25 后**仍只声明不强制**，默认不改为全局强制（会打断 `_meta`/别名/自定义元数据调用，且与 §10.1 的"行为确定"冲突），新增**单次调用 opt-in**：`arguments._meta.strict_arguments=true` 时拒绝 `unknown`/`alias`，返回 `code="invalid_arguments"` + 未声明/已声明参数 + `alias_suggestions`，`_meta` 与 `null` 可选字段不误拒；不提供进程级开关，schema/`tools_sha256` 不变。证据：3 条新用例先红后绿（红原文 `isError` 为 `false`）、真实 stdio 四探针回执见 `docs/TOOLS-DESCRIPTION-AUDIT.md` §31；Windows 110 integration 全绿；POSIX 同套 1 条 v1.28 argv 反斜杠用例失败（与本项无关，记为独立待办）；**分档仍未实现** |
| v1.31.1 | 2026-11-07 | **仅修 POSIX 测试夹具，不改实现、契约语义与断言强度**：`standalone_argv_scope_rejects_escapes_before_execution` 的 `argv-backslash` 夹具改为 `--out=..{sep}..{sep}escape.txt`（`sep = std::path::MAIN_SEPARATOR`）——Windows 仍是原来的 `..\..\escape.txt`，POSIX 变为真实的 `../../escape.txt`；不添加 `cfg` 豁免分支，仍断言 `status=blocked`、原因码 `standalone_argv_path_escape`、原因包含具体参数、拒绝发生在占号前。红原文（HEAD `97e76b3`，WSL）：`left: String("ok") / right: "blocked"`，`test result: FAILED. 107 passed; 1 failed`；绿：Windows 单项 `1 passed`，WSL 全量 `9 lib + 108 integration` 全绿，日志 `/root/.aw-posix-verify/logs/posix-verify-20261107-053436.log`；详见 `docs/TOOLS-DESCRIPTION-AUDIT.md` §32 |
| v1.32 | 2026-11-07 | **把已知边界做成调用方可查的事实 + 残留标记可观察可清（§2.7）**：新增只读工具 `capabilities`，13 条 claim 每条给出 `id`/`holds`（`true|false|null`）/`status`/`scope`/`reason_code`/`platforms_verified`/`platforms_unverified`/`automatic`/`evidence`；`holds=false` 必带非空原因码，`not_proven_total=5`；`#3` 窄判据（`timeout.cleans_entire_process_tree`/`timeout.enumerates_escaped_descendants` → `no_complete_descendant_enumeration`）、POSIX 覆盖（`platform.posix_inflight_semantics_verified=false`/`partial`/`posix_unverified`）、TOCTOU（`concurrency.check_then_delete_atomic=false`/`check_then_delete_not_atomic`）、不自动清（`inflight.stale_markers_auto_swept=false`/`explicit_cleanup_required`）均可机器判定。新增 `runs.inflight`（`action=list|clean`）：`list` 只读列出 `runs/.inflight/*.json` 的 `pid`/`pid_alive`/`stale`/`stale_reason`（`pid_alive`/`pid_missing_grace`/`pid_missing`/`deadline_in_grace`/`deadline_expired`/`unreadable`）/时间窗与 `stale_rule`；`clean` 默认 `dry_run=true` 只回 `would_remove`，真删需 `dry_run=false`+`confirm=true`，只删 `stale=true`（pid 存活优先于 deadline，活跃标记永不删除），删除前原子改名为 `.tmp` 并复核 `task_id+pid+startedAt`，坏 JSON 保留记 `unreadable`；不触碰 attempt 目录，不进入 `runs.list`/`runs.prune` 候选。证据：5 条新用例先红后绿（HEAD 红原文 `{"error":"未知工具：capabilities"}` / `{"error":"未知工具：runs.inflight"}`），另加强既有工具名单断言到 8 个工具；Windows `10 lib + 115 integration` 全绿；真实 stdio 独立进程实跑（正常结束 `total=0`、构造 `pid=999999999`+超期标记被 `stale_reason=pid_missing` 识别并清掉、`pid` 存活的活跃标记保留）原样回执见 `docs/TOOLS-DESCRIPTION-AUDIT.md` §33；`tools/list` 新增 G17–G19 行，`tools_sha256` 更新为 `37f7f9b0ee7d121e3f588bb65459fdedb7266d17ed744028204f915499dbcd5b`；**整树 `#3` 仍 `NOT_PROVEN`，POSIX 标记/rename 语义仍 `partial`，TOCTOU 未消除**；v1.32 新用例仅在 Windows 实跑（WSL 当时无 cargo，未复跑 POSIX），故这 4 条新能力的 `platforms_verified` 只写 `["windows"]`、`platforms_unverified` 写 `["linux","macos"]`；该 Windows-only 口径已由 v1.32.1 更新（见下行） |
| v1.32.1 | 2026-11-07 | **POSIX 双侧复跑 + PM 判活口径收敛（§2.7 / §2.8，不改执行语义）**：WSL Ubuntu 24.04 上确认 `cargo` 缺失是 Linux rustup shim 未安装（`which cargo` 无输出/rc=1、`/root/.cargo/bin` 不存在/rc=2，PATH 里只有 Windows 挂载），rustup 最小安装卡在 `rustup-init` 下载（原始报错 `curl: (28) Operation timed out after 120001 milliseconds with 1136884 out of 21347088 bytes received`），改用已存在的 `cargo 1.88.0` 实跑；四条 `inflight.*` 与 `platform.posix_inflight_semantics_verified` 的 `platforms_verified` 更新为 `["windows","linux"]`、`platforms_unverified` 仅 `["macos"]`，POSIX claim 仍 `holds=false/partial/posix_unverified`（rename 活跃 attempt 未单独复核）。新增 `runs.inflight action=list` 的机器可读 `liveness_rule`（`inflight-liveness/1`）：PM 判活**只认** `markers[].task_id`+`markers[].stale`；`removable` 仅用于清理；`pid`/`pid_alive`/`stale_reason`/时间字段仅作诊断；attempt 缺 `handback`/`dispatch`、裸看 `.inflight`、`pid` 数字、标记年龄均**不得**用于判活；`fallback.allowed=false`（仅旧版 AW 可降级并标注）；§2.8 写死同一口径。证据：WSL 上以 HEAD 旧代码跑新断言先红（`platforms_verified` 仅 `["windows"]`、`liveness_rule` 为 `Null`），换当前实现后两条用例均绿；WSL `9 lib + 115 integration` 全绿、Windows `10 lib + 117 integration` 全绿；描述门禁 `3 + 4` 绿，`tools_sha256` 更新为 `a3591dbfe339e2ddb61400bd810ca58e17faf7b098756b044f186cb4cce88084`；详见 `docs/TOOLS-DESCRIPTION-AUDIT.md` §34 |
| v1.33 | 2026-11-07 | **残留 inflight 标记顺带清扫（§2.7 / §2.9，无后台线程）**：不新增定时器，只在三条既有路径触发——同 `task_id` 重启（只清自身标记后重试 `create_new`）、`runs.prune` 真删前（清完候选不再被 `task_inflight_stale` 跳过）、`runs.inflight action=clean`；判据复用 `stale_rule`（`pid` 存活优先于 `deadline`，活跃永不删；坏 JSON 保留并报 `unreadable`）；每次实际清扫写 `runs/inflight-sweep.jsonl`（`inflight-sweep/1`，先 `phase=plan` 后 `phase=result`，含 `trigger`/`task_id`/`stale_reason`/`path`），审计写失败则零删除，无残标时零删除且不追加审计（幂等）；删除沿用原子 rename + 复核 `task_id`+`pid`+`startedAt`，不一致还原记 `marker_changed`；`list` 新增 `sweep_rule`。能力位如实改为 `inflight.stale_markers_auto_swept=false/partial/path_triggered_only/automatic=false`（`not_proven_total` 5→4），明确“只做到特定路径顺带清扫”。边界仍保留：跨进程无事务、判活与删除间可能加入新进程、坏标记不自愈。证据：5 条新用例先红后绿（Windows `10 lib + 122 integration`、WSL `9 lib + 120 integration`），`tools_sha256=36dbcfc9c9ac83eede1a4ef581801b0b4dcb9b955761d5e3fcb085ea093af993`；详见审计 §35 |
| v1.34 | 2026-11-07 | **补后代观测点，`#3` 从全线 `NOT_PROVEN` 变为按平台分裂（§2.7 / 设计 §6.8 / §6.10，不放宽判据）**：新增 `process_scope::DescendantSnapshot`——Windows 用 `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)` 父链快照 × `JobObjectBasicProcessIdList` 交叉核对（`ParentProcessID` 不因父进程退出而改写，且 `CREATE_SUSPENDED` 先入作业、本作业未设 `BREAKAWAY_OK`，故全部后代可枚举且受控）；Linux 用 `/proc/<pid>/stat` 的 `ppid` 闭环 × `pgrp` 成员交叉核对。终止路径在原有判据之后**追加**复核：快照不可观测 → `Unavailable`，仍有后代 → `StillAlive`，两者都空才 `Cleaned`（只收紧不放松）。能力位：新增 `timeout.descendant_snapshot_cross_check=true/proven/automatic=true`；`timeout.cleans_entire_process_tree` 与 `timeout.enumerates_escaped_descendants` 改为 `holds=false/partial/posix_escaped_after_reparent_unobservable`（Windows 侧成立，POSIX 换父被收养后不可枚举/不可终止）；条目 13→14，`not_proven_total` 仍 4。红→绿：POSIX 占位实现首轮 `FAILED. 9 passed; 2 failed`（`必须能观测到进程组内的后代` / `根存活期间，逃逸后代必须能被父链快照标出`）；Windows 变异红灯（停用作业 PID 列表，已还原）`FAILED. 0 passed; 2 failed`（`必须能观测到作业内的后代` / `未加入作业的进程树也必须可枚举`）；绿：Windows `12 lib + 122 integration`、WSL Ubuntu 24.04 `11 lib + 120 integration` 全绿。证据行：Windows 终止前 `controlled=[124800,113168]` = 根+后代、终止后 `descendants=[] controlled=[]`；Linux 终止前 `controlled=[701,702,703]`、终止后同为空；Linux 逃逸负例 `escaped_alive=true` 但快照 `descendants=[]`（POSIX 剩余边界的直接证据）。未改 `tools/list` 描述，`tools_sha256` 不变；详见设计文档 §6.10 与审计 §36 |
| v1.35 | 2026-11-07 | **POSIX 补 cgroup v2 作用域观测：被收养的逃逸后代终于可见（§2.7 / 设计 §6.8 / §6.11）**：`DescendantSnapshot` 增加 `scope`（`JobObject`/`CgroupV2`/`ProcessGroup`）与 `escaped_scope_unobservable`；Linux 在 `configure` 阶段建独立 cgroup v2，`pre_exec` 在 fork 后、exec 前把自身 PID 写入 `cgroup.procs`（无「先派生后入组」窗口），`attach` 核对成员关系后才置 `Active`；后代集合变为「父链闭包 ∪ cgroup 成员 − 根」，`controlled_pids` 仍表示**终止作用域**（`pgrp`），因此 `in_controlled_set=false` 即「可见但杀不到」。建立失败（cgroup v1 / 容器 / 非 root）不阻断执行，`escaped_scope_unobservable` 记 `cgroup_v2_unavailable` / `cgroup_create_failed:*` / `cgroup_membership_not_established`，观测退回进程组——**绝不把空快照读成没有后代**。行为修正：`setsid` 逃逸在终止后仍存活时，回执由假 `Cleaned` 变为 `StillAlive`。能力位：新增 `timeout.cgroup_v2_scope_observation=true/proven`（`platforms_verified=[linux]`，`platforms_unverified=[windows,macos]`）；`timeout.cleans_entire_process_tree` 与 `timeout.enumerates_escaped_descendants` 保持 `holds=false/partial`，reason 分别改为 `posix_escaped_descendant_observable_not_terminated` 与 `posix_escaped_scope_requires_writable_cgroup_v2`（条件性能力不写成无条件 proven）；条目 14→15，`not_proven_total` 仍为 5（更正：v1.33/v1.34 行写的“4”是笔误，按 `holds=false` 实数为 5）。红→绿：禁用 cgroup 建立（临时变异，已还原）时 `cgroup_scope_enumerates_adopted_escapee` 失败 `scope=ProcessGroup`；绿：Windows `12 lib + 122 integration`、WSL Ubuntu 24.04（cgroup v2，root）`13 lib + 120 integration` 全绿；证据 `cgroup_scope after outcome=StillAlive escaped_alive=true escaped_ppid=Some(740) root=936 descendants=[pid 945, in_controlled_set=false] controlled=Some([]) scope=CgroupV2`。未改 `tools/list` 描述，`tools_sha256` 不变；另修正 `StillAlive` 回执里「换父被收养的逃逸进程不可观测」这一定论（cgroup 下已可观测）。详见审计 §37 |
| v1.36 | 2026-11-07 | **把「检查后删除」的并发边界拆成可证窄结论（§2.7 / §2.9，不放宽判据）**：先定可观测量——清扫者 rename 认领后若复核发现内容已变，原名位置可能已被更新者占住，此时**只允许 create-if-absent 还原**，不得用 `rename` 覆盖。原实现 `fs::rename(tmp, path)` 在 POSIX 与 Windows 上都会覆盖已存在的同名文件：W1 在「检查→认领」之间替换标记（被认领带走）、W2 在「认领→复核」之间新建标记（占住原名），还原就会把 W2 换成 W1，等于把一条活跃标记改写成过期记录。修复：新增 `restore_without_clobber`（`create_new`/O_EXCL 写回，`AlreadyExists` 时保留新标记、丢弃手上副本并记 `marker_superseded`），`marker_changed` 仅表示原名仍为空已放回；`sweep_rule.atomic_delete` 与 `limits` 同步写实（新增 `no_compare_and_rename_primitive`）。能力位：`concurrency.check_then_delete_atomic` **保持 `false/not_proven/check_then_delete_not_atomic`**（扫描与认领之间的窗口仍在，无 compare-and-rename）；另加 `concurrency.sweep_restore_never_clobbers=true/proven`（`platforms_verified=[windows,linux]`，`platforms_unverified=[macos]`），条目 15→16、`not_proven_total` 仍 5。红→绿：`sweep_restore_never_clobbers_concurrent_new_marker` 在 HEAD 实现上原文失败 `assertion left == right failed: 还原不得覆盖并发写入的 W2（覆盖后会是 W1 的时间戳）：{"startedAt":"2098-01-01T00:00:00+00:00",...} / left: String("2098-01-01T00:00:00+00:00") / right: String("2099-01-01T00:00:00+00:00")`，`test result: FAILED. 0 passed; 1 failed`；绿：Windows `13 lib + 122 integration`、WSL Ubuntu 24.04 `14 lib + 120 integration`（日志 `/root/.aw-posix-verify/logs/posix-verify-20261107-195601.log`）。未改 `tools/list` 描述，`tools_sha256` 不变；另记：`removed[].path` 只表示本次删掉的文件名，不代表该路径当前无标记（写入契约 §2.7）。详见审计 §38 |
| v1.37 | 2026-11-07 | **用 `cgroup.kill` 闭合 Linux「超时终止整棵作用域」的 POSIX 边界（§2.7 / 设计 §6.8 / §6.11，先红后绿、不放宽判据）**：v1.35 能观测 cgroup 内被收养的逃逸后代，但终止仍只发 `kill(-pgid)`，逃逸者存活（`outcome=StillAlive escaped_alive=true`）。本轮先定可观测量：Linux 可写 cgroup v2 且内核提供 `cgroup.kill`（v5.14+，root cgroup 无该文件、子 cgroup 才有，实测 WSL 内核 `6.18.33.2-microsoft-standard-WSL2`）时，`terminate` 在进程组 SIGTERM/SIGKILL 之后向作用域写 `cgroup.kill`，**三个独立观测量全部成立才允许 `Cleaned`**——`cgroup.procs` 为空、`cgroup.events` 为 `populated 0`、逃逸 PID 已不存在；任一不满足或读不出即 `StillAlive`/`Unavailable`。`cgroup.kill` 只杀「当前成员」，fork 压力下可能漏掉正在派生的后代，故每轮轮询重试写入（重复写返回 0，幂等；探针实测 13 个成员 + 持续 fork 的进程写入后 `procs=0`、`populated 0`）。降级口径不变：容器/非 root/cgroup v1/内核 < 5.14 时作用域建不起来或没有 `cgroup.kill`，退回进程组窄判据并如实记 `escaped_scope_unobservable`。能力位：只新增条件性窄结论 `timeout.cgroup_kill_terminates_scope=true/proven`（`platforms_verified=[linux]`，`platforms_unverified=[windows,macos]`，evidence `cgroup_kill_terminates_adopted_escapee`）；`timeout.cleans_entire_process_tree` **不上调**，仍 `holds=false/partial`，reason 改为 `posix_escaped_scope_requires_cgroup_v2_and_kill`（macOS 未验证 + 无 cgroup.kill 环境仍会存活）；`timeout.cgroup_v2_scope_observation` 保持 `proven` 但 detail 改为「只声明观测」。条目 16→17、`not_proven_total` 仍 5。红→绿：新用例 `cgroup_kill_terminates_adopted_escapee` 在 HEAD 上原文失败 `[evidence] cgroup_kill after outcome=StillAlive escaped_alive=true members=Ok([1135]) populated=Ok(1) descendants=[DescendantProcess { pid: 1135, in_controlled_set: false }] unobservable=None` + `panicked ... 逃逸后代必须在作用域终止后不再存活`，`test result: FAILED. 14 passed; 1 failed`；绿：`[evidence] cgroup_kill after outcome=Cleaned escaped_alive=false members=Ok([]) populated=Ok(0) descendants=[] unobservable=None`，WSL Ubuntu 24.04 `15 lib + 120 integration` 全绿（日志 `/root/.aw-posix-verify/logs/posix-verify-20261107-204702.log`），Windows `13 lib + 122 integration` 全绿（首次全量时 `timeout_cleans_controlled_set_including_grandchild` 因 8s 窗口在并发负载下抖动失败，单跑与二次全量均通过，与本轮改动路径无关）。两条 v1.35 旧用例（`cgroup_scope_enumerates_adopted_escapee`、`setsid_escape_is_flagged_then_unobservable_after_reparent`）按新事实分支：`cgroup.kill` 可用 → 断言逃逸者已死 + `Cleaned`；不可用 → 断言仍存活 + `StillAlive` 且可枚举，**未放宽任何一支**。未改 `tools/list` 描述，`tools_sha256` 不变。详见审计 §39 |
| v1.38 | 2026-11-07 | **闭合最后一个可证的 `holds=false` 条目：POSIX 上 rename 活跃 attempt 目录的语义（§2.7，先红后绿、不放宽判据、不牵动整树）**：`runs.prune` 的 `delete_attempt_dir` 依赖「把整个 attempt 目录 rename 进 `.prune-trash` 再删」。本轮先定可观测量：(a) 进程 cwd 指向该目录、且持有目录内文件 fd 时 `rename` 仍成功；(b) `/proc/<pid>/cwd` 跟随到 trash 新路径；(c) 已打开的 fd 仍读到原内容；(d) 旧路径不存在、新路径存在；(e) 随后对 trash 路径 `remove_dir_all` 成功，进程仍存活且 fd 内容不变。新增 Linux 单测 `posix_rename_active_attempt_semantics` 逐条断言。能力位：`platform.posix_inflight_semantics_verified` 由 `false/partial/posix_unverified` 改为 `true/proven/null`，evidence 增加新用例，detail 改为已单独复核；`platforms_verified` 仍 `["windows","linux"]`、`platforms_unverified` 仍 `["macos"]`；条目仍 17，`not_proven_total` 5→4。**整树不上调**：`timeout.cleans_entire_process_tree`、`timeout.enumerates_escaped_descendants`、`concurrency.check_then_delete_atomic`、`inflight.stale_markers_auto_swept` 全部保持原状。红→绿：契约断言在 HEAD 上原文失败 `left: Bool(false) / right: true` + `"reason_code":"posix_unverified","status":"partial"`；绿：WSL Ubuntu 24.04 `16 lib + 120 integration`、Windows `13 lib + 122 integration` 全绿（Windows 首次全量 `descendant_snapshot_cross_checks_job_accounting` 报 `StillAlive`，当时记为并发抖动；v1.38.1 复核证明不是窗口抖动，而是父链闭包在 PID 复用下把无关进程树误判为后代，详见审计 §41，与本轮 Linux-only 改动路径无关）。另修 `scripts/posix-timeout-verify/verify.sh` 退出码缺陷：`local rc` 单独一行会把 `PIPESTATUS` 重置为 0，导致测试失败仍打印「全部通过」（本轮实际观测到 integration `1 failed` 后脚本仍报成功），改为 `local rc=${PIPESTATUS[0]}`。未改 `tools/list` 描述，`tools_sha256` 不变。详见审计 §40 |
| v1.38.1 | 2026-11-07 | **修复验证脚本后重跑历史；更正 Windows 父链快照的「抖动」误判（先取证再改口径，不保连续性）**：`verify.sh` 的 `PIPESTATUS` 缺陷修复后，用同一份修复版脚本在 v1.33–v1.38 六个 worktree 上逐个重跑（WSL Ubuntu 24.04，`AW_VENDOR=/root/vendor`）：六版 `EXIT=0`，`9/11/13/14/15/16 lib + 120 integration` 全绿，关键用例逐条在日志中为 `ok`（`inflight_sweep_on_task_start_removes_only_target_stale_marker`、`inflight_sweep_on_prune_removes_stale_marker_and_unblocks_candidate`、`descendant_snapshot_cross_checks_process_group`、`timeout_cleans_controlled_set_including_grandchild`、`cgroup_scope_enumerates_adopted_escapee`、`sweep_restore_never_clobbers_concurrent_new_marker`、`cgroup_kill_terminates_adopted_escapee`、`posix_rename_active_attempt_semantics`）——**未发现任何一条假绿**；旧脚本只在「测试已失败但四类证据行仍齐全」时才误报成功，这六版复跑均无失败行。Windows 侧另做专项复核：24 路负载下全量重复 9 次复现 `descendant_snapshot_cross_checks_job_accounting` 的 `StillAlive`，现场为 `active=Some(0)` 但快照列出 20 个进程（`WeChatAppEx.exe`/`WXWork.exe`/`crashpad_handler.exe`/`conhost.exe` 等），父链根为 `165548`（本轮 `cmd` 的 PID）→ 不是 3s 窗口太紧，而是 `ParentProcessID` 残留 + PID 复用：`WXWork.exe` 的父进程早已退出，同号 PID 被新的 `cmd` 复用，父链闭包把整棵无关进程树算成后代。据此更正：`timeout.cleans_entire_process_tree` 的 Windows 侧只能证「作业记账归零」，不能证「父链快照为空」；`timeout.enumerates_escaped_descendants` 的 Windows 枚举存在误报、只能作旁证。两条仍 `holds=false`，不新增能力位、条目仍 17、`not_proven_total` 仍 4。修复方向（未在本轮实现）：父链闭包加「子进程创建时间 ≥ 父进程创建时间」的边校验，杜绝 PID 复用误连；macOS 仍未验证。未改 `tools/list` 描述，`tools_sha256` 不变。详见审计 §41 |
| v1.39 | 2026-11-07 | **修 Windows 父链闭包的 PID 复用误连（§2.7，先红后绿、不放宽判据）**：进程表改为可注入的 `ProcEntry{pid, parent_pid, created_at, alive}`，父链闭包加三道过滤：(a) 每条边要求「子进程创建时间 ≥ 父进程创建时间」——旧父进程退出后同号 PID 被新进程持有，旧子进程的 `ParentProcessID` 会把无关树挂到新根上，创建时间顺序能拒绝这类边；(b) 「根 → 直接子进程」再加「子创建时间 ≤ 根退出时间」上界（退出时间从子进程句柄读 `GetProcessTimes`，不经 PID），拒绝根 PID 被后来者复用后产生的子进程；(c) 剔除 `GetExitCodeProcess` 判为已终止、或 `OpenProcess` 返回 `ERROR_INVALID_PARAMETER`（PID 已不存在，Toolhelp 枚举与逐个打开之间的残行）的行。任一时间/存活状态读不到时保留旧行为（保留该行），不因缺证据扩大误判。确定性单测 `pid_reuse_parent_chain_edge_is_rejected`、`terminated_rows_are_not_counted_as_descendants` 覆盖根级复用、中间层复用、根退出后复用、时间不可读、僵尸行、存活状态不可读六类边，不依赖真机碰运气。红（先把 `edge_respects_creation_order` 改成恒 `true`）：`[evidence] pid_reuse rows=6 result=[900, 901, 902, 903, 905, 906]` + `left: [900, 901, 902, 903, 905, 906] / right: [902, 903, 906]`；退出上界失效时红出 `result=[902, 903, 906, 907]`。绿：`15 passed`（含新增两例），后续补 `create_evidence_error` 映射用例后 `17 passed`。24 路负载复跑（原判据不改）：lib 单测最终版连跑 20 轮全绿；全量 4 轮均为 `15 lib` 全绿 + integration `120 passed / 2 failed`，两个失败都与父链快照无关——(1) `timeout_cleans_controlled_set_including_grandchild`：同一负载下实测 PowerShell 从启动到写出 `pids.txt` 需 10.7–16.2 s，超过用例 8 s 窗口，属夹具启动延迟，不放宽；(2) `standalone_active_attempt_deleted_is_reported_not_silent_ok`：目录在创建 `stdout.txt` 前已被删掉，报出裸错误 `创建 stdout 失败：系统找不到指定的路径。 (os error 3)` 而非具名 `attempt_dir_lost`，本轮补 `create_evidence_error`（`NotFound` 且目录不存在 → `attempt_dir_lost`），并加确定性单测。最终回归：Windows `17 lib + 122 integration`、WSL Ubuntu 24.04 `18 lib + 120 integration` 全绿（WSL 日志 `/root/.aw-posix-verify/logs/posix-verify-20261107-231315.log`）。能力位不上调：`timeout.cleans_entire_process_tree` 仍 `holds=false/partial`，Windows 侧能证「作业记账归零」+「快照在创建时间/存活状态可读时不再误连 PID 复用树、不把已终止/已不存在的行算作后代」，证不到作业外进程（如由 csrss 创建、不进作业的 `conhost.exe`，其存活时 `StillAlive` 是真实观测）、换父被收养、以及时间/存活状态不可读的后代；`timeout.descendant_snapshot_cross_check` 保持 `proven`，detail 补三道过滤边界。条目仍 17，`not_proven_total` 仍 4。未改 `tools/list` 描述，`tools_sha256` 不变。详见审计 §42 |
| v1.40 | 2026-11-07 | **Linux 父链闭包补「子进程启动时刻 ≥ 父进程启动时刻」边校验（§2.7，先红后绿、不放宽判据、能力位不变）**：`ProcRow` 增 `start_time`（`/proc/<pid>/stat` 第 22 字段 `starttime`；`comm` 可含空格与括号，以最后一个 `)` 为界），`descendants_of(root, root_start_time, rows)` 对每条边要求子 ≥ 父——PID 复用后旧父链残留的 `ppid` 边（子早于新父）被拒绝，祖先被拒则其后代不可达；`Supervisor::attach` 记录根启动时刻（`proc_start_time(child.id())`）作为根边下界。**逐边放宽**：只有「这条边的父进程启动时刻读不到」时才退回旧行为（接受该边），不因一个未知时刻放开整棵树；时刻读不到的下界与 v1.39 Windows 侧口径一致。确定性单测 `pid_reuse_parent_chain_edge_is_rejected` 用注入 `ProcRow` 表覆盖根级复用、祖先被拒后后代不可达、中间层复用、根时刻不可读四种情形。红（临时停用边校验，WSL）：`[evidence] linux_pid_reuse rows=6 result=[900, 901, 902, 903, 905, 907]` + `left: [900, 901, 902, 903, 905, 907] / right: [902, 903, 907]`，`test result: FAILED. 0 passed; 1 failed`；绿：WSL Ubuntu 24.04 `19 lib + 120 integration`、Windows `17 lib + 122 integration` 全绿（WSL 日志 `/root/.aw-posix-verify/logs/posix-verify-20261108-004812.log`）。**Linux 侧能力位没有变化**：`timeout.cleans_entire_process_tree` 仍 `holds=false/partial/posix_escaped_scope_requires_cgroup_v2_and_kill`（边校验只回答「枚举到的 PID 是否属于本任务」，不解决「逃逸成员杀不到」）、`timeout.enumerates_escaped_descendants` 仍 `holds=false/partial`（范围仍受 cgroup 是否可写限制）、`timeout.descendant_snapshot_cross_check` 仍 `proven`，条目仍 17、`not_proven_total` 仍 4（只在 detail 补边校验说明）。另改 Windows `timeout_cleans_controlled_set_including_grandchild` 夹具：去掉 PowerShell 与 `pids.txt`，孙进程改为唯一命名的 `ping.exe` 副本，PID 由测试进程用 Toolhelp 轮询观测——纯夹具改动，不改契约、能力位与执行语义；用例新增 `[verdict] grandchild_case=pass|terminate_defect|premise_not_established|unobservable` 判定行，把「产品没清理干净」与「机器太慢、前提没建立」在日志里分开。**该用例的覆盖边界**：`全量并发 × 24 路 burner` 下「孙进程能否在 8s 窗口内创建」这一前提无法稳定建立（实测当轮单次进程创建 10–15s），故该场景下整树清理能力**未被验证**，该用例不进该负载矩阵；已成立的是「单用例 × 24 路 burner」（9 轮全绿）与「空载全量」（Windows `17 lib + 122 integration` 全绿）两种。窗口、判据、实现均未放宽；24 路负载原始数据与判定依据见审计 §43.4–§43.5。未改 `tools/list` 描述，`tools_sha256` 不变 |
| v1.41 | 2026-11-08 | **拆清 Linux 整树清理的证成范围与降级路径，换掉笼统 reason（§2.7，先取证后改口径、不上调跨平台能力位）**：在满足条件的环境（cgroup v2 可写 + 内核提供 `cgroup.kill`）里，Linux 侧整作用域终止**能证成**——WSL Ubuntu 24.04（root，cgroup v2，内核 `6.18.33.2-microsoft-standard-WSL2`）用真实 `setsid` 逃逸进程取得证据：逃逸者被收养（`escaped_ppid=Some(432)`）且受控进程组为空（`controlled=Some([])`）时，cgroup 仍枚举到它；写 `cgroup.kill` 后 `cgroup_kill after outcome=Cleaned escaped_alive=false members=Ok([]) populated=Ok(0) descendants=[] unobservable=None`，四项观测量同时成立才报 `Cleaned`。降级路径另用两个确定性用例验证：**无 `cgroup.kill`** → 不写 kill，成员仍在，`outcome=StillAlive`、`escaped_alive=true`、`members=Some(Ok([1175]))`、`populated=Some(Ok(1))`（`cgroup_without_kill_reports_still_alive`），不会谎报 `Cleaned`；**作用域建不起来**（`Degraded`）→ 只按进程组判据，被收养逃逸进程既打不到也枚举不到，`outcome=Cleaned` 但 `escaped_alive=true`、`escaped_scope_unobservable=Some("cgroup_v2_unavailable")`（`cgroup_degraded_terminate_is_group_only`），这是残余缺口，不是静默漏杀：降级原因写入快照与 capabilities，`Cleaned` 回执固定写「逃逸进程是否仍在运行未验证」。能力位：`timeout.cleans_entire_process_tree` 保持 `holds=false/partial`（跨平台单一三态还要覆盖 Windows 作业外进程与未验证的 macOS，且 Linux 侧为条件性），reason 由 `posix_escaped_scope_requires_cgroup_v2_and_kill` 改为点名实际卡点的 `processes_outside_controlled_scope_may_survive`，statement/detail 拆三分支，evidence 增加上述两个降级用例；**其他能力位零改动**，条目仍 17、`not_proven_total` 仍 4。回归：WSL `21 lib + 120 integration`、Windows `17 lib + 122 integration` 全绿（WSL 日志 `/root/.aw-posix-verify/logs/posix-verify-20261108-013052.log`）。未改 `tools/list` 描述，`tools_sha256` 不变；详见审计 §44 |
| v1.42 | 2026-11-08 | **补上 inflight 残标自动清扫（启动一次 + 默认每小时周期），能力位按实际上调（§2.7 / §2.9，先真实进程取证再改口径）**：`main` 在进入 stdio 循环前调用 `run_startup_sweep()`（`trigger=startup`），并按 `auto_sweep_schedule()` 启动后台线程（`trigger=periodic`）；两条自动路径都直接调用既有 `sweep_inflight_markers`，复用同一 `stale_rule`，不另写判据，与 `task_start` / `runs.prune` / `runs.inflight.clean` 写同一 `runs/inflight-sweep.jsonl`（plan+result 均带 `trigger`）。周期默认**开启**、每小时一次：一次清扫只是只读扫描 `.inflight` 目录，成本低；只在启动时清扫会让长驻进程里的残标一直躺着。`AW_INFLIGHT_SWEEP_SECS` 可调：`0`=仅启动、`off`=关闭自动清扫、正整数=周期秒数，非法值按默认并写 stderr。审计 plan 行写不进去则本轮零删除（`inflight_auto_sweep_audit_failure_deletes_nothing`）。真实进程证据：把 bridge 可执行文件复制到独立目录（隔离 `runs/`），造 `pid=999999999` 的真残标与 `pid=当前测试进程` 的活跃标记，重启 bridge 后残标被清、活跃标记保留，审计 `trigger=startup phase=result removed_total=1 stale_reason=pid_missing kept_total=1`；另一用例用 `AW_INFLIGHT_SWEEP_SECS=1`，在 initialize 之后才造残标，确认只有 `trigger=periodic` 的 result 行清掉它（`inflight_startup_sweep_removes_stale_and_keeps_active` / `inflight_periodic_sweep_removes_stale_marker`）。能力位：`inflight.stale_markers_auto_swept` 由 `false/partial/path_triggered_only` 上调为 `true/proven/reason_code=null`，`automatic=true`，evidence 增加上述三个用例；`inflight.stale_marker_cleanable` 的 `automatic=false`（显式 clean 仍需调用方 `dry_run=false+confirm=true`）不变。`runs.inflight` 与 `runs.prune` 的 tools/list 描述同步（不再声称「无启动/定时自动清理」），`tools_sha256` 更新为 `915974d2663fa2289c9747e2a048d59784e93c397c643784643f9d57d2221937`。条目仍 17，`not_proven_total` 4→3。回归：Windows `17 lib + 125 integration`、WSL 全量见审计 §45 |
| v1.43 | 2026-11-08 | **把「检查后删除」做成真原子语义：rename 认领 + 认领后重跑 `stale_rule`（§2.7 / §2.9，先红后绿、不放宽判据）**：删除判定从「认领副本与扫描快照的 `task_id`+`pid`+`startedAt` 三元组一致」改为「对认领副本重读并重跑同一份 `stale_rule`」——仍 `stale` 才删（`outcome=deleted`）；已不 `stale`（刷新者常只延长 `deadlineAt`、三元组不变）就地 create-if-absent 还原并记 `outcome=not_mine`（`marker_changed` / `marker_superseded` / `recheck_unreadable`）；认领副本已不存在记 `outcome=lost_race`，不算错误。审计 `runs/inflight-sweep.jsonl` 的 `removed[]` / `kept[]` 逐条带 `outcome`，result 增加 `deleted_total` / `lost_race_total` / `not_mine_total`（保留 `removed_total`），plan 写不进去仍零删除。Windows 实测发现并发 `rename` 同一源文件会有多个调用都返回成功、但只有一个目的地产生文件，故「rename 返回 Ok」不当作归属依据，以认领副本是否存在判定。红证据（新测试跑 HEAD 实现原文）：`sweep_concurrent_claimers_delete_once_others_lost_race` → `assertion left == right failed: 同一个残标只允许一方真正删除，实际 0 ... left: 0 / right: 1`，4 个清扫者结果为 1 个 `removed_total=1`、3 个 `failed: [{"error": "marker_changed_restore_failed: 系统找不到指定的文件。 (os error 2)"}]`，且 `deleted_total` / `lost_race_total` 均为 `Null`；`sweep_concurrent_refresher_never_deletes_fresh_marker` → `panicked ... 认领后必须能识别出「已不是我的」，实际 0`，结果里 `removed_total=1`（刷新后的标记被删）、`not_mine_total: Null`；`test result: FAILED. 1 passed; 2 failed`。绿：两条用例连续 20 轮全绿（`deleted_total=1`、`lost_race_total=N-1`；`deleted_total=0`、`not_mine_total>=1` 且原位仍是刷新后的 `deadlineAt`），Windows `19 lib + 125 integration`、WSL Ubuntu 24.04 `23 lib + 123 integration` 全绿。能力位：`concurrency.check_then_delete_atomic` 由 `false/not_proven/check_then_delete_not_atomic` 上调为 `true/proven/reason_code=null`、`automatic=true`，evidence 为上述两条并发用例 + `sweep_restore_never_clobbers_concurrent_new_marker`；detail 写明残余边界（跨卷 `EXDEV` → 认领失败、零删除；认领与还原之间原名短暂空缺；审计 append 非事务；Windows 并发 rename 的判定口径）。`concurrency.sweep_restore_never_clobbers` 与 `inflight.clean_preserves_active_markers` 的 detail 同步为「重跑 stale_rule」。`sweep_rule.atomic_delete` / `limits` 改为 `same_volume_rename_required` / `transient_name_absence_during_claim` / `audit_append_not_transactional` / `unreadable_not_self_healing`，新增 `outcome_totals`。条目仍 17，`not_proven_total` 3→2。未改 `tools/list` 描述，`tools_sha256` 不变；详见审计 §46 |
| v1.44 | 2026-11-08 | **全库核查「rename 返回值当归属/互斥依据」，并把孙进程夹具改成握手式（§2.7 / §8，先红后绿、判据与四态不放宽）**：逐处判定 10 个 rename/替换点，只有 `delete_attempt_dir` 靠返回值判搬成功——Windows 并发下双方 `Ok`、只有一个目的目录，败者被误报成 `trashed_not_deleted`（数据其实已被另一方删掉），POSIX 败者被误报成 `retained_at_source`（源其实已不在）。改为 rename `Ok` 后用 `symlink_metadata(staged)` 复核目的是否存在，源 `NotFound`/目的不存在/删除前被清走 → 具名 `LostRace`；inflight 删除失败回滚 `rename(tmp→原名)` 也改用 `restore_without_clobber`（`create_new`），不再覆盖并发新标记；其余 8 处（session 唯一目的名、inflight 认领、POSIX 语义探针测试、测试刷新者、updater 发布、launcher 下载、安装器/venv 迁移、plan JSONL append）判定为不靠 rename 返回值判归属，不动。prune 结果新增**独立** `lost_race[]` / `lost_race_total`：不计入 `failed_total`、`status` 不因它变 `partial`（只有它时仍 `ok`、`isError=false`），逐条带 `task_id`/`attempt`/`source_path`/`error`/`rule`/`fingerprint`；不变式改为 `deleted + failed + trashed + lost_race + excluded == batch_candidates_total`（代码注释、工具描述、§8.11 三处一致）。与清扫侧 `lost_race` 同词同义（另一方先赢、本进程零删除），区别只在对象：清扫侧是 `.inflight` 认领副本读不到，prune 侧是 attempt 目录源已消失/目的目录不存在。红（HEAD 原文）：`prune_claim_lost_is_named_not_trashed` → `目的目录不存在不是「删除暂存失败」：trashed_not_deleted`；`prune_concurrent_claimers_delete_once_others_lost_race` 第 0 轮 → Windows `[deleted, trashed_not_deleted]`、WSL `[deleted, retained_at_source]`。绿：真并发 20 轮 `[evidence] prune_claim_race rounds=20 deleted=20 lost_race=20 other=0`，每轮恰好一方删除、另一方 `lost_race`、源不存在、无半截暂存目录。孙进程夹具改为握手式：`handshake.cmd` 写 `started`→拉起孙进程→写 `ready`→保持存活；窗口外测量 `spawn→ready` 真实耗时，窗口内 `timeout_secs=max(10, handshake_ms/1000+6)`，`task.execute` 放独立线程、主线程等 `ready` 并记录 `ready_seen`/`ready_delay_ms`；verdict 增加 `ready_seen`/`ready_delay_ms`/`timeout_ms`/`handshake=before_timeout|after_timeout|not_completed`，`ready_seen=false` 明写「握手未完成，本条不构成裁决依据」，四态不变。实测 Windows：`[evidence] grandchild_handshake warm_ms=412 timeout_ms=10000`、`[verdict] grandchild_case=pass ready_seen=true ready_delay_ms=412 timeout_ms=10000 handshake=before_timeout observed=1 survivors=[]`。`timeout.cleans_entire_process_tree` 的 evidence 增加 `timeout_cleans_controlled_set_including_grandchild`，但 **holds 仍 `false/partial`**（只证成受控集合内孙进程，证不到作业外进程）；其余两条 `not_proven`（`timeout.enumerates_escaped_descendants`）的可证路径逐条写入 §2.7。`runs.prune` 描述同步新不变式，`tools_sha256` `915974d2…` → `85da0303…`。回归：Windows `21 lib + 125 integration`、WSL Ubuntu 24.04 `25 lib + 123 integration` 全绿（WSL 日志 `/root/.aw-posix-verify/logs/posix-verify-20261108-031610.log`）。详见审计 §47/§48 |
| v1.45 | 2026-11-08 | **把两条 `not_proven` 里「Linux 可证、当时没证」的部分证掉，能力位按平台拆分（§2.7，先红后绿、判据不放宽）**：新增 `cgroup_kill_terminates_escapee_that_left_cgroup_procs`——父（`sh`）→子（`sh`）→孙（`sh`），孙进程 `setsid` 后把自己写进旁路 cgroup（`sink_members=[<pid>]`、`move_status=moved`），**真正离开本作用域 `cgroup.procs`**；HEAD 实现在 `cgroup.kill` 后误报 `Cleaned` 而逃逸进程仍存活，红证据：`[evidence] cgroup_left after outcome=Cleaned escaped_alive=true members=Ok([]) populated=Ok(0) descendants=[] unobservable=None` → `panicked ... 逃出 cgroup.procs 的后代在终止后必须不再存活（cgroup.kill 打不到它，必须另行收口）`，`test result: FAILED. 25 passed; 1 failed`。修复 `Supervisor::terminate`：动手前用「`/proc` 父链闭包 ∪ `cgroup.procs` 成员」建立名单并附 `starttime`，`cgroup.kill` 后对仍存活者逐个 `SIGKILL`（starttime 不一致即 PID 复用，绝不发信号），每轮把存活名单成员的新后代并入名单；四观测量（`cgroup.procs` 空、`populated 0`、逃逸 PID 不存在、快照无后代）同时成立才报 `Cleaned`，否则 `StillAlive`。绿（WSL Ubuntu 24.04 LTS，root，cgroup2fs，内核 6.18.33.2-microsoft-standard-WSL2）：`[evidence] cgroup_left after outcome=Cleaned escaped_alive=false members=Ok([]) populated=Ok(0) descendants=[] unobservable=None`、`test result: ok. 27 passed`。枚举权威性：`cgroup_scope_enumerates_adopted_escapee` 增断言「根退出、逃逸者被收养（`escaped_ppid` 不再是根）后 `members == vec![escaped]`」——成员集合本身是权威枚举，不依赖父链；新增反例用例 `cgroup_members_unreadable_is_not_empty`（Active 但 `cgroup.procs` 读不出 → `[evidence] cgroup_unreadable ... unobservable=Some("cgroup_procs_unreadable:...") is_empty=false`），并逐条写清退化条件（无写权限→`escaped_scope_unobservable` 退回进程组旁证；无 `cgroup.kill`→枚举仍权威、只影响终止；已离开 `cgroup.procs`→作用域不含它，只在脱离前靠父链可见）。`cgroup_without_kill_reports_still_alive` 口径同步：v1.45 按名单杀掉已知逃逸者（`escaped_alive=false`），但静态记账仍非空 → 如实 `StillAlive`，不因自己的名单放行。能力位：`timeout.cleans_entire_process_tree` 与 `timeout.enumerates_escaped_descendants` 改为 `true/proven`（`platforms_verified=[linux]`、`platforms_unverified=[windows,macos]`，scope/detail 写明 platform=linux + cgroup v2 + `cgroup.kill` + 内核 VERSION 与残余边界）；新增 `timeout.cleans_entire_process_tree_windows` 与 `timeout.enumerates_escaped_descendants_windows` 两条 `false/not_proven`，reason 均为 `requires_out_of_job_enumerable_terminable_os_primitive`，detail 明写「需要作业外进程可枚举可终止的 OS 原语，本机不存在」。条目 17→19，`not_proven_total` 仍 2（均为 Windows 结构性条目）。`mcp_contract` 的平台断言同步。未改 `tools/list` 描述，`tools_sha256` 不变（`85da0303…`，门禁通过）。回归：Windows `21 lib + 125 integration`、WSL `27 lib + 123 integration` 全绿（WSL 日志 `/root/.aw-posix-verify/logs/posix-verify-20261108-033331.log`）。详见审计 §49 |
| v1.46 | 2026-11-08 | **真实链路端到端验证（MCP over stdio，不是夹具）+ 双平台**：新增驱动 `scripts/real-chain-verify/verify.py`（复制真实可执行文件、隔离 `runs/`、逐行 JSON-RPC、请求/响应落 `wire.jsonl`）。A：真实小任务产出文件，`attempt-1/{dispatch,handback,stdout,stderr}`、`.inflight` 空、`artifacts[].hash` 等于盘上 sha256、`receipt.get` 同一条、无 `*.tmp`（Windows + Linux）。B1：真超时 `status=blocked`、`blocked_reason` 三态文案如实（Windows 2090ms 带挂起态/BREAKAWAY 子句，Linux 2509ms 无），返回后心跳静默。B2：产品无 cancel 工具（`tools/list` 8 项实测），取消只能以杀 bridge 进程触达——重启后 startup 清扫审计 `trigger=startup`、`stale_reason=pid_missing`、`removed_total=1`；attempt-1 留在原位且只有 dispatch、无 handback，`receipt.get` 回「回执不存在」，不存在「completed 但产物不在」。B3：5s 容差内重启保留标记（`pid_missing_grace`、`stale=false`）且不写审计，过容差才删。C1：同 id 顺序重投回放逐字段相同（dispatched 口径不引入 `replayed`），异参具名 `task_id_conflict` 且零执行。C2：同 id 两进程同时派只落一个 attempt、命令只跑一次、回执逐字段相同。C3：两 id 并行时 inflight 标记各自点名，两个任务目录与产物互不混入。发现并修复测试基建 `ETXTBSY` 竞态（红原文 `ExecutableFileBusy: "Text file busy"` + `PoisonError`，`121 passed; 2 failed`；新增 `spawn_bridge()` 仅对 ETXTBSY 有界重试）；另记录 `_session/` 不进入 `runs.prune` 回收（未修，待决策）与 Linux bridge 死亡遗留子进程（未承诺路径）。未改 `tools/list` 描述，`tools_sha256` 不变（`85da0303…`）。回归：Windows `21 lib + 125 integration`、WSL `27 lib + 123 integration`（连跑两轮）全绿。详见审计 §50 |
| v1.47 | 2026-11-08 | **按顺序落地三件事，并把真实链路两项空白补齐（先红证据、双平台实跑）**。**(一) Linux 孤儿兜底**：`process_scope::configure` 在 `process_group(0)` 之后给子进程装 `PR_SET_PDEATHSIG(SIGKILL)`（`pre_exec` 内设置；设置后立即复查 `getppid()`，父已死则 `_exit(128)`，堵住 fork→prctl 竞态）。真实链路 B2 从 v1.46 的 `child_survived=True`（心跳 20→35）变为 `child_survived=False`（心跳 20→20），Windows 同为 `False`；「先完成任务再杀 bridge」的产物与 handback 字节不变（正常退出/已完成任务不受影响）。**边界如实写清**：`PDEATHSIG` 不遗传，孙进程仍存活（B5 WSL `grandchild_survived=True`，心跳 10→25），故契约 §2.7 给出运维兜底（巡检 `runs/.inflight` 标记 + cgroup 清单，实测 `populated 1` pid 9428 → `kill -9` → `populated 0` → `rmdir ok`）。**(二) `_session` 保留**：新增 `AW_SESSION_RETENTION_DAYS`（1..=3650，默认 30）与 `sweep_session_records`，在 `startup`/`periodic` 档执行；只删「文件名毫秒时间戳与 mtime 都过期」的记录，旧格式/不可解析/未来时间/只有一边过期一律保留；先写 `runs/session-retention.jsonl`（`session-retention/1`，plan+result）再删，写不进审计则零删除；口径与任务目录分开（§2.10），`runs.prune` 候选与不变式零变化。**(三) 显式取消 `task.cancel`**：入参 `task_id`/`reason`，只对**本进程**持有子进程的执行生效；取消只置位、由执行线程走与超时相同的终止流程并落盘 `status=canceled`（非 blocked/completed），`blocked_reason` 前缀 `canceled：` 并与 `timeout：`/`failed` 可区分；四态返回 `canceled`/`completed_before_cancel`/`cancel_requested`/`not_running`，只有 `canceled` 才 `cleanup_claimed=true`，看不到的进程不谎报已清理（沿用四观测量）；`runs.prune.status` 扩为四值。**真实链路补跑**：B4（同连接执行中取消，两平台绿）、B5（Linux 孙进程边界，如实存活）、D1（bridge 猝死后 1s 周期清扫点名删除，startup 容差内不删）、D2（Linux `setsid` 孙进程写出旁路 cgroup，超时后 `escaped_pid` 消失、`procs` 空、`populated 0`，平台补齐）、D3（45 天前删除/1 天后保留/坏名保留，实测 106.7–106.8/107.0 B 每条）。`tools/list` 8→9，`tools_sha256` 更新为 `744da2614a3730f08d309033b7ca86c956edfab82d369443f06fd4ceecaba9da`（门禁 G1/G20/G21 同步）；另修并发 prune 的 `batch_id` 同毫秒冲突（加原子序号，Windows 两条 race 用例连跑 3 轮全绿）。回归：Windows `21 lib + 128 integration`、WSL `27 lib + 126 integration`；真实链路两平台各 `17 evidence / 0 failures`。详见审计 §51 |
| v1.48 | 2026-11-08 | **把 v1.47 的两条「已知限制 + 人工巡检」改成自动收敛，并让取消跨进程可见**。**(一) 启动回收遗留 cgroup 作用域**：新增 sidecar `runs/.cgroup-scopes/<scope>.json`（`cgroup-scope/1`，带 `bridge_pid`/`bridge_start`/`task_id`/`attempt`/`root_start`），启动时扫描 `aw-mcp-bridge-*`，判据是「sidecar 自洽 **且** `attempt-<n>/dispatch.json` 存在 **且** owner bridge 已死（`/proc/<pid>/stat` starttime 防 PID 复用）」——不是「看着像孤儿」；任一不满足只报不删，`kept` 行写 `reason`/`action=none`/`members_before`/`members_after`。审计 `runs/cgroup-reclaim.jsonl`（`cgroup-reclaim/1`，`trigger=startup`），每个扫到的作用域都有一行；空作用域直接删目录，非空先 plan 审计再 `cgroup.kill`/逐成员 SIGKILL，3s 内轮询空且 `populated 0` 才删。**(二) `task.cancel` 跨进程**：请求落盘 `runs/.cancel/<task_id>.json`（`cancel-request/1`，`requested_by`/`requestedAt`/`target{pid,startedAt}`），执行侧 `CancelHeartbeat` 在 spawn 前与执行循环每 200ms 读取并原子改状态；三态 `pending`（已落盘未读）/`acknowledged`（已读在清理）/`done`（收尾完成）可在时间戳上区分，`ignored` 为 target 不符不误杀；响应新增 `cross_process`/`request_state`/`requested_at`/`acknowledged_at`/`finished_at`/`handback_status`/`cleanup_detail`，`cleanup_claimed` 仍只有 `canceled` 为 true；请求清扫写 `runs/cancel-requests.jsonl`（`cancel-requests/1`），判不准保留。**(三) B5 自动收敛**：bridge `kill -9` 后孙进程存活 + 作用域残留 → 重启后孙进程消失、作用域与 sidecar 删除、`attempt-1`/`dispatch.json` 保留，审计 `outcome=reclaimed`；负向 decoy 无 sidecar → 目录保留且 `outcome=kept`/`reason=owner_record_missing`。`tools/list` 9 项不变，`tools_sha256` 更新为 `9f1ee7443846d6c0e46e40e7c8f9f0ff206105a13444c1e191dc261b65c0203c`（门禁 G20/G22 同步）。详见审计 §52 |
| v1.49 | 2026-11-08 | **两条不稳定用例诊断 / 定性 / quarantine，并实证 B7 纯文件跨进程取消**：`descendant_snapshot_cross_checks_job_accounting` 与 `standalone_active_attempt_deleted_is_reported_not_silent_ok` 的偶发失败分别定性为「并发负载下前提未建立」与「夹具删除早于证据文件创建」两类夹具问题，按定性结果 quarantine/加固，判据与产品行为零变化；B7 证明取消不依赖第二条 MCP 连接：取消方是独立进程，只写 `runs/.cancel/<task>.json`，执行侧心跳读到后收敛，三态/target 绑定/`handledBy`/handback `canceled` 两平台实测（见 §2.12）。`tools_sha256` 当时为 `9f1ee7443846d6c0e46e40e7c8f9f0ff206105a13444c1e191dc261b65c0203c`。详见审计 §53 |
| v1.50 | 2026-11-08 | **A 类缺陷的复现驱动 / 根因定向 / quarantine 临时化，门禁负向对照常驻，并补 PM 接入须知**：把 A 类联调缺陷逐条写成可重复执行的驱动与断言，失败时先区分「产品缺陷」与「夹具前提未建立」；描述门禁的扰动证明从一次性手工操作改为常驻负向对照用例；新增 `docs/PM-INTEGRATION-GUIDE.md`（只依赖公开 MCP 接口与 `runs/` 目录约定）与 `scripts/pm-integration-selftest/pm_selftest.py`（正常派单 / 超时 / 取消三种 PM 行为），并在 §2.13 给出接入须知与自测入口。未改执行语义与能力位。详见审计 §54 |
| v1.51 | 2026-11-08 | **按 PM 联调实测收口 B1 / B2 / A3 / B4（12 处文档缺陷同步处理）**。**B1（改文档，不改行为）**：执行中同参重复派发不是「立即返回 `task_in_progress`」，而是阻塞到原执行结束再原样回放回执，上限 `timeout_secs + 5s`（≤605s）；等待 `attempt-1/dispatch.json` 最多 5s（仍缺失才 `task_id_conflict`）；N 个并发重投各自阻塞回放，`task.cancel` 仍可在主循环处理。理由：`task.execute` 本就阻塞，重投多为传输层重试，立即报错会把合法重试变成假失败；且行为有明确上限。实测：3s/11s 窗口均未提前返回，原执行结束后回放同一 `finishedAt`。写入 §2.6.11 与 `PM-INTEGRATION-GUIDE.md` §2.4。**B2（改文档）**：`task.cancel` 实测 110ms 返回 `status=canceled`、`canceled=true`、`cleanup_claimed=true`、`request_state=done`、`handback_status=canceled`，并含未文档化的 18 键 `handback`（完整 canceled 回执）与 `message`；只有等不到回执（`CANCEL_WAIT_SECS=10s` 超时）才返回 `cancel_requested`。§4.3 改用实测返回并补字段说明。**A3（改代码 + 文档）**：新增机器可读 `cleanup_status`（`cleaned`/`still_alive`/`unavailable`/`not_started`/`null`），写入 handback、`task.cancel` 响应与取消请求 `done` 终态；`cleanup_claimed` 收紧为 `handback_status=canceled && cleanup_status=cleaned`（此前「只要 canceled 就 true」会把「取消成功但进程可能还在」写成「已清理」）；文件通道等效判据为 `status=done && handback_status=canceled && cleanup_status=cleaned`。回归 `cancel_request_terminal_state_never_overclaims_cleanup` 对真实取消记录断言 true，并对 `still_alive` / `unavailable` / 缺字段 / `status=acknowledged` / `handback_status=ok` 五种部分条件变异断言 false。**B4**：`status` 顶层新增 `contract_version="1.51"`（与本文档标题一致），握手 `serverInfo.version="0.1.0"` 明确只是二进制版本。其余 12 条按 S1/S2/S3 逐条以实测为准处理，逐条状态见 PM 联调缺陷清单回填。`tools_sha256` 更新（门禁 G10/G20 同步）。详见审计 §55 |

