# 超时进程树杀灭（`#3`）设计评估与窄定义实现

- 状态：**v1.16 已按窄定义实现**：只清理并判定"**受控集合**"（POSIX 进程组 / Windows Job Object）；v1.16.1 曾退回旧文案，**v1.17 状态门满足后已启用新措辞**；**v1.20 把 Windows 的 `spawn → Assign` 窗口收窄为 W2a（挂起创建 → 入作业 → 恢复执行）**；**整棵进程树仍不可判定，`#3` 在契约中保持 `NOT_PROVEN`**。
- 实现边界：代码改 `apps/mcp-bridge/src/server.rs`（`process_scope` 模块 + 超时路径 + `timeout_reason`）；用例改 `apps/mcp-bridge/tests/mcp_contract.rs` 与 `server.rs` 内单元测试；契约 v1.16 → v1.17 → v1.20；依赖仅把已锁定的 `windows-sys 0.61.2` / `libc 0.2.186` 转为 target-specific 正常依赖（v1.20 仅启用 `windows-sys` 已有的 `Win32_System_Diagnostics_ToolHelp` feature，**未新增包、未改版本**）。
- 权威契约：[`PM-AW-DISPATCH-CONTRACT.md`](./PM-AW-DISPATCH-CONTRACT.md)（v1.16 起 G9 行指向本文件，当前 v1.20；v1.18/v1.19 未改超时/回执行为，v1.20 收窄 Windows 窗口）。
- Windows 取舍：**仍未走 W2c**（自举 launcher 成本高、回归面大）。v1.16 选择 W2d（接受窗口 + 披露）；**v1.20 在不引入 launcher 的前提下实现了 W2a**：`CREATE_SUSPENDED` 创建、`AssignProcessToJobObject`、Toolhelp 定位主线程后 `ResumeThread`。恢复执行前子进程不执行任何代码，**窗口内派生作业外后代这一条对后代不再成立**。残余逃逸为 `BREAKAWAY`、加入作业失败、嵌套作业策略；回执已改为"挂起态创建、先入作业再恢复执行"。**W2c 仍固定为不做（§4.5），不是待办项。**

---

## 0. 结论先行

1. **"整棵进程树已终止"在当前环境仍不可判定**，`#3` 应继续保持 `NOT_PROVEN`。理由见 §6.4：两个平台都缺少对"逃逸后代"的可靠、可移植、无竞态观测点。
2. **已实现可执行的窄判据**：只证明"**受控集合**（Windows Job Object / POSIX 进程组）内没有存活进程"。它**不等于**整棵树，但能真正区分"进程不存在"与"进程还活着但不写盘"（§3、§6.2）。
3. **回执文案已按状态门启用（v1.17）**："已清理受控集合；逃逸进程是否仍在运行未验证"，三态分流；解锁依据是 POSIX 四类用例实跑通过 + Windows 三类通过 + 契约版本递增（v1.16.1 记录退回、v1.17 记录启用）。
4. **明确否掉"超时后两次 `stat` 无变化"**：它测的是"是否写盘"，不是"是否存在"，会把 SIGSTOP、IO 阻塞、活着但本就不写盘的进程判成"已退出"（§2）。实现与用例均未使用该判据。
5. 平台实现：POSIX 用 `process_group(0)` 在 **exec 前**建组、无加入竞态；Windows 用 Job Object + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，**v1.20 起以 `CREATE_SUSPENDED` 创建、先入作业再恢复主线程**，恢复前不执行代码（§4.2）；残余 `BREAKAWAY` / 加入失败 / 嵌套作业仍不可观测。
6. 依赖：`windows-sys 0.61.2` 与 `libc 0.2.186` **已在 `Cargo.lock`**，转为 **target-specific 直接依赖后不新增包、不改版本**；许可均为 `MIT OR Apache-2.0`；MSRV 1.71 / 1.65，均低于 workspace 的 1.85（§7）。

---

## 1. 当前事实

### 1.1 实现现状（v1.16 实现、v1.17 启用文案，已核实）

超时路径在 [`apps/mcp-bridge/src/server.rs`](../apps/mcp-bridge/src/server.rs) 的 `execute_task` 内（约 L773-L808）：

- `process_scope::Supervisor::new()` → `supervisor.configure(&mut command)` → `spawn` → `supervisor.attach(&child)`；
- 以 20 ms 间隔轮询 `child.try_wait()`；到达 `deadline` 后 `timed_out = true`，调用 `supervisor.terminate(&mut child)` 并据返回值生成文案；
- `timeout_reason(timeout_secs, outcome)`（约 L1537-L1559）按三态分流，**只有 `Cleaned` 才说"已清理受控集合"**（v1.17 已启用）。

`process_scope` 模块（约 L1076-L1531）：

- 公共类型 `CleanupOutcome { Cleaned, StillAlive, Unavailable }`；
- Unix：`process_group(0)`（exec 前建组）→ 超时 `SIGTERM` → 500 ms 宽限期 → 仍存在则 `SIGKILL` → `child.wait()` → 轮询 `kill(-pgid, 0)` 至 `ESRCH`（3 s 上限）；
- Windows：`CreateJobObjectW` + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`；`spawn` 后 `AssignProcessToJobObject`；超时 `TerminateJobObject` → `child.wait()` → 轮询 `ActiveProcesses` 至 0（3 s 上限）；查询失败按 `Unavailable`，**绝不当 0**；
- 其他平台：只 `child.kill()`，返回 `Unavailable`。

历史（v1.15 及以前）：超时路径只 `child.kill()` + `child.wait()`，文案为"已向直接子进程发送终止信号；其派生的子进程是否仍在运行未验证"。

### 1.2 `child.kill()` 的实际作用域

- Windows：`std::process::Child::kill()` → `TerminateProcess(直接子进程句柄)`。只结束直接子进程，**不遍历子进程树**。
- Unix：`std::process::Child::kill()` → 向直接子进程发送 `SIGKILL`。只作用于该 PID，**不作用于进程组**。
- 因此，无论哪个平台，`child.kill()` 都**只保证直接子进程收到终止动作**。

### 1.3 包装层会派生孙进程

- Windows 的 `shell_command()` 用 `cmd /C <cmd>` 启动（server.rs 约 L1060-L1066）；`cmd` 执行外部命令（如 `ping`）时会再派生一个子进程。
- POSIX 用 `sh -c <cmd>`；简单命令可能被 shell `exec` 优化成"直接子进程就是目标命令"，但复合命令、管道、后台任务、脚本解释器都会产生孙进程。
- 结论：**直接子进程 ≠ 实际执行命令的进程**，这是 `#3` 的根因。

### 1.4 上一轮实测（观测事实，不是推断）

> **观测事实（上一轮实测）**：超时返回后，父/子进程仍存活；最终由测试脚本以强制信号（`SIGKILL` 等价手段）清理。

补充说明（避免把这条读成"已证明"）：

- 这条是**一次性观测**，不是自动化回归证据；v1.15 时测试只覆盖文案。v1.16 起新增了断言进程/作业状态的用例（§6.5），但**上述"上一轮实测"仍只作为历史观测保留**，不当作自动化证据。
- 观测到的"仍存活"与本文的机制解释（§1.2、§1.3）一致，但**机制一致不等于已证明**：要证明，需要 §6.6 的观测点与已知 PID 的对照实验。

### 1.5 已有诚实性约束（不得回退）

契约 §4、§6 与 v1.11 / v1.16 变更记录已写死：

- `timeout` 文案的作用域**写死为"受控集合"，不是整棵进程树**；
- 当前回执（v1.20）三态：`已清理受控集合；逃逸进程是否仍在运行未验证`（Cleaned）/ 仍存活 / 不可用；Windows 额外明写"子进程以挂起态创建并先加入作业再恢复执行（恢复前不会派生子进程）；BREAKAWAY 或加入作业失败的作业外进程仍不可观测"；
- 只有判据返回 `Cleaned` 才能写"已清理受控集合"；`StillAlive` / `Unavailable` **不得升级为"已清理"**；
- 任何分支都**不得**出现"已终止进程树 / 无残留"；
- 调用方需自行确认受控集合之外的残留进程。

状态约束：**`#3` 仍为 `NOT_PROVEN`**；实现窄定义不改变该状态，状态变更只能由用例结果另行决定。2026-11-07 复核：本机可补跑的 Windows/POSIX 窄判据证据已补齐，仍缺 §6.8 的完整枚举观测点，状态不变；证据链见 [`TOOLS-DESCRIPTION-AUDIT.md`](./TOOLS-DESCRIPTION-AUDIT.md) §30。

---

## 2. 明确否掉的坏判据："超时后两次 `stat` 无变化"

该判据的问题不在实现细节，而在**测量的对象错了**：它测的是"文件是否被写"，不是"进程是否存在"。

| 场景 | `stat` 两次是否不变 | 进程是否仍存在 | 判据结论 | 是否正确 |
|---|---|---|---|---|
| 进程被 `SIGSTOP`/`TSTP` 暂停 | 不变 | **存在** | "已退出" | **假通过** |
| 进程阻塞在 IO（等待网络/磁盘/锁） | 不变 | **存在** | "已退出" | **假通过** |
| 进程活着，但本就不写盘（`sleep`、等待事件） | 不变 | **存在** | "已退出" | **假通过** |
| 进程已退出，但另一进程在同一路径写盘 | 变化 | 不存在 | "仍存活" | **假失败** |
| 进程已退出，但文件系统时间戳粒度粗 | 可能不变 | 不存在 | "已退出" | 偶然正确，不可依赖 |

