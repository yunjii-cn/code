# AI 团队协作引擎设计 — 多 Agent DAG 协作 + 异构模型绑定

> **版本**：v1.0
> **日期**：2026-06-17
> **状态**：设计稿（待 M3 落地）
> **配套**：[TIMEFLOW-DESIGN.md](TIMEFLOW-DESIGN.md) · [AST-NATIVE-DESIGN.md](AST-NATIVE-DESIGN.md) · [VERIFICATION-GUARDRAILS.md](VERIFICATION-GUARDRAILS.md)
> **参考**：CrewAI / AutoGen / MetaGPT / Temporal Workflow / Cursor Planner-Worker

---

## 0. 一句话定位

**AI 团队协作引擎**让多个 AI Agent 像真实研发团队一样协作：项目负责人拆解任务生成 DAG，后端/前端/测试 Agent 在独立 Git 分支并行执行，编译期验证护栏把关质量，TimeFlow 自动合并版本。

**核心差异**：唯一同时具备"多 Agent DAG 真并行 + 异构模型差异化绑定 + Git 分支隔离 + TimeFlow 版本控制 + 编译期验证护栏"的产品。

---

## 1. 为什么需要 AI 团队协作

### 1.1 单 Agent 的天花板

| 限制 | 说明 |
|------|------|
| **上下文窗口有限** | 单 Agent 无法同时持有前端+后端+测试的完整上下文 |
| **模型能力单一** | 一个模型不可能在所有领域都最强（推理/代码/视觉/测试） |
| **串行执行慢** | 单 Agent 串行处理多任务，无法利用并行加速 |
| **无监督闭环** | 单 Agent 自己写自己审，质量无法保证 |

### 1.2 多 Agent 协作的价值

```
单 Agent 模式:
  用户 → Agent 串行处理 → 交付
  （慢、质量不稳定、无监督）

多 Agent 团队模式:
  用户 → 项目负责人拆解任务 → DAG 调度
       → 后端 Agent ──┐
       → 前端 Agent ──┤ 并行执行（独立分支）
       → 测试 Agent ──┘
       → 验证护栏把关 → TimeFlow 合并 → 交付
  （快、质量有保证、有监督闭环）
```

---

## 2. 核心架构

```
┌─────────────────────────────────────────────────────────────┐
│                    用户提交需求                               │
│         "开发一个用户登录功能，支持手机号和邮箱"                │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│         Orchestrator Agent (项目负责人)                       │
│  模型: Qwen3.7 / Claude Opus (强推理)                        │
│  职责: 需求分析 → 任务拆分 → 生成 JSON DAG → 分配 → 验收     │
└──────────────────────┬──────────────────────────────────────┘
                       │ 输出结构化 DAG (非自然语言)
                       ▼
┌─────────────────────────────────────────────────────────────┐
│              DAG 调度引擎 (Temporal Workflow)                │
│  - 解析依赖关系                                              │
│  - 无依赖节点真并行 (多沙箱实例)                              │
│  - 故障自愈 (进程崩溃 ≤5s 从断点恢复)                        │
│  - Git 分支隔离 (每个 Worker 独立分支)                       │
└──────────────────────┬──────────────────────────────────────┘
                       │
          ┌────────────┼────────────┐
          │            │            │
          ▼            ▼            ▼
   ┌────────────┐ ┌────────────┐ ┌────────────┐
   │ 后端 Agent │ │ 前端 Agent │ │ 数据库 Agent│
   │ GLM5.2     │ │ MiniMax3   │ │ Qwen3.7    │
   │ (代码强)   │ │ (视觉强)   │ │ (推理强)   │
   │            │ │            │ │            │
   │ 分支:      │ │ 分支:      │ │ 分支:      │
   │ task/api   │ │ task/ui    │ │ task/db    │
   └─────┬──────┘ └─────┬──────┘ └─────┬──────┘
         │              │              │
         └──────────────┼──────────────┘
                        │
                        ▼
         ┌──────────────────────────────┐
         │   编译期验证护栏 (VERIFICATION) │
         │   - 类型系统静态分析            │
         │   - Lint 规则检查               │
         │   - 破坏类型安全 → 拒绝合并     │
         │   - 失败 → 喂回 Coder 重试     │
         └──────────────┬───────────────┘
                        │ 通过
                        ▼
         ┌──────────────────────────────┐
         │   TimeFlow 版本控制引擎       │
         │   - git merge-tree 冲突预检   │
         │   - 冲突 → Merge Resolver     │
         │   - 无冲突 → 合并到 main      │
         │   - 自动快照 + AI commit msg  │
         └──────────────┬───────────────┘
                        │
                        ▼
         ┌──────────────────────────────┐
         │   测试验收 Agent (Tester)     │
         │   模型: GLM5.2 / 本地 Llama   │
         │   - 运行测试套件               │
         │   - 通过 → 标记候选版本        │
         │   - 失败 → 回退给对应 Agent    │
         └──────────────────────────────┘
```

