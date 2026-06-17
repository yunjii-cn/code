# AST-Native 代码认知引擎设计 — 活体代码知识图谱

> **版本**：v1.0
> **日期**：2026-06-17
> **状态**：设计稿（待 M4 落地）
> **配套**：[AGENT-TEAM-DESIGN.md](AGENT-TEAM-DESIGN.md) · [VERIFICATION-GUARDRAILS.md](VERIFICATION-GUARDRAILS.md)

---

## 0. 一句话定位

**AST-Native 引擎**用 Tree-sitter + LanceDB 构建"类-方法-调用链-依赖关系"的活体代码知识图谱，让 AI Agent 精准理解代码结构，Token 消耗降低 70%，准确率 90%+，并支持跨语言契约监听。

**核心价值**：从"读全文"升级到"读结构"，从"概率性理解"升级到"确定性认知"。

---

## 1. 为什么需要 AST-Native

### 1.1 传统 AI 编程的痛点

| 痛点 | 说明 |
|------|------|
| **Token 浪费** | LLM 读取整个文件，90% 内容与任务无关 |
| **跨文件理解弱** | 改一个函数签名，LLM 不知道哪些文件调用了它 |
| **跨语言断裂** | 后端改 API，前端类型定义不会自动更新 |
| **幻觉风险** | LLM 凭空想象不存在的函数/类 |

### 1.2 AST-Native 的价值

```
传统方式 (Cursor/Copilot):
  用户: "修改 login 函数"
  LLM: 读取整个 auth.rs (2000 行) → 浪费 Token
       不知道哪些文件调用 login() → 可能漏改

AST-Native 方式 (我们):
  用户: "修改 login 函数"
  AST 引擎: 精确定位 login() 定义 + 12 处调用点
  LLM: 只读取相关代码片段 (50 行) → Token 降低 70%
       自动列出所有受影响文件 → 零漏改
```

---

## 2. 核心架构

```
┌──────────────────────────────────────────────────────────┐
│                  AST-Native 代码认知引擎                  │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  ┌────────────┐   ┌────────────┐   ┌────────────┐       │
│  │ Tree-sitter│   │  LanceDB   │   │  调用链    │       │
│  │ 多语言解析 │──▶│ 向量图谱存储│──▶│  分析器    │       │
│  └────────────┘   └────────────┘   └────────────┘       │
│        │                │                │               │
│        ▼                ▼                ▼               │
│   增量 AST 更新    类-方法-调用链     影响范围分析        │
│   (≤200ms)        向量索引           (跨文件/跨语言)     │
│                                                          │
│  ┌────────────────────────────────────────────────┐     │
│  │              契约监听器 (Contract Listener)      │     │
│  │  .proto / Swagger / GraphQL Schema 变更监听     │     │
│  │  → 自动触发跨语言任务（如 TS 类型更新）          │     │
│  └────────────────────────────────────────────────┘     │
│                                                          │
└──────────────────────────────────────────────────────────┘
        │                              │
        ▼                              ▼
┌────────────────┐            ┌────────────────┐
│ AI 团队协作引擎 │            │  验证护栏引擎   │
│ - 智能上下文提取│            │ - 类型一致性检查│
│ - 任务依赖识别  │            │ - 调用链完整性  │
│ - Token 优化 70%│            │ - 接口契约匹配  │
└────────────────┘            └────────────────┘
```

---

## 3. 核心组件

### 3.1 Tree-sitter 多语言解析器

