# 编译期验证护栏设计 — 零幻觉交付的确定性保证

> **版本**：v1.0
> **日期**：2026-06-17
> **状态**：设计稿（待 M5 落地）
> **配套**：[AGENT-TEAM-DESIGN.md](AGENT-TEAM-DESIGN.md) · [AST-NATIVE-DESIGN.md](AST-NATIVE-DESIGN.md)

---

## 0. 一句话定位

**编译期验证护栏**在 DAG merge 节点前插入非 LLM 的确定性验证层，用类型系统/编译器/Lint 规则把关代码质量，破坏类型安全的代码直接拒绝合并，实现"零幻觉交付"。

**核心价值**：从"概率性 AI 生成"升级到"确定性工程交付"。

---

## 1. 为什么需要验证护栏

### 1.1 AI 代码的三大风险

| 风险 | 说明 | 后果 |
|------|------|------|
| **类型不匹配** | AI 修改了函数签名，但漏改调用处 | 编译失败 |
| **接口契约破坏** | 后端改 API，前端调用不匹配 | 运行时崩溃 |
| **隐式依赖断裂** | AI 删除了"看似无用"的函数，实则有调用 | 功能丢失 |

### 1.2 验证护栏的价值

```
无护栏 (Cursor/Devin):
  AI 生成代码 → 直接合并 → 用户发现编译错误 → 手动修复
  （质量不稳定，需人工兜底）

有护栏 (我们):
  AI 生成代码 → 验证护栏检查 → 通过才合并
                ↓ 失败
                编译器错误喂回 AI → AI 自动修复 → 重试
  （质量有保证，零幻觉交付）
```

---

## 2. 核心架构

```
┌──────────────────────────────────────────────────────────┐
│              编译期验证护栏引擎                            │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  ┌──────────────────────────────────────────────────┐   │
│  │           验证管道 (Verification Pipeline)         │   │
│  │                                                  │   │
│  │  Stage 1: Lint 规则检查 (快, <1s)                │   │
│  │     ↓                                            │   │
│  │  Stage 2: 类型系统静态分析 (中, <5s)             │   │
│  │     ↓                                            │   │
│  │  Stage 3: 编译器编译检查 (慢, <30s)              │   │
│  │     ↓                                            │   │
│  │  Stage 4: 接口契约匹配 (基于 AST)                │   │
│  │     ↓                                            │   │
│  │  Stage 5: 调用链完整性检查 (基于 AST)            │   │
│  │                                                  │   │
│  └──────────────────────────────────────────────────┘   │
│                                                          │
│  ┌────────────────┐    ┌────────────────┐               │
│  │  错误反馈器     │    │  白名单机制    │               │
│  │  编译器错误 →   │    │  误杀豁免      │               │
│  │  喂回 AI Agent  │    │                │               │
│  └────────────────┘    └────────────────┘               │
│                                                          │
└──────────────────────────────────────────────────────────┘
        │
        ▼
┌────────────────┐     ┌────────────────┐
│  通过 → 合并   │     │  失败 → 重试   │
│  TimeFlow merge│     │  AI 自动修复   │
└────────────────┘     └────────────────┘
```

---

## 3. 五级验证管道

### 3.1 Stage 1: Lint 规则检查（快速）

```rust
// platformkit/crates/verification-guardrails/src/lint.rs

/// Lint 规则检查
/// 最快，<1s 完成
pub struct LintChecker {
    rules: Vec<LintRule>,
}

impl LintChecker {
    pub async fn check(&self, branch: &str) -> Result<LintResult> {
        let changed_files = self.get_changed_files(branch).await?;
        let mut violations = Vec::new();

        for file in changed_files {
            let content = fs::read_to_string(&file.path).await?;
            let lang = detect_language(&file.path)?;

            for rule in &self.rules {
                if rule.applies_to(lang) {
                    if let Some(v) = rule.check(&content)? {
                        violations.push(v);
                    }
                }
            }
        }

        Ok(LintResult {
            passed: violations.is_empty(),
            violations,
        })
    }
}
```

**Lint 规则示例**：
- 禁止 `unwrap()` / `expect()`（Rust）
- 禁止 `any` / `as unknown as`（TypeScript）
- 强制错误处理（无裸 `throw`）
- 禁止 `console.log`（生产代码）

### 3.2 Stage 2: 类型系统静态分析