结论：**任何以"有没有副作用/有没有写盘"为代理的判据都不可用于判定存活**。判定存活必须直接查询"进程/作业/进程组是否还存在"。

---

## 3. 观测点候选与判定表

> 判定口径：**"存在"= 观测点能直接回答该作用域内是否还有进程；"存活但不写盘"必须被判为"存在"，不能被判为"不存在"。**

| 编号 | 观测点 | 平台 | 作用域 | 判定语义 | 假通过（把"还在"判成"没了"） | 假失败（把"没了"判成"还在"） |
|---|---|---|---|---|---|---|
| O1 | `kill(-pgid, 0)` | POSIX | 进程组 | 返回 0 → 组内至少一个进程存在（含被暂停、含僵尸）；`ESRCH` → 组内无进程；`EPERM` → 存在但无权限 | 后代 `setsid`/`setpgid` 自建新组，逃出作用域；pgid 被复用 | 组内有僵尸尚未被回收；pgid 被新进程复用 |
| O2 | `child.wait()` / `try_wait()` | 两平台 | 直接子进程 | 返回 `Some` → 直接子进程已终止并回收 | 对"整棵树"而言是假通过（孙进程不在作用域） | 无（这是最确定的一条） |
| O3 | `kill(pid, 0)`（已知 PID） | POSIX | 单个 PID | 0 → 存在；`ESRCH` → 不存在；`EPERM` → 存在但无权限 | 需要事先知道 PID；生产环境拿不到逃逸后代的 PID | PID 复用（需配合启动时间校验） |
| O4 | Job `ActiveProcesses`（`JobObjectBasicAccountingInformation`） | Windows | 作业成员 | `0` → 作业内无活动进程；`>0` → 仍有成员 | `spawn` 后、`Assign` 前派生的孙进程不在作业内；`BREAKAWAY` 逃逸；`Assign` 失败却继续判定 | 查询失败/句柄无效被误读为 0；进程终止过程中的瞬时计数（需轮询到期限） |
| O5 | Job `JobObjectBasicProcessIdList` | Windows | 作业成员 | 返回作业内 PID 列表，可逐个复核 | 同 O4 的逃逸问题 | 列表缓冲区不足/截断；查询失败 |
| O6 | `WaitForSingleObject(process_handle)` | Windows | 直接子进程 | 句柄有信号 → 进程已终止 | 对"整棵树"而言是假通过 | 无 |
| O7 | `/proc/*/stat` 按 `pgrp` 扫描 | Linux | 同组进程 | 枚举当前 pgrp 相同的进程 | 逃逸组不可见；隐藏 PID 的挂载选项 | 扫描与进程退出之间的 TOCTOU；任务瞬间退出 |
| O8 | cgroup v2 `cgroup.procs` 为空 | Linux（需 cgroup 委派） | cgroup 成员 | 空 → 组内无进程（**最接近整棵树**） | 有权限的进程可把自己挪出 cgroup；委派未开启时无法建组 | cgroup 文件不可读/未创建；容器内权限不足 |
| O9 | macOS `kill(pid, 0)` / 进程组 | macOS | 同 O1/O3 | 同 O1/O3 | 无 cgroup/subreaper 等价物 | 同 O1 |

**只要判据用的是 O1/O2/O3/O4/O5/O6，它就已经能区分"进程不存在"与"进程还活着但不写盘"**——因为查询对象是进程/作业本身，不是文件。O7/O8 是增强，不是当前环境可用的默认路径。

---

## 4. Windows 方案：Job Object + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`

### 4.1 设计步骤（伪代码，不是实现）

```text
job = CreateJobObjectW(NULL, NULL)
SetInformationJobObject(job,
    JobObjectExtendedLimitInformation,
    { BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE })
spawn child with CREATE_SUSPENDED              // v1.20：子进程不执行任何代码
AssignProcessToJobObject(job, child_handle)    // 入作业；此时不存在"已执行的后代"
ResumeThread(primary_thread)                   // Toolhelp 快照定位主线程后恢复
超时:
    TerminateJobObject(job, 1)                 // 终止作业内全部进程
    轮询 QueryInformationJobObject(job, JobObjectBasicAccountingInformation).ActiveProcesses
    直到 0 或到达清理期限
    child.wait()
CloseHandle(job)                               // 最后句柄关闭时，KILL_ON_JOB_CLOSE 再兜底
```

优点：

- 一次终止**作业内全部成员**，不依赖父子关系；
- 即使 bridge 进程崩溃，操作系统关闭最后一个作业句柄时也会按 `KILL_ON_JOB_CLOSE` 清理成员（**这是设计意图，未实测**）；
- `ActiveProcesses` 是直接的"存在性"观测点（§3/O4），不受"是否写盘"影响。

### 4.2 窗口与收窄：v1.16 W2d → v1.20 W2a

v1.16 走 W2d：`std::process::Command::spawn()` 返回时子进程**已经开始执行**，`cmd /C ping ...` 可能已经派生了 `ping.exe`；在 `AssignProcessToJobObject` 成功之前派生的后代**不在作业内**，`TerminateJobObject` 与 `ActiveProcesses` 都看不到。该窗口当时已在回执与文档明写。

v1.20 改为 W2a（**不是 W2b/W2c**）：`Command::creation_flags(CREATE_SUSPENDED)` 创建子进程，`AssignProcessToJobObject` 后再恢复主线程。std 不返回主线程句柄，因此用 Toolhelp 线程快照按进程 ID 找到唯一的主线程，`OpenThread + ResumeThread` 恢复。**恢复之前子进程不执行任何代码，所以"窗口内派生后代"这一条对后代不再成立**；恢复之后派生的后代默认继承作业。

可选的闭合路径与代价（v1.20 后更新）：

| 路径 | 做法 | 代价 | 残余风险 |
|---|---|---|---|
| W2a | `CREATE_SUSPENDED` + `Assign` + `ResumeThread` | **已实现（v1.20）**：std 不直接给主线程句柄，用 Toolhelp 快照 + `OpenThread/ResumeThread`；局部 `unsafe`，不替换 spawn 路径 | 恢复失败需终止并记 `process_resume_failed`；`BREAKAWAY` / 加入作业失败 / 嵌套作业策略仍可能逃逸 |
| W2b | `CreateProcessW` + `STARTUPINFOEX` + `PROC_THREAD_ATTRIBUTE_JOB_LIST`，创建即入作业 | 需替换现有 `shell_command`/spawn 路径，大段 `unsafe` FFI；与 std 的 stdout/stderr 句柄、环境继承都要自己拼 | 大幅增加实现与回归面积；不解决 `BREAKAWAY` |
| W2c | bridge 建作业（可继承句柄），把句柄值经环境变量传给一个小型 launcher；launcher **先把自己加入作业，再启动 `cmd`** | 新增一个辅助可执行文件；句柄继承与生命周期管理；打包/安装多一个产物 | 后代在 launcher 入作业之后才产生；但显式 `BREAKAWAY` 仍可能逃逸 |
| W2d | 接受竞态，只做"尽量加入" | 实现小 | **不能宣称确定性**；只能报"已清理作业内成员"（v1.16 曾采用，已被 v1.20 取代） |

**v1.20 的实际选择：W2a。** 未走 W2c 的原因不变：需要新增辅助可执行文件、句柄继承/生命周期管理与打包产物变更，代价与回归面远大于收益；W2b 更是替换 spawn 路径的大段 `unsafe` FFI。实现要点：

- 子进程挂起创建；`Assign` **成功**时先入作业再恢复，恢复前的进程不可能派生子进程，窗口对后代闭合；
- `Assign` 失败（`ACCESS_DENIED` / `INVALID_PARAMETER` / 进程已退出）时仍恢复执行，不让任务永远挂起；`attached = false`，超时只 `child.kill()` 并返回 `Unavailable`，**不宣称已清理**；
- `ResumeThread` 失败时终止该挂起进程，记 `process_resume_failed`，**不把"从未执行"当成命令结果**；
- 残余逃逸：`BREAKAWAY`、加入作业失败后的后代、嵌套作业/外层策略；这些仍不可观测，回执继续写"逃逸进程是否仍在运行未验证"；
- 确定性证据：单元用例 `suspended_start_defers_execution_until_job_assign` 用标记文件证明**入作业前 300ms 内命令零执行**（`marker_before_assign=false`），恢复后标记出现且作业内 `ActiveProcesses > 0`，`terminate=Cleaned` 后为 0（§6.5）。该夹具只证明"恢复前未执行"，**不证明"不存在任何逃逸"**。

