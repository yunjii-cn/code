# PM → AW 派单与回执契约 v1

状态：草案，待 PM 侧确认第 10 节的 4 个问题（其中第 1 条通道归属已冻结，等用户拍板）
适用范围：PM 向 AW 的 `mcp-bridge` 派单、AW 回执、失败处置
实现状态：G1 / G2 / G3 / G4（仅审计）已落地（见 §11）；v1.3 起回执补充执行上下文；v1.4 起支持 `env` 透传；v1.5 起提供 `runs.list` 只读枚举；v1.6 起派单与回执带 UTC 时间戳；v1.7 起 `env.PATH` fail-closed（不许整体覆盖）；v1.8 起 `runs.list` 的 `total` 与 `attempts` 恒等；v1.9 起 `path_prepend` 已实现；v1.10 起 `path_prepend: []` 与缺省等价；v1.11 起超时文案只写"已发送终止信号、孙进程未验证"；v1.12 起提供显式两段式证据清理 `runs.prune`（**默认不删、无任何自动清理**，见 §8）；v1.13 起 `initialize` 审计改为逐份文件留存、`status.session` 聚合（并发不丢）；v1.14 起 `tools/list` 描述与契约单一措辞（env fail-closed + path_prepend 声明）；v1.15 起新增 `#3` 进程树杀灭的**设计评估**（仅文档，**未实现**，`NOT_PROVEN` 不变）；v1.16 起按窄定义实现 `#3` 的**受控集合**清理（POSIX 进程组 / Windows Job；Windows 采用 `spawn → Assign`，窗口存在），判据只用 `kill(-pgid, 0) == ESRCH` 与 `ActiveProcesses == 0`；v1.16.1 曾因状态门未满足退回旧文案；**v1.17 起状态门已满足（POSIX 四类用例实跑通过 + Windows 三类通过 + 版本递增），启用"已清理受控集合；逃逸进程是否仍在运行未验证"三态文案**，Windows 逃逸负例仍为 `UNPROVEN`；**整棵进程树仍不可判定，`#3` 保持 `NOT_PROVEN`**；认证与并发上限仍未实现

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

**审计落点与边界（写死）**：`initialize` 收到的 `clientInfo` 原样落盘为 `runs/_session/initialize-<pid>-<毫秒时间戳>-<序号>.json`（含 `receivedAt`），**每次 initialize 一份独立文件、互不覆盖**；`status.session` 只读聚合全部记录（含旧版单文件 `initialize.json`）。这些记录：

- **不构成身份**，不产生任何凭据或会话标识；**多份记录不等于多个可信会话**，也不等于"已认证"；
- 调用方可任意自称/伪造，内容与真实调用方无关；
- **任何拒绝/放行（准入）逻辑都不得读取它**；它只用于事后审计与问题追溯。

**为什么改为逐份留存（v1.13，写死取舍）**：多 bridge 共享同一 `runs/` 后，旧的单文件 `initialize.json` 是覆盖写——两个 bridge 同时启动会**静默丢掉**先写的那条，且双方都不报错。覆盖语义天生只能留"最后一次"，**不是完整审计**。v1.13 起每次 `initialize` 写一份新文件（文件名含 pid、毫秒时间戳与进程内序号，`create_new` 语义避免覆盖，tmp + rename 保证可见时内容完整），因此并发与先后写入都**不丢**。旧 `initialize.json` 保留可读并纳入聚合，**不回填、不改写**。

### 2.2 当前强度声明

- v1 不做密码学认证，不做准入拦截；`clientInfo` 已记录（`runs/_session/initialize*.json` 逐份留存，`status.session` 聚合回显），但**仅用于审计追溯，不参与任何准入判断**。
- 真正的准入边界是“本地进程由谁启动 + 本地配置由谁可写”，本轮不在契约内展开。
- 引入认证时只允许在**会话层**（`initialize` 之后、`tools/call` 之前）加一次性凭据，且必须复用 PM/MH 已有通道；不得在每条请求体里新造字段。