```rust
// platformkit/crates/ast-native/src/parser.rs

use tree_sitter::{Parser, Tree, Node};

/// 多语言 AST 解析器
pub struct AstParser {
    parsers: HashMap<Language, Parser>,
}

impl AstParser {
    /// 支持的语言
    pub fn supported_languages() -> Vec<Language> {
        vec![
            Language::Rust,
            Language::TypeScript,
            Language::JavaScript,
            Language::Python,
            Language::Go,
            Language::Java,
            Language::Kotlin,
            Language::Swift,
            Language::Cpp,
            Language::CSharp,
        ]
    }

    /// 解析文件，返回 AST
    pub fn parse(&self, path: &Path, content: &str) -> Result<Tree> {
        let lang = self.detect_language(path)?;
        let parser = self.parsers.get(&lang)
            .ok_or(Error::UnsupportedLanguage(lang))?;
        parser.parse(content, None)
            .ok_or(Error::ParseFailed(path.to_path_buf()))
    }

    /// 增量解析（文件保存后 ≤200ms 完成）
    pub fn parse_incremental(
        &self,
        path: &Path,
        old_tree: &Tree,
        new_content: &str,
        changes: &[Range],
    ) -> Result<Tree> {
        let lang = self.detect_language(path)?;
        let parser = self.parsers.get(&lang)?;
        parser.parse_with_tree(new_content, Some(old_tree), changes)
            .ok_or(Error::ParseFailed(path.to_path_buf()))
    }
}
```

### 3.2 代码知识图谱 (LanceDB)

```rust
// platformkit/crates/ast-native/src/graph.rs

/// 代码知识图谱节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeNode {
    pub id: String,              // 节点 ID (路径+符号名)
    pub kind: NodeKind,          // 节点类型
    pub name: String,            // 符号名
    pub file_path: PathBuf,      // 文件路径
    pub location: Location,      // 行列位置
    pub signature: Option<String>, // 函数签名/类定义
    pub doc_comment: Option<String>, // 文档注释
    pub embedding: Vec<f32>,     // 语义向量（用于搜索）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    Module,
    Class,
    Interface,
    Function,
    Method,
    Variable,
    TypeAlias,
    Enum,
}

/// 代码知识图谱边（关系）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEdge {
    pub from: String,            // 源节点 ID
    pub to: String,              // 目标节点 ID
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeKind {
    Calls,        // A 调用 B
    CalledBy,     // A 被 B 调用
    Implements,   // A 实现 B
    Extends,      // A 继承 B
    Imports,      // A 导入 B
    Depends,      // A 依赖 B
    Defines,      // A 定义 B
}

/// 代码知识图谱
pub struct CodeGraph {
    /// LanceDB 向量存储
    db: lancedb::Connection,
    /// 节点表
    nodes: lancedb::Table,
    /// 边表
    edges: lancedb::Table,
}

impl CodeGraph {
    /// 增量更新（文件保存后触发）
    pub async fn update_file(&mut self, path: &Path, content: &str) -> Result<()> {
        // 1. 解析 AST
        let tree = self.parser.parse(path, content)?;

        // 2. 提取节点和边
        let (new_nodes, new_edges) = self.extract_symbols(path, &tree)?;

        // 3. 删除旧节点（该文件的）
        self.nodes.delete(&format!("file_path = '{}'", path.display())).await?;

        // 4. 插入新节点
        self.nodes.add(&new_nodes).await?;
        self.edges.add(&new_edges).await?;

        Ok(())
    }

    /// 查询调用链
    pub async fn get_callers(&self, function_id: &str) -> Result<Vec<CodeNode>> {
        // 查询所有调用该函数的节点
        let edges = self.edges.query()
            .filter(&format!("to = '{}' AND kind = 'CalledBy'", function_id))
            .execute().await?;

        let caller_ids: Vec<String> = edges.iter().map(|e| e.from.clone()).collect();
        self.nodes.get_by_ids(&caller_ids).await
    }

    /// 查询被调用链
    pub async fn get_callees(&self, function_id: &str) -> Result<Vec<CodeNode>> {
        let edges = self.edges.query()
            .filter(&format!("from = '{}' AND kind = 'Calls'", function_id))
            .execute().await?;

        let callee_ids: Vec<String> = edges.iter().map(|e| e.to.clone()).collect();
        self.nodes.get_by_ids(&callee_ids).await
    }
}
```

### 3.3 智能上下文提取器