`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 仍在 `Supervisor::new` 设置，`Drop` 关闭最后一个句柄时由操作系统兜底清理作业成员（**设计意图；本批未做崩溃场景实测**）。

### 4.3 失败情形（逐条写死，均不得当作成功）

| 情形 | 表现 | 正确处置 |
|---|---|---|
| `CreateJobObjectW` 失败 | 无作业可用 | 记录"受控集合未建立"；清理状态按未证明处理，**不得**回退成"杀直接子进程即成功" |
| `SetInformationJobObject` 失败 | 未设置 `KILL_ON_JOB_CLOSE` | 关闭句柄；清理状态未证明 |
| `AssignProcessToJobObject` 失败 | 常见 `ACCESS_DENIED`（进程已在某个不允许嵌套/有 UI 限制的作业中）、`INVALID_PARAMETER`（句柄无效）、进程已退出 | 不得假定成员已受控；按未证明处理 |
| 进程已退出竞态 | `Assign` 时子进程已终止 | 若作业内计数为 0 且直接子进程已退出，可判"作业内无成员"，但**仍不能证明整棵树** |
| 作业嵌套 | bridge 本身已在 IDE/CI 的作业中；Win8+ 默认支持嵌套，但外层作业的 `BREAKAWAY` 策略会影响语义 | 需实测确认；不能按"必然可加入"设计 |
| `TerminateJobObject` 失败 | 返回 0 | 清理未证明；继续轮询计数不能替代终止成功 |
| `QueryInformationJobObject` 失败 | 拿不到 `ActiveProcesses` | 判"未证明"，**不得**把查询失败当 0 |
| 进程主动 `BREAKAWAY` | 仅当外层作业允许时可能 | 逃出作业 → 观测点看不到；属不可判定残余 |
| 权限不足 | 非同一用户/受保护进程 | `Assign`/`Terminate` 失败 → 未证明 |

### 4.4 依赖与构建影响（已核实的事实）

- **当前是否已有可用依赖**：
  - `windows-sys 0.61.2` **在 `Cargo.lock` 中**，但只经 `dev-dependencies` 的 `tempfile 3.27.0` 进入**测试构建**；`cargo tree -e normal -p aw-mcp-bridge` **不显示**它。生产代码现在**不能**直接使用。
  - `winapi` **不在依赖树**（`cargo tree -i winapi` 报不存在）。
- 新增方式：`[target.'cfg(windows)'.dependencies] windows-sys = { version = "0.61.2", features = [...] }`。
  - **不新增包、不改变 `Cargo.lock` 版本**（0.61.2 已锁定）；`windows-link 0.2.1` 也已在图中（chrono 的正常依赖 + windows-sys 的依赖）。
  - 许可：`MIT OR Apache-2.0`，与 workspace 的 `Apache-2.0` 兼容。
  - 源码体积：**17.34 MB**（registry 中全特性源码；实际编译只按 feature 选中的模块）。
  - MSRV：**1.71**（workspace 为 1.85，满足）。
  - 特性集（预计，以编译器实际要求为准）：`Win32_Foundation`（`HANDLE`/`CloseHandle`）、`Win32_System_JobObjects`（Job API 与常量）；`CreateJobObjectW` 的参数类型引用 `SECURITY_ATTRIBUTES`，可能还需 `Win32_Security`。
  - 构建影响：仅 Windows 目标编译更多 `windows-sys` 模块；`windows-sys` 只提供 `extern` 声明与常量，**不引入额外运行时代码**；二进制增量主要是我们自己新增的少量代码，**未实测**。
- 不推荐的替代：`winapi`（不在树、需新增包）；手写 `extern "system"` FFI（零依赖但需自己维护 ABI，全部 `unsafe`）。

### 4.5 W2c（自举 launcher）专项评估（2026-11-06，只评估，不实现）

本节回答"要不要做 W2c"。结论先行：**不做**，理由与 `#3` 现状在 §4.5.5 固定。以下评估基于 v1.17 现有实现（`execute_task` + `process_scope`），不假设任何尚未落地的改动。

#### 4.5.1 方案形态（W2c 具体长什么样）

1. bridge 用**可继承句柄**创建作业（`CreateJobObjectW` 传 `SECURITY_ATTRIBUTES{ bInheritHandle: TRUE }`），把句柄数值经环境变量（如 `AW_JOB_HANDLE`）传给 launcher；
2. bridge 不再直接 spawn `cmd /C <cmd>`，而是 spawn 辅助可执行文件 `aw-exec-launcher`，参数为原始 `cmd` 字符串；
3. launcher 先用 `AssignProcessToJobObject(job, GetCurrentProcess())` **把自己加入作业**，再把作业句柄标记为不可继承（`SetHandleInformation(HANDLE_FLAG_INHERIT, 0)`）并清掉控制环境变量，然后 spawn `cmd /C <cmd>`；
4. launcher 等待 `cmd` 结束，把 `cmd` 的退出码原样作为自己的退出码；bridge 仍用 `try_wait` 观察 launcher，超时仍 `TerminateJobObject`；
5. launcher 的"入作业失败 / 启动 `cmd` 失败"必须与"`cmd` 的退出码"区分开，否则回执会把工具链故障记成命令失败。

与现有 W2d 的差别只在第 1-3 步：**把"bridge spawn 后再 Assign"改成"launcher 先入作业再 spawn"**，从而让 bridge 派生的所有后代（launcher 自己 + `cmd` + 孙进程）都在作业内，`spawn → Assign` 窗口只存在于"launcher 自身被 spawn 到 launcher 执行 Assign"之间（launcher 尚未启动任何后代，因此该窗口对**后代**不再存在）。

#### 4.5.2 真实工作量：要改什么

| 位置 | 改动 | 说明 |
|---|---|---|
| `apps/mcp-bridge/src/server.rs` | 作业创建改为可继承句柄；`execute_task` 改为 spawn launcher；新增 launcher 控制文件/错误映射；保留 `ActiveProcesses` 复核 | 改动集中在 `process_scope::Supervisor::new`、`execute_task` 的 spawn 段（约 L741-L802）与 `timeout_reason` 的 `Unavailable` 分支说明；`stdout/stderr` 仍用同一对文件句柄 |
| 新增 `apps/mcp-bridge/src/bin/aw-exec-launcher.rs`（或独立 crate） | launcher 本体：解析参数、`AssignProcessToJobObject`、`SetHandleInformation`、spawn `cmd`、等待、退出码透传、控制文件写入、句柄关闭 | 全部 `unsafe` FFI（`windows-sys` 已可用）；约百行量级，但每一条都是新的正确性面 |
| `apps/mcp-bridge/Cargo.toml` | 若 launcher 与 bridge 同 crate：显式 `[[bin]]`；若独立 crate：新增 workspace 成员 | 不新增第三方包；`windows-sys` 特性已够用（`Win32_System_JobObjects` / `Win32_Foundation` / `Win32_Security` / `Win32_System_Threading`） |
| `apps/mcp-bridge/tests/mcp_contract.rs` | 新增用例：launcher 缺失、launcher 入作业失败、launcher 启动 `cmd` 失败、退出码透传、超时清理、逃逸仍逃逸 | **测试定位问题**：集成测试的可执行文件在 `target/debug/deps`，launcher 在 `target/debug`；bridge 运行时按"自身 exe 同目录"找 launcher 会找不到，必须新增 `AW_LAUNCHER_PATH` 覆盖或测试内复制，测试夹具复杂度上升 |
| 契约 `PM-AW-DISPATCH-CONTRACT.md` | 新版本（v1.18）：新增 `blocked_reason` 前缀（如 `launcher_unavailable:` / `launcher_failed:`）、`task.execute` 前置条件（launcher 必须随 bridge 分发） | **本轮明确不动契约**；这是 W2c 的额外成本，不是顺带 |
| `tools/list` 描述 + 审计文档 | 描述要写 launcher 依赖与新失败码；§12 复审需重跑 | 与上一轮刚稳定的描述口径再次分叉 |
| 仓库外：桌面端打包/安装/签名 | 第二个可执行文件必须随 bridge 一起安装、同版本、签名、过企业 AV/EDR 白名单 | 本仓库没有打包脚本（已核实：只有 `Cargo.toml` 的 `[[bin]]`，无安装器），成本落在消费方 |

**对现有流程 / 回执 / 幂等签名的影响**：

- `task.execute` 流程：多了一跳进程；超时计时仍从 spawn 前开始（launcher 启动耗时计入 `timeout_secs`），命令可用时间略缩短（毫秒级）。
- **回执字段 JSON 不变**（`exit_code` / `stdout_digest` / `stderr_digest` / `duration_ms` / `blocked_reason` 等结构不动），但**语义必须补丁**：`exit_code` 必须是 `cmd` 的退出码，launcher 自身错误必须走新的 `blocked_reason`，不能混入 `exit_code`，否则回执会说谎。这是 W2c 必须同步改契约的唯一硬理由。
- **幂等签名不能变**：`signature` 目前是 `task_id/cmd/cwd/timeout_secs/env/path_prepend` 六项（`server.rs` 约 L703-L710）。若把 launcher 路径或版本加入签名，已存在 `task_id` 的重投会因签名不同被拒（`replay_existing` 的语义被破坏）。因此 launcher 只能作为实现细节，签名保持六项——这意味着 launcher 版本漂移不会体现在签名里，需要额外的握手/版本校验，又增加一个失败面。
- `env_keys` 仍只记录派单方给出的 env；launcher 的控制变量必须在 spawn `cmd` 前删除，否则 `cmd` 的实际环境与 v1.17 不同（可观测行为变化）。

#### 4.5.3 新增失败模式：会不会比竞态更糟

