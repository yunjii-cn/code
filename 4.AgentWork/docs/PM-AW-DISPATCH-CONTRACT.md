# PM → AW 派单与回执契约 v1

状态：草案，待 PM 侧确认第 8 节的 3 个问题
适用范围：PM 向 AW 的 `mcp-bridge` 派单、AW 回执、失败处置
本批约束：**只定义契约，不改实现**（`server.rs`、测试、认证均未动）

## 0. 结论

- 身份复用 MCP 已有的 `initialize.params.clientInfo` 与 stdio 进程归属，**不新造 token/凭据格式**；当前只作为审计与弱识别，不构成准入。
- `task_id` 是唯一幂等键。**重投 = 返回既有回执、复用既有 attempt 编号，零执行、零新目录**；有意重跑必须换新 `task_id`。
- 回执完全映射到已实现字段，不新增第二套字段；失败原因只借用既有的 `blocked_reason` 字符串前缀做分类。
- 一句话边界：**AW 只执行，不验收、不判停、不定义判据、不写 L0/事件；PM 是唯一写权威。**

## 1. 术语

| 术语 | 含义 |
|---|---|
| 派单 | PM 调用 `task.execute` 的一次逻辑任务，由 `task_id` 唯一标识 |
| 重投 | 同一次派单因传输/重试被再次发送（`task_id`、`cmd`、`cwd`、`timeout_secs` 完全一致） |
| 重跑 | PM 有意重新执行（判据不变或变更），须使用**新的** `task_id` |
| attempt | 同一 `task_id` 下的一次执行记录，落盘于 `runs/<task_id>/attempt-<N>/` |
| 回执 | `task.execute` 的返回值，持久化为 `handback.json`，可由 `receipt.get` 读回 |

## 2. 身份

### 2.1 身份来源与传递形式

| 项 | 定义 |
|---|---|
| 派单方身份 | PM。以 MCP `initialize` 的 `params.clientInfo{name, version}` 随会话传递（协议既有字段） |
| 执行方身份 | AW。以 `health` 返回的 `server.name` / `server.version` / `server.buildTime` 标识 |
| 通道 | MCP stdio，一问一答；一个 bridge 进程同一时刻只服务一个启动方（1:1） |
| 进程归属 | 谁拉起 bridge 进程即谁是调用方；这是当前的**事实准入边界** |

**不新造凭据**：不使用自定义 header、token、签名或独立握手字段。身份以 `clientInfo` 记录在原会话内，属于弱识别，可被自称伪造，因此**不得单独作为授权依据**。

### 2.2 当前强度声明

- v1 不做密码学认证，不做准入拦截；`clientInfo` 仅用于回执关联与审计追溯。
- 真正的准入边界是“本地进程由谁启动 + 本地配置由谁可写”，本轮不在契约内展开。
- 引入认证时只允许在**会话层**（`initialize` 之后、`tools/call` 之前）加一次性凭据，且必须复用 PM/MH 已有通道；不得在每条请求体里新造字段。

## 3. 幂等

### 3.1 规则

1. `task_id` 是唯一幂等键，格式沿用实现校验：字母或数字开头，仅含字母数字 `. _ -`，长度 ≤ 64。
2. **重投**（`task_id` + `cmd` + `cwd` + `timeout_secs` 全等）必须：
   - 直接返回该 `task_id` 的最新回执；
   - 不启动新进程、不创建新 attempt 目录；
   - `attempt` 与被重投的那次完全一致。
3. 同一 `task_id` 但 `cmd`/`cwd`/`timeout_secs` 任一不同 → **拒绝**（视为契约违规），不得覆盖或追加，必须显式报错并要求新 `task_id`。
4. **重跑**一律使用新 `task_id`（建议后缀 `-r2`、`-r3`，由 PM 生成），不得用“同 id 再调一次”表达重跑。
5. 一个 `task_id` 对应 PM 侧一条派单记录；AW 侧 `attempt > 1` 仅允许出现在：崩溃恢复、人工运维重试、或未来由 PM 显式授权的重试语义（v1 未定义该输入）。

