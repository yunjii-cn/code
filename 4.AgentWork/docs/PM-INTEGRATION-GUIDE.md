# PM 侧接入须知（AW MCP Bridge v1.50）

本文面向要把任务派给 AgentWork（AW）并需要中途取消的 PM 侧开发者。
**只看这一份即可接入，不需要读 AW 源码。**

- 传输：MCP over stdio，一行一个 JSON-RPC 消息（必须以 `\n` 结尾）。
- 连接：一个 bridge 进程只服务一条 stdio 连接。多进程部署时，每个进程各起一个 bridge。
- 协议版本：`2025-06-18`；服务名：`aw-mcp-bridge`。
- 工具清单：`health`、`status`、`capabilities`、`task.execute`、`task.cancel`、`receipt.get`、`runs.inflight`、`runs.list`、`runs.prune`。

---

## 0. 三种行为对照

| PM 想做的事 | 调用 | 判定成功/完成的关键字段 |
| --- | --- | --- |
| 正常派单，等结果 | `task.execute` | `status=ok`（或 `failed` 但拿到了退出码/产物） |
| 任务超时 | `task.execute` 传 `timeout_secs` | `status=blocked` 且 `blocked_reason` 以 `timeout：` 开头 |
| 中途取消 | `task.cancel` | `status=canceled` 且 `cleanup_claimed=true` 且 `handback_status=canceled` |

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
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"aw-mcp-bridge","version":"0.x.y","title":"AgentWork MCP Bridge"}}}
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
| `canceled` | 被取消并完成清理 | 看 `blocked_reason` 前缀 `canceled：` |

### 2.4 幂等与重投

- 同一个 `task_id` 重复调用：参数（`cmd`/`cwd`/`timeout_secs`/`env`/`path_prepend`）完全一致 → **回放已有回执**，不会重跑；不一致 → `isError=true`，`error` 含 `task_id_conflict`。
- 想重跑同一个任务：**换新的 `task_id`**（例如追加序号或时间戳）。
- 同一 `task_id` 若上次仍在执行 → 返回 `task_in_progress`，稍后重试即可。
- 不同 `task_id` 可以并发；同一 bridge 连接可同时有多条 `task.execute`。

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
文件已存在时不要覆盖：说明已有取消请求在途。

### 4.3 `task.cancel` 的返回与「三态」

```json
{
  "task_id": "build-20261108-001",
  "status": "cancel_requested",
  "canceled": false,
  "cleanup_claimed": false,
  "finished_observed": false,
  "cross_process": true,
  "request_state": "acknowledged",
  "request_file": "runs/.cancel/build-20261108-001.json",
  "requested_by": "pm-scheduler-1",
  "requested_at": "2026-11-08T10:12:00Z",
  "acknowledged_at": "2026-11-08T10:12:00Z",
  "finished_at": null,
  "handback_status": null,
  "cleanup_detail": null,
  "requested_reason": "上层需求变更"
}
```

`request_state` 是**取消请求的进度**，不是任务是否已清理：

| request_state | 含义 | PM 能否宣布「已取消并清理」 |
| --- | --- | --- |
| `pending` | 请求已落盘，执行侧**还没读到** | 不能 |
| `acknowledged` | 执行侧已读到，正在终止/清理 | 不能 |
| `done` | 执行侧已收尾，回执已落盘 | 看 `handback_status` |

`status` 四态：

| status | 含义 | PM 动作 |
| --- | --- | --- |
| `canceled` | 回执确为 `canceled`，清理已收尾 | 完成。`cleanup_claimed=true` |
| `completed_before_cancel` | 取消到达前任务已正常结束，未终止任何进程 | 按原回执处理 |
| `cancel_requested` | 请求已落盘但尚未看到 `done` | 继续轮询 `task.cancel` 或 `receipt.get` |
| `not_running` | 没有仍有效的活跃标记：未写请求、未发信号、未做清理 | 任务已不在跑；不要当成「取消成功」 |

### 4.4 如何区分「已清理」与「只是收到取消请求」

只有同时满足以下三条，才能对外宣布**取消并清理完成**：

1. `task.cancel` 返回 `status == "canceled"`
2. `cleanup_claimed == true`
3. `handback_status == "canceled"`（`receipt.get(task_id)` 读到的回执 `status` 也是 `canceled`）

只满足 1/2 或只看到 `request_state=pending/acknowledged`，一律表述为「取消请求已送达/正在清理」，**不要说已清理**。
`not_running` 与 `completed_before_cancel` 都意味着「没有发生过本次取消的终止动作」，不能计入取消成功。

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
| `blocked_reason` | 阻断原因；`timeout：` / `canceled：` 前缀区分两类 |
| `duration_ms` | 从启动到结束的毫秒数 |
| `changed_files` | git 工作区变更的前 20 个；`changed_files_truncated` 表示是否截断 |
| `commit` | 执行前 `git rev-parse --short HEAD`（非 git 目录为 null） |
| `artifacts[]` | stdout/stderr 文件：`locator`（相对 bridge 可执行文件目录，含 `runs/` 前缀）+ `hash`（sha256） |
| `finishedAt` | 回执落盘时间，UTC 秒级 |

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
| `task_in_progress:` | 同 task_id 的尝试仍在执行 | 稍后重试 |
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
- 同一个 task_id 不要由两个 PM 进程同时派单：bridge 的独占创建会让后来者按重投处理（参数不同则 `task_id_conflict`）。

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
3. **取消**：派发长任务后，在同一连接发 `task.cancel`，校验 `status=canceled`、`cleanup_claimed=true`、`request_state=done`、`handback_status=canceled`，且执行侧最终回执也是 `canceled`。

输出 `[PASS] ...` / `[FAIL] ...`，全过退出 0，否则退出 1。建议把这条命令放进 PM 侧联调流水线。

---

## 9. PM 侧验收清单

- [ ] 能完成 initialize，并按 `id` 正确匹配响应（不依赖顺序）。
- [ ] 派单后能按 `status` 四态分流，`failed` 时能读到 stderr 产物。
- [ ] 超时能识别 `timeout：` 前缀，并能区分三档清理措辞。
- [ ] 取消能识别 `pending` / `acknowledged` / `done`，不把 `cancel_requested` 当成功。
- [ ] 只在 `status=canceled && cleanup_claimed=true && handback_status=canceled` 时宣布「已清理」。
- [ ] 重投使用新 task_id；同 task_id 参数变化时能处理 `task_id_conflict`。
- [ ] 多进程场景下能通过 `runs/.cancel/` 文件取消，并能从 `runs/.inflight/` 判断目标。
- [ ] 自测脚本 `pm_selftest.py` 三个场景全绿。
