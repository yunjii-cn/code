# 云集智能体工作台 - 贡献指南

> **版本**：v1.0
> **更新日期**：2026-06-17
> **配套**：[AGENTS.md](../../AGENTS.md) · [ARCHITECTURE.md](ARCHITECTURE.md) · [ROADMAP.md](ROADMAP.md)

欢迎参与 **AgentWork** 项目！本文档说明如何参与开发、提 PR、报告 Bug。

---

## 0. 阅读顺序（新人必读）

加入项目前请按顺序阅读：

1. 📄 [README.md](../../README.md) — 5 分钟看懂项目
2. 📄 [AGENTS.md](../../AGENTS.md) — 4 代产品线契约（项目宪法）
3. 📄 [tier.yaml](../../tier.yaml) — 商业模式
4. 📄 [docs/ARCHITECTURE.md](ARCHITECTURE.md) — 系统架构
5. 📄 [docs/ROADMAP.md](ROADMAP.md) — 详细路线图
6. 📄 **本文档** — 贡献流程

---

## 1. 提 Bug / Feature

### 1.1 提 Bug

用 GitHub Issues 提交 bug，请包含：

- **标题**：`[Bug] 简短描述`
- **环境**：OS / Rust 版本 / Node 版本 / Docker 版本
- **复现步骤**：详细到每一步
- **预期行为** vs **实际行为**
- **截图/日志**（如有）
- **可能原因**（如有猜测）

### 1.2 提 Feature

- **标题**：`[Feature] 简短描述`
- **场景**：用户故事（As a [user], I want to [action], so that [value]）
- **验收标准**：什么算"完成"
- **替代方案**：考虑过但放弃的方案
- **影响范围**：哪些模块/文件需要改

---

## 2. 提 PR（Pull Request）

### 2.1 流程

```
1. Fork 仓库（GitHub/Gitee）
   ↓
2. 克隆你的 fork
   ↓
3. 创建特性分支: feat/xxx / fix/xxx
   ↓
4. 写代码 + 测试
   ↓
5. 跑本地检查: cargo clippy + cargo test + pnpm test
   ↓
6. 提交 (Conventional Commits)
   ↓
7. push 到你的 fork
   ↓
8. 在 GitHub 上开 PR
   ↓
9. CI 通过 + review 通过
   ↓
10. 合并
```

### 2.2 分支命名

- `feat/*` — 新功能（如 `feat/aw-git-clone`）
- `fix/*` — bug 修复（如 `fix/sandbox-network-leak`）
- `docs/*` — 文档（如 `docs/arch-adr-010`）
- `refactor/*` — 重构（如 `refactor/aw-core-traits`）
- `test/*` — 测试（如 `test/aw-git-e2e`）
- `chore/*` — 杂项（如 `chore/bump-deps`）

### 2.3 Commit 规范

