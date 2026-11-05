# 超时进程树杀灭（`#3`）设计评估与窄定义实现

- 状态：**v1.16 已按窄定义实现**：只清理并判定"**受控集合**"（POSIX 进程组 / Windows Job Object）；**v1.16.1 起超时回执文案退回 v1.11 旧版**（状态门未满足：POSIX 四类用例未实跑、Windows 逃逸负例 `UNPROVEN`）；**整棵进程树仍不可判定，`#3` 在契约中保持 `NOT_PROVEN`**。
- 实现边界：代码改 `apps/mcp-bridge/src/server.rs`（`process_scope` 模块 + 超时路径 + `timeout_reason`）；用例改 `apps/mcp-bridge/tests/mcp_contract.rs` 与 `server.rs` 内单元测试；契约升到 v1.16；依赖仅把已锁定的 `windows-sys 0.61.2` / `libc 0.2.186` 转为 target-specific 正常依赖，**未新增包、未改版本**。
- 权威契约：[`PM-AW-DISPATCH-CONTRACT.md`](./PM-AW-DISPATCH-CONTRACT.md)（v1.16 起 G9 行指向本文件）。
- Windows 取舍：**未走 W2c**（自举 launcher 成本高、回归面大），选择 W2d——接受 `spawn → Assign` 竞态窗口；该窗口在本文 §4.2 明写。**v1.16.1 的旧版回执不含该披露**（旧文案未提及窗口），新措辞启用后才会写入回执；此处不假装窗口不存在。

---

## 0. 结论先行

1. **"整棵进程树已终止"在当前环境仍不可判定**，`#3` 应继续保持 `NOT_PROVEN`。理由见 §6.4：两个平台都缺少对"逃逸后代"的可靠、可移植、无竞态观测点。
2. **已实现可执行的窄判据**：只证明"**受控集合**（Windows Job Object / POSIX 进程组）内没有存活进程"。它**不等于**整棵树，但能真正区分"进程不存在"与"进程还活着但不写盘"（§3、§6.2）。
3. **回执文案仍为 v1.11 旧版**：受控集合清理已实现，但新措辞"已清理受控集合；逃逸进程是否仍在运行未验证"**尚未启用**；解锁条件是 POSIX 四类用例**实跑**通过 + Windows 三类通过 + 契约版本递增（v1.16.1 记录本次退回）。
4. **明确否掉"超时后两次 `stat` 无变化"**：它测的是"是否写盘"，不是"是否存在"，会把 SIGSTOP、IO 阻塞、活着但本就不写盘的进程判成"已退出"（§2）。实现与用例均未使用该判据。
5. 平台实现：POSIX 用 `process_group(0)` 在 **exec 前**建组、无加入竞态；Windows 用 Job Object + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，但 `std::process::Command` 只能在 `spawn` **之后** `Assign`，**竞态窗口存在且已披露**（§4.2）。
6. 依赖：`windows-sys 0.61.2` 与 `libc 0.2.186` **已在 `Cargo.lock`**，转为 **target-specific 直接依赖后不新增包、不改版本**；许可均为 `MIT OR Apache-2.0`；MSRV 1.71 / 1.65，均低于 workspace 的 1.85（§7）。

---

## 1. 当前事实

### 1.1 实现现状（v1.16，已核实）

超时路径在 [`apps/mcp-bridge/src/server.rs`](../apps/mcp-bridge/src/server.rs) 的 `execute_task` 内（约 L773-L808）：

- `process_scope::Supervisor::new()` → `supervisor.configure(&mut command)` → `spawn` → `supervisor.attach(&child)`；
- 以 20 ms 间隔轮询 `child.try_wait()`；到达 `deadline` 后 `timed_out = true`，调用 `supervisor.terminate(&mut child)` 并记录返回值；
- `timeout_reason(timeout_secs, _outcome)`（约 L1520-L1527）**当前返回 v1.11 旧版文案**，受控集合结果暂不进入回执（v1.16.1 状态门未满足）。

`process_scope` 模块（约 L1075-L1492）：

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
- **当前回执仍为 v1.11 旧版**："已向直接子进程发送终止信号；其派生的子进程是否仍在运行未验证"；
- 新措辞"已清理受控集合；逃逸进程是否仍在运行未验证"**只在状态门满足后启用**（POSIX 四类实跑 + Windows 三类通过 + 契约版本递增）；
- 任何分支都**不得**出现"已终止进程树 / 无残留"；
- 调用方需自行确认受控集合之外的残留进程。