| 失败模式 | 触发点 | 后果 | 与 W2d 竞态相比 |
|---|---|---|---|
| **launcher 缺失 / 被杀软隔离** | 分发漏拷、升级只换 bridge、AV/EDR 误报 | **所有 Windows `task.execute` 直接 blocked**（`launcher_unavailable`） | **更糟**：竞态只影响"超时任务里的少数后代"，这是全量不可用 |
| 可继承句柄创建/继承失败 | `SECURITY_ATTRIBUTES` 或 std 的句柄继承行为变化 | launcher 拿不到有效句柄 → 入作业失败 → 回落成"只有直接子进程被杀" | 更糟：多一层实现却回到甚至弱于 W2d |
| 作业句柄泄漏进 `cmd` 后代 | 未在 spawn `cmd` 前 `SetHandleInformation` 去掉继承位 | `KILL_ON_JOB_CLOSE` 要等最后一个句柄关闭；bridge 崩溃时清理被拖延；后代还能拿到句柄终止同伴 | 新增风险，W2d 不存在 |
| launcher 崩溃 / 提前退出 | launcher 自身 bug、被杀 | `cmd` 可能已启动但没人等待：退出码丢失、`status` 误判；若 `cmd` 仍在作业内，超时可清，但正常结束的回执不可信 | **更糟**：回执失真比"披露过的窗口"更难接受 |
| 退出码语义混淆 | 用哨兵退出码区分 launcher 错误，与 `cmd` 真实退出码碰撞 | 把 launcher 故障记成命令失败，或反之 | 更糟：需要控制文件或带外通道，复杂度继续上升 |
| 参数/引号二次传递 | 命令含 `"` `%` `!` `^` 等 | 特殊字符命令行为回归 | 新增回归面（两次 argv 解析） |
| launcher 版本漂移 | 升级只替换其一 | 协议不一致，行为不确定 | 新增风险：需要版本握手 |
| 安全面扩大 | 可继承作业句柄进入不受信命令的子进程 | 子进程可 `TerminateJobObject` 杀同作业成员 | 新增风险 |
| 调试/支持成本 | 两进程、两段生命周期、控制文件 | 故障定位更难 | 新增成本 |

关键判断：**W2a 已用极小代价闭合"恢复前派生子进程"的窗口，且不引入新产物；W2c 的边际收益接近零，却引入"全局、单点、能让所有 Windows 执行不可用"的新风险。** 用全局可用性换一个已被覆盖的窗口，风险收益比不成立。

#### 4.5.4 与 v1.20 W2a 的对比

| 维度 | W2c（自举 launcher） | v1.20 W2a（挂起创建 + 入作业 + 恢复） |
|---|---|---|
| `spawn → Assign` 窗口（对后代） | 基本闭合（launcher 先入作业再生后代） | **已闭合**：恢复执行前不产生任何后代；有确定性夹具（§4.2/§6.5） |
| `BREAKAWAY` / 外层作业策略 | **仍可能逃逸** | 同样可能逃逸 |
| `#3`（整棵树） | **仍是 `NOT_PROVEN`** | `NOT_PROVEN` |
| 收益可验证性 | 窗口闭合可验证，但整体仍不可判定 | 窗口闭合已用标记文件验证；逃逸负例仍 `UNPROVEN` |
| 失败爆炸半径 | 全局（launcher 缺失 = 所有 Windows 执行 blocked） | 局部（Toolhelp/Resume 失败只影响该次执行，且记 `process_resume_failed`） |
| 回执真实性风险 | 新增 launcher 错误/退出码混淆面 | 无新增；三态判据已实测 |
| 分发/签名/AV | 新增可执行文件 + 仓库外打包改造 | 无新增产物 |
| 契约/描述/审计面 | 需新失败码 + 描述/审计重跑 | v1.20 已同步（契约/描述/审计） |
| 实现与回归面积 | launcher FFI + bridge 改造 + 新测试夹具 | 已落地并两平台实测（Windows 4 unit + 65 integration，POSIX 3 + 64） |

**决定性问题**：W2a 已以极小代价闭合了 W2c 唯一想收窄的"后代窗口"，W2c 的边际收益接近零；而 `BREAKAWAY`/整树仍不可判定，`#3` 不会因此变绿。继续做 W2c 只增加全局失败面与分发成本，因此不做。

#### 4.5.5 终态结论：不做 W2c（固定，不留待办）

- **决定：不实施 W2c，也不实施 W2b；Windows 在 v1.16 用 W2d，v1.20 起用 W2a。** 本节是终态结论，不是"以后再说"。
- **理由（固定）**：
  1. 收益已被 W2a 覆盖：`spawn → Assign` 的后代窗口在 v1.20 用挂起创建闭合，且有确定性夹具；W2c 的边际收益接近零。`BREAKAWAY` 仍逃逸，`#3` 仍是 `NOT_PROVEN`。
  2. 失败模式更糟：launcher 缺失/被隔离会让**全部** Windows 执行 blocked，爆炸半径远大于局部竞态。
  3. 代价外溢：需新可执行文件、分发/签名/AV、契约与描述审计重跑；其中分发成本不在本仓库可控。
  4. 回执风险：launcher 错误与 `cmd` 退出码混淆会直接破坏"回执不说谎"的底线，而现状三态判据已实测。
- **`#3` 现状固定**：契约与文档保持 `NOT_PROVEN`（整棵树不可判定）；Windows 逃逸负例 `UNPROVEN`；回执只在 `Cleaned` 分支写"已清理受控集合；逃逸进程是否仍在运行未验证"并写明挂起创建/恢复顺序与残余逃逸；任何分支不写"已终止进程树 / 无残留"。
- **重新开题的门槛（不是待办）**：①确定性"无逃逸"观测点（不是"恢复前未执行"夹具）②可验证的整树闭合判据 ③覆盖 `BREAKAWAY`/嵌套作业 ④新的契约版本；四者缺一即维持现状。**不把 W2c 列入任何待办或候选清单。**

---

## 5. POSIX 方案：独立进程组（`setsid`/`setpgid` + `kill(-pgid, ...)`）

### 5.1 实现（v1.16，已落地）

```text
Command::new("sh").arg("-c").arg(cmd)
    .process_group(0)          // std::os::unix::process::CommandExt，exec 前建组
    .spawn()
超时:
    kill(-pgid, SIGTERM)       // 先礼
    等待最多 500 ms（轮询 kill(-pgid, 0)）
    仍存在 → kill(-pgid, SIGKILL)
    child.wait()
    轮询 kill(-pgid, 0) 直到 ESRCH 或 3 s 期限
      ESRCH        → Cleaned
      0 / EPERM    → 仍存在（EPERM 按存在处理）
      超期仍非 ESRCH → StillAlive
```

优点：

- `process_group(0)` 在**子进程 exec 之前**设置进程组，**没有"spawn 后加入"的竞态**；这是相对 Windows 的明显优势。
- 管道、非交互 `sh -c` 的后台任务默认留在同一进程组；`kill(-pgid)` 可一次覆盖组内成员。
- 观测点 O1 直接回答"组内是否还有进程"，不受写盘影响。

### 5.2 关键不确定性：包装/脚本可能自建新组

| 场景 | 是否仍在原进程组 | 说明 |
|---|---|---|
| `sh -c 'sleep 30'` | 是（多数 shell 会 `exec` 优化，直接子进程即 `sleep`） | 简单命令反而最容易被直接杀掉 |
| `sh -c 'sleep 30 & wait'` | 是（非交互 shell 未开 job control，后台任务同组） | 孙进程在组内，O1 可覆盖 |
| 管道 `a | b` | 是（非交互 shell 不建新组） | 组内多进程 |
| 脚本内 `setsid ...` / `daemon()` 双 fork | **否** | 逃逸到新会话/新组，O1 看不到 |
| `systemd-run` / `sudo`（某些配置） / 容器启动器 | **可能否** | 取决于工具是否新建会话/组 |
| 交互式 shell | 可能建新组 | bridge 用非交互 `sh -c`，通常不触发 |

### 5.3 失败情形（逐条写死，均不得当作成功）

| 情形 | 表现 | 正确处置 |
|---|---|---|
| `process_group(0)` 失败 | `spawn` 直接报错 | 任务不成立；按工具错误处理（现有 spawn 失败路径） |
| 直接子进程已退出 | `kill(-pgid, ...)` 可能 `ESRCH` | 若组内确无成员，可判"组内已空"；但需先确认不是"组尚未建立"的误判 |
| 进程已自建新组 | `kill(-pgid)` 命中不到 | 观测点看不到 → 未证明；**不得**判"全树已清" |
| 权限不足 | `EPERM` | 存在但不可信号 → 未证明 |
| pgid 复用 | 组已空后新进程复用同一 pgid | 短窗口内概率极低但非零；需配合启动时间或直接子进程状态降低误判 |
| 僵尸未回收 | `kill(-pgid, 0)` 仍返回 0 | 需轮询到期限；若期限内不消失，如实报"组内仍有进程（可能是僵尸）" |
| `SIGTERM` 被忽略 | 组内仍存活 | 进入 `SIGKILL` 阶段；仍以 O1 观察结果为准 |

### 5.4 依赖与构建影响（已核实的事实）

- `std::os::unix::process::CommandExt::process_group`：**std 自带**，Rust 1.64 稳定，workspace MSRV 1.85 满足，**无需新增依赖**。
- 发信号需要 `kill()`：
  - `libc 0.2.186` **在 `Cargo.lock` 中**，但同样只经 `dev-dependencies` 的 `tempfile` 进入测试构建；`cargo tree -e normal` 不显示。新增 `[target.'cfg(unix)'.dependencies] libc = "0.2"` **不新增包、不改版本**。
  - 许可：`MIT OR Apache-2.0`；源码体积 **4.17 MB**；MSRV **1.65**。
  - 替代：手写 `extern "C" { fn kill(pid: i32, sig: i32) -> i32; }`，零依赖但需自己声明常量与处理 errno，全部 `unsafe`。