遵循 [Conventional Commits](https://www.conventionalcommits.org/)：

```
<type>(<scope>): <subject>

<body>

<footer>
```

**type**：
- `feat` — 新功能
- `fix` — bug 修复
- `docs` — 文档
- `style` — 代码格式（不改语义）
- `refactor` — 重构（既不是 feat 也不是 fix）
- `test` — 加测试
- `chore` — 杂项（依赖、构建）
- `perf` — 性能优化

**scope**：`desktop` / `web` / `cli` / `api` / `platformkit` / `runtime` / `skills` / `sandbox` / `docs` / `ci`

**subject**：中文，50 字以内，动词开头，不加句号

**示例**：

```
feat(aw-git): 实现 GitRepo::clone 方法

- 加 git2 依赖
- 实现 Repository 结构体的 clone 关联函数
- 加 5 个单元测试

Closes #42
```

### 2.4 PR 模板

开 PR 时请用以下模板：

```markdown
## 改动类型

- [ ] feat（新功能）
- [ ] fix（bug 修复）
- [ ] docs（文档）
- [ ] refactor（重构）
- [ ] test（测试）
- [ ] chore（杂项）

## 改动说明

<!-- 简述改了什么、为什么改 -->

## 测试

<!-- 怎么验证的？单元测试？E2E？手动？ -->

## 截图/录屏

<!-- UI 改动必填 -->

## Checklist

- [ ] 代码通过 `cargo clippy -- -D warnings`
- [ ] 代码通过 `cargo test`
- [ ] 前端通过 `pnpm lint` + `pnpm test`
- [ ] 加了单元测试
- [ ] 加了 E2E 测试（如适用）
- [ ] 文档已更新（如适用）
- [ ] 跨代影响已考虑（1.PC/2.WEB/3.dev）

## 关联 Issue

Closes #
```

---

## 3. 开发环境准备

### 3.1 必需工具

| 工具 | 版本 | 安装 |
|---|---|---|
| **Rust** | 1.78+ | [rustup.rs](https://rustup.rs) |
| **Node.js** | 20+ | [nodejs.org](https://nodejs.org) |
| **pnpm** | 9+ | `npm install -g pnpm` |
| **Docker** | 24+ | [docker.com](https://docker.com) |
| **Git** | 2.40+ | 系统自带 |

### 3.2 可选工具

| 工具 | 用途 |
|---|---|
| **cargo-watch** | 文件改动自动重跑测试 |
| **cargo-tarpaulin** | 单元测试覆盖率 |
| **tauri-cli** | `cargo install tauri-cli --version "^2.0"` |
| **just** | 命令运行器（类似 make） |
| **gVisor (runsc)** | 沙箱加固（生产环境） |

### 3.3 验证环境

```bash
# Rust
rustc --version        # 应 ≥ 1.78
cargo --version

# Node / pnpm
node --version         # 应 ≥ 20
pnpm --version         # 应 ≥ 9

# Tauri CLI
cargo tauri --version  # 应 ≥ 2.0

# Docker
docker --version       # 应 ≥ 24

# Git
git --version          # 应 ≥ 2.40
```

---

## 4. 项目结构

```
4.AgentWork/
├── apps/                  4 个端应用
│   ├── desktop/           Tauri 2 桌面
│   ├── web/               Next.js 14 Web
│   ├── cli/               Rust CLI
│   └── mcp-bridge/        MCP 协议桥
├── platformkit/           共享层
│   ├── crates/            Rust crates（aw-git, aw-store, aw-mcp, aw-core, aw-runtime）
│   └── packages/          TS packages（@aw/ui, @aw/sdk, @aw/types）
├── runtime/               OpenHands fork
├── skills/                Agent Skills
├── sandbox-image/         Docker 沙箱
├── docs/                  架构/路线图/ADR
├── tests/                 E2E + 集成测试
├── scripts/               构建/部署脚本
├── examples/              示例项目
└── .github/workflows/     CI
```

详见 [ARCHITECTURE.md](ARCHITECTURE.md) §1。

---

## 5. 开发流程

### 5.1 写代码前

1. **确认任务**：在 [ROADMAP.md](ROADMAP.md) 里找到对应任务
2. **读架构**：相关模块的 ARCHITECTURE.md 章节
3. **看现有代码**：`apps/` 和 `platformkit/` 下的实现
4. **查 ADR**：是否已有相关决策

### 5.2 写代码时

- ✅ 遵循 Rust 命名规范（snake_case 函数, PascalCase 类型）
- ✅ 公共 API 必加文档注释（`///`）
- ✅ 错误处理用 `Result<T, E>` + `thiserror`
- ✅ 异步代码用 `tokio`
- ✅ 测试覆盖率 ≥ 80%
- ❌ 不写 `unwrap()` 除非能证明安全
- ❌ 不写 `panic!` 在库代码里

### 5.3 提 PR 前

```bash
# 1. 跑 clippy
cargo clippy --all-targets -- -D warnings

# 2. 跑测试
cargo test --workspace

# 3. 跑格式检查
cargo fmt --check
pnpm format:check

# 4. 跑前端 lint
pnpm lint

# 5. 跑前端测试
pnpm test

# 6. 跑 E2E（如改动了 desktop/web）
pnpm test:e2e

# 7. 写 commit message
git commit -m "feat(aw-git): 实现 clone API"

# 8. push 到你的 fork
git push origin feat/aw-git-clone
```

---

## 6. 跨代协作

本项目 (`4.AgentWork/`) 跟前 3 代共享 root 仓库，但**代码完全独立**。

### 6.1 复用 vs 引用

| 场景 | 处理 |
|---|---|
| 借鉴 3.dev `skill_market` 思想 | ✅ 复制代码到 `platformkit/`，不直接 import |
| 借 1.PC 单实例控制经验 | ✅ 提取到 `aw-config`，写新的 Rust 实现 |
| 借 2.WEB 的 17 个 API 设计 | ✅ 参考设计模式，不复制代码 |
| 改 3.dev 的 platformkit | ❌ 不动，3.dev 是独立产品 |

### 6.2 跨代修改建议

如果发现跨代有价值的功能，按以下顺序：

1. **先在 4.AgentWork 实现新版本**（用 Rust，不用 Python）
2. **验证后** 写 RFC 评估是否同步回 3.dev
3. **永远不**把 4.AgentWork 的代码反向同步到 1.PC/2.WEB

---

## 7. Skills 贡献

### 7.1 创建 Skill

每个 Skill 是一个目录：

```
skills/your-skill/
├── SKILL.md          # 元数据 + 描述
├── main.py 或 main.rs # 实现
├── prompts/          # LLM prompt 模板
│   └── default.txt
├── tests/            # 单元测试
└── README.md         # 用户文档
```

### 7.2 SKILL.md 格式

```markdown
# Skill 名称

> 简短描述

## 元数据

- **name**: my-skill
- **version**: 1.0.0
- **author**: your-name
- **license**: Apache-2.0
- **tags**: [refactor, code-quality]

## 描述

详细描述这个 Skill 做什么。

## 输入

- `repo_path`: 仓库路径
- `target_files`: 要处理的文件列表

## 输出

- 修改后的文件
- 详细报告

## 依赖

- `git2` (Rust)
- `tree-sitter-python`

## 示例

输入: "重构 utils.py 中的重复代码"
输出: 自动识别 3 处重复，重命名为共享函数
```

### 7.3 提交到市场

- 提交 PR 到 `skills/community/your-skill/`
- 通过 CI 验证 + 1 个核心维护者 review
- 合并后自动发布到 Skills 市场

---

## 8. MCP 工具贡献

每个 MCP 工具是一个独立 crate：

```
platformkit/crates/aw-mcp-tools/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── read_file.rs
│   ├── write_file.rs
│   └── run_shell.rs
└── tests/
```

工具实现必须：
- 实现 `aw_mcp::Tool` trait
- 提供 JSON Schema 描述输入/输出
- 至少 5 个单元测试
- 至少 1 个 E2E 测试

---

## 9. 沟通渠道

| 渠道 | 用途 | 响应时间 |
|---|---|---|
| **GitHub Issues** | Bug 报告、Feature 建议 | 48h |
| **GitHub Discussions** | 提问、讨论 | 24h |
| **Gitee Issues** | 国内用户（备用） | 72h |
| **邮件** | 安全漏洞、私密问题 | 7d |

**安全漏洞**请发邮件到 `security@yunji.ai`，**不要**在 GitHub Issues 公开。

---

## 10. Code of Conduct

### 10.1 我们的承诺

为了营造开放友好的环境，我们承诺：

- 欢迎所有背景的人参与
- 尊重不同的观点和经验
- 优雅地接受建设性批评
- 聚焦对社区最有利的事

### 10.2 不可接受的行为

- 使用性暗示的语言或图像
- 人身攻击、侮辱、贬低
- 公开或私下的骚扰
- 未经允许发布他人隐私信息
- 其他不道德或不专业的行为

### 10.3 举报

违反行为请举报到 `conduct@yunji.ai`。

---

## 11. 许可证

贡献的代码默认采用 **Apache 2.0** 许可（详见根目录 `LICENSE`）。

贡献 Skills 默认采用 **Apache 2.0 + 商业**双许可。

---

## 12. 变更历史

| 日期 | 变更 | 作者 |
|---|---|---|
| 2026-06-17 | 初始贡献指南 v1.0 | Trae |
