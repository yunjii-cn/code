# PM 侧接入须知（AW MCP Bridge v1.51）

本文面向要把任务派给 AgentWork（AW）并需要中途取消的 PM 侧开发者。
**只看这一份即可接入，不需要读 AW 源码。**

- 传输：MCP over stdio，一行一个 JSON-RPC 消息（必须以 `\n` 结尾）。
- 连接：一个 bridge 进程只服务一条 stdio 连接。多进程部署时，每个进程各起一个 bridge。
- 协议版本：`2025-06-18`；服务名：`aw-mcp-bridge`。
- 工具清单：`health`、`status`、`capabilities`、`task.execute`、`task.cancel`、`receipt.get`、`runs.inflight`、`runs.list`、`runs.prune`（逐个进参/出参/读写性质见 §10）。
- **版本识别**：握手 `serverInfo.version` 是二进制版本（当前 `0.1.0`），**不代表契约版本**；机器校验请读 `status.contract_version`（v1.51 起，当前 `1.51`），并对照本文标题。

---

## 0. 三种行为对照

| PM 想做的事 | 调用 | 判定成功/完成的关键字段 |
| --- | --- | --- |
| 正常派单，等结果 | `task.execute` | `status=ok`（或 `failed` 但拿到了退出码/产物） |
| 任务超时 | `task.execute` 传 `timeout_secs` | `status=blocked` 且 `blocked_reason` 以 `timeout：` 开头 |
| 中途取消 | `task.cancel` | `status=canceled` 且 `cleanup_status=cleaned`（等价 `cleanup_claimed=true`）且 `handback_status=canceled` |

注意：`task.execute` 是**阻塞调用**，任务结束才返回。取消必须**另发一条 `task.cancel`**（同一连接即可，见 §4）。

---

## 1. 启动与握手

启动 bridge 二进制（`aw-mcp-bridge.exe` / `aw-mcp-bridge`），把 stdin/stdout 当作消息通道，stderr 是日志。

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"pm","version":"1.0.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
```

响应：

```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"aw-mcp-bridge","version":"0.1.0","title":"AgentWork MCP Bridge"}}}
```

调用工具统一用 `tools/call`：

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"task.execute","arguments":{...}}}
```

响应里 `result.content[0].text` 是**一个 JSON 字符串**（不是对象），需要再解析一次；`result.isError=true` 表示参数/执行错误：

```json
{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"{\"task_id\":\"t-1\",\"status\":\"ok\",...}"}],"isError":false}}
```

> 读取建议：按 `id` 匹配响应，不要假设响应顺序。`task.execute` 在 bridge 内部由后台线程执行，期间 `task.cancel` 的响应可能先到。

运行目录固定为**bridge 可执行文件同级的 `runs/`**。所有 `locator` 都相对 **bridge 可执行文件所在目录**（形如 `runs/<task_id>/attempt-1/stdout.txt`）。

### 1.1 连接关闭语义（实测）

- 关闭 stdin（EOF）表示「不再接受新请求」，**不等于立刻终止正在跑的任务**：已派发的 `task.execute` 会继续执行到自然结束并落盘回执，bridge 在最后一个任务收尾后以退出码 `0` 退出。
  实测：派发 `ping -n 30`（约 29s）后立刻关 stdin，bridge 在 29.1s 后退出，`handback.status=ok`，无残留 ping 进程。
- 若要立刻停止：先发 `task.cancel`（或写取消请求文件），确认回执落盘后再关 stdin。
- 断线但进程仍持有 stdout 时任务照常运行；重连后可用 `receipt.get` 补读。

---

## 2. 派单：`task.execute`

### 2.1 参数

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `task_id` | 是 | 派单标识。字母或数字开头，仅含字母数字 `.` `_` `-`，≤64 字符。**同一个 task_id 只能对应同一组参数** |
| `cmd` | dispatched 必填 | 命令字符串，经平台 shell 执行（Windows `cmd /C`，POSIX `sh -c`） |
| `cwd` | 否 | 工作目录，缺省为 bridge 进程当前目录；必须是已存在目录 |
| `timeout_secs` | 否 | 整数 1..=600，默认 120 |
| `env` | 否 | 追加/覆盖子进程环境变量（字符串 map）。**不接受 PATH**（任何大小写变体都拒绝），需要追加目录用 `path_prepend` |
| `path_prepend` | 否 | 字符串数组，前插到 PATH；参与幂等签名 |
| `mode` | 否 | 缺省 `dispatched`。PM 派单**不要传** `standalone`；`argv`/`acceptance`/`max_attempts` 只属于 standalone，dispatched 下传入会被拒 |