## 3. 幂等

### 3.1 规则

1. `task_id` 是唯一幂等键，格式沿用实现校验：字母或数字开头，仅含字母数字 `. _ -`，长度 ≤ 64。
2. **重投**（`task_id` + `cmd` + `cwd` + `timeout_secs` + `env` + `path_prepend` 全等）必须：
   - 直接返回该 `task_id` 的最新回执；
   - 不启动新进程、不创建新 attempt 目录；
   - `attempt` 与被重投的那次完全一致。
3. 同一 `task_id` 但 `cmd`/`cwd`/`timeout_secs`/`env`/`path_prepend` 任一不同 → **拒绝**（视为契约违规），不得覆盖或追加，必须显式报错并要求新 `task_id`。
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
- **`blocked_reason` 只描述"已执行的动作与受控集合的可判定结果"，不描述"整棵树的结果"**：`timeout:` 文案的作用域**写死为"受控集合"**（POSIX 进程组 / Windows Job Object），**不是整棵进程树**。v1.17 起三态与逐字文案：
  - 受控集合已清空：`timeout：超过 {timeout_secs}s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证`；Windows 额外写明 `Windows 下 spawn→Assign 窗口内派生的进程可能未被纳入作业`；
  - 受控集合内仍有存活进程：`timeout：超过 {timeout_secs}s 未结束，已向受控集合发送终止信号，但受控集合内仍有存活进程；逃逸进程是否仍在运行未验证`；
  - 受控集合未能建立或无法查询：`timeout：超过 {timeout_secs}s 未结束，受控集合未能建立或无法查询，仅向直接子进程发送终止信号；其派生的子进程是否仍在运行未验证`。
  - 判据只有两个可判定观测点：POSIX `kill(-pgid, 0)` 返回 `ESRCH`；Windows Job 的 `ActiveProcesses == 0`。**任何分支都不写"已终止进程树 / 无残留"**。`#3` 整棵进程树杀灭仍不可判定、状态为 `NOT_PROVEN`，调用方不得据此认为超时任务已彻底停止，需自行确认受控集合之外的残留进程（含 POSIX `setsid` 逃逸与 Windows `spawn → Assign` 窗口内派生的进程）。
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
| 15 | `timeout` 文案诚实性（第 4 / 6 节） | **一致** | v1.17：文案只描述**受控集合**（POSIX 进程组 / Windows Job）的清理结果，三态分流（已清理 / 仍存活 / 不可用），明写"逃逸进程是否仍在运行未验证"与 Windows `spawn → Assign` 窗口；任何分支都不出现"已终止进程树 / 无残留"；整树 `#3` 仍 `NOT_PROVEN`；证据见设计文档 §6.5（POSIX 四类实跑 + Windows 三类） |

统计：**已一致 14 条、缺 1 条**（第 8 条按分工归 PM 侧，AW 不做；认证仍待通道归属拍板）。

### 5.1 缺口补齐规划（后续批次）