状态约束：**`#3` 仍为 `NOT_PROVEN`**；实现窄定义不改变该状态，状态变更只能由用例结果另行决定。

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
spawn child（沿用现有 shell_command）
AssignProcessToJobObject(job, child_handle)        // ← 竞态窗口在这里
超时:
    TerminateJobObject(job, 1)                     // 终止作业内全部进程
    轮询 QueryInformationJobObject(job, JobObjectBasicAccountingInformation).ActiveProcesses
    直到 0 或到达清理期限
    child.wait()
CloseHandle(job)                                   // 最后句柄关闭时，KILL_ON_JOB_CLOSE 再兜底
```

优点：

- 一次终止**作业内全部成员**，不依赖父子关系；
- 即使 bridge 进程崩溃，操作系统关闭最后一个作业句柄时也会按 `KILL_ON_JOB_CLOSE` 清理成员（**这是设计意图，未实测**）；
- `ActiveProcesses` 是直接的"存在性"观测点（§3/O4），不受"是否写盘"影响。

### 4.2 关键缺陷：`Assign` 在 `spawn` 之后，存在竞态窗口

`std::process::Command::spawn()` 返回时，子进程**已经开始执行**。`cmd /C ping ...` 在这之后可能已经派生了 `ping.exe`。在 `AssignProcessToJobObject` 成功之前派生的后代**不在作业内**，后续 `TerminateJobObject` 与 `ActiveProcesses` 都看不到它们。

可选的闭合路径与代价：

| 路径 | 做法 | 代价 | 残余风险 |
|---|---|---|---|
| W2a | `CREATE_SUSPENDED` + `Assign` + `ResumeThread` | `std::process::Command` 不暴露主线程句柄，**无法用 std 完成** | 若改用原生 API 则等同 W2b |
| W2b | `CreateProcessW` + `STARTUPINFOEX` + `PROC_THREAD_ATTRIBUTE_JOB_LIST`，创建即入作业 | 需替换现有 `shell_command`/spawn 路径，大段 `unsafe` FFI；与 std 的 stdout/stderr 句柄、环境继承都要自己拼 | 大幅增加实现与回归面积 |
| W2c | bridge 建作业（可继承句柄），把句柄值经环境变量传给一个小型 launcher；launcher **先把自己加入作业，再启动 `cmd`** | 新增一个辅助可执行文件；句柄继承与生命周期管理；打包/安装多一个产物 | 后代在 launcher 入作业之后才产生，窗口基本闭合；但显式 `BREAKAWAY` 仍可能逃逸 |
| W2d | 接受竞态，只做"尽量加入" | 实现小 | **不能宣称确定性**；只能报"已清理作业内成员" |

**v1.16 的实际选择：W2d（接受竞态，只做"尽量加入"）。** 未走 W2c 的原因：需要新增辅助可执行文件、句柄继承/生命周期管理与打包产物变更，代价与回归面都远大于收益；W2b 更是替换 spawn 路径的大段 `unsafe` FFI。因此本批继续使用 `std::process::Command`，在 `spawn` 后立即 `AssignProcessToJobObject`：

- 若 `Assign` 失败（`ACCESS_DENIED` / `INVALID_PARAMETER` / 进程已退出），`attached = false`，`terminate` 只 `child.kill()` 并返回 `Unavailable`，**不宣称已清理**；
- `spawn → Assign` 之间派生的后代**不在作业内**，`TerminateJobObject` 与 `ActiveProcesses` 都看不到；
- 该窗口**存在**，已在回执文案（`Windows 下 spawn→Assign 窗口内派生的进程可能未被纳入作业`）与本节明写，**不默认它不存在**。

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
| C2 | `spawn`→`Assign` 窗口内派生的孙进程；`BREAKAWAY`；`Assign` 失败但被忽略 | `Query` 失败被误读；终止过程中的瞬时计数（未轮询到期限）；句柄失效 |

**所以 C1/C2 只能证明作用域内清空，不能证明整棵树。** 这正是它们不能被写成"`#3` 已解决"的原因。

### 6.4 不可判定的全树判据（`#3` 保持 `NOT_PROVEN` 的理由）

要判定"整棵进程树已不存在"，必须在**任意时刻**枚举起从 bridge 派生出去的所有进程，包括已经脱离父进程、脱离进程组、跑到别的会话里的后代。当前环境做不到：