### 3.2 与“保留历史”的关系

上一轮实现的“同 id 重跑追加 attempt、历史不覆盖”仍然成立，但它保护的是**执行记录**，不是**传输重投**。v1 要求把两者分开：重投不得产生新 attempt。当前实现无法区分二者（见第 5 节 G1）。

## 4. 回执

字段一律沿用已实现 handback，不新增第二套命名。

| 字段 | 类型 | 语义 | 当前实现 |
|---|---|---|---|
| `task_id` | string | 派单标识 | 已实现 |
| `attempt` | integer | 该 id 的第几次执行，从 1 起连续 | 已实现（原子取号） |
| `status` | `ok` / `failed` / `blocked` | 仅描述进程结果，**不是验收结论** | 已实现 |
| `verify_cmd` | string | 实际执行的命令（回显派单参数，供 PM 逐字比对判据） | 已实现 |
| `exit_code` | integer / null | 进程退出码；`blocked` 时为 null | 已实现 |
| `changed_files` | string[] | 执行前后 `git status --porcelain` 差集，相对路径，最多 20 条 | 已实现（有 20 条上限） |
| `commit` | string / null | 执行时工作目录的短 HEAD | 已实现 |
| `stdout_digest` | `sha256:<hex>` | stdout 原始日志摘要 | 已实现 |
| `artifacts` | object[] | `{kind, locator, hash}`，locator 为相对路径 `runs/<task_id>/attempt-<N>/stdout.txt` 等 | 已实现 |
| `duration_ms` | integer | 执行耗时（契约要求可审计，字段已存在） | 已实现 |
| `stderr_digest` | `sha256:<hex>` | stderr 原始日志摘要 | 已实现（既有附加字段） |
| `blocked_reason` | string / null | 仅 `blocked` 时非空，见第 6 节 | 已实现（自由文本，未分类） |

回执使用规则：

- PM **不采信 `status` 自述**，只按 `exit_code`、`verify_cmd` 与判据的逐字一致性、`stdout_digest` 与 `artifacts` 中原始日志的重算一致性判定。
- `status=ok` 只表示进程退出码为 0，不表示验收通过。
- 历史读取：`receipt.get` 默认返回最新一次；`include_history=true` 返回 `attempts / latest / history[]`，用于回答“执行过几次、每次 exit_code 与耗时”。

## 5. 契约 vs 现有实现

| # | 契约项 | 状态 | 说明 / 缺口所在层（本批不补） |
|---|---|---|---|
| 1 | 回执字段映射（第 4 节） | **一致** | 全部字段已实现，含 `attempt`/`duration_ms`/`artifacts` 哈希 |
| 2 | attempt 编号连续无洞、并发不撞号 | **一致** | `create_dir` 原子占位，并发取号已在实现中 |
| 3 | 边界（不验收、不写 L0/事件、不成为写权威） | **一致** | bridge 无任何写权威接口，只落本地 `runs/` |
| 4 | `blocked_reason` 分类（第 6 节） | **部分一致** | 仅 `timeout` 有原因字段；缺命令/cwd 目前是工具错误（`isError`），未转为 blocked 回执；无机器可读分类码 |
| 5 | 身份传递与记录（第 2 节） | **缺** | `initialize` 未读取/记录 `clientInfo`；无会话身份审计。补在 MCP 会话层 |
| 6 | 重投幂等（第 3 节） | **缺** | 当前每次 `task.execute` 都新建 attempt，无法识别重投。补在 `tools/call` 入口、取号之前 |
| 7 | 同 id 异参拒绝 | **缺** | 当前不比对 `cmd`/`cwd`/`timeout_secs`，直接新建 attempt |
| 8 | 超并发上限、上游拒单 | **缺（不在 AW）** | AW 无并发闸门；上游拒单属 PM 侧，AW 不产生该码 |

统计：**已一致 3 条、部分一致 1 条、缺 4 条**（其中第 8 条按分工归 PM 侧）。