---

## 3. 五大核心组件

### 3.1 团队模板 (Team Template)

```yaml
# .yunji/team.yml
team:
  name: "全栈开发团队"
  description: "包含 PM、后端、前端、测试的完整开发团队"

  roles:
    - id: orchestrator
      name: "项目负责人"
      description: "需求分析、任务拆分、DAG 生成、进度监督、质量验收"
      model: "qwen3.7"  # 强推理模型
      model_fallback: ["claude-opus", "glm5.2"]
      skills:
        - requirement-analysis
        - task-decomposition
        - dag-generation
        - progress-monitoring
        - quality-review
      knowledge:
        - project-standards
        - architecture-guidelines
      permissions:
        - can_create_subtasks
        - can_merge_pr
        - can_reject_work

    - id: backend
      name: "后端工程师"
      description: "API 设计、业务逻辑、数据库交互"
      model: "glm5.2"  # 代码能力强
      model_fallback: ["qwen-coder"]
      skills:
        - api-design
        - business-logic
        - database-query
      knowledge:
        - backend-best-practices
        - security-guidelines
      permissions:
        - can_edit_backend_files
        - can_run_tests

    - id: frontend
      name: "前端工程师"
      description: "UI 设计、交互实现、样式开发"
      model: "minimax3"  # 视觉能力强
      model_fallback: ["gpt-4o"]
      skills:
        - ui-design
        - component-development
        - css-styling
      knowledge:
        - design-system
        - accessibility-guidelines
      permissions:
        - can_edit_frontend_files
        - can_run_tests

    - id: tester
      name: "测试工程师"
      description: "测试用例编写、自动化测试、BUG 修复"
      model: "glm5.2"  # 代码能力强，成本低
      model_fallback: ["local-llama-3"]
      skills:
        - test-case-design
        - automated-testing
        - bug-fixing
      knowledge:
        - testing-standards
      permissions:
        - can_edit_test_files
        - can_run_tests
        - can_reject_build
```

### 3.2 任务依赖图 (Task DAG)

Orchestrator 输出**结构化 JSON DAG**（非自然语言），调度引擎直接解析：

```json
{
  "workflow_id": "wf_20260617_login",
  "requirement": "开发用户登录功能，支持手机号和邮箱",
  "tasks": [
    {
      "id": "task-1",
      "title": "设计数据库表结构",
      "assignee": "orchestrator",
      "dependencies": [],
      "estimated_effort": "30min",
      "acceptance_criteria": "用户表包含 id/phone/email/password_hash 字段"
    },
    {
      "id": "task-2",
      "title": "实现用户登录 API",
      "assignee": "backend",
      "dependencies": ["task-1"],
      "estimated_effort": "2h",
      "acceptance_criteria": "POST /api/login 接口，返回 JWT token"
    },
    {
      "id": "task-3",
      "title": "设计登录页面 UI",
      "assignee": "frontend",
      "dependencies": [],
      "estimated_effort": "1h",
      "acceptance_criteria": "响应式登录表单，支持手机号/邮箱切换"
    },
    {
      "id": "task-4",
      "title": "实现登录页面交互",
      "assignee": "frontend",
      "dependencies": ["task-2", "task-3"],
      "estimated_effort": "2h",
      "acceptance_criteria": "表单提交调用 /api/login，处理成功/失败状态"
    },
    {
      "id": "task-5",
      "title": "编写测试用例",
      "assignee": "tester",
      "dependencies": ["task-2", "task-4"],
      "estimated_effort": "1h",
      "acceptance_criteria": "单元测试 + E2E 测试覆盖率 ≥ 80%"
    },
    {
      "id": "task-6",
      "title": "集成测试 + 验收",
      "assignee": "orchestrator",
      "dependencies": ["task-5"],
      "estimated_effort": "30min",
      "acceptance_criteria": "全流程跑通，无严重 bug"
    }
  ]
}
```

