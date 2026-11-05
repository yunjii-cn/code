# PM → AW 派单与回执契约 v1

状态：草案，待 PM 侧确认第 10 节的 4 个问题（其中第 1 条通道归属已冻结，等用户拍板）
适用范围：PM 向 AW 的 `mcp-bridge` 派单、AW 回执、失败处置
实现状态：G1 / G2 / G3 / G4（仅审计）已落地（见 §11）；v1.3 起回执补充执行上下文；v1.4 起支持 `env` 透传；v1.5 起提供 `runs.list` 只读枚举；v1.6 起派单与回执带 UTC 时间戳；v1.7 起 `env.PATH` fail-closed（不许整体覆盖）；v1.8 起 `runs.list` 的 `total` 与 `attempts` 恒等；v1.9 起 `path_prepend` 已实现；v1.10 起 `path_prepend: []` 与缺省等价；证据保留与清理**只有方案、未实现、默认关闭**（见 §8）；认证与并发上限仍未实现

## 0. 结论

- 复用 MCP 已有的 `initialize.params.clientInfo` 与 stdio 进程归属，**不新造 token/凭据格式**；`clientInfo` 只作**审计弱识别**，不构成身份、不构成准入。
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

## 2. 调用方标识（审计弱识别）

### 2.1 来源与传递形式

| 项 | 定义 |
|---|---|
| 调用方自称 | MCP `initialize` 的 `params.clientInfo{name, version, ...}`（协议既有字段）。**仅作审计弱识别，不是身份、不构成准入** |
| 执行方标识 | AW。以 `health` 返回的 `server.name` / `server.version` / `server.buildTime` 标识 |
| 通道 | MCP stdio，一问一答；一个 bridge 进程同一时刻只服务一个启动方（1:1） |
| 进程归属 | 谁拉起 bridge 进程即谁是调用方；这是当前的**事实准入边界** |

**不新造凭据**：不使用自定义 header、token、签名或独立握手字段。

**审计落点与边界（写死）**：`initialize` 收到的 `clientInfo` 原样落盘到 `runs/_session/initialize.json`（含 `receivedAt`），并由 `status.session` 只读回显。该记录：

- **不构成身份**，不产生任何凭据或会话标识；
- 调用方可任意自称/伪造，内容与真实调用方无关；
- **任何拒绝/放行（准入）逻辑都不得读取它**；它只用于事后审计与问题追溯。

### 2.2 当前强度声明

- v1 不做密码学认证，不做准入拦截；`clientInfo` 已记录（`runs/_session/initialize.json` + `status.session`），但**仅用于审计追溯，不参与任何准入判断**。
- 真正的准入边界是“本地进程由谁启动 + 本地配置由谁可写”，本轮不在契约内展开。
- 引入认证时只允许在**会话层**（`initialize` 之后、`tools/call` 之前）加一次性凭据，且必须复用 PM/MH 已有通道；不得在每条请求体里新造字段。

## 3. 幂等

### 3.1 规则

1. `task_id` 是唯一幂等键，格式沿用实现校验：字母或数字开头，仅含字母数字 `. _ -`，长度 ≤ 64。
2. **重投**（`task_id` + `cmd` + `cwd` + `timeout_secs` + `env` 全等）必须：
   - 直接返回该 `task_id` 的最新回执；
   - 不启动新进程、不创建新 attempt 目录；
   - `attempt` 与被重投的那次完全一致。