```rust
// platformkit/crates/ast-native/src/context_extractor.rs

/// 智能上下文提取器
/// 为 AI Agent 提取任务相关的最小代码片段
pub struct ContextExtractor {
    graph: CodeGraph,
}

impl ContextExtractor {
    /// 根据任务描述提取相关代码
    ///
    /// 这是 AST-Native 的核心价值：
    /// - 传统方式：读取整个文件（2000 行，浪费 Token）
    /// - AST 方式：只读取相关符号 + 调用链（50 行，省 70% Token）
    pub async fn extract_relevant_code(
        &self,
        task_description: &str,
        acceptance_criteria: &str,
    ) -> Result<Vec<CodeSnippet>> {
        // 1. 语义搜索相关符号
        let query_embedding = self.embed(task_description).await?;
        let relevant_symbols = self.graph.vector_search(&query_embedding, 10).await?;

        // 2. 扩展调用链（上下游）
        let mut all_symbols = HashSet::new();
        for symbol in &relevant_symbols {
            all_symbols.insert(symbol.clone());
            // 上游（谁调用了它）
            for caller in self.graph.get_callers(&symbol.id).await? {
                all_symbols.insert(caller);
            }
            // 下游（它调用了谁）
            for callee in self.graph.get_callees(&symbol.id).await? {
                all_symbols.insert(callee);
            }
        }

        // 3. 提取代码片段（带上下文）
        let mut snippets = Vec::new();
        for symbol in all_symbols {
            let snippet = self.read_symbol_with_context(&symbol).await?;
            snippets.push(snippet);
        }

        Ok(snippets)
    }

    /// 读取符号及其上下文（前后各 5 行）
    async fn read_symbol_with_context(&self, symbol: &CodeNode) -> Result<CodeSnippet> {
        let content = fs::read_to_string(&symbol.file_path).await?;
        let lines: Vec<&str> = content.lines().collect();

        let start = symbol.location.start_line.saturating_sub(5);
        let end = (symbol.location.end_line + 5).min(lines.len());

        Ok(CodeSnippet {
            file_path: symbol.file_path.clone(),
            symbol_name: symbol.name.clone(),
            kind: symbol.kind.clone(),
            content: lines[start..end].join("\n"),
            location: symbol.location.clone(),
        })
    }
}
```

### 3.4 跨语言契约监听器

```rust
// platformkit/crates/ast-native/src/contract_listener.rs

/// 跨语言契约监听器
/// 监听 .proto / Swagger / GraphQL Schema 变更，自动触发跨语言任务
pub struct ContractListener {
    watchers: HashMap<ContractType, Box<dyn Watcher>>,
    event_tx: mpsc::Sender<ContractEvent>,
}

#[derive(Debug, Clone)]
pub enum ContractType {
    Proto,        // gRPC .proto 文件
    OpenApi,      // Swagger/OpenAPI 规范
    GraphQL,      // GraphQL Schema
    TypeScript,   // TypeScript 类型定义（前端契约）
}

#[derive(Debug, Clone)]
pub struct ContractEvent {
    pub contract_type: ContractType,
    pub file_path: PathBuf,
    pub changes: Vec<ContractChange>,
    pub affected_languages: Vec<Language>,
}

impl ContractListener {
    /// 处理契约变更
    pub async fn handle_change(&self, event: ContractEvent) -> Result<Vec<CrossLanguageTask>> {
        let mut tasks = Vec::new();

        for change in &event.changes {
            match change {
                ContractChange::ApiAdded(api) => {
                    // 后端新增 API → 前端需生成调用代码
                    tasks.push(CrossLanguageTask {
                        title: format!("为 {} 生成前端调用代码", api.path),
                        target_language: Language::TypeScript,
                        source_language: Language::Rust,
                        trigger: TaskTrigger::ContractChange,
                    });
                }
                ContractChange::ApiModified(api) => {
                    // 后端修改 API → 前端需更新调用 + 类型
                    tasks.push(CrossLanguageTask {
                        title: format!("更新 {} 的前端类型和调用", api.path),
                        target_language: Language::TypeScript,
                        source_language: Language::Rust,
                        trigger: TaskTrigger::ContractChange,
                    });
                }
                ContractChange::ApiRemoved(api) => {
                    // 后端删除 API → 前端需移除调用
                    tasks.push(CrossLanguageTask {
                        title: format!("移除 {} 的前端调用", api.path),
                        target_language: Language::TypeScript,
                        source_language: Language::Rust,
                        trigger: TaskTrigger::ContractChange,
                    });
                }
            }
        }

        // 发送事件给 DAG 调度引擎
        for task in &tasks {
            self.event_tx.send(TaskEvent::NewTask(task.clone())).await?;
        }

        Ok(tasks)
    }
}
```