### 2.2 请求示例

```json
{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"task.execute","arguments":{
  "task_id":"build-20261108-001",
  "cmd":"npm run build",
  "cwd":"D:\\work\\proj",
  "timeout_secs":300,
  "env":{"NODE_ENV":"production"},
  "path_prepend":["D:\\tools\\node\\bin"]
}}}
```

### 2.3 返回（dispatched 回执，节选）

```json
{
  "task_id": "build-20261108-001",
  "attempt": 1,
  "finishedAt": "2026-11-08T10:11:12Z",
  "status": "ok",
  "exit_code": 0,
  "duration_ms": 4210,
  "blocked_reason": null,
  "cleanup_status": null,
  "cwd": "D:\\work\\proj",
  "timeout_secs": 300,
  "env_keys": ["NODE_ENV"],
  "path_prepend": ["D:\\tools\\node\\bin"],
  "verify_cmd": "npm run build",
  "stdout_digest": "sha256:...",
  "stderr_digest": "sha256:...",
  "commit": "1a2b3c4",
  "changed_files": ["dist/app.js"],
  "changed_files_truncated": false,
  "artifacts": [
    {"kind":"stdout","locator":"runs/build-20261108-001/attempt-1/stdout.txt","hash":"sha256:..."},
    {"kind":"stderr","locator":"runs/build-20261108-001/attempt-1/stderr.txt","hash":"sha256:..."}
  ]
}
```

`status` 四态：

| status | 含义 | PM 该怎么处理 |
| --- | --- | --- |
| `ok` | 命令退出码 0 | 成功 |
| `failed` | 命令已执行，退出码非 0 | 读 `exit_code` 与 stderr 产物 |
| `blocked` | 没跑成或没跑完：命令不存在、cwd 不存在、超时、启动失败 | 看 `blocked_reason` 前缀（`timeout：` / `command_not_found:` / `cwd_not_found:` …） |
| `canceled` | 被取消，终止/清理流程已收尾（**不等于已清理**） | 看 `cleanup_status`：只有 `cleaned` 才算已清理；`still_alive` / `unavailable` 见 §4.4 |

### 2.4 幂等、重投与并发（含执行中重复派发）

- 同一个 `task_id` 重复调用：参数（`cmd`/`cwd`/`timeout_secs`/`env`/`path_prepend`）完全一致 → **回放已有回执**，不会重跑；不一致 → `isError=true`，`error` 含 `task_id_conflict`。
- 想重跑同一个任务：**换新的 `task_id`**（例如追加序号或时间戳）。
- 不同 `task_id` 可以并发；同一 bridge 连接可同时有多条 `task.execute`。

**执行中再次派发同一 `task_id` 是「阻塞回放」，不是立即报错**（v1.51 实测）：

- 第二个调用**不会启动第二份执行**，也不会立刻返回 `task_in_progress`；它会阻塞等待原执行结束（或取消）并落下回执，然后**原样回放那份回执**（`attempt`、`finishedAt`、`status` 与首个调用一致）。
  实测：原任务 `ping -n 20`，0.8s 后重复派发；重复调用阻塞 4.3s，直到 `task.cancel` 让原执行收尾，随后回放同一份 `canceled` 回执。
- 最长阻塞 = 本次请求的 `timeout_secs + 5s`（签名一致，故两个调用的 `timeout_secs` 相同，上限 605s）。只有超过这个上限仍读不到回执，才返回工具错误 `task_in_progress:`。
- 极窄窗口：首个调用刚创建 attempt 目录、尚未写 `dispatch.json` 时到达的重复派发，会先等 `dispatch.json` 最多 5s；仍读不到则返回 `task_id_conflict:`（同样不会启动第二份）。
- 并发派单者**不排队、也不互相取消**：N 个重复派发各自阻塞、各自回放同一份回执，各占一个后台执行线程；bridge 主循环仍继续处理其他消息（例如 `task.cancel`）。
- 设计取舍：`task.execute` 本来就是阻塞调用，重复派发通常意味着调用方重试；阻塞回放保证「重试最终拿到同一份结果」。若改成立即返回错误，会把合法的重试变成假失败，调用方仍要再查回执，反而更复杂。因此这里**改文档不改行为**；PM 若不想阻塞，请先用 `receipt.get` 非阻塞查询，或另起一条连接。