- 构建影响：仅 Unix 目标编译 `libc`；`libc` 是薄 FFI 声明，二进制增量主要是新增代码，**未实测**。

---

## 6. 验收判据：哪些可判定、哪些不可判定

### 6.1 明确否掉"两次 `stat` 无变化"

理由见 §2。**任何以文件副作用为代理的判据都不进入验收。**

### 6.2 可判定的窄判据（仅证明"受控集合"清空；v1.16 已实现）

**C1（POSIX）**：

```text
在超时路径执行 SIGKILL 之后，轮询 O1：
  - kill(-pgid, 0) 返回 ESRCH  → 组内无进程 → PASS（作用域：进程组）
  - 返回 0 / EPERM            → 组内仍有进程 → FAIL（继续轮询到期限，仍非 ESRCH 即 FAIL）
  - 超过清理期限仍非 ESRCH     → FAIL（如实报"组内仍有存活进程"）
辅助条件：child.wait() 返回 Some（直接子进程已终止）
```

**C2（Windows）**：

```text
TerminateJobObject 成功后，轮询 O4：
  - ActiveProcesses == 0 → 作业内无活动进程 → PASS（作用域：作业成员）
  - 查询失败             → UNPROVEN（不得当作 0）
  - 超过清理期限仍 > 0   → FAIL
辅助条件：直接子进程句柄有信号（O6）
```

两条判据的共同点：**查询的是进程/作业是否存在，不是文件是否变化**。因此：

- 被 `SIGSTOP` 暂停的进程：O1 返回 0 / O4 计数 >0 → 判"存在"，**不会假通过**；
- IO 阻塞但活着的进程：同上 → 判"存在"；
- 已退出的进程：`ESRCH` / `ActiveProcesses == 0` → 判"不存在"。

### 6.3 两条窄判据的假通过 / 假失败矩阵

| 判据 | 假通过（判"清空"但实际有进程） | 假失败（判"仍有进程"但实际已无） |
|---|---|---|
| C1 | 后代 `setsid`/`setpgid` 逃出组；pgid 复用 | 僵尸未被回收；权限导致 `EPERM`；pgid 复用 |
| C2 | `BREAKAWAY`；`Assign` 失败但被忽略（v1.20 已关闭"恢复执行前派生后代"窗口） | `Query` 失败被误读；终止过程中的瞬时计数（未轮询到期限）；句柄失效 |

**所以 C1/C2 只能证明作用域内清空，不能证明整棵树。** 这正是它们不能被写成"`#3` 已解决"的原因。

### 6.4 不可判定的全树判据（`#3` 保持 `NOT_PROVEN` 的理由）

要判定"整棵进程树已不存在"，必须在**任意时刻**枚举起从 bridge 派生出去的所有进程，包括已经脱离父进程、脱离进程组、跑到别的会话里的后代。当前环境做不到：

1. **POSIX 没有可移植的后代枚举**：`/proc` 扫描只在 Linux 有，且是 TOCTOU（扫描后进程可再派生/退出），macOS 无对应物；`kill(-pgid)` 只看组。
2. **`setsid` 逃逸不可观测**：一旦后代自建会话，bridge 没有任何 OS 句柄/计数器能追踪它。
3. **Linux `PR_SET_CHILD_SUBREAPER` 只能覆盖"成为孤儿后被 reparent 到 bridge"的进程**，仍不是完整后代枚举；且会改变 bridge 全局的孤儿回收语义，影响其他任务；macOS 无此机制。
4. **cgroup v2 是唯一接近"整棵树"的机制**，但要求 cgroup 委派/容器权限，当前构建与运行环境未验证可用；即便可用，有权限的进程仍可把自己移出 cgroup。
5. **Windows 的逃逸**：v1.20 已关闭"恢复执行前派生后代"的窗口，但 `BREAKAWAY`、`Assign` 失败后的后代、嵌套作业/外层策略仍会让成员不在作业内；`ActiveProcesses` 只数作业成员。
6. **"某一时刻看不到"不等于"以后不会出现"**：只要还有逃逸的父进程活着，它可以在观测之后继续派生。没有对逃逸父进程的追踪，就无法用"多次采样为 0"推出"整树不存在"。

**结论：`#3` 应保持 `NOT_PROVEN`。** v1.16 实现 C1/C2 后，能诚实宣称的只有："已清理**受控集合**（进程组/作业）；逃逸进程是否仍在运行未验证"。该句在 v1.17 状态门满足后**已进入回执**（只有 `Cleaned` 分支使用），且它**不等于**整棵树，状态不升级。

### 6.5 判据的可执行性：已落地的用例（v1.16 实现，v1.17 两平台实跑）

| 测试目的 | 用例（位置） | 夹具 | 断言 | Windows 结果 | POSIX 结果 |
|---|---|---|---|---|---|
| 正向：孙进程在受控集合内 | 集成 `timeout_cleans_controlled_set_including_grandchild`；单元 `job_counts_grandchild_and_empties_after_terminate`（Windows） | POSIX `echo $$ > pids.txt; sleep 30 & echo $! >> pids.txt; wait`；Windows PowerShell `Start-Process ping -n 30` 并写 PID；Windows 单元 `cmd /C ping` | 回执走 v1.20 `Cleaned` 分支；记录的 PID 均不再存活；POSIX 另断言 `kill(-pgid,0)` 为 `ESRCH`；Windows 单元打印 `ActiveProcesses_before > 0` → `terminate=Cleaned` → `ActiveProcesses_after == 0` | **通过（实跑）**：`grandchild attached=true ActiveProcesses_before=Some(1)` / `terminate=Cleaned ActiveProcesses_after=Some(0)`；集成打印 `grandchild pid=… process_exists_after=false`（PID 随运行变化） | **通过（实跑）**：`running 64 tests` 全通过；`[evidence] grandchild_reason=timeout：超过 1s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证` |
| **反假通过**：活着但不写盘 | 单元 `stopped_process_still_counts_as_alive`（Unix）/ `alive_but_idle_process_is_counted_in_job`（Windows） | POSIX `sleep 30` + `SIGSTOP`；Windows `ping -n 30`（活着但不写盘） | 判据必须判"存在"（`kill(-pgid,0)` 为 0 / `ActiveProcesses > 0`），清空后才转 `Cleaned` | **通过（实跑）**：`alive_but_idle attached=true ActiveProcesses_before=Some(1)` → `terminate=Cleaned ActiveProcesses_after=Some(0)` | **通过（实跑）**：`stopped pgid=939 kill(-pgid,0)_exists=true` → `stopped after_SIGKILL group_empty=true` |
| 反向：已退出 | 单元 `exited_process_group_has_no_members`（Unix）/ `exited_process_leaves_job_empty`（Windows） | POSIX `sh -c 'exit 0'`；Windows `cmd /C "ping -n 2 ..."` 自然退出 | `ESRCH` / `ActiveProcesses == 0` → 判"不存在" | **通过（实跑）**：`exited_process ActiveProcesses_final=Some(0)` | **通过（实跑）**：`exited pgid=937 group_empty=true` |
| **作用域负例**：逃逸 | 单元 `setsid_escape_is_outside_the_observed_group`（`#[cfg(target_os="linux")]`） | `setsid sleep 30 &`，PID 写入临时文件 | 组判据返回 `ESRCH`，但 `kill(已知PID, 0)` 为 0 → **证明 C1 是窄判据**（不断言"#3 解决"） | 不适用（Windows 无等价确定性夹具，记为 `UNPROVEN`） | **通过（实跑）**：`setsid escaped_pid=941 escaped_alive=true original_group_empty=true` |
| 文案门禁 | 集成 `timeout_reason_states_action_not_tree_death` + `legacy_timeout_reason_is_returned_verbatim_and_not_rewritten` | 真实超时任务 + 旧 `handback.json` | **必须为 v1.20 三态文案**（`Cleaned` 分支含"已清理受控集合…未验证"，Windows 另含"挂起态创建…加入作业…恢复执行"，且禁止 `spawn→Assign`）；**不得**出现"已终止进程树 / 无残留"；旧回执原样返回、字节不变 | **通过（实跑）**：`[evidence] timeout_reason=timeout：超过 1s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证；Windows 下子进程以挂起态创建并先加入作业再恢复执行（恢复前不会派生子进程）；BREAKAWAY 或加入作业失败的作业外进程仍不可观测` | **通过（实跑）**：`[evidence] timeout_reason=timeout：超过 1s 未结束，已清理受控集合；逃逸进程是否仍在运行未验证` |
| **窗口收窄**：恢复前零执行 | Windows 单元 `suspended_start_defers_execution_until_job_assign`（`#[cfg(windows)]`） | `cmd /C "echo started> started.txt & ping -n 30 ..."`，以 `CREATE_SUSPENDED` 创建 | 入作业前 300ms 内标记文件不得出现、`ActiveProcesses == 0`；恢复后标记出现、`ActiveProcesses > 0`；`terminate=Cleaned` 后为 0 | **通过（实跑）**：`[evidence] suspended_start marker_before_assign=false marker_after_resume=true attached=true ActiveProcesses_before_terminate=Some(2)`；`terminate=Cleaned ActiveProcesses_after=Some(0)` | 不适用（Windows 专属路径；POSIX 无 spawn→Assign 竞态） |