**DAG 可视化**：
```
task-1 (数据库) ──┐
                  ├──→ task-2 (API) ──┐
                  │                    ├──→ task-4 (交互) ──┐
task-3 (UI) ──────┘                    │                    ├──→ task-5 (测试) ──→ task-6 (验收)
                                       │                    │
                                       └────────────────────┘
```

### 3.3 DAG 调度引擎 (Temporal Workflow)

```rust
// platformkit/crates/agent-team/src/scheduler.rs

use std::collections::HashMap;
use tokio::sync::mpsc;

/// DAG 调度引擎
pub struct DagScheduler {
    /// 任务图
    dag: TaskDag,
    /// Agent 工作池
    agents: HashMap<RoleId, AgentWorker>,
    /// TimeFlow 引擎（分支管理）
    timeflow: TimeFlow,
    /// 验证护栏
    guardrails: VerificationGuardrails,
    /// 任务状态追踪
    task_states: HashMap<TaskId, TaskState>,
}

/// 任务状态
#[derive(Debug, Clone)]
pub enum TaskState {
    Pending,
    Running { agent: RoleId, branch: String, started_at: DateTime<Utc> },
    Verifying { branch: String },
    Merged { snapshot_id: String },
    Failed { error: String, retry_count: u32 },
    Rejected { reason: String },
}

impl DagScheduler {
    /// 启动调度循环
    pub async fn run(&mut self, mut event_rx: mpsc::Receiver<TaskEvent>) -> Result<()> {
        loop {
            // 1. 找出所有依赖已满足的待执行任务
            let ready_tasks = self.dag.get_ready_tasks();

            // 2. 并行分配给对应 Agent（真并行，多沙箱）
            for task in ready_tasks {
                self.assign_task(task).await?;
            }

            // 3. 等待任务事件
            match event_rx.recv().await {
                Some(TaskEvent::Completed { task_id, result }) => {
                    self.handle_completion(task_id, result).await?;
                }
                Some(TaskEvent::Failed { task_id, error }) => {
                    self.handle_failure(task_id, error).await?;
                }
                None => break, // 所有任务完成
            }
        }
        Ok(())
    }

    /// 分配任务给 Agent
    async fn assign_task(&mut self, task: &Task) -> Result<()> {
        let agent = self.agments.get(&task.assignee)
            .ok_or_else(|| Error::AgentNotFound(task.assignee.clone()))?;

        // 创建独立 Git 分支（分支隔离）
        let branch = format!("task/{}", task.id);
        let base_snapshot = self.timeflow.current_snapshot("main")?;
        self.timeflow.create_branch(&branch, &base_snapshot)?;

        // 更新状态
        self.task_states.insert(task.id.clone(), TaskState::Running {
            agent: task.assignee.clone(),
            branch: branch.clone(),
            started_at: Utc::now(),
        });

        // 异步执行（不阻塞调度循环）
        agent.execute(task.clone(), branch.clone()).await;

        Ok(())
    }

    /// 处理任务完成
    async fn handle_completion(&mut self, task_id: &str, result: TaskResult) -> Result<()> {
        let state = self.task_states.get(task_id).unwrap();
        let branch = match state {
            TaskState::Running { branch, .. } => branch,
            _ => return Err(Error::InvalidState),
        };

        // 1. 编译期验证护栏检查
        let verification = self.guardrails.verify(branch).await?;
        if !verification.passed {
            // 验证失败，喂回 Agent 重试（最多 3 次）
            if result.retry_count < 3 {
                self.task_states.insert(task_id.into(), TaskState::Failed {
                    error: verification.errors.join("; "),
                    retry_count: result.retry_count + 1,
                });
                self.agents.get(&result.agent)
                    .unwrap()
                    .retry_with_errors(task_id, verification.errors).await;
                return Ok(());
            } else {
                // 重试耗尽，拒绝合并
                self.task_states.insert(task_id.into(), TaskState::Rejected {
                    reason: "验证失败超过 3 次".to_string(),
                });
                return Ok(());
            }
        }

        // 2. 冲突预检
        let conflict_check = self.timeflow.check_conflict(branch, "main")?;
        if conflict_check.has_conflict {
            // 冲突 → 触发 Merge Resolver Agent
            self.resolve_conflict(branch, conflict_check).await?;
        }

        // 3. 合并到 main
        let snapshot_id = self.timeflow.merge_branch(branch, "main")?;
        self.task_states.insert(task_id.into(), TaskState::Merged { snapshot_id });

        // 4. 更新 DAG（标记完成，触发下游任务）
        self.dag.mark_completed(task_id);

        Ok(())
    }
}
```