---

## 3. 超时怎么表示

超时不是异常，而是一条**正常回执**：

```json
{
  "task_id": "build-...",
  "status": "blocked",
  "exit_code": null,
  "blocked_reason": "timeout：超过 300s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证",
  "artifacts": [ ... ]
}
```

判定：`status=blocked` 且 `blocked_reason` 以 `timeout：` 开头。

超时原因按清理结果分三档，**不要把它们混为一谈**：

| `blocked_reason` 措辞 | 含义 |
| --- | --- |
| `已清理受控集合` | 观测到受控集合（POSIX 进程组 / Windows Job Object）内已无存活进程 |
| `受控集合内仍有存活进程…` | 已发终止信号，但仍有存活进程；PM 应视为「可能残留」 |
| `受控集合未能建立或无法查询…` | 只向直接子进程发了终止信号；其派生进程是否还在未验证 |

AW **不承诺**杀掉整棵进程树：主动 `setsid`/`BREAKAWAY` 逃逸的进程可能仍在运行；回执里不会谎报已清理。
PM 若需要强隔离，应在容器/独立账户层面做兜底。

---

## 4. 发起取消

### 4.1 方式 A：MCP `task.cancel`（推荐）

同一个连接上，`task.execute` 还没返回时，直接发：

```json
{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"task.cancel","arguments":{
  "task_id":"build-20261108-001",
  "reason":"上层需求变更",
  "requested_by":"pm-scheduler-1"
}}}
```

参数：

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `task_id` | 是 | 要取消的派单标识 |
| `reason` | 否 | ≤1024 字符，写入回执 `blocked_reason` 的「请求方理由」 |
| `requested_by` | 否 | ≤128 字符，发起者标识，供审计；缺省 `pid:<bridge 进程 pid>` |

### 4.2 方式 B：直接写取消请求文件（无法连 MCP 时）

PM 的另一个进程可以只靠文件取消，无需 MCP 连接：

1. 确保目录存在：`<bridge 目录>/runs/.cancel/`
2. 原子创建 `runs/.cancel/<task_id>.json`（`O_CREAT | O_EXCL`），内容：

```json
{
  "version": "cancel-request/1",
  "task_id": "build-20261108-001",
  "requestedAt": "2026-11-08T10:12:00Z",
  "requested_by": "pm-scheduler-1",
  "reason": "上层需求变更",
  "target": {"pid": 12345, "startedAt": "2026-11-08T10:11:08Z"},
  "status": "pending",
  "acknowledgedAt": null,
  "handledBy": null,
  "handback_status": null,
  "finishedAt": null
}
```

`target` 必须取自 `runs/.inflight/<task_id>.json` 里的 `pid` 与 `startedAt`（同一次读到的两个值），这样只影响那一轮执行，不会误杀同 task_id 的新一轮。
执行侧每 200ms 在自己的心跳里读这个文件并推进状态；PM 轮询该文件即可看到 `pending → acknowledged → done`。

**状态推进不保证每一步都被观察到**（实测）：执行侧可能在一个心跳内直接完成「读到请求 → 终止 → 收尾」，PM 只会看到 `pending` 然后直接 `done`，`acknowledgedAt` 保持 `null`。因此：

- 不要把「见过 `acknowledged`」当作前提，也不要因为 `acknowledgedAt=null` 就认为请求没被处理；
- 只看终态：`status=done` + `handback_status` + `cleanup_status`（见 §4.4）。

文件已存在时不要覆盖：说明已有取消请求在途。

`runs/.inflight/<task_id>.json` 的实际内容（字段固定，供 `target` 取值）：

```json
{
  "version": "inflight/1",
  "task_id": "build-20261108-001",
  "pid": 12345,
  "mode": "dispatched",
  "state": "active",
  "startedAt": "2026-11-08T10:11:08Z",
  "deadlineAt": "2026-11-08T10:16:08Z"
}
```

读取注意：写者是「独占创建 → 写入 → `sync_all`」，在创建与写完之间有极短窗口，读到的可能是空文件或半截 JSON。解析失败时短暂重试（例如 5 × 20ms）；仍失败就当作「未知」，**不要**据此判定「任务不存在」。`runs.inflight action=list` 会把这类文件标为 `unreadable` 并保留，不会误删。

### 4.3 `task.cancel` 的返回与「三态」