| 缺口 | 补齐层 | 只管什么 | 状态 |
|---|---|---|---|
| G1 重投幂等 | `mcp-bridge` 的 `tools/call` 入口，先查同 id+同参的签名，命中即回放 | 零执行、零新 attempt | **已落地** |
| G2 同 id 异参拒绝 | 同上，比对签名后返回 `task_id_conflict` | 不改 schema | **已落地** |
| G3 失败码分类 | 参数校验层：`command_not_found`/`cwd_not_found` 转为 `blocked` + 前缀码；`timeout` 复用 | 只约定 `blocked_reason` 前缀，不加字段 | **已落地** |
| G4 身份记录 | MCP `initialize` 会话层，记录 `clientInfo` 供审计 | **仅审计，不做准入**；记录不得被放行逻辑读取；每次 initialize 一份文件，`status.session` 聚合 | **已落地（仅审计；v1.13 起逐份留存、并发不丢）** |
| G5 并发上限 | PM 侧排队产生（AW 不内置闸门） | 拒单时不占用 attempt | 归 PM 侧 |
| G6 上游拒单 | PM 侧，不进入 AW | AW 不感知 | 归 PM 侧 |
| G7 证据清理 | `runs.prune` 显式入口（默认 dry-run、无自动清理） | 只按 `finishedAt` 判龄、只删非最新 attempt 整目录、先记账后删除 | **已落地（v1.12）**；自动清理与按结果分档阈值**明确不做**（§8.8） |
| G8 `env.PATH` fail-closed | `task.execute` 参数校验层（占号之前） | `env` 中出现 `PATH`（任意大小写）一律 `invalid_env`；不静默忽略 | **已落地（v1.7）** |
| G9 超时文案诚实 | `task.execute` 回执生成处 | 文案只写**受控集合**（POSIX 进程组 / Windows Job）的清理结果与可判定判据（`kill(-pgid,0)==ESRCH` / `ActiveProcesses==0`），逃逸进程标"未验证"；不写"已终止进程树 / 无残留" | **已落地（v1.11 文案诚实；v1.16 受控集合实现；v1.17 新措辞启用）**：POSIX 四类用例实跑通过（3 unit + 61 integration）、Windows 三类通过（3 unit + 62 integration）；Windows 逃逸负例仍 `UNPROVEN`；整树杀灭仍不可判定，`#3` 保持 `NOT_PROVEN`；证据见 [`TIMEOUT-PROCESS-TREE-DESIGN.md`](./TIMEOUT-PROCESS-TREE-DESIGN.md) §6.5 |

## 6. `blocked` 失败码分类

分类以 `blocked_reason` 的**前缀**表达（如 `timeout: 超过 120s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证`），不新增字段。**前缀码后面的文案描述对受控集合已执行的动作与可判定结果，作用域不是整棵进程树**。

| 分类 | 产生方 | 触发条件 | 调用方（PM）处置 |
|---|---|---|---|
| `command_not_found` | AW | 命令不存在、不可执行，或缺少 `cmd`（占号之前判定，不产生 attempt） | 修正 `cmd` 或环境；**不得原样重投**，修正后换新 `task_id` 重派 |
| `cwd_not_found` | AW | `cwd` 不存在或不是目录（占号之前判定，不产生 attempt） | 修正工作目录/仓库路径；换新 `task_id` 重派 |
| `timeout` | AW | 超过 `timeout_secs` 未结束；对**受控集合**（POSIX 进程组 / Windows Job）发送 `SIGTERM → SIGKILL` / `TerminateJobObject`，并按 `kill(-pgid, 0) == ESRCH` / `ActiveProcesses == 0` 判定是否清空；逃逸进程未验证（整树 `#3` 仍 `NOT_PROVEN`） | 评估拆小任务或延长超时；**换新 `task_id` + 新参数**，不得假设原任务可续跑；**需自行确认受控集合之外的残留进程**（含 POSIX `setsid` 逃逸与 Windows `spawn → Assign` 窗口内派生的进程） |
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

**方向写死：默认什么都不删。** 清理一旦写错就是销毁证据，比不清理严重得多，因此本节每条规则都按"宁可不删"取向。**系统内不存在任何自动清理**：没有定时任务、没有启动时扫描、没有"超过 N 天自动删除"；唯一入口是调用方显式调用 `runs.prune`。

### 8.0 前置事实（已核实）

- 证据只存在于**已占号**的 attempt：`runs/<task_id>/attempt-<N>/`（`dispatch.json`、`handback.json`、`stdout.txt`、`stderr.txt`）。占号前就被拒的 `blocked`（缺 `cmd`、`cwd` 不存在、参数非法）**不落盘**，没有证据可删（见 §4 末条）。
- **时间戳（v1.6 起已落地）**：`dispatch.json` 有 `dispatchedAt`、`handback.json` 有 `finishedAt`（均 RFC3339/UTC/秒级，见 §3.4 与第 4 节）。**v1.6 之前的历史记录没有这两个字段**，只能按"年龄未知"处理（不补、不猜）。
- **mtime 不是判据**：可被复制、解压、备份恢复、`touch` 改写；Windows 上目录 mtime 语义与文件不同。删除判据只认 `finishedAt`，不认 mtime，也不认目录顺序、文件名或 `attempt` 编号（与 §4 的"年龄判定只认这两个时间戳"是同一句话）。
- `task.execute` 自身不追加 attempt（重投走 `replay_existing` 回放）；`attempt-N`（N>1）目前只可能来自手工运维或未来的"显式重试"特性。因此实践中被清理的候选很少，这是预期的。