```rust
// platformkit/crates/verification-guardrails/src/type_check.rs

/// 类型系统静态分析
/// 使用语言原生的类型检查器
pub struct TypeChecker {
    checkers: HashMap<Language, Box<dyn TypeCheckEngine>>,
}

#[async_trait]
pub trait TypeCheckEngine: Send + Sync {
    async fn check(&self, project_path: &Path) -> Result<TypeCheckResult>;
}

/// Rust 类型检查 (cargo check)
pub struct RustTypeChecker;
#[async_trait]
impl TypeCheckEngine for RustTypeChecker {
    async fn check(&self, project_path: &Path) -> Result<TypeCheckResult> {
        let output = Command::new("cargo")
            .args(&["check", "--message-format=json"])
            .current_dir(project_path)
            .output().await?;

        let errors = parse_cargo_messages(&output.stdout)?;
        Ok(TypeCheckResult {
            passed: errors.is_empty(),
            errors,
        })
    }
}

/// TypeScript 类型检查 (tsc --noEmit)
pub struct TypeScriptTypeChecker;
#[async_trait]
impl TypeCheckEngine for TypeScriptTypeChecker {
    async fn check(&self, project_path: &Path) -> Result<TypeCheckResult> {
        let output = Command::new("npx")
            .args(&["tsc", "--noEmit", "--pretty", "false"])
            .current_dir(project_path)
            .output().await?;

        let errors = parse_tsc_output(&output.stdout)?;
        Ok(TypeCheckResult { passed: errors.is_empty(), errors })
    }
}
```

### 3.3 Stage 3: 编译器编译检查

```rust
// platformkit/crates/verification-guardrails/src/compiler.rs

/// 编译器编译检查
/// 最严格，确保代码可编译
pub struct CompilerChecker {
    compilers: HashMap<Language, Box<dyn Compiler>>,
}

#[async_trait]
pub trait Compiler: Send + Sync {
    async fn compile(&self, project_path: &Path) -> Result<CompileResult>;
}

/// Rust 编译器
pub struct RustCompiler;
#[async_trait]
impl Compiler for RustCompiler {
    async fn compile(&self, project_path: &Path) -> Result<CompileResult> {
        let output = Command::new("cargo")
            .args(&["build", "--message-format=json"])
            .current_dir(project_path)
            .output().await?;

        let errors = parse_cargo_messages(&output.stdout)?;
        Ok(CompileResult {
            passed: errors.is_empty(),
            errors,
            warnings: parse_cargo_warnings(&output.stdout)?,
        })
    }
}
```

### 3.4 Stage 4: 接口契约匹配

```rust
// platformkit/crates/verification-guardrails/src/contract.rs

/// 接口契约匹配
/// 基于 AST 引擎，检查前后端接口是否一致
pub struct ContractChecker {
    ast_engine: Arc<AstEngine>,
}

impl ContractChecker {
    pub async fn check(&self, branch: &str) -> Result<ContractResult> {
        // 1. 提取后端 API 定义
        let backend_apis = self.ast_engine.extract_api_definitions().await?;

        // 2. 提取前端 API 调用
        let frontend_calls = self.ast_engine.extract_api_calls().await?;

        // 3. 匹配检查
        let mut mismatches = Vec::new();

        for call in &frontend_calls {
            if let Some(api) = backend_apis.iter().find(|a| a.path == call.path) {
                // 检查方法匹配
                if api.method != call.method {
                    mismatches.push(ContractMismatch {
                        kind: MismatchKind::MethodMismatch,
                        api_path: api.path.clone(),
                        expected: api.method.clone(),
                        actual: call.method.clone(),
                    });
                }
                // 检查参数类型匹配
                for (param, expected_type) in &api.params {
                    if let Some(actual_type) = call.params.get(param) {
                        if expected_type != actual_type {
                            mismatches.push(ContractMismatch {
                                kind: MismatchKind::ParamTypeMismatch,
                                api_path: api.path.clone(),
                                expected: format!("{}: {}", param, expected_type),
                                actual: format!("{}: {}", param, actual_type),
                            });
                        }
                    }
                }
            } else {
                // 前端调用了不存在的 API
                mismatches.push(ContractMismatch {
                    kind: MismatchKind::ApiNotFound,
                    api_path: call.path.clone(),
                    expected: "API should exist".to_string(),
                    actual: "API not found in backend".to_string(),
                });
            }
        }

        Ok(ContractResult {
            passed: mismatches.is_empty(),
            mismatches,
        })
    }
}
```