### 3.4 共享上下文池 (Shared Context Pool)

**关键设计**：以 Git 仓库作为上下文总线，而非另建数据库。

```rust
// platformkit/crates/agent-team/src/context.rs

/// 共享上下文池
/// 所有 Agent 共享项目知识，存储在 .yunji/context/ 目录（Git 管理）
pub struct SharedContext {
    /// 仓库路径
    repo_path: PathBuf,
    /// AST 代码认知引擎（见 AST-NATIVE-DESIGN.md）
    ast_engine: AstEngine,
    /// 项目文档
    documentation: Documentation,
    /// 任务历史（已完成任务的上下文）
    task_history: Vec<TaskContext>,
}

impl SharedContext {
    /// Agent 读取上下文（按角色过滤）
    ///
    /// 例如：前端 Agent 不需要看到数据库细节
    pub fn read_for_role(&self, role: &RoleId, task: &Task) -> AgentContext {
        let mut context = AgentContext::default();

        // 1. 任务相关代码（AST 智能提取，降低 Token 70%）
        let relevant_code = self.ast_engine.extract_relevant_code(
            &task.title,
            &task.acceptance_criteria,
        );
        context.code_snippets = relevant_code;

        // 2. 角色相关文档
        context.docs = self.documentation.for_role(role);

        // 3. 上游任务产出（依赖任务的 commit）
        context.upstream_artifacts = self.get_upstream_artifacts(&task.dependencies);

        // 4. 项目规范
        context.standards = self.get_standards_for_role(role);

        context
    }

    /// Agent 写入上下文（完成任务后）
    pub fn write(&mut self, agent_id: &str, updates: ContextUpdates) -> Result<()> {
        // 更新 .yunji/context/ 目录（Git 管理，可追溯）
        // 触发 AST 增量索引更新
        self.ast_engine.incremental_update(&updates.changed_files)?;
        Ok(())
    }
}
```

### 3.5 冲突解决 (Merge Resolver)

```rust
// platformkit/crates/agent-team/src/merge_resolver.rs

/// 冲突解决 Agent
/// 当 git merge-tree 检测到冲突时触发
pub struct MergeResolver {
    /// 使用强推理模型解决冲突
    model: Box<dyn LlmClient>,
}

impl MergeResolver {
    pub async fn resolve(
        &self,
        branch: &str,
        conflict: ConflictReport,
    ) -> Result<Resolution> {
        let mut resolutions = Vec::new();

        for file_conflict in conflict.files {
            // 1. 读取冲突标记的文件
            let conflicted_content = fs::read_to_string(&file_conflict.path)?;

            // 2. 调用 LLM 分析冲突
            let prompt = format!(
                "分析以下 Git 冲突，给出合并方案（保留双方意图）:\n\
                 文件: {}\n\
                 冲突内容:\n{}",
                file_conflict.path.display(),
                conflicted_content
            );

            let resolution = self.model.complete(&prompt).await?;

            // 3. 验证合并结果可编译（调用验证护栏）
            // 4. 记录解决方案
            resolutions.push(FileResolution {
                path: file_conflict.path,
                resolved_content: resolution,
            });
        }

        Ok(Resolution { files: resolutions })
    }
}
```

---

## 4. 异构模型绑定策略

### 4.1 模型能力矩阵

| 模型 | 推理 | 代码 | 视觉 | 长文本 | 成本 | 适合角色 |
|------|:---:|:---:|:---:|:---:|:---:|------|
| Qwen3.7 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ❌ | ⭐⭐⭐⭐⭐ | 中 | Orchestrator / 数据库 |
| Claude Opus | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⚠️ | ⭐⭐⭐⭐⭐ | 高 | Orchestrator (备选) |
| GLM5.2 | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ❌ | ⭐⭐⭐⭐⭐ | 中 | 后端 / 测试 |
| MiniMax3 | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | 中 | 前端 (视觉) |
| GPT-4o | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | 高 | 前端 (备选) / Reviewer |
| 本地 Llama-3 | ⭐⭐⭐ | ⭐⭐⭐ | ❌ | ⭐⭐ | 低 | 测试 (降本) |