1. **POSIX 没有可移植的后代枚举**：`/proc` 扫描只在 Linux 有，且是 TOCTOU（扫描后进程可再派生/退出），macOS 无对应物；`kill(-pgid)` 只看组。
2. **`setsid` 逃逸不可观测**：一旦后代自建会话，bridge 没有任何 OS 句柄/计数器能追踪它。
3. **Linux `PR_SET_CHILD_SUBREAPER` 只能覆盖"成为孤儿后被 reparent 到 bridge"的进程**，仍不是完整后代枚举；且会改变 bridge 全局的孤儿回收语义，影响其他任务；macOS 无此机制。
4. **cgroup v2 是唯一接近"整棵树"的机制**，但要求 cgroup 委派/容器权限，当前构建与运行环境未验证可用；即便可用，有权限的进程仍可把自己移出 cgroup。
5. **Windows 的逃逸**：`Assign` 在 `spawn` 之后的窗口、`BREAKAWAY`、`Assign` 失败，都会让成员不在作业内；`ActiveProcesses` 只数作业成员。
6. **"某一时刻看不到"不等于"以后不会出现"**：只要还有逃逸的父进程活着，它可以在观测之后继续派生。没有对逃逸父进程的追踪，就无法用"多次采样为 0"推出"整树不存在"。

**结论：`#3` 应保持 `NOT_PROVEN`。** v1.16 实现 C1/C2 后，能诚实宣称的只有："已清理**受控集合**（进程组/作业）；逃逸进程是否仍在运行未验证"——但这句**尚未进入回执**（v1.16.1 文案退回旧版），且它**不等于**整棵树，状态不升级。

### 6.5 判据的可执行性：已落地的用例（v1.16）

| 测试目的 | 用例（位置） | 夹具 | 断言 | Windows 结果 | POSIX 结果 |
|---|---|---|---|---|---|
| 正向：孙进程在受控集合内 | 集成 `timeout_cleans_controlled_set_including_grandchild`；单元 `job_counts_grandchild_and_empties_after_terminate`（Windows） | POSIX `echo $$ > pids.txt; sleep 30 & echo $! >> pids.txt; wait`；Windows PowerShell `Start-Process ping -n 30` 并写 PID；Windows 单元 `cmd /C ping` | 超时文案保持旧版；记录的 PID 均不再存活；Windows 单元打印 `ActiveProcesses_before > 0` → `terminate=Cleaned` → `ActiveProcesses_after == 0`；POSIX 另查 `kill(-pgid,0)` 为 `ESRCH` | **通过**：`ActiveProcesses_before=Some(1)` / `terminate=Cleaned` / `ActiveProcesses_after=Some(0)`；集成 PID `121712`，`process_exists_after=false` | 代码已编译，运行未在本机执行 |
| **反假通过**：活着但不写盘 | 单元 `stopped_process_still_counts_as_alive`（Unix）/ `alive_but_idle_process_is_counted_in_job`（Windows） | POSIX `sleep 30` + `SIGSTOP`；Windows `ping -n 30`（活着但不写盘） | 判据必须判"存在"（`kill(-pgid,0)` 为 0 / `ActiveProcesses > 0`），清空后才转 `Cleaned` | **通过** | 代码已编译，运行未在本机执行 |
| 反向：已退出 | 单元 `exited_process_group_has_no_members`（Unix）/ `exited_process_leaves_job_empty`（Windows） | POSIX `sh -c 'exit 0'`；Windows `cmd /C "ping -n 2 ..."` 自然退出 | `ESRCH` / `ActiveProcesses == 0` → 判"不存在" | **通过** | 代码已编译，运行未在本机执行 |
| **作用域负例**：逃逸 | 单元 `setsid_escape_is_outside_the_observed_group`（`#[cfg(target_os="linux")]`） | `setsid sleep 30 &`，PID 写入临时文件 | 组判据返回 `ESRCH`，但 `kill(已知PID, 0)` 为 0 → **证明 C1 是窄判据**（不断言"#3 解决"） | 不适用（Windows 无等价确定性夹具，记为 `UNPROVEN`） | 代码已编译，运行未在本机执行 |
| 文案门禁 | 集成 `timeout_reason_states_action_not_tree_death` + `legacy_timeout_reason_is_returned_verbatim_and_not_rewritten` | 真实超时任务 + 旧 `handback.json` | **当前必须为旧文案**（"已向直接子进程发送终止信号…未验证"）；**不得**出现"已清理受控集合 / 已终止进程树 / 无残留"；旧回执原样返回、字节不变 | **通过** | 代码已编译，运行未在本机执行 |