3. 同一 `task_id` 但 `cmd`/`cwd`/`timeout_secs`/`env` 任一不同 → **拒绝**（视为契约违规），不得覆盖或追加，必须显式报错并要求新 `task_id`。
4. **重跑**一律使用新 `task_id`（建议后缀 `-r2`、`-r3`，由 PM 生成），不得用“同 id 再调一次”表达重跑。
5. 一个 `task_id` 对应 PM 侧一条派单记录；AW 侧 `attempt > 1` 仅允许出现在：崩溃恢复、人工运维重试、或未来由 PM 显式授权的重试语义（v1 未定义该输入）。

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
| 追加目录 | 若确实需要往 PATH 追加目录，必须用独立显式字段 `path_prepend`（名字在此写死），由 AW 显式拼接并在本节另起一行写明顺序；**当前未实现**，`env.PATH` 永远不承担这件事 |
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
| `changed_files_truncated` | boolean | 差集是否超过 20 条而被截断；`true` 表示还有未列出的改动 | 已实现 |
| `cwd` | string | 本次实际执行目录（规范化绝对路径；Windows 下可能带 `\\?\` 前缀），供 PM 复算 `changed_files` / `commit` | 已实现 |
| `timeout_secs` | integer | 本次实际生效的超时秒数（含默认值 120），用于解释 `timeout` 类 `blocked_reason` | 已实现 |
| `env_keys` | string[] | 本次实际覆盖/追加的 env **键名**（不含值），让 PM 知道哪些变量被改写 | 已实现（v1.4） |
| `commit` | string / null | 执行时工作目录的短 HEAD | 已实现 |
| `stdout_digest` | `sha256:<hex>` | stdout 原始日志摘要 | 已实现 |
| `artifacts` | object[] | `{kind, locator, hash}`，locator 为相对路径 `runs/<task_id>/attempt-<N>/stdout.txt` 等 | 已实现 |
| `duration_ms` | integer | 执行耗时（契约要求可审计，字段已存在） | 已实现 |
| `stderr_digest` | `sha256:<hex>` | stderr 原始日志摘要 | 已实现（既有附加字段） |
| `blocked_reason` | string / null | 仅 `blocked` 时非空，见第 6 节 | 已实现（自由文本，未分类） |
| `finishedAt` | string | 该 attempt **结束并落盘回执**的时刻，RFC3339/UTC/秒级/`Z`；`ok` / `failed` / `blocked` 一律写实际结束时间。**落盘回执中必填、不为 null**；不落盘的预检 `blocked` 回执不含此字段（它没有 attempt） | 已实现（v1.6） |
| `path_prepend` | string[] | 本次实际前插的目录列表（原样，不做规范化）；未提供时为空数组。只给目录名，不含任何 env 值 | 已实现（v1.9） |

回执使用规则：

- PM **不采信 `status` 自述**，只按 `exit_code`、`verify_cmd` 与判据的逐字一致性、`stdout_digest` 与 `artifacts` 中原始日志的重算一致性判定。
- `status=ok` 只表示进程退出码为 0，不表示验收通过。
- 历史读取：`receipt.get` 默认返回最新一次；`include_history=true` 返回 `attempts / latest / history[]`，用于回答“执行过几次、每次 exit_code 与耗时”。
- `cwd` / `timeout_secs` / `changed_files_truncated`（v1.3）与 `env_keys`（v1.4）属**纯增量字段**：既有字段语义未变；旧 `handback.json` 不含这些字段，`receipt.get` 原样返回、**不回填、不改写**。
- `dispatchedAt`（派单记录，§3.4）与 `finishedAt`（回执）同样**纯增量**：v1.6 起写入；旧记录缺失，`receipt.get` **原样返回、不回填**，消费方按“**缺失即未知**”处理。
- **年龄判定只认这两个时间戳**：`dispatchedAt` / `finishedAt` 之外的一切信号（mtime、目录顺序、文件名、`attempt` 编号）都不得用于推断年龄或作为清理判据。全文档只有这一套说法，§8 与本节一致。
- **`env_keys` 行为不变**：它只列"本次实际生效的 env 键名"。`env` 被拒（含 `env.PATH`）的派单**不占号、不落盘、不产生回执**，因此不会出现在任何回执或 `runs.list` 中。
- `attempt=null` 的 `blocked` 预检回执（缺命令 / cwd 不存在）不落盘，也不带执行上下文（没有实际执行目录与超时）。

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
| 10 | 证据保留与清理（第 8 节） | **缺（仅有方案）** | 本批只出方案：默认关闭、按结果分档、先记账后删除；代码未实现 |
| 11 | 证据时间戳 `dispatchedAt` / `finishedAt`（§3.4 / 第 4 节） | **一致** | v1.6 起随派单与回执落盘（UTC/秒级）；不参与幂等签名；旧记录不回填、缺失即未知 |
| 12 | `env.PATH` fail-closed（§3.2） | **一致** | 大小写任意变体一律 `invalid_env`，占号前拒绝；继承 PATH 永远保留；不静默忽略 |
| 13 | `runs.list` 的 `total` 与 `attempts` 一致性（第 7 节） | **一致** | `total == Σ attempts`（attempt 目录数，非任务数）；分页不改变 `total`；新增 `total_tasks` 供换算页数 |
| 14 | `path_prepend`（§3.5） | **一致** | 目录数组前插、继承 PATH 在后、分隔符按平台；参与幂等签名；占号前校验；回执只给目录名 |

统计：**已一致 12 条、缺 2 条**（第 8 条按分工归 PM 侧，AW 不做；第 10 条方案待拍板、未实现；认证仍待通道归属拍板）。

### 5.1 缺口补齐规划（后续批次）

| 缺口 | 补齐层 | 只管什么 | 状态 |
|---|---|---|---|
| G1 重投幂等 | `mcp-bridge` 的 `tools/call` 入口，先查同 id+同参的签名，命中即回放 | 零执行、零新 attempt | **已落地** |
| G2 同 id 异参拒绝 | 同上，比对签名后返回 `task_id_conflict` | 不改 schema | **已落地** |
| G3 失败码分类 | 参数校验层：`command_not_found`/`cwd_not_found` 转为 `blocked` + 前缀码；`timeout` 复用 | 只约定 `blocked_reason` 前缀，不加字段 | **已落地** |
| G4 身份记录 | MCP `initialize` 会话层，记录 `clientInfo` 供审计 | **仅审计，不做准入**；记录不得被放行逻辑读取 | **已落地（仅审计）** |
| G5 并发上限 | PM 侧排队产生（AW 不内置闸门） | 拒单时不占用 attempt | 归 PM 侧 |
| G6 上游拒单 | PM 侧，不进入 AW | AW 不感知 | 归 PM 侧 |
| G7 证据保留 | AW 落盘层 + 一个显式删除入口（默认关闭） | 只看时间阈值与结果分档，不做"按大小/按条数" | **仅有方案（§8），未实现** |
| G8 `env.PATH` fail-closed | `task.execute` 参数校验层（占号之前） | `env` 中出现 `PATH`（任意大小写）一律 `invalid_env`；不静默忽略 | **已落地（v1.7）** |

## 6. `blocked` 失败码分类

分类以 `blocked_reason` 的**前缀**表达（如 `timeout: 超过 120s 未结束`），不新增字段。

| 分类 | 产生方 | 触发条件 | 调用方（PM）处置 |
|---|---|---|---|
| `command_not_found` | AW | 命令不存在、不可执行，或缺少 `cmd`（占号之前判定，不产生 attempt） | 修正 `cmd` 或环境；**不得原样重投**，修正后换新 `task_id` 重派 |
| `cwd_not_found` | AW | `cwd` 不存在或不是目录（占号之前判定，不产生 attempt） | 修正工作目录/仓库路径；换新 `task_id` 重派 |
| `timeout` | AW | 超过 `timeout_secs` 未结束，已终止进程 | 评估拆小任务或延长超时；**换新 `task_id` + 新参数**，不得假设原任务可续跑 |
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
| 权威性 | **目录枚举结果不是执行状态的权威来源**：它只镜像磁盘现状，可能滞后或被人工改动；PM 判定某次派单必须用 `receipt.get`，不得以 `runs.list` 的 `status` 作为验收依据。若将来启用第 8 节的清理，**已清理的 task 是"不出现"，不等于"从未派单"**；`receipt.get` 的"回执不存在"同样只表示本机当前无证据 |

## 8. 证据保留与清理（**方案，未实现，默认关闭**）

本节只是方案，`runs/` 目前**没有任何清理规则**，代码未实现、未提交。清理一旦做错就是"证据被悄悄删掉"，比不清理严重得多，因此所有结论都按"宁可不删"取向。

### 8.0 前置事实（先核实，避免方案建在错误前提上）

- 证据只存在于**已占号**的 attempt：`runs/<task_id>/attempt-<N>/`（`dispatch.json`、`handback.json`、`stdout.txt`、`stderr.txt`）。占号前就被拒的 `blocked`（缺 `cmd`、`cwd` 不存在）**不落盘**，没有证据可删（见 §4 末条）。
- **时间戳（v1.6 起已落地）**：`dispatch.json` 有 `dispatchedAt`、`handback.json` 有 `finishedAt`（均 RFC3339/UTC/秒级，见 §3.4 与第 4 节）。**v1.6 之前的历史记录没有这两个字段**，只能按"年龄未知"处理（不补、不猜）。
- mtime 会被复制、解压、备份恢复、`touch` 改写，Windows 上目录 mtime 语义与文件不同。**以 mtime 为判据的自动清理不可靠**：删除判据只认 `dispatchedAt` / `finishedAt`，不认 mtime，这是本方案最重要的约束（与 §4 的"年龄判定只认这两个时间戳"是同一句话，不是两套说法）。
- attempt 目录名当前只会是 `attempt-1`（`task.execute` 不追加 attempt，重投走 `replay_existing` 回放）；`attempt-N`（N>1）目前只可能来自手工或未来的"显式重试"特性。

### 8.1 保留期限：三个可选方案

| 方案 | 规则 | 磁盘代价 | 可追溯性代价 |
|---|---|---|---|
| **A. 统一期限** | 所有 attempt 共用同一阈值 T（如 30 天） | 只由"派单量 × T"决定；大日志场景下会先撞磁盘、再被迫删近期证据 | 最差：成功、失败、超时证据同寿，追溯窗口与任务后果无关 |
| **B. 按结果分档（推荐）** | `ok`（`exit_code=0`）短档；`failed`（非 0）中档；`blocked`（含 `timeout:`）长档；**无 `handback.json`（未完成/崩溃）永不删** | 最省，且省在低价值证据上 | 较好；唯一风险在 `ok` 短档——若 PM 验收滞后于清理阈值，会删掉仍要用的成功证据 |
| **C. 按引用分档** | 被 `receipt.get` / `runs.list` 命中过的 task 记入引用表→长期；从未被引用的→短档 | 需新增引用记账落盘，本身又产生新证据 | 表面最好，实际自指：引用表也要保留，且把只读接口变成"会改变清理结果"的状态源，直接违背 §7 的零写原则 |

- 分档输入只用 **`status` + `exit_code` 的组合**，**禁止**按 `blocked_reason` 自由文本做规则（那等于把 §6.1 拒绝的启发式又请回来）。
- 建议阈值（仅为提案，待 PM 定）：`ok` ≥ 30 天、`failed` ≥ 180 天、`blocked` ≥ 365 天、无回执**永不删**。
- **推荐**：默认关闭；若开启，用方案 B，且 `ok` 档不得短于 30 天。方案 C 不采用。

### 8.2 删除权限

**删什么：**

- **只能整 task 删**（`runs/<task_id>/` 整目录），不删单个文件、不删单个 attempt 的部分内容。留下"有 `dispatch.json`、没 `handback.json`"的半证据，会让 `receipt.get`（"回执不存在"）与 `runs.list`（`handback=false`）给出互相矛盾的现象，且事后无法自证是清理还是崩溃。
- **未完成 attempt 一律不删**（无 `handback.json`）：它是"执行发生过但没结果"的唯一线索。
- `runs/_session/`、`runs/_audit/` **天然不在候选内**：规则显式排除 `_` 前缀目录，不依赖命名巧合。

**谁有权删（三种，权衡如下）：**

| 主体 | 可行性 | 留痕能力 | 结论 |
|---|---|---|---|
| 人工直接删目录 | 现实可行，AW 无法阻止（也不应假装能阻止） | **无**，只能事后比对审计日志里的"预期清单" | 不作为常规手段，但要在文档承认这个缺口 |
| PM 显式发起（如新增 `runs.prune`） | 需要**新增写接口**，属新契约面，必须单独拍板 | 完整（谁、何时、依据哪条规则、删了什么） | 唯一能"证据删证据且可查"的方式；**本批不做** |
| AW 定时自动 | 默认关闭，开启需显式策略文件 | 完整（`actor=aw:auto`） | 仅在 PM 明确要求时提供，且必须 dry-run + 上限 |

**删除对既有能力的破坏（必须写进契约，不能默默承受）：**

- 删掉 attempt 下的 `dispatch.json` → 同 `task_id` 重投变成 `task_id_conflict`（"已有运行记录但缺少派单签名，无法证明参数一致"），**幂等回放能力被破坏**。
- 删掉整个 task 目录 → 同 `task_id` 重投会**重新执行**，幂等被静默重置，可能重复产生副作用。
- 因此：**已派单的 `task_id` 一旦被清理，就不能再当作可安全重投的键**。PM 侧必须视为"该 id 退役"并换新 `task_id`。

### 8.3 删除留痕（先记账，后删除）

- 落点：`runs/_audit/retention.jsonl`，append-only JSONL，一行一个清理批次；`_` 前缀使 `runs.list` 自动排除。
- 每行字段：`at`（RFC3339/UTC）、`actor`（`human:<name>` / `pm` / `aw:auto`）、`rule`（规则 id + 当时阈值快照）、`dry_run`（bool）、`candidates[]`（`task_id`、`attempts[]`、每个文件的相对路径 + `sha256:<hex>`）、`deleted[]`、`skipped[]`（含原因）、`reason`（自由文本）。
- **先记账后删除**：先把候选清单与文件摘要写入并 `fsync`，再执行删除。中途崩溃会留下"已记录但未删完"的痕迹，可人工按 `sha256` 对账。**禁止"删完再记"**。
- **审计日志不能被同一次清理删掉**：它位于 `_audit/`，规则显式排除 `_` 前缀；"清理 `_audit` 自身"不在任何策略里，只能人工归档。
- 审计日志会无限增长：本批先声明"不自动删、人工归档"，不引入二级保留（否则又回到"证据删证据"）。
- 审计行**不含 env 值、不含 stdout/stderr 正文**，只含摘要与相对路径（沿用 v1.4 口径）。

### 8.4 默认关闭（fail-closed 口径）

**结论：默认关闭更合适。** 依据：清理的收益（磁盘）远小于误删的代价（不可恢复的可追溯性）；磁盘问题可由人工/外部归档解决，而删掉的证据无法证伪。实现口径：

- 没有策略文件 / 策略为空 / 出现未知键 / 解析失败 → **一律不删**。出错方向必须是"什么都不做"，不是"按内置默认值删"。
- 只允许"时间阈值 + 结果分档"两类规则。**不支持**"按总大小 / 按条数只留最近 N 个"——这类规则会在磁盘告急时优先删掉最新（最可能仍被引用）的证据。
- 任何删除**默认 dry-run**：只有显式 `confirm=true` 才真删；单批最多删 N 个 task，N 写死在策略里。
- 时钟保护：系统时间明显回拨或前跳时跳过本轮（宁可不删）。

### 8.5 与权威性的关系（对 §7 的补充）

- `runs.list` 的 `state=missing|empty` 与 `receipt.get` 的"回执不存在"，一旦启用清理，就只表示**本机当前没有证据**，**不表示"从未派单"或"未执行过"**；已清理的 task 是"不出现"，不是"不存在"。
- 判定某次派单是否执行过、结果如何，权威顺序是：**PM 自己的任务账本 > 未清理的 `receipt.get` > 审计日志**；`runs.list` 在任何情况下都不是权威。
- 该补充已写进 §7 的"权威性"行。

### 8.6 开启前的前置条件（不满足就不应实现）

1. ~~回执补纯增量字段 `finishedAt`（RFC3339），派单签名补 `dispatchedAt`；删除判据只认这两个字段，**不认 mtime**。~~ **已落地（v1.6）**：`finishedAt` / `dispatchedAt` 均已写入；删除判据只认这两个字段、不认 mtime。
2. 由 PM 明确"验收最大滞后"，才能给 `ok` 短档定下限。
3. 由 PM 明确承担"清理后 `task_id` 不得再重投"的后果（§8.2 末条），并保持 `task_id_conflict` 的既有文案不变。
4. 明确 `runs/_audit/retention.jsonl` 的查阅方式（至少人工可查；是否需要只读工具另议）。

## 9. 边界（写死）

**AW 只执行：不验收、不判停、不定义判据、不写 L0/事件。PM 是唯一写权威。**

由此派生：

- AW 不写项目状态、任务、审批，不改 TK 的 L0/L1/L2，不发 `[task]/[delivery]/[verification]/[blocker]` 标记。
- AW 不判定 `ok` 是否代表“完成”，也不提供“验收通过/退回/升级”结论。
- AW 不派单给其他 AI 工具，不做第二个协调者。
- PM 验收后按 TK 既有事件 schema 记账；AW 仅提供回执证据。

## 10. 待 PM 确认（4 条）

1. **通道归属**：PM 是否直接以 stdio 子进程方式拉起 `mcp-bridge`？若是，身份锚点即该进程归属；若经 MH 拉起，需要 MH 提供“这个 bridge 进程属于哪个调用方”的可信信息，否则第 2 节只能停留在弱识别。
2. **重跑表达**：是否接受“重跑必须换新 `task_id`”（推荐，零新增字段）？还是需要“同 id + 显式重试标记”的表达方式（将扩展 `task.execute` 输入契约，回执字段不变）。
3. **失败码归属**：确认 `upstream_rejected` 由 PM 自行记录、AW 不产生；以及 `concurrency_limit` 是先由 PM 侧排队，还是需要在 AW 内置限流闸门（后者需定义上限与拒绝语义）。
4. **证据保留**：时间戳已落地（v1.6），年龄现在**可判**；仍待定的是：是否启用 §8 的清理（默认建议**关闭**）、选方案 A 还是 B、三档阈值、删除主体（"PM 显式发起"还是"AW 定时自动"）、以及 PM 的验收最大滞后天数（决定 `ok` 短档下限）。

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
| v1.9 | 2026-11-06 | 实现 `path_prepend`（§3.5）：字符串数组按字面顺序前插、继承 PATH 在后、分隔符按平台（Windows `;` / POSIX `:`）；参与幂等签名；空数组/空元素/NUL/超 16 个/单项超 4096 字符 → `invalid_path_prepend`（占号前拒绝）；目录存在性不校验；回执新增 `path_prepend` 目录列表 |
| v1.10 | 2026-11-06 | 消除空集合口径不一致（选 A）：`path_prepend: []` 改为**与缺省完全等价**（执行相同、签名相同、重投命中同一 attempt），与 §3.2 的 `env: {}` 一致；两字段各自独立归一，"都缺省/都为空/一空一缺/一缺一空"四种组合签名全等 |