**如实说明**：POSIX 四类用例已在 WSL（Ubuntu 24.04、Rust 1.88.0、gnu target）真实运行：单元 `3 passed` + 集成 `64 passed`，`0 failed`，退出码 0。Windows 侧单元 `4 passed` + 集成 `65 passed`，`0 failed`（v1.20 复跑，同一判据打印）。Windows 的 `BREAKAWAY` / 加入作业失败仍无法确定性复现，按 §6.6 记为 `UNPROVEN`，不做"已覆盖"断言；v1.20 只证明"恢复执行前未执行"，**不证明整棵树**。可复跑脚本见 §6.7。

### 6.6 判据的判定边界（写死）

- **PASS** 只表示"受控集合内无进程"；不得在任何文档/回执里升级为"整棵树已终止"。
- **FAIL** 表示"受控集合内仍有进程"或"观测不可用"；必须如实报告，不得自动重试、不得降级为"已清理"。
- **UNPROVEN** 表示观测点不可用（作业创建/加入失败、查询失败、权限不足）；必须原样暴露。

### 6.7 POSIX 用例复跑（脚本，v1.17 固化）

**先说清一条环境限制（不是"用例不存在"）**：Windows 本机直接 `cargo test` 跑不到这四类 POSIX 用例——三个 Unix 单元用例在 `#[cfg(unix)]` 下编译，Windows 目标根本不会生成它们；集成用例 `timeout_cleans_controlled_set_including_grandchild` 的 POSIX 分支依赖 `sh -c` + `kill(-pgid, 0)`。四类用例在源码中真实存在（[`server.rs`](../apps/mcp-bridge/src/server.rs) 的三个 Unix 单元 + [`mcp_contract.rs`](../apps/mcp-bridge/tests/mcp_contract.rs) 的该集成用例），必须在 Linux 主机（WSL 也算）上编译运行。本机 Windows 只跑 Windows 侧（4 unit + 65 integration）。

**一行复跑**：

```powershell
wsl -d <WSL 发行版> -e bash /mnt/e/软件开发/云集编程工作站/4.AgentWork/scripts/posix-timeout-verify/verify.sh run
```

（已进入 WSL 时：`cd <仓库根> && bash scripts/posix-timeout-verify/verify.sh run`。）

**脚本**：[`scripts/posix-timeout-verify/verify.sh`](../scripts/posix-timeout-verify/verify.sh)

| 子命令 | 联网 | 行为 |
|---|---|---|
| `check` | 不需要 | 自检：Linux 平台、`setsid`、rustc >= 1.85（推荐 1.88.0）、rustc host 为 `linux-*`、cc/gcc、vendor 完整性、仓库完整性 |
| `prepare` | **需要** | 缺 Rust 时用 rustup / rustup-init 安装 1.88.0；生成只含 `apps/mcp-bridge` 的裁剪 workspace；`cargo vendor --versioned-dirs` 生成 vendor；写 `.cargo/config.toml` 指向 vendor |
| `run` | 不需要（强制 `--offline`） | 把当前仓库源码复制进 work 目录 → 裁剪 workspace → `cargo test --offline -p aw-mcp-bridge -- --nocapture` → 校验四类用例名与 9 条证据串 → 打印日志路径 |
| `all` | 同 `prepare` | `prepare` + `run` |

- 环境变量：`AW_WORK`（默认 `$HOME/.aw-posix-verify`，持久目录，不用 `/tmp`）、`AW_VENDOR`（默认 `$AW_WORK/vendor`）、`AW_OFFLINE=1`（禁止联网）。
- **离线预置清单**（`--help` 与 `check` 原样打印）：① Rust >= 1.85；② `cc`/`gcc` + libc 头文件（Ubuntu：`sudo apt install -y gcc libc6-dev`）；③ vendor 目录（含 `libc-*` / `serde-*` / `serde_json-*` / `sha2-*` / `hex-*` / `chrono-*` / `tempfile-*`，每个 crate 带 `.cargo-checksum.json`）；④ 本仓库 checkout（根 `Cargo.toml` / `Cargo.lock` / `apps/mcp-bridge`）。
- **失败原因码**：所有失败统一 `[FAIL] reason=<码> hint=<处置>`、退出码 2，不静默失败、不跳过用例：`wrong_platform` / `wrong_target` / `missing_setsid` / `rust_missing` / `rust_broken` / `rust_too_old` / `rust_install_failed` / `missing_linker` / `repo_incomplete` / `vendor_missing` / `vendor_incomplete` / `vendor_failed` / `test_failed` / `missing_evidence` / `bad_usage`。
- **本次脚本自检（2026-11-06，现有 WSL Ubuntu 24.04）**：`check` 通过（`rustc 1.88.0`、`host=x86_64-unknown-linux-gnu`、vendor 完整）；`run` 在现有 work 目录上离线跑通 `3 passed` + `64 passed`，四类证据齐全、退出码 0；负例三条均给出明确原因：无 Rust → `rust_missing`、vendor 缺失 → `vendor_missing`、伪造 `host: x86_64-pc-windows-msvc` → `wrong_target`。
- **本机网络限制（如实说明）**：该 WSL 对外网络受限（apt 源指向不可用代理、`static.rust-lang.org` 极慢），本次验证使用**预置** Rust 与 vendor（`AW_VENDOR=/root/vendor`），未在本机重跑 `prepare` 的下载分支；缺 vendor 且 `AW_OFFLINE=1` 时脚本按 `vendor_missing` 明确失败，不会联网猜测或静默跳过。
- **2026-11-07 复核（v1.31 收尾）**：同一 WSL 环境重跑 `run`：`#3` 四类证据行齐全，`timeout_cleans_controlled_set_including_grandchild` / `timeout_reason_states_action_not_tree_death` 通过；POSIX 总计 `104 passed; 1 failed`，失败项与本项无关——`standalone_argv_scope_rejects_escapes_before_execution` 的 `argv-backslash` 用例（`..\..\escape.txt` 在 POSIX 是合法文件名而非路径分隔，扫描判 `in_scope` 后执行成功）。该用例属 v1.28，v1.31.1 已按平台分隔符修正夹具并重跑，POSIX `9 lib + 108 integration` 全绿（红/绿原文见审计 §32）；该失败与 `#3` 无关。

### 6.8 "逃逸未验证"何时可被机器判定（门槛，不是待办）

当前三态回执里的"逃逸进程是否仍在运行未验证"不是措辞保守，而是**缺少观测点**。要把它换成机器判定（无论结论是"有逃逸"还是"无逃逸"），必须同时满足：

1. **完整枚举**：平台提供可枚举"全部后代"的稳定句柄/列表，包括已 `setsid`、已换父进程、已被收养的进程，并能给出 PID + 启动时间（防止 PID 复用）；仅有"受控集合成员数"不够。
2. **全时段覆盖**：观测点从创建到判定结束持续有效，没有 TOCTOU；判定"无逃逸"必须覆盖"逃逸父进程在观测之后继续派生"的可能，不能靠单次采样。
3. **三态可分**：观测点必须能区分"确实不存在"与"不可观测"；查询失败只能记 `Unavailable`，不得当 0（现有 C1/C2 已遵守）。
4. **平台闭合**：POSIX 需要 cgroup v2 委派且进程不能被移出，或 `PR_SET_CHILD_SUBREAPER` + 带启动时间的全量 `/proc` 扫描；Windows 需要作业覆盖全部后代、禁止 `BREAKAWAY`/嵌套逃逸，并能枚举"作业外候选"。

现状逐条核对（v1.34 新增观测点后；实测见 §6.10）：

| 平台 | 1 完整枚举 | 2 全时段 | 3 三态可分 | 4 平台闭合 | 结论 |
|---|---|---|---|---|---|
| POSIX | 否（`/proc` 父链在孤儿被收养时断开；subreaper 只覆盖孤儿，macOS 无） | 否（换父后无法追；根退出后快照必为空，属于假空） | 是（`ESRCH`/`EPERM` 可分；`unobservable` 与空快照分离） | 否（cgroup v2 未验证，进程可被移出） | 继续 `partial / NOT_PROVEN`（PD-3 剩余） |
| Windows | 是（Toolhelp 父链快照；`ParentProcessID` 不因父进程退出而改写，PID 复用只会多报不会漏报） | 是（终止后快照为空即不存在可继续派生的存活后代） | 是（快照/作业列表查询失败一律 `Unavailable`） | 是（`CREATE_SUSPENDED` + 恢复前入作业；本作业未设 `BREAKAWAY_OK`，嵌套作业仍在外层作业内） | 整树清理可在 Windows 证明 |

v1.20 的挂起创建只满足"恢复执行前不产生后代"这一条，**不提供逃逸观测点**，因此不改变上表任何一格，也不改变 `#3 = NOT_PROVEN`。四条中任何一条不满足，"逃逸未验证"就必须保留。

v1.34 补上了观测点后，Windows 四格全部满足：`timeout.descendant_snapshot_cross_check` 为 `proven`，`timeout.cleans_entire_process_tree` 在 Windows 侧成立；POSIX 仍卡在"换父被收养"这一格，因此整条 claim 只能写 `holds=false / partial / posix_escaped_after_reparent_unobservable`，**不是**放宽判据换来的绿。