### 4.2 模型路由策略

```rust
// platformkit/crates/agent-team/src/model_router.rs

pub struct ModelRouter {
    /// 角色 → 主模型 + 备用模型
    routing: HashMap<RoleId, ModelConfig>,
    /// 模型客户端池
    clients: HashMap<ModelId, Box<dyn LlmClient>>,
    /// 健康检查（自动故障转移）
    health: ModelHealthMonitor,
}

impl ModelRouter {
    /// 路由到最合适的模型
    pub async fn route(&self, role: &RoleId, task: &Task) -> Result<&dyn LlmClient> {
        let config = self.routing.get(role)?;

        // 1. 尝试主模型
        if self.health.is_healthy(&config.primary) {
            return Ok(self.clients.get(&config.primary).unwrap().as_ref());
        }

        // 2. 故障转移到备用模型
        for fallback in &config.fallbacks {
            if self.health.is_healthy(fallback) {
                tracing::warn!("模型故障转移: {} → {}", config.primary, fallback);
                return Ok(self.clients.get(fallback).unwrap().as_ref());
            }
        }

        Err(Error::NoHealthyModel)
    }
}
```

### 4.3 成本控制

| 策略 | 说明 |
|------|------|
| **任务分级** | 简单任务用便宜模型，复杂任务用强模型 |
| **本地模型优先** | 测试、Lint 等任务用本地 Llama-3，零成本 |
| **缓存复用** | 相同 prompt+context 直接返回缓存结果 |
| **Token 监控** | 实时监控每个 Agent 的 Token 消耗，超限告警 |
| **断点续传** | Temporal 持久化，崩溃不浪费已消耗的 Token |

---

## 5. 与 TimeFlow 的深度整合

TimeFlow 是多 Agent 协作的基础设施：

| TimeFlow 功能 | 多 Agent 协作用途 |
|--------------|------------------|
| **分支管理** | 每个 Agent 在独立分支工作，互不干扰 |
| **自动快照** | Agent 每步操作自动快照，可回溯任意时刻 |
| **AI commit msg** | 自动生成"backend-agent: 实现用户登录 API" |
| **冲突检测** | `git merge-tree` 预检，冲突触发 Merge Resolver |
| **回滚能力** | 任务失败一键回滚到任意快照 |
| **版本整理** | AI 自动识别"可合并的候选版本" |

---

## 6. 与 AST-Native 引擎的整合

AST 引擎为多 Agent 协作提供"智能上下文"：

| AST 引擎功能 | 多 Agent 协作用途 |
|-------------|------------------|
| **调用链提取** | 后端改 API → 自动识别受影响的前端文件 |
| **接口契约监听** | .proto/Swagger 变更 → 自动触发跨语言任务 |
| **Token 优化** | 只提取任务相关代码片段，降低 Token 70% |
| **依赖分析** | 任务拆解时自动识别隐含依赖 |

**示例场景**：
```
后端 Agent 修改了 UserService.login() 方法签名
  → AST 引擎检测到调用链变化
  → 自动通知前端 Agent: "login() 签名变更，需更新调用处"
  → 前端 Agent 在 task/ui 分支同步修改
  → 验证护栏检查类型一致性
  → 合并通过
```

---

## 7. 与验证护栏的整合

验证护栏是"零幻觉交付"的保证：

```rust
// DAG merge 节点前插入验证层
async fn handle_completion(&mut self, task_id: &str, result: TaskResult) -> Result<()> {
    // ... Agent 完成任务 ...

    // ⭐ 验证护栏检查（非 LLM，确定性）
    let verification = self.guardrails.verify(branch).await?;
    if !verification.passed {
        // 失败 → 编译器错误喂回 Coder Agent 重试
        self.agents.get(&result.agent)
            .unwrap()
            .retry_with_errors(task_id, verification.errors).await;
        return Ok(());
    }

    // 通过 → 合并
    self.timeflow.merge_branch(branch, "main")?;
    Ok(())
}
```

---

## 8. 实现路径