---

## 4. 性能指标

| 指标 | 目标 | 说明 |
|------|:---:|------|
| **增量 AST 更新** | ≤200ms | 文件保存后完成索引更新 |
| **调用链查询** | ≤50ms | 单次调用链查询响应时间 |
| **语义搜索** | ≤100ms | Top-10 相关符号搜索 |
| **Token 节省** | ≥70% | 相比读取整个文件 |
| **准确率** | ≥90% | 相关符号召回率 |
| **支持语言** | ≥10 | 主流编程语言全覆盖 |

---

## 5. 与其他模块的整合

### 5.1 与 AI 团队协作引擎

```
AI 团队协作引擎 ←→ AST-Native 引擎

1. 任务拆解阶段:
   Orchestrator 调用 AST.get_callers() 识别隐含依赖
   → 生成更准确的 DAG

2. 上下文准备阶段:
   Agent 调用 ContextExtractor.extract_relevant_code()
   → Token 降低 70%

3. 任务完成阶段:
   AST 检测调用链变化
   → 自动通知受影响的其他 Agent
```

### 5.2 与验证护栏引擎

```
验证护栏 ←→ AST-Native 引擎

1. 类型一致性检查:
   AST 提取函数签名
   → 验证护栏检查所有调用点是否匹配

2. 接口契约匹配:
   AST 提取 API 定义
   → 验证护栏检查前端调用是否匹配后端契约

3. 调用链完整性:
   AST 提供调用图
   → 验证护栏检查是否有断链
```

### 5.3 与 TimeFlow 版本控制

```
TimeFlow ←→ AST-Native 引擎

1. 快照触发 AST 增量更新:
   每次快照后，AST 引擎更新受影响文件的索引

2. 版本对比:
   TimeFlow diff 两个快照
   → AST 引擎提供"语义级 diff"（不只是文本 diff）
```

---

## 6. 实现路径

| 阶段 | 目标 | 周期 |
|------|------|:---:|
| **M4.1** | Tree-sitter 集成 + 基础 AST 解析 | 2 周 |
| **M4.2** | LanceDB 向量存储 + 代码图谱构建 | 2 周 |
| **M4.3** | 智能上下文提取器 + Token 优化验证 | 1 周 |
| **M5.1** | 调用链分析 + 影响范围查询 | 2 周 |
| **M5.2** | 跨语言契约监听器（Proto/OpenAPI） | 2 周 |
| **M6.1** | 与 AI 团队协作引擎整合 | 1 周 |
| **M6.2** | 与验证护栏引擎整合 | 1 周 |

**总计：约 11 周**，融入路线图 M4-M6。

---

## 7. 风险与对策

| 风险 | 等级 | 对策 |
|------|:---:|------|
| **大文件解析阻塞** | 🟡 中 | 后台线程异步解析 + 分片处理 |
| **LanceDB 学习曲线** | 🟡 中 | 先用 SQLite 验证逻辑，再替换 LanceDB |
| **Tree-sitter 语法覆盖** | 🟢 低 | 主流语言已覆盖，小众语言后置 |
| **增量更新一致性** | 🟡 中 | 版本号校验 + 失败回滚 |
| **Embedding 模型成本** | 🟡 中 | 本地 bge-small 优先 |

---

## 8. 变更历史

| 日期 | 变更 | 作者 |
|------|------|------|
| 2026-06-17 | 初始设计文档 v1.0 | Trae |

---

> **AST-Native 是 AgentWork 的认知壁垒**：让 AI 从"读文本"升级到"读结构"，从"概率性理解"升级到"确定性认知"。