`task.cancel` 最多阻塞 10s 等执行侧收尾：**能等到回执就直接返回 `canceled`**，等不到才返回 `cancel_requested` 让 PM 继续轮询。下面是一次真实取消（`ping -n 20`，约 110ms 返回）的完整返回：

```json
{
  "task_id": "build-20261108-001",
  "status": "canceled",
  "canceled": true,
  "cleanup_claimed": true,
  "cleanup_status": "cleaned",
  "finished_observed": true,
  "cross_process": true,
  "request_state": "done",
  "request_file": "runs/.cancel/build-20261108-001.json",
  "requested_by": "pm-scheduler-1",
  "requested_at": "2026-11-08T10:12:00Z",
  "acknowledged_at": "2026-11-08T10:12:00Z",
  "finished_at": "2026-11-08T10:12:00Z",
  "handback_status": "canceled",
  "cleanup_detail": "canceled：收到 task.cancel，已清理受控集合；逃逸进程是否仍在运行未验证；请求方理由：上层需求变更",
  "requested_reason": "上层需求变更",
  "message": "取消已生效：执行线程完成终止并落盘 canceled 回执",
  "handback": {
    "task_id": "build-20261108-001",
    "attempt": 1,
    "status": "canceled",
    "cleanup_status": "cleaned",
    "blocked_reason": "canceled：收到 task.cancel，已清理受控集合；逃逸进程是否仍在运行未验证；请求方理由：上层需求变更",
    "artifacts": [ ... ]
  }
}
```

字段说明（`cleanup_claimed`、`cleanup_status`、`handback`、`message` 是 v1.51 起显式文档化的字段）：

| 字段 | 说明 |
| --- | --- |
| `status` | `canceled` / `completed_before_cancel` / `cancel_requested` / `not_running` |
| `cleanup_status` | 机器可读的终止结果：`cleaned` / `still_alive` / `unavailable` / `not_started` / `null`。**只有 `cleaned` 才算已清理** |
| `cleanup_claimed` | 布尔便利字段，等价 `status=="canceled" && cleanup_status=="cleaned"`；**不再**仅凭 `status=canceled` 置 true |
| `handback` | 完整的 canceled 回执（与 `receipt.get` 读到的一致），便于一次调用拿全证据；取不到时为 `null` |
| `cleanup_detail` | 取消回执里的 `blocked_reason` 原文（`string \| null`），与回执逐字一致 |
| `message` | 人类可读的一句话结论，不承载判定逻辑，PM 不要用它做分支 |
| `handback_status` | 回执的 `status`（`canceled` / `ok` / `failed` / `blocked` / `null`） |
| `request_state` | 取消请求文件进度：`pending` / `acknowledged` / `done` |

`request_state` 是**取消请求的进度**，不是任务是否已清理：

| request_state | 含义 | PM 能否宣布「已取消并清理」 |
| --- | --- | --- |
| `pending` | 请求已落盘，执行侧**还没读到** | 不能 |
| `acknowledged` | 执行侧已读到，正在终止/清理 | 不能 |
| `done` | 执行侧已收尾，回执已落盘 | 看 `cleanup_status` |

`status` 四态：

| status | 含义 | PM 动作 |
| --- | --- | --- |
| `canceled` | 回执确为 `canceled`，清理流程已收尾 | 完成与否看 `cleanup_status`（见 §4.4） |
| `completed_before_cancel` | 取消到达前任务已正常结束，未终止任何进程 | 按原回执处理 |
| `cancel_requested` | 请求已落盘但 10s 内尚未看到 `done` | 继续轮询 `task.cancel` 或 `receipt.get` |
| `not_running` | 没有仍有效的活跃标记：未写请求、未发信号、未做清理 | 任务已不在跑；不要当成「取消成功」 |

### 4.4 如何区分「已清理」与「只是收到取消请求」

**MCP 通道**：只有同时满足以下三条，才能对外宣布**取消并清理完成**：

1. `task.cancel` 返回 `status == "canceled"`
2. `cleanup_status == "cleaned"`（等价 `cleanup_claimed == true`）
3. `handback_status == "canceled"`，且 `receipt.get(task_id)` 的 `status` 也是 `canceled`、`cleanup_status` 也是 `cleaned`

**文件通道**（没有 MCP 连接，只能读 `runs/.cancel/<task_id>.json`）：只有同时满足以下三条：