### 6.9 单次终止尝试是否足够（v1.24 实测；结论：一次即足够，不加重试）

要回答的问题：一次 `TerminateJobObject`（Windows）/ 一次 `kill(-pgid, SIGKILL)`（POSIX）之后，受控集合内还会不会有存活成员？"刚 spawn 未入集合 / 进程在内核态 / 被杀进程再拉起子进程"这三种条件哪几种真实存在？

#### 实测（真实进程，不是推理）

| 平台 / 夹具 | 轮数 | 做法 | 结果 |
|---|---|---|---|
| Windows Job Object：`cmd /C "for /L %i in (1,1,20000) do @start /b ping -n 30 127.0.0.1 > NUL"` 持续拉起后代 | 20 | 走生产路径 `CREATE_SUSPENDED → Assign → Resume`，30..220ms 后**一次** `TerminateJobObject(job, 1)`，再每 10ms 轮询 `ActiveProcesses` 3 秒 | `ActiveProcesses_before` = 2..21；**20/20 首次轮询即为 0**（`saw_nonzero_after_terminate=false`），`at_3s=Some(0)` |
| POSIX：`sh -c 'while :; do sleep 20 & done'` 高速 fork | 40 | `setsid` 建组，12..83ms 后一次 `kill(-pgid, SIGKILL)`，每 20ms 查 `kill(-pgid, 0)` 3 秒 | **40/40 组内 0 存活**（`alive_after_first_3s=False`，`members=0`） |
| POSIX：2GB 进程持续 `os.fork()`（单次 fork ≈16ms，SIGKILL 必落在 fork 窗口内） | 20 | 分配并触碰 2GB 后就绪，再 2..29ms 一次 `kill(-pgid, SIGKILL)` | **20/20 组内 0 存活** |
| POSIX：400 个已存在子进程（早期探针 `setsid sh -c 'i=0; while [ $i -lt 400 ]; do sleep 5 & ...'`） | 8 | 30ms 后一次 `kill(-pgid, SIGKILL)` | **8/8 组内 0 存活** |

合计 POSIX 68 轮、Windows 20 轮，**0 次出现"一次终止后仍有存活"**。

#### 为什么 fork 竞态没有留下幸存者

第 3 行把"fork 与 SIGKILL 同时发生"的窗口放大到约 16ms（2GB 页表复制），仍然 0 幸存。原因不是运气：Linux `copy_process` 在返回前检查 `signal_pending(current)`，致命信号已到达时**中止本次 fork**，不会产生"信号派发之后才出生"的组成员。Windows 侧作业成员关系在 `CreateProcess` 时原子继承（本实现不允许 `BREAKAWAY`），`CREATE_SUSPENDED` 又关掉了"恢复执行前派生后代"的窗口；20 轮持续派生场景下 `TerminateJobObject` 返回后 `ActiveProcesses` 首次查询即为 0。三种条件逐条落点：

- 刚 spawn 未入集合：Windows 由 `CREATE_SUSPENDED → Assign → Resume` 关闭（§6.5 `suspended_start_defers_execution_until_job_assign` 实跑）；POSIX 由 `process_group(0)` 在 exec 前建组，不存在"先运行后入组"。
- 进程在内核态：Windows `TerminateJobObject` 由内核终止当前全部成员；POSIX SIGKILL 对内核态进程同样生效（除 D 状态，见下）。
- 被杀进程再拉起子进程：POSIX 的 fork 在致命信号 pending 时被内核中止；Windows 的子进程在创建时原子继承作业，`TerminateJobObject` 覆盖当前全部成员。

#### 哪些情况仍会观测到 `StillAlive`，以及为什么重试不能把它变成 `Cleaned`

- **僵尸成员**：`kill(-pgid, 0)` 对僵尸仍返回成功，"组里只有僵尸"会被判为存在。生产代码在 SIGKILL 后立刻 `child.wait()` 收掉直接子进程；其余孤儿的父进程已死，由 init/收养者回收。对僵尸再发 SIGKILL 是 no-op，重试不会缩短这段时间。
- **冻结 / 不可中断**：cgroup v2 冻结构造（`cgroup.freeze=1` 后 `kill(-pgid,SIGKILL)`）显示进程**立即变 Z**（不是"杀不死"），二次 SIGKILL 无变化，解冻后消失（`alive_after_unfreeze=False`）。真正的 D 状态（I/O 不可中断）在本环境无法构造；但 SIGKILL 一旦 pending 就不会丢失，重发是同一个 pending 信号，不能加速唤醒。
- **判据本身**：`ESRCH` 与 `ActiveProcesses==0` 是"观测"而非"命令成功"；任何未观测到空的路径只能记 `StillAlive` / `Unavailable`，本批不放宽。

因此 v1.24 **不新增第二次终止尝试**：没有实测场景能证明"第二次尝试把 `StillAlive` 变成 `Cleaned`"；能构造出的 `StillAlive` 场景（僵尸、冻结）重试也无效。加轮询内重试只会把"如实报告仍有存活"变成"多杀几次再如实报告"，不改变判据，反而增加实现与门禁复杂度。

#### 锁定用例（两平台，均为真实进程）

- POSIX：`single_sigkill_suffices_under_fork_pressure` —— 组长 `trap '' TERM` 且持续 fork，宽限期结束后**必然**派发 SIGKILL；断言 `Cleaned`。绿：`[evidence] fork_pressure pgid=1472 group_exists_before=true outcome=Cleaned elapsed_ms=673`。
- Windows：`single_terminate_suffices_under_spawn_pressure` —— 成员持续 `start /b` 拉起 ping 子进程，6 轮；断言每轮 `Cleaned` 且 `after=Some(0)`。绿：`ActiveProcesses_before=Some(2..13) outcome=Cleaned ActiveProcesses_after=Some(0)`。

变异红灯（证明断言非空转；变异已逐字还原）：

- POSIX 把 `kill(-pgid, SIGKILL)` 改成只杀组长 `kill(pgid, SIGKILL)`：`outcome=StillAlive elapsed_ms=3524`，断言失败原文 `left: StillAlive / right: Cleaned`。
- Windows 把 `TerminateJobObject` 改成只 `child.kill()`：trial 1 `outcome=StillAlive ActiveProcesses_after=Some(4) elapsed_ms=3019`，同一断言失败。

### 6.10 后代观测点（v1.34 实测：Windows 可证整树，POSIX 仍缺换父一段）

设计（`process_scope::DescendantSnapshot`，只读观测，不新增终止逻辑）：

- 统一输出：`root` / `descendants[]{pid, in_controlled_set}` / `controlled_pids` / `unobservable`；`unobservable` 非空时 `is_empty()` 恒为 false，调用方不得读成"没有后代"。
- Windows：`CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)` 一次读出 `(pid, parent_pid)`，按父链闭环求后代；再用 `QueryInformationJobObject(JobObjectBasicProcessIdList)` 读作业成员；两集合交叉核对。
- Linux：扫描 `/proc/<pid>/stat`，以最后一个 `)` 为界解析 `pid (comm) state ppid pgrp`（兼容 `comm` 含空格/括号）；按 `ppid` 闭环求后代、按 `pgrp` 求组成员；`NotFound` 跳过，其余读失败一律 `unobservable`。
- 生产路径：终止后先看 `ActiveProcesses == 0` / `kill(-pgid, 0) == ESRCH`，再用快照复核；快照不可观测 → `Unavailable`，快照仍有后代 → `StillAlive`，只有两者都空才 `Cleaned`。这是收紧判据，不是放宽。POSIX 的快照复核在根退出后通常必然为空（换父已完成），因此它只可能把结论收紧为 `StillAlive`，不可能把 POSIX 的空快照当成"整树已清"的证据。

实测（真实进程）：

- Windows `descendant_snapshot_cross_checks_job_accounting`：

```
[evidence] descendant_snapshot before root=124800 descendants=[DescendantProcess { pid: 113168, in_controlled_set: true }] controlled=Some([124800, 113168])
[evidence] descendant_snapshot after terminate=Cleaned descendants=[] controlled=Some([]) unobservable=None
```

  作业 PID 列表 = 根进程 + 父链快照后代；终止后两个观测点同时为空。

- Windows 负例 `descendant_snapshot_flags_descendants_outside_controlled_set`：

```
[evidence] descendant_snapshot outside_controlled root=172680 descendants=[DescendantProcess { pid: 187288, in_controlled_set: false }] outside=[187288] controlled=Some([])
```

  未加入作业的后代会被标成集合外，证明交叉核对不是"恒接受控"。

- Linux `descendant_snapshot_cross_checks_process_group`：

```
[evidence] posix_descendant_snapshot before root=701 descendants=[DescendantProcess { pid: 702, in_controlled_set: true }, DescendantProcess { pid: 703, in_controlled_set: true }] controlled=Some([701, 702, 703])
[evidence] posix_descendant_snapshot after terminate=Cleaned descendants=[] controlled=Some([]) unobservable=None
```

- Linux 边界 `setsid_escape_is_flagged_then_unobservable_after_reparent`：

```
[evidence] setsid_snapshot before escaped=874 descendants=[DescendantProcess { pid: 874, in_controlled_set: false }] outside=[874]
[evidence] setsid_snapshot after outcome=Cleaned escaped_alive=true descendants=[] unobservable=None
```

  根存活时逃逸可被标出；根退出后逃逸进程被收养，快照为空而进程仍存活——这正是 POSIX 仍不可证整树的实测依据。