### 3.5 Stage 5: 调用链完整性检查

```rust
// platformkit/crates/verification-guardrails/src/call_chain.rs

/// 调用链完整性检查
/// 基于 AST 引擎，检查是否有断链
pub struct CallChainChecker {
    ast_engine: Arc<AstEngine>,
}

impl CallChainChecker {
    pub async fn check(&self, branch: &str) -> Result<CallChainResult> {
        let changed_files = self.get_changed_files(branch).await?;

        let mut broken_chains = Vec::new();

        for file in &changed_files {
            // 获取被删除的符号
            let removed_symbols = self.ast_engine.get_removed_symbols(file).await?;

            for symbol in &removed_symbols {
                // 查询谁还在调用这个被删除的符号
                let callers = self.ast_engine.get_callers(&symbol.id).await?;

                if !callers.is_empty() {
                    broken_chains.push(BrokenChain {
                        removed_symbol: symbol.name.clone(),
                        removed_from: file.path.clone(),
                        still_called_by: callers.iter().map(|c| c.name.clone()).collect(),
                    });
                }
            }
        }

        Ok(CallChainResult {
            passed: broken_chains.is_empty(),
            broken_chains,
        })
    }
}
```

---

## 4. 错误反馈器

```rust
// platformkit/crates/verification-guardrails/src/feedback.rs

/// 错误反馈器
/// 将验证错误喂回 AI Agent，触发自动修复
pub struct ErrorFeedback {
    /// 最大重试次数
    max_retries: u32,
}

impl ErrorFeedback {
    /// 生成反馈 prompt
    pub fn generate_feedback_prompt(
        &self,
        original_task: &Task,
        errors: &[VerificationError],
        retry_count: u32,
    ) -> String {
        format!(
            "你之前的代码未通过验证，请修复以下错误后重新提交:\n\n\
             原始任务: {}\n\n\
             验证错误 (第 {} 次重试，最多 {} 次):\n{}\n\n\
             要求:\n\
             1. 修复所有验证错误\n\
             2. 不要引入新的错误\n\
             3. 保持原有功能不变\n\
             4. 只修改必要部分，不要重构无关代码",
            original_task.title,
            retry_count,
            self.max_retries,
            errors.iter().enumerate()
                .map(|(i, e)| format!("{}. [{}] {}", i + 1, e.severity, e.message))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}
```

---

## 5. 白名单机制

```rust
// platformkit/crates/verification-guardrails/src/whitelist.rs

/// 白名单机制
/// 避免误杀合法代码
pub struct Whitelist {
    /// 豁免的文件路径模式
    file_patterns: Vec<Regex>,
    /// 豁免的规则
    rule_ids: HashSet<String>,
    /// 豁免的符号
    symbols: HashSet<String>,
}

impl Whitelist {
    pub fn is_exempted(&self, file: &Path, rule_id: &str) -> bool {
        // 检查文件模式
        if self.file_patterns.iter().any(|p| p.is_match(&file.to_string_lossy())) {
            return true;
        }
        // 检查规则 ID
        if self.rule_ids.contains(rule_id) {
            return true;
        }
        false
    }
}
```

**白名单配置示例**：
```yaml
# .yunji/guardrails-whitelist.yml
files:
  - "**/test/**"          # 测试文件豁免部分规则
  - "**/migrations/**"    # 数据库迁移文件豁免

rules:
  - "no-unwrap"           # 测试代码允许 unwrap
  - "no-console-log"      # 开发环境允许 console.log

symbols:
  - "main"                # main 函数豁免部分检查
```

---

## 6. 验证流程