1. `request.status == "done"`
2. `request.handback_status == "canceled"`
3. `request.cleanup_status == "cleaned"`（等价 `request.cleanup_claimed == true`）

其余一律表述为「取消请求已送达/正在清理」，**不要说已清理**：

- `cleanup_status == "still_alive"`：已发终止信号但受控集合内仍有存活 → 表述「可能残留」，并按需人工清理。
- `cleanup_status == "unavailable"`：受控集合没能建立或无法查询，只向直接子进程发过信号 → 派生进程是否还在**未验证**。
- `cleanup_status == "not_started"`：取消在启动任何进程之前生效 → 可表述「已取消（未启动进程）」，但**不要**说「已清理」。
- `cleanup_status` 缺失/`null`：旧版 bridge 或回执不可读 → 按「未验证」处理，不得宣布已清理。
- `request_state=pending/acknowledged`、`not_running`、`completed_before_cancel` 都意味着「没有发生过本次取消的、已确认的清理」。

只满足其中一部分（例如 `status=canceled` 但 `cleanup_status=still_alive`，或文件里 `status=done` 但缺 `cleanup_status`）**不算已清理**。AW 的回归用例 `cancel_request_terminal_state_never_overclaims_cleanup` 用真实终态做变异，逐一验证这五种「只满足部分条件」都不会被判成已清理。

---

## 5. 读回执与证据

- `task.execute` 的返回值本身就是回执（handback）。
- 任务可能由别的调用方发起，或你断线重连后需要补读：

```json
{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"receipt.get","arguments":{"task_id":"build-20261108-001","include_history":true}}}
```

`include_history=false`（默认）返回最新 attempt；`true` 返回 `{task_id, attempts, latest, history[]}`。

回执关键字段：

| 字段 | 说明 |
| --- | --- |
| `status` | `ok` / `failed` / `blocked` / `canceled` |
| `exit_code` | 退出码；被 timeout/cancel 终止或未能启动时为 `null` |
| `blocked_reason` | 阻断原因；前缀区分原因（完整清单见下表） |
| `cleanup_status` | 仅 canceled / timeout 回执非 `null`：`cleaned` / `still_alive` / `unavailable` / `not_started`；其余状态为 `null` |
| `duration_ms` | 从启动到结束的毫秒数 |
| `changed_files` | git 工作区变更的前 20 个；`changed_files_truncated` 表示是否截断 |
| `commit` | 执行前 `git rev-parse --short HEAD`（非 git 目录为 null） |
| `artifacts[]` | stdout/stderr 文件：`locator`（相对 bridge 可执行文件目录，含 `runs/` 前缀）+ `hash`（sha256） |
| `finishedAt` | 回执落盘时间，UTC 秒级 |

`blocked_reason` 的前缀是稳定契约（冒号/全角冒号按原文），按前缀分流不要做模糊匹配：

| 前缀 | 出现场景 | status | `cleanup_status` |
| --- | --- | --- | --- |
| `timeout：`（全角） | 超过 `timeout_secs` | `blocked` | 三态之一（`cleaned` / `still_alive` / `unavailable`） |
| `canceled：`（全角） | 收到 `task.cancel` 或取消请求文件 | `canceled` | 三态之一，启动前取消为 `not_started` |
| `command_not_found:` | 命令不存在 | `blocked` | `null` |
| `cwd_not_found:` | 工作目录不存在 | `blocked` | `null` |
| `standalone_*` 系列（如 `standalone_cmd_rejected:` / `standalone_acceptance_required:` / `standalone_program_unresolved:` / `standalone_retry_exhausted_without_result:`） | 仅 `mode=standalone`；PM 不会遇到 | `blocked` | `null` |

`cleanup_detail`（出现在 `task.cancel` 返回里，不在回执里）就是该回执 `blocked_reason` 的原文，类型 `string | null`，逐字一致，不做二次加工。

所有终态回执（`ok` / `failed` / `blocked` / `canceled`）都保证 `artifacts` 有两个条目（stdout、stderr），文件在 spawn 之前就已创建；只有参数校验失败（工具错误）不产生回执。取消/超时不改变产物文件，仍按 `locator` 校验 sha256。

`receipt.get` 的两种失败形态：

- 任务从未派发或回执目录已被 `runs.prune` 删掉 → 工具错误，`isError=true`，`text={"error":"回执不存在：<task_id>"}`；PM 按「无回执」处理，不要当成执行失败。
- `include_history=true` 时返回 `{task_id, attempts, latest, history[]}`；`attempts` 为 0 不会出现（无回执时直接报错）。