### 5.1 缺口补齐规划（后续批次）

| 缺口 | 补齐层 | 只管什么 |
|---|---|---|
| G1 重投幂等 | `mcp-bridge` 的 `tools/call` 入口，先查同 id+同参的最新回执，命中即返回 | 零执行、零新 attempt |
| G2 同 id 异参拒绝 | 同上，比对参数后返回具名错误 | 不改 schema |
| G3 失败码分类 | 参数校验层：`command_not_found`/`cwd_not_found` 转为 `blocked` + 前缀码；`timeout` 复用 | 只约定 `blocked_reason` 前缀，不加字段 |
| G4 身份记录 | MCP `initialize` 会话层，记录 `clientInfo` 供审计 | 不做准入 |
| G5 并发上限 | AW 侧计数（未定策略，可先在 PM 侧排队） | 拒单时不占用 attempt |
| G6 上游拒单 | PM 侧，不进入 AW | AW 不感知 |

## 6. `blocked` 失败码分类

分类以 `blocked_reason` 的**前缀**表达（如 `timeout: 超过 120s 未结束`），不新增字段。

| 分类 | 产生方 | 触发条件 | 调用方（PM）处置 |
|---|---|---|---|
| `command_not_found` | AW | 命令不存在或不可执行（当前表现为工具错误，待 G3 转为 blocked） | 修正 `cmd` 或环境；**不得原样重投**，修正后换新 `task_id` 重派 |
| `cwd_not_found` | AW | `cwd` 不存在或不是目录（当前为工具错误，待 G3） | 修正工作目录/仓库路径；换新 `task_id` 重派 |
| `timeout` | AW | 超过 `timeout_secs` 未结束，已终止进程 | 评估拆小任务或延长超时；**换新 `task_id` + 新参数**，不得假设原任务可续跑 |
| `upstream_rejected` | PM | PM 在派单前拒绝（判据不成立、权限不足、依赖未就绪） | 不派单；记录 blocker；条件解除后再派 |
| `concurrency_limit` | AW（未来） | AW 达到并发上限，**未占号、未执行** | 可安全地以**同 `task_id` 重投**（幂等），按退避重试 |

约束：

- `blocked` 必须带 `blocked_reason`，禁止空字符串；空原因视为契约违规。
- `blocked` 的 `exit_code` 为 null；`status` 只描述进程结果，不构成验收。
- 未被识别的 `blocked_reason` 前缀按最严格策略处理：不自动重试，由 PM 人工裁定。

## 7. 边界（写死）

**AW 只执行：不验收、不判停、不定义判据、不写 L0/事件。PM 是唯一写权威。**

由此派生：

- AW 不写项目状态、任务、审批，不改 TK 的 L0/L1/L2，不发 `[task]/[delivery]/[verification]/[blocker]` 标记。
- AW 不判定 `ok` 是否代表“完成”，也不提供“验收通过/退回/升级”结论。
- AW 不派单给其他 AI 工具，不做第二个协调者。
- PM 验收后按 TK 既有事件 schema 记账；AW 仅提供回执证据。

## 8. 待 PM 确认（3 条）

1. **通道归属**：PM 是否直接以 stdio 子进程方式拉起 `mcp-bridge`？若是，身份锚点即该进程归属；若经 MH 拉起，需要 MH 提供“这个 bridge 进程属于哪个调用方”的可信信息，否则第 2 节只能停留在弱识别。
2. **重跑表达**：是否接受“重跑必须换新 `task_id`”（推荐，零新增字段）？还是需要“同 id + 显式重试标记”的表达方式（将扩展 `task.execute` 输入契约，回执字段不变）。
3. **失败码归属**：确认 `upstream_rejected` 由 PM 自行记录、AW 不产生；以及 `concurrency_limit` 是先由 PM 侧排队，还是需要在 AW 内置限流闸门（后者需定义上限与拒绝语义）。

## 9. 变更记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1 | 2026-11-05 | 首版：身份、幂等、回执映射、失败码分类、边界、实现对照 |