### 8.1 触发方式：只有显式调用（写死）

| 项 | 规则 |
|---|---|
| 唯一入口 | `runs.prune` 工具。**不提供任何其他删除入口**：无命令行、无启动清理、无定时任务，`runs.list` 绝不隐式删除 |
| 默认行为 | 不调用就永不删除；调用时 `dry_run` 缺省为 `true` |
| 自动清理 | **不存在**：不读策略文件、不在启动时扫描、不按天数自动删。若要自动，必须另立契约重新评审，不属本批 |
| 删除对象 | 只可能是 `runs/<task_id>/attempt-<N>/` **整个目录**，且 `N` 严格小于该 task 的最大 attempt 编号 |
| 排除 | `_` 前缀目录（`_session` 等）、非任务目录、符号链接、`runs/` 根下任何文件一律不进入候选；不删任务目录之外的任何路径 |

### 8.2 两段式：先 dry-run，再显式 confirm

| 项 | 规则 |
|---|---|
| `dry_run` | 布尔，缺省 `true`。`true` 时**零写入**：不删目录、不创建/追加 `prune.jsonl`、不建目录、不改任何文件 |
| `confirm` | 布尔，缺省 `false`。**只有 `dry_run=false` 且 `confirm=true` 才真删**；缺一即拒绝 |
| 拒绝语义 | `dry_run=false` 而 `confirm` 不为 `true` → 工具错误 `confirm_required`，**零写入**；反之 `confirm=true` 但未显式 `dry_run=false` → 仍按 dry-run 处理（`confirm` 不隐含真删） |
| 阈值必填 | `older_than_days` 必填（整数，1..=3650）；缺失或越界 → 工具错误 `invalid_retention`，**零写入**。**不提供默认阈值**——"没写阈值"绝不能变成"按某个默认值删" |
| dry-run 输出（稳定可解析） | `dry_run=true`、`state`（`missing`/`empty`/`ready`）、`older_than_days`、`evaluated_at`（UTC）、`limit`、`candidates_total`、`truncated`、`candidates[]`、`skipped_total`、`skipped[]`；`candidates[]` 每项固定字段：`task_id` / `attempt` / `path`（相对路径 `runs/<task_id>/attempt-<N>`）/ `dispatchedAt` / `finishedAt` / `age_days` |
| `skipped[]` 原因码 | `latest_attempt_retained`（最新 attempt，永不删）/ `unfinished_or_missing_handback`（无回执或不可解析）/ `missing_dispatch`（缺派单签名，年龄与一致性都无法证明）/ `age_unknown`（回执无 `finishedAt`）/ `invalid_finished_at`（时间戳不可解析）/ `future_timestamp`（晚于当前时刻，按时钟异常跳过）/ `not_old_enough`（未达阈值）/ `unsafe_attempt_path`（符号链接或非真实目录） |
| 单批上限 | `limit` 缺省 20、范围 1..=100；按 `(task_id, attempt)` 升序取前 `limit` 个；`candidates_total > limit` 时 `truncated=true`。**真实删除只删被列出的这一批**，剩余的下一次调用再评估 |
| 两段一致性 | 相同参数、磁盘未变时，dry-run 的 `candidates[]` 与真实删除的 `deleted[]` **逐项一致**（同样的排序与同样的 `limit` 截断）；不一致即契约违规 |

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
| 整条链一致 | 只删**整个 attempt 目录**：先把目录整体移到同卷的 `runs/.prune-trash/<batch_id>/`（重命名一次完成，整链一起离开 attempt 命名空间），再删除该暂存目录；重命名失败 → 原样不动。**禁止**选择性删文件（例如只删 `dispatch.json` 留下孤立 attempt）。任一必要文件缺失或不可解析 → 该 attempt 不进入候选 |
| 删除后的可见变化 | 保留最新 attempt，`runs.list` 的 `attempts` 相应减少，`latest_attempt` 不变；`receipt.get` 仍可读回最新回执 |