校验产物：按 `locator` 找到文件，重算 sha256 与 `hash` 比对。文件在 `runs/<task_id>/attempt-<n>/` 下：

```
dispatch.json   派单签名（task_id / cmd / cwd / timeout_secs / env 摘要 / path_prepend / dispatchedAt）
stdout.txt      stdout 原文
stderr.txt      stderr 原文
handback.json   回执（与 task.execute 返回一致）
verdict.json    仅 standalone 模式存在
```

任务被取消时，`receipt.get` 仍可读到最后一次回执（`status=canceled`），`attempt` 保留。

---

## 6. 错误约定

两类错误要分开处理：

1. **JSON-RPC 错误**：响应里没有 `result`，只有 `error.code` / `error.message`（如 `-32700` 解析失败、`-32601` 未知方法）。
2. **工具错误**：`result.isError=true`，`content[0].text` 的 JSON 里有 `error` 字段（字符串）。PM 应把 `error` 原文记入日志。

常见 `error` 前缀：

| 前缀 | 含义 | 处理 |
| --- | --- | --- |
| `task_id_conflict:` | 同 task_id 已存在且参数不一致 | 换新 task_id |
| `mode_conflict:` | 同 task_id 已属于另一种 mode | 换新 task_id |
| `task_in_progress:` | 同 task_id 的既有尝试在 `timeout_secs+5s` 内仍未落盘回执（正常重投会阻塞回放，不走这里） | 稍后重试或改用 `receipt.get` |
| `回执不存在：` | `receipt.get` 查不到该 task_id 的回执（从未派发或已被 `runs.prune` 删除） | 按「无回执」处理，不要当成执行失败 |
| `invalid_task_id` / `task_id 非法` | 命名不合规 | 修正 task_id |
| `timeout_secs 必须在 1..=600 之间` | 参数越界 | 修正 |
| `invalid_reason:` | reason 超长 | 缩短 |
| `invalid_requested_by:` | requested_by 为空/超长 | 修正 |

前置校验失败（缺 `cmd`、`cwd` 不存在、命令启动失败）不会抛工具错误，而是返回 `status=blocked` 的正常回执。

---

## 7. 多进程部署要点

- **每个 bridge 进程一条 stdio 连接**；不要在多个 PM 进程间共享同一个 bridge 的 stdin/stdout。
- 跨进程取消有两条可用路径：
  1. PM 自己有 MCP 连接 → `task.cancel`（bridge 负责写请求文件）；
  2. PM 无 MCP 连接 → 按 §4.2 直接写 `runs/.cancel/<task_id>.json`，执行侧心跳会读到。
- 判断「另一个进程是否还在跑」：读 `runs/.inflight/<task_id>.json` 的 `pid` / `startedAt` / `deadlineAt`；**它只是磁盘观测**，要配合 `task.cancel` 返回的 `status` 使用。
- 同一个 task_id 不要由两个 PM 进程同时派单：先进者执行，后到者按「阻塞回放」处理（参数不同则 `task_id_conflict`，完全相同则阻塞到回执落盘后回放同一份回执）。后到者若不想被阻塞，应改用 `receipt.get` 轮询。

---

## 8. 自测入口（PM 行为模拟）

仓库自带一个只依赖公开 MCP 接口的模拟器，覆盖三种行为（在 `4.AgentWork` 目录下执行；workspace 的 target 在该目录）：

```bash
# Windows
python scripts/pm-integration-selftest/pm_selftest.py --binary target/debug/aw-mcp-bridge.exe

# Linux / WSL
python3 scripts/pm-integration-selftest/pm_selftest.py --binary target/debug/aw-mcp-bridge
```

它会：

1. **正常派单**：执行一个写文件并打印的命令，校验 `status=ok`、`exit_code=0`、两个产物、`receipt.get` 可回读；
2. **超时**：`timeout_secs=1` 执行长命令，校验 `status=blocked`、`blocked_reason` 以 `timeout：` 开头；
3. **取消**：派发长任务后，在同一连接发 `task.cancel`，校验 `status=canceled`、`cleanup_status=cleaned`、`cleanup_claimed=true`、`request_state=done`、`handback_status=canceled`，且执行侧最终回执也是 `canceled`、`cleanup_status=cleaned`。

输出 `[PASS] ...` / `[FAIL] ...`，全过退出 0，否则退出 1。建议把这条命令放进 PM 侧联调流水线。