| 阶段 | 目标 | 周期 | 依赖 |
|------|------|:---:|------|
| **M3.1** | 团队模板 + 角色定义 + 模型路由 | 2 周 | 无 |
| **M3.2** | DAG 调度引擎（简化版内存 DAG） | 2 周 | M3.1 |
| **M4.1** | 双 Agent 并行 MVP（架构师 + Coder） | 2 周 | M3.2 + TimeFlow |
| **M4.2** | Git 分支隔离 + 冲突预检 | 1 周 | M4.1 |
| **M5.1** | 验证护栏集成（编译期静态分析） | 2 周 | M4.2 + 验证护栏 |
| **M5.2** | Merge Resolver Agent | 1 周 | M5.1 |
| **M5.3** | 共享上下文池 + AST 整合 | 2 周 | M5.2 + AST 引擎 |
| **M6.1** | 多角色全流程（PM+后端+前端+测试） | 3 周 | M5.3 |
| **M6.2** | Temporal 替换内存 DAG（生产级） | 2 周 | M6.1 |

**总计：约 17 周**，融入路线图 M3-M6。

---

## 9. MVP 验证场景

**M4.1 双 Agent MVP 验证**：
```
用户: "添加一个健康检查接口 /api/health"

Orchestrator (Qwen3.7):
  - 拆分任务: [后端实现 /api/health, 测试用例]
  - 生成 DAG: task-1(后端) → task-2(测试)

后端 Agent (GLM5.2):
  - 在 task/api 分支工作
  - 实现 GET /api/health 返回 {"status":"ok"}
  - 提交: "feat(api): 添加健康检查接口"

验证护栏:
  - 类型检查通过 ✅
  - Lint 通过 ✅

TimeFlow:
  - 合并 task/api → main
  - 自动快照 + AI commit msg

测试 Agent (GLM5.2):
  - 在 task/test 分支工作
  - 编写测试: test_health_check_returns_ok
  - 运行测试: 通过 ✅

TimeFlow:
  - 合并 task/test → main

Orchestrator:
  - 验收通过
  - 标记为候选版本
```

---

## 10. 风险与对策

| 风险 | 等级 | 对策 |
|------|:---:|------|
| **Temporal 学习曲线陡峭** | 🟡 中 | 先用内存 DAG 验证，M6 再替换 Temporal |
| **多 Agent 上下文冲突** | 🟡 中 | 共享上下文池 + AST 智能提取 |
| **Git 状态不一致** | 🔴 高 | 用 libgit2 而非 CLI + 每次任务前 fetch+reset |
| **模型故障** | 🟡 中 | 模型路由 + 自动故障转移 + 备用模型 |
| **成本失控** | 🟡 中 | 本地模型优先 + Token 监控 + 缓存复用 |
| **冲突解决失败** | 🟡 中 | 重试 3 次后人工介入 |
| **沙箱逃逸** | 🔴 高 | 仓库级沙箱 + AppContainer/Docker 硬隔离 |
| **范围蔓延** | 🔴 高 | MVP 严格聚焦双 Agent 场景，不追求全角色 |

---

## 11. 竞品对比

| 产品 | 多 Agent | 异构模型 | 真并行 | Git-Native | 桌面原生 | 验证护栏 | 全链路发布 |
|------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **CrewAI** | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| **AutoGen** | ✅ | ✅ | ⚠️ | ❌ | ❌ | ❌ | ❌ |
| **MetaGPT** | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Cursor** | ⚠️ | ❌ | ✅ | ❌ | ✅ | ❌ | ❌ |
| **Devin** | ⚠️ | ❌ | ✅ | ⚠️ | ❌ | ❌ | ❌ |
| **OpenHands** | ⚠️ | ⚠️ | ✅ | ⚠️ | ❌ | ❌ | ❌ |
| **我们(AW)** | ✅ | ✅ | ✅DAG | ✅TimeFlow | ✅Tauri | ✅ | ✅ |

**唯一组合**：多 Agent DAG 真并行 + 异构模型差异化绑定 + Git 分支隔离 + TimeFlow 版本控制 + 编译期验证护栏 + 全链路发布。

---

## 12. 变更历史

| 日期 | 变更 | 作者 |
|------|------|------|
| 2026-06-17 | 初始设计文档 v1.0 | Trae |

---

> **AI 团队协作引擎是 AgentWork 的王炸**：让 AI 从"单兵作战"升级到"团队协作"，从"概率性生成"升级到"确定性交付"。