```rust
// platformkit/crates/verification-guardrails/src/pipeline.rs

/// 验证管道
pub struct VerificationPipeline {
    lint: LintChecker,
    type_check: TypeChecker,
    compiler: CompilerChecker,
    contract: ContractChecker,
    call_chain: CallChainChecker,
    whitelist: Whitelist,
    feedback: ErrorFeedback,
}

impl VerificationPipeline {
    /// 执行完整验证
    pub async fn verify(&self, branch: &str) -> Result<VerificationResult> {
        // Stage 1: Lint
        let lint_result = self.lint.check(branch).await?;
        if !lint_result.passed {
            return Ok(VerificationResult::failed(lint_result.violations));
        }

        // Stage 2: 类型检查
        let type_result = self.type_check.check(branch).await?;
        if !type_result.passed {
            return Ok(VerificationResult::failed(type_result.errors));
        }

        // Stage 3: 编译检查
        let compile_result = self.compiler.compile(branch).await?;
        if !compile_result.passed {
            return Ok(VerificationResult::failed(compile_result.errors));
        }

        // Stage 4: 接口契约
        let contract_result = self.contract.check(branch).await?;
        if !contract_result.passed {
            return Ok(VerificationResult::failed(contract_result.mismatches));
        }

        // Stage 5: 调用链
        let chain_result = self.call_chain.check(branch).await?;
        if !chain_result.passed {
            return Ok(VerificationResult::failed(chain_result.broken_chains));
        }

        Ok(VerificationResult::passed())
    }
}
```

---

## 7. 与 AI 团队协作的整合

```rust
// 在 DAG 调度引擎的 handle_completion 中调用验证护栏
async fn handle_completion(&mut self, task_id: &str, result: TaskResult) -> Result<()> {
    let branch = self.get_task_branch(task_id)?;

    // ⭐ 验证护栏检查
    let verification = self.guardrails.verify(branch).await?;

    if !verification.passed {
        // 失败 → 喂回 AI 重试
        if result.retry_count < 3 {
            let prompt = self.feedback.generate_feedback_prompt(
                &result.task,
                &verification.errors,
                result.retry_count + 1,
            );
            self.agents.get(&result.agent)
                .unwrap()
                .retry_with_feedback(task_id, prompt).await;
            return Ok(());
        } else {
            // 重试耗尽 → 拒绝合并 + 通知人工
            self.task_states.insert(task_id.into(), TaskState::Rejected {
                reason: "验证失败超过 3 次，需人工介入".to_string(),
            });
            self.notify_human_intervention(task_id, &verification.errors).await?;
            return Ok(());
        }
    }

    // 通过 → 合并
    self.timeflow.merge_branch(branch, "main")?;
    Ok(())
}
```

---

## 8. 性能指标

| 指标 | 目标 | 说明 |
|------|:---:|------|
| **Lint 检查** | <1s | 快速规则检查 |
| **类型检查** | <5s | 静态类型分析 |
| **编译检查** | <30s | 完整编译 |
| **契约检查** | <2s | 基于 AST 的接口匹配 |
| **调用链检查** | <2s | 基于 AST 的调用图 |
| **首次编译通过率** | ≥80% | AI 代码重试 3 次后通过率 |
| **误杀率** | <5% | 白名单豁免后的误杀比例 |

---

## 9. 实现路径

| 阶段 | 目标 | 周期 |
|------|------|:---:|
| **M5.1** | Lint 检查 + 基础规则 | 1 周 |
| **M5.2** | 类型检查（Rust + TypeScript） | 2 周 |
| **M5.3** | 编译检查 + 错误反馈器 | 1 周 |
| **M5.4** | 白名单机制 | 1 周 |
| **M6.1** | 接口契约匹配（需 AST 引擎） | 2 周 |
| **M6.2** | 调用链完整性检查（需 AST 引擎） | 1 周 |
| **M6.3** | 与 AI 团队协作整合 + 重试闭环 | 1 周 |

**总计：约 9 周**，融入路线图 M5-M6。

---

## 10. 风险与对策

| 风险 | 等级 | 对策 |
|------|:---:|------|
| **编译耗时过长** | 🟡 中 | 增量编译 + 缓存 + 超时跳过 |
| **误杀率高** | 🟡 中 | 白名单机制 + 初期放宽规则 |
| **跨语言编译环境** | 🟡 中 | Docker 统一构建环境 |
| **AST 依赖** | 🟡 中 | Stage 4/5 依赖 AST 引擎，可降级跳过 |

---

## 11. 变更历史

| 日期 | 变更 | 作者 |
|------|------|------|
| 2026-06-17 | 初始设计文档 v1.0 | Trae |

---

> **验证护栏是 AgentWork 的可信壁垒**：让 AI 从"概率性生成"升级到"确定性交付"，让企业敢用、放心用。