### 8.5 删除留痕（先记账，后删除）

- 落点：`runs/prune.jsonl`，append-only JSONL；`runs.list` 只列目录，因此该文件不会出现在任务列表里。
- **真删（`dry_run=false` + `confirm=true`）且候选非空时**，每批追加两行，共享 `batch_id`：
  1. `phase="plan"`：**删除之前**写入并 `fsync`。字段：`at`（UTC）、`batch_id`、`older_than_days`、`candidate_count`、`candidates[]`（`task_id` / `attempt` / `path` / `dispatchedAt` / `finishedAt` / `age_days`）。
  2. `phase="result"`：删除结束后写入。字段：`at`、`batch_id`、`deleted[]`、`failed[]`（含错误原因）、`deleted_total`、`failed_total`。
- **先记账后删除**：`plan` 行写不进去（目录不可写、磁盘满）→ **放弃本轮删除**，返回工具错误，一个目录都不删。禁止"删完再记"。
- 崩溃可对账：只有 `plan` 没有 `result` = "已记录、未完成"，人工按清单核对；两行都在 = 可回答"删了什么、凭什么时候删的"。
- `dry_run=true` 或候选为空时**不写审计**（没有发生删除，也就不该产生删除记录）。
- 审计行**不含 env 值、不含 stdout/stderr 正文**，只含相对路径与时间戳（沿用 v1.4 口径）。
- 审计文件自身不被清理：`runs.prune` 只处理 `attempt-*` 目录，不删根下文件，也不删 `_` 前缀目录。
- 暂存目录 `runs/.prune-trash/`（以 `.` 开头、非合法 `task_id`）**不会出现在 `runs.list`**；删除失败留下的整链暂存**不自动清理**，由人工核对后手工移除（宁可不删）。

### 8.6 失败语义（不许报成功）

| 项 | 规则 |
|---|---|
| 部分失败 | 某个 attempt 的暂存重命名或删除失败 → 继续处理其余候选，返回 `status="partial"`、`deleted_total`、`failed_total` 与逐项错误；工具响应 `isError=true`。失败项要么**原样未动**（重命名失败），要么**整链留在 `.prune-trash/`**（删除失败，数据未丢）。**不许把"删了一半"报成成功** |
| 不重试 | **不自动重试**任何删除；失败项只能由调用方下一次显式调用重新评估 |
| 记账失败 | `plan` 行写失败 → 不删；`result` 行写失败 → 删除已发生，返回工具错误并明写"已删除 N 个、失败 M 个，结果行写入失败"，不得声称成功 |
| 非删除类错误 | 参数非法（`invalid_retention` / `confirm_required`）在**任何写入之前**拒绝 |

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
| A 统一期限 | 可追溯性最差（成功/失败/超时证据同寿），不采用 |
| B 按结果分档 | 方向正确，但需 PM 先定三档阈值与"验收最大滞后"；**本批不实现**，入口只收统一阈值 |
| C 按引用分档 | 自指（引用表本身成为新证据），且把只读接口变成影响清理结果的状态源，违背 §7 零写原则；不采用 |

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
4. **证据清理**：入口已落地（v1.12，`runs.prune`，显式两段式、默认不删、无自动清理），但**尚未由 PM 实际调用过**。仍待 PM 定：是否有清理需求、统一阈值 `older_than_days` 取多少、是否要按结果分档（若需要，另立契约）、以及清理后 `task_id` 一律退役的纪律（§8.7）。

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