---

## 9. PM 侧验收清单

- [ ] 能完成 initialize，并按 `id` 正确匹配响应（不依赖顺序）。
- [ ] 派单后能按 `status` 四态分流，`failed` 时能读到 stderr 产物。
- [ ] 超时能识别 `timeout：` 前缀，并能区分三档清理措辞。
- [ ] 取消能识别 `pending` / `acknowledged` / `done`，不把 `cancel_requested` 当成功。
- [ ] 只在 `status=canceled && cleanup_status=cleaned && handback_status=canceled` 时宣布「已清理」；缺 `cleanup_status` 一律按未验证处理。
- [ ] 知道执行中重复派发是阻塞回放（上限 `timeout_secs+5s`），不会把 `task.execute` 当立即返回。
- [ ] 重投使用新 task_id；同 task_id 参数变化时能处理 `task_id_conflict`。
- [ ] 多进程场景下能通过 `runs/.cancel/` 文件取消，并能从 `runs/.inflight/` 判断目标。
- [ ] 自测脚本 `pm_selftest.py` 三个场景全绿。

---

## 10. 工具清单：进参 / 出参 / 读写性质

以 `tools/list` 实际返回为准（必填项取自 `inputSchema.required`）。PM 只会用到 `task.execute`、`task.cancel`、`receipt.get`、`runs.inflight`、`runs.list`、`status`，其余列在这里是为了排障时不被工具名误导。

> **schema 不是校验器**：`task.execute` 的 `inputSchema.required` 只有 `task_id`，因为按 `mode` 分派的必填项（dispatched 的 `cmd`）与仅 standalone 可用的字段（`argv` / `acceptance` / `max_attempts`）无法用同一份 JSON Schema 表达互斥。参数约束以 §2.1 为准；缺 `cmd`、dispatched 下传 standalone 字段等由 bridge 侧返回 `status=blocked` 或工具错误，**不要用 schema 做本地前置拒绝**。

| 工具 | 必填进参 | 可选进参 | 出参（关键） | 读写性质 |
| --- | --- | --- | --- | --- |
| `health` | 无 | 无 | 存活状态、服务名、版本、构建时间 | 只读，不落盘 |
| `status` | 无 | 无 | `contract_version`、`desktop`、`session`、`connections`、`server` | 只读；不回显任何配置值 |
| `capabilities` | 无 | 无 | `version="capabilities/1"`、`entries[]`、`entries_total`、`not_proven_total`、`platform`、`status_enum`、`decision_rule` | 只读，不落盘 |
| `task.execute` | `task_id` | `mode`、`cmd`、`cwd`、`env`、`path_prepend`、`timeout_secs`（dispatched）；`argv`、`acceptance`、`max_attempts`（standalone） | 回执 JSON（`status` 四态、`exit_code`、`blocked_reason`、`cleanup_status`、`artifacts[]`…） | 写 `runs/<task_id>/attempt-<n>/`（dispatch、stdout、stderr、handback；standalone 另有 verdict） |
| `task.cancel` | `task_id` | `reason`、`requested_by` | 见 §4.3（`status`、`cleanup_status`、`cleanup_claimed`、`handback`、`message`…） | 写 `runs/.cancel/<task_id>.json`；读回执 |
| `receipt.get` | `task_id` | `include_history` | 单份回执；`include_history=true` 时 `{task_id, attempts, latest, history[]}` | 只读；无回执时工具错误 `回执不存在：` |
| `runs.inflight` | `action` | `task_id`、`dry_run`、`confirm` | `markers[]`（`pid`、`pid_alive`、`stale`、`stale_reason`、`removable`…）、`stale_rule` | `list` 只读；`clean` 默认 dry_run，真删需 `dry_run=false` + `confirm=true` |
| `runs.list` | 无 | `offset`、`limit` | `tasks[]`（`task_id`、`attempts`、最新 `status`、`env_keys` 等），按 `task_id` 升序 | 只读；排除 `_session` 等预留目录；只回 env 键名 |
| `runs.prune` | `older_than_days` | `dry_run`、`confirm`、`status`、`exclude`、`limit` | `candidates[]`、`deleted[]`、`excluded[]`、`failed[]`、`trashed[]`/`trashed_total`、`rule` | 默认 `dry_run=true` 零写入；真删需 `dry_run=false` + `confirm=true`；最新 attempt 永不删 |