**如实说明**：Windows 的 `spawn → Assign` 窗口与 `BREAKAWAY` 无法确定性复现，按 §6.6 记为 `UNPROVEN`，不做"已覆盖"断言。POSIX 用例在本机只做了交叉编译核对（`cargo check --tests --target x86_64-unknown-linux-gnu`），**运行期结果不在本机 Windows 环境产生**。

### 6.6 判据的判定边界（写死）

- **PASS** 只表示"受控集合内无进程"；不得在任何文档/回执里升级为"整棵树已终止"。
- **FAIL** 表示"受控集合内仍有进程"或"观测不可用"；必须如实报告，不得自动重试、不得降级为"已清理"。
- **UNPROVEN** 表示观测点不可用（作业创建/加入失败、查询失败、权限不足）；必须原样暴露。

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
2. 窄定义已由 PM 接受并落地实现；**但超时文案在 v1.16.1 退回旧版**（POSIX 四类未实跑、Windows 逃逸负例 `UNPROVEN`），新措辞待门禁满足后启用；契约先升到 v1.16、再以 v1.16.1 记录退回。
3. **`#3` 保持 `NOT_PROVEN`**：窄判据通过不改变"整棵树不可判定"的事实；若未来要把状态升级，必须先给出整棵树的可靠观测方案（当前不存在）。

---

## 9. 边界：本批做了什么、明确没做什么

**做了**：

- POSIX 用 `process_group(0)` + `kill(-pgid, SIGTERM → SIGKILL)` 清理受控集合；
- Windows 用 Job Object + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，`spawn` 后 `Assign`（W2d，竞态窗口已披露）；
- 判据只用 `kill(-pgid, 0) == ESRCH` 与 `ActiveProcesses == 0`，作用域写死"受控集合"；
- 超时文案**保持 v1.11 旧版**（v1.16.1 只保留实现、退回文案），旧回执原样返回、不改写；
- 契约 v1.16 + v1.16.1 + 四类用例（正向 / 反假通过 / 反向 / 作用域负例）。

**明确没做**：

- 不实现 W2b/W2c，不闭合 `spawn → Assign` 竞态窗口；
- 不引入 cgroup v2 委派、不实现整棵树枚举；
- 不把 `#3` 改成已证明，不改 `NOT_PROVEN` 状态；
- 不写"已终止进程树 / 无残留"，也不在任何分支把 `StillAlive` / `Unavailable` 升级为"已清理"。

---

## 10. 事实来源

- 超时路径：[`server.rs`](../apps/mcp-bridge/src/server.rs) 约 L773-L808；`process_scope` 约 L1075-L1492；`timeout_reason` 约 L1496-L1518；`shell_command` 约 L1060-L1073。
- 超时用例：[`mcp_contract.rs`](../apps/mcp-bridge/tests/mcp_contract.rs) 约 L2108-L2250（文案门禁、正向孙进程清理、旧回执原样返回、PID 观测辅助）。
- 契约 v1.16 文案约束：`PM-AW-DISPATCH-CONTRACT.md` §4（L155-L160）、§5 第 15 行 / G9（L180、L196）、§6（L200-L206）、§11（v1.16 行）。
- 依赖事实：`apps/mcp-bridge/Cargo.toml`（`serde_json`/`sha2`/`hex`/`chrono` + dev `tempfile` + v1.16 的 target-specific `windows-sys` / `libc`）；`Cargo.lock` 中 `windows-sys 0.61.2`、`libc 0.2.186`、`windows-link 0.2.1` 版本未变。
- `windows-sys 0.61.2` 源码核对：`Win32/Foundation/mod.rs`（`HANDLE = *mut c_void`）、`Win32/System/JobObjects/mod.rs`（`CreateJobObjectW` / `AssignProcessToJobObject` / `TerminateJobObject` / `QueryInformationJobObject` / `SetInformationJobObject`、`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 8192`、`JobObjectExtendedLimitInformation = 9`、`JOBOBJECT_BASIC_ACCOUNTING_INFORMATION.ActiveProcesses`）。
- 实测观测：上一轮超时实测记录（由调用方转述）：超时返回后父/子进程仍存活，由测试脚本以 `SIGKILL` 等价手段清理；**单次观测，未固化为自动化用例**。