红→绿（真实输出）：

- POSIX 首轮（占位实现）：`test result: FAILED. 9 passed; 2 failed`，失败原文 `必须能观测到进程组内的后代` / `根存活期间，逃逸后代必须能被父链快照标出`。
- Windows 变异红灯（临时停用作业 PID 列表观测点，已逐字还原）：`test result: FAILED. 0 passed; 2 failed`，失败原文 `必须能观测到作业内的后代` / `未加入作业的进程树也必须可枚举`。
- 绿：Windows `12 lib + 122 integration`、WSL Ubuntu 24.04（`cargo 1.88.0`）`11 lib + 120 integration` 全绿。

残余边界（明确不覆盖）：

1. Linux 孤儿被 init/子收割者收养后父链断开（实测），无法枚举；
2. macOS 无 `/proc`，快照恒为 `unobservable`；
3. Windows 的 PID 复用可能让快照多报"后代"——方向保守（会报 `StillAlive`，不会报假 `Cleaned`）；
4. 观测是终止后的采样，不是内核级订阅；平台若允许后代脱离父链并继续存活（POSIX 换父正是此类），采样无法证明"不存在"。

---

## 7. 依赖与构建影响汇总（已核实）

| 平台 | 能力 | 当前依赖状态 | 建议新增 | 版本 | 许可 | 源码体积 | MSRV | 是否新增包 | 构建影响 |
|---|---|---|---|---|---|---|---|---|---|
| Windows | Job Object API | v1.16 起为 `[target.'cfg(windows)'.dependencies]` 正常依赖 | `windows-sys`（已加入 `Cargo.toml`） | 0.61.2 | MIT OR Apache-2.0 | 17.34 MB（全特性源码） | 1.71 | 否（已锁定） | 仅 Windows 编译更多模块；未实测二进制增量 |
| Windows（备选） | Job Object API | `winapi` 不在树 | 不推荐 | — | — | — | — | 是 | 新增包 |
| POSIX | 建进程组 | `std` 自带 `process_group` | 无 | — | — | — | 1.64+ | 否 | 无 |
| POSIX | `kill(-pgid)` | v1.16 起为 `[target.'cfg(unix)'.dependencies]` 正常依赖 | `libc`（已加入 `Cargo.toml`） | 0.2.186 | MIT OR Apache-2.0 | 4.17 MB | 1.65 | 否（已锁定） | 仅 Unix 编译 libc；未实测二进制增量 |

补充事实：

- workspace `rust-version = "1.85"`，上述 MSRV 全部满足；
- workspace 许可为 `Apache-2.0`，两个候选依赖均为 `MIT OR Apache-2.0`，兼容；
- 转换前 `cargo tree -e normal -p aw-mcp-bridge` 不含 `windows-sys` / `libc`（只经 dev `tempfile`）；v1.16 改为 target-specific 正常依赖后**仍不新增包、不改版本**，`Cargo.lock` 版本不变。

---

## 8. 方案对比与建议

| 方案 | 能证明什么 | 成本 | 残余风险 | 是否可宣称 `#3` 解决 |
|---|---|---|---|---|
| A. 只做受控集合清理（POSIX 进程组 / Windows 作业）+ C1/C2 | "受控集合内无进程" | 中（Windows 需 FFI ~150-250 行；POSIX 较小） | 逃逸后代不可见；Windows `Assign` 竞态 | **否**（只能宣称窄判据） |
| B. 闭合 Windows 竞态（W2b/W2c）+ POSIX 保持组语义 | 同上，且 Windows 竞态窗口显著缩小 | 高（替换 spawn 路径或新增 launcher） | `BREAKAWAY`/`setsid` 仍不可见 | **否** |
| C. 引入 Linux cgroup v2 委派 | 接近整棵树（Linux） | 很高（部署/权限/容器适配） | 非 Linux 平台无等价物；可被移出 cgroup | **否**（跨平台仍不成立） |
| D. 不实现 | 无 | 零 | 维持现状 | 否（保持 `NOT_PROVEN`） |

**v1.16 的选择与结果**：

1. 采用**方案 A**（受控集合清理 + C1/C2），**不做方案 B/C**：W2c/W2b 与 cgroup 委派的成本与回归面超出本批范围。
2. 窄定义已由 PM 接受并落地实现；**v1.16.1 曾退回旧版文案**（POSIX 四类未实跑、Windows 逃逸负例 `UNPROVEN`）；**v1.17 状态门满足后启用三态新措辞**（POSIX 四类实跑通过 + Windows 三类通过 + 版本递增）。契约版本轨迹：v1.16（实现）→ v1.16.1（文案退回）→ v1.17（文案启用）。
3. **`#3` 保持 `NOT_PROVEN`**：窄判据通过不改变"整棵树不可判定"的事实；若未来要把状态升级，必须先给出整棵树的可靠观测方案（当前不存在）。

---

## 9. 边界：本批做了什么、明确没做什么

**做了**：

- POSIX 用 `process_group(0)` + `kill(-pgid, SIGTERM → SIGKILL)` 清理受控集合；
- Windows 用 Job Object + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，v1.20 起 `CREATE_SUSPENDED` 创建、先 `Assign` 再 `ResumeThread`（W2a）；
- 判据只用 `kill(-pgid, 0) == ESRCH` 与 `ActiveProcesses == 0`，作用域写死"受控集合"；
- 超时文案为 v1.20 三态措辞（只有 `Cleaned` 分支写"已清理受控集合；逃逸进程是否仍在运行未验证"，Windows 另写挂起创建/入作业/恢复执行与残余逃逸）；旧回执原样返回、不改写；
- 契约 v1.16 + v1.16.1（文案退回）+ v1.17（文案启用）+ v1.20（Windows 窗口收窄）+ v1.24（单次终止实测确认）+ 六类用例（正向 / 反假通过 / 反向 / 作用域负例 / 恢复前零执行 / 单次终止压力）；

**明确没做**：

- 不实现 W2b/W2c：W2c 已在 §4.5 专项评估并固定结论（不做，非待办）；W2a 已覆盖其唯一想收窄的后代窗口；
- 不引入 cgroup v2 委派、不实现整棵树枚举，也不提供"逃逸存在性"观测点（门槛见 §6.8）；
- 不把 `#3` 改成已证明，不改 `NOT_PROVEN` 状态；
- 不写"已终止进程树 / 无残留"，也不在任何分支把 `StillAlive` / `Unavailable` 升级为"已清理"；
- **不新增第二次终止尝试**（v1.24）：POSIX 68 轮 + Windows 20 轮实测均未出现"一次终止后仍有存活"，能构造出的 `StillAlive` 场景（僵尸、冻结）重试也无效（§6.9）；描述已明写"终止只尝试一次（不重试）"。

---

## 10. 事实来源

- 超时路径：[`server.rs`](../apps/mcp-bridge/src/server.rs) 约 L776-L805；`process_scope` 约 L1083-L1700；`timeout_reason` 约 L1693-L1715；`shell_command` 约 L1066-L1081。
- 超时用例：[`mcp_contract.rs`](../apps/mcp-bridge/tests/mcp_contract.rs) 约 L2108-L2250（文案门禁、正向孙进程清理、旧回执原样返回、PID 观测辅助）；Windows 挂起窗口单元用例 `suspended_start_defers_execution_until_job_assign` 在 [`server.rs`](../apps/mcp-bridge/src/server.rs) 约 L1575-L1618。
- 契约 v1.20 文案约束：`PM-AW-DISPATCH-CONTRACT.md` §4、§5 第 15 行 / G9、§6、§11（v1.20 行）。
- POSIX 复跑脚本：[`scripts/posix-timeout-verify/verify.sh`](../scripts/posix-timeout-verify/verify.sh)（`check` / `prepare` / `run` / `all`）；自检输出见 §6.7。
- 依赖事实：`apps/mcp-bridge/Cargo.toml`（`serde_json`/`sha2`/`hex`/`chrono` + dev `tempfile` + v1.16 的 target-specific `windows-sys` / `libc`）；`Cargo.lock` 中 `windows-sys 0.61.2`、`libc 0.2.186`、`windows-link 0.2.1` 版本未变。
- `windows-sys 0.61.2` 源码核对：`Win32/Foundation/mod.rs`（`HANDLE = *mut c_void`）、`Win32/System/JobObjects/mod.rs`（`CreateJobObjectW` / `AssignProcessToJobObject` / `TerminateJobObject` / `QueryInformationJobObject` / `SetInformationJobObject`、`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 8192`、`JobObjectExtendedLimitInformation = 9`、`JOBOBJECT_BASIC_ACCOUNTING_INFORMATION.ActiveProcesses`）、`Win32/System/Diagnostics/ToolHelp/mod.rs`（`CreateToolhelp32Snapshot` / `TH32CS_SNAPTHREAD` / `THREADENTRY32` / `Thread32First` / `Thread32Next`）、`Win32/System/Threading/mod.rs`（`CREATE_SUSPENDED` / `OpenThread` / `ResumeThread` / `THREAD_SUSPEND_RESUME`）。
- 实测观测：上一轮超时实测记录（由调用方转述）：超时返回后父/子进程仍存活，由测试脚本以 `SIGKILL` 等价手段清理；**单次观测，未固化为自动化用例**。
