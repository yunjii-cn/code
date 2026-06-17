# TimeFlow 设计文档 — 本地优先的 AI 版本控制与全链路发布引擎

> **版本**：v1.0
> **日期**：2026-06-17
> **状态**：设计稿（待 M1 落地）
> **配套**：[AGENTS.md](../AGENTS.md) · [ROADMAP.md](ROADMAP.md) · [ARCHITECTURE.md](ARCHITECTURE.md)

---

## 0. 一句话定位

**TimeFlow** 是 AgentWork 的核心差异化引擎：**本地优先的版本控制 + Git 双模兼容 + AI 语义化版本管理 + 全链路自动发布**。

让开发者拥有"云端文档般的版本控制体验"，同时保留 Git 的协作能力，并自动完成从代码到多平台分发的全流程。

---

## 1. 为什么需要 TimeFlow

### 1.1 现有方案的痛点

| 痛点 | Git | Time Machine / Dropbox | VS Code Local History |
|------|-----|------------------------|----------------------|
| 需要手动 commit | ✅ 是 | ❌ 自动 | ❌ 自动 |
| 支持分支/回滚 | ✅ 是 | ❌ 仅线性 | ❌ 仅单文件 |
| 远程协作 | ✅ 是 | ❌ 无 | ❌ 无 |
| 隐私（不上云） | ✅ 可 | ✅ 是 | ✅ 是 |
| 语义搜索版本 | ❌ 无 | ❌ 无 | ❌ 无 |
| 自动版本整理 | ❌ 无 | ❌ 无 | ❌ 无 |
| 自动发布 | ❌ 无 | ❌ 无 | ❌ 无 |

**结论**：没有任何现有工具同时满足"自动快照 + 分支回滚 + Git 兼容 + AI 语义 + 自动发布"。

### 1.2 TimeFlow 的差异化

```
传统 Git 工作流:
  手动 add → 手动 commit msg → 手动 push → 手动 tag → 手动 build → 手动上传

TimeFlow 工作流:
  写代码 → 自动快照 → AI 生成 commit → AI 整理正式版本
       → 自动构建多平台 → 自动分发多平台 → 生成 Release Notes
```

---

## 2. 四层架构

```
┌──────────────────────────────────────────────────────────┐
│  Layer 4: 全链路发布引擎 (Release Pipeline)               │
│  多目标构建矩阵 → 多平台分发 → 技术 Release Notes         │
│  边界: 生成"给开发者看的发布说明", 不做营销物料            │
├──────────────────────────────────────────────────────────┤
│  Layer 3: 工作流引擎 (Workflow Engine)                    │
│  触发词 / 定时 / 事件 → DAG 执行                          │
├──────────────────────────────────────────────────────────┤
│  Layer 2: AI 语义层 (Semantic Layer) ⭐核心差异化         │
│  commit msg / 版本号建议 / changelog / 语义搜索 / 可运行标记 │
├──────────────────────────────────────────────────────────┤
│  Layer 1: 存储引擎 (Storage Engine)                       │
│  自研 VCS (内容寻址+快照链) + libgit2 兼容层              │
└──────────────────────────────────────────────────────────┘
```

---

## 3. Layer 1: 存储引擎

### 3.1 设计原则

- **本地优先**：默认纯本地，代码永不上云
- **内容寻址**：类似 Git 的 blob/tree/commit 模型，但更简单
- **快照链**：每次变更生成快照，形成父子链
- **加密可选**：企业版支持 AES-256 加密存储
- **Git 兼容**：可一键导出为 git 仓库 / 从 git 导入

### 3.2 数据模型

```rust
// 快照（Snapshot）= Git 的 commit
struct Snapshot {
    id: SnapshotId,           // 内容哈希
    parent: Option<SnapshotId>, // 父快照（形成链）
    timestamp: DateTime,
    tree: TreeId,             // 内容寻址树根
    author: String,
    metadata: SnapshotMetadata,
}

struct SnapshotMetadata {
    trigger: TriggerType,     // file_change | manual | schedule | workflow
    ai_summary: Option<String>,    // AI 生成的变更摘要
    ai_commit_msg: Option<String>, // AI 生成的 commit message
    build_status: BuildStatus,     // green | yellow | red | unknown
    snapshot_type: SnapshotType,   // wip | progress | candidate | release
    tags: Vec<String>,
}

// 内容寻址树
struct Tree {
    entries: HashMap<PathBuf, BlobId>,
}

struct Blob {
    id: BlobId,               // SHA-256 of content
    content: Vec<u8>,         // 实际内容（压缩存储）
}
```

### 3.3 存储布局

```
.yunji/timeflow/
├── snapshots/
│   ├── snap_a3f8...json     # 快照元数据
│   └── snap_b7c2...json
├── blobs/
│   ├── blob_x7y2...         # 内容寻址 blob（压缩）
│   └── blob_k9m3...
├── trees/
│   └── tree_...json
├── branches/
│   ├── main.json            # 分支指针（指向最新快照）
│   └── feature-login.json
├── index/
│   ├── embeddings.db        # AI 语义索引（Layer 2）
│   └── build_status.db      # 构建状态缓存
└── config.toml              # TimeFlow 配置
```

### 3.4 核心操作

| 操作 | 复杂度 | 说明 |
|------|:---:|------|
| `snapshot()` | O(变更文件数) | 扫描变更文件，生成快照 |
| `rollback(snap_id)` | O(文件数) | 恢复到指定快照（保留历史） |
| `branch(name)` | O(1) | 创建分支指针 |
| `diff(snap_a, snap_b)` | O(差异) | 计算两快照差异 |
| `list_snapshots(filter)` | O(快照数) | 列出快照（支持过滤） |
| `export_to_git(path)` | O(快照数) | 导出为 git 仓库 |
| `import_from_git(path)` | O(commit 数) | 从 git 导入 |

### 3.5 与 libgit2 的关系

- **TimeFlow 是主存储**：所有快照存在 `.yunji/timeflow/`
- **libgit2 是兼容层**：仅在"同步模式"下使用，把 TimeFlow 快照镜像为 git commit
- **不依赖 git 也能用**：隐身模式下完全不碰 git

---

## 4. Layer 2: AI 语义层

### 4.1 五大 AI 能力

| 能力 | 输入 | 输出 | 触发时机 |
|------|------|------|---------|
| **变更摘要** | diff | 一句话描述变更 | 每次快照后 |
| **commit msg** | diff + 历史 | Conventional Commits 格式 | 快照后（同步模式） |
| **版本号建议** | 自上次 release 的 commits | semver 建议 + 理由 | 候选版本检测时 |
| **语义搜索** | 自然语言查询 | 匹配快照列表 | 用户搜索时 |
| **可运行标记** | 快照 + 构建结果 | green/yellow/red | 后台构建后 |

### 4.2 快照分类（AI 自动）

```
快照类型自动分类:
  🗑️  WIP 快照     - 改动小/未完成/编译失败 → 7天后自动清理
  📝 进度快照     - 有意义但非正式 → 保留，不发布
  🎯 候选版本     - 编译通过+测试通过+改动充分 → 提示用户
  🚀 正式版本     - 用户确认/触发词 → 自动发布
```

### 4.3 语义搜索实现

```rust
// 每次快照后，后台生成 embedding
async fn index_snapshot(snap: &Snapshot) {
    let summary = ai.generate_summary(&snap.diff).await;
    let embedding = ai.embed(&summary).await;
    db.insert_embedding(snap.id, &embedding);
}

// 用户搜索
async fn semantic_search(query: &str) -> Vec<SnapshotMatch> {
    let q_emb = ai.embed(query).await;
    let candidates = db.vector_search(&q_emb, top_k=10);
    candidates.into_iter()
        .map(|(snap_id, score)| SnapshotMatch { snap_id, score })
        .collect()
}
```

### 4.4 Embedding 模型选择

| 模式 | 模型 | 隐私 | 速度 |
|------|------|:---:|:---:|
| 隐身模式 | bge-small-zh（本地） | ✅ 不上云 | 🟡 中 |
| 同步模式 | OpenAI text-embedding-3-small | ❌ 上云 | 🟢 快 |
| 企业版 | 可配置私有模型 | ✅ | 🟡 |

---

## 5. Layer 3: 工作流引擎

### 5.1 触发器类型

| 触发器 | 说明 | 示例 |
|--------|------|------|
| `keyword` | 输入框触发词 | `/release 1.3.0` |
| `schedule` | 定时 | 每小时快照、每周检查发布 |
| `file_change` | 文件变化 | 保存 .py 文件时 AI 审查 |
| `snapshot` | 快照事件 | 新快照后自动构建测试 |
| `manual` | 手动 | UI 按钮点击 |

### 5.2 工作流定义（YAML）

```yaml
# .yunji/workflows.yml
workflows:
  - name: "稳定版本自动发Release"
    trigger:
      type: keyword
      pattern: "/release"
    actions:
      - run_tests
      - if: tests_passed
        then:
          - ai_suggest_version
          - ai_generate_changelog
          - build_all_targets
          - publish_all_platforms
          - notify: "Release v{{version}} 已发布"

  - name: "每小时自动快照"
    trigger:
      type: schedule
      cron: "0 * * * *"
    actions:
      - snapshot
      - ai_summarize
      - build_check
      - cleanup_older_than: "7d"

  - name: "AI 代码审查"
    trigger:
      type: file_change
      pattern: "src/**/*.py"
    actions:
      - ai_review
      - if: issues_found
        then: notify
```

### 5.3 DAG 执行模型

```
工作流 = DAG（有向无环图）
节点 = Action（snapshot / build / publish / ai_*）
边 = 依赖关系 + 条件分支

执行器:
  - 异步执行（tokio）
  - 失败重试（可配置）
  - 状态持久化（中断可恢复）
  - 日志审计（所有操作可追溯）
```

---

## 6. Layer 4: 全链路发布引擎

### 6.1 发布边界（重要）

```
✅ 做到（技术分发）:
   - 多目标构建（Windows exe / macOS app / Linux / Web）
   - 多平台分发（GitHub Release / Gitee / 网盘 / 官网）
   - 技术 Release Notes（changelog / 更新公告）
   - 版本号自动管理（semver）

❌ 不做（营销物料）:
   - 不做短视频 / 海报 / 封面图生成
   - 不做朋友圈文案 / 小红书种草文
   - 不做投流素材
   - 营销方向应另起独立产品
```

**原则**：生成"给开发者看的发布说明"，不是"给消费者看的广告"。

### 6.2 多目标构建矩阵

```yaml
# .yunji/release.yml
targets:
  windows:
    build: "python build/build_pc.py"
    artifacts: ["dist/*.exe"]
    platform: windows-x86_64

  macos:
    build: "python build/build_mac.py"
    artifacts: ["dist/*.app", "dist/*.dmg"]
    platform: macos-universal

  linux:
    build: "cargo tauri build"
    artifacts: ["target/release/bundle/appimage/*.AppImage"]
    platform: linux-x86_64

  web:
    build: "pnpm build"
    artifacts: ["ui/dist/**"]
    platform: web
```

### 6.3 多平台分发适配器

| 平台 | 适配器 | 认证 | 分发内容 |
|------|--------|------|---------|
| GitHub Release | `octocrab` (Rust) | Personal Access Token | exe/app/dmg + Release Notes |
| Gitee Release | HTTP API | Personal Access Token | 同上 |
| 蓝奏云网盘 | HTTP API | Cookie/Token | exe（小文件） |
| 阿里云盘 | HTTP API | Refresh Token | 大文件 |
| 官网 CDN | S3 兼容 API | Access Key | 全部产物 |
| Docker Hub | Docker API | Token | 镜像（Web 端） |
| npm | npm API | Token | 前端包（如适用） |

### 6.4 Release Notes 自动生成

```rust
async fn generate_release_notes(from: SnapshotId, to: SnapshotId) -> String {
    let snapshots = list_snapshots(from, to);
    let commits: Vec<CommitInfo> = snapshots.iter()
        .map(|s| s.metadata.ai_commit_msg.clone().unwrap_or_default())
        .collect();

    let prompt = format!(
        "基于以下 commit 生成 Release Notes，按 Conventional Commits 分类:\n{}",
        commits.join("\n")
    );

    let notes = ai.complete(&prompt).await;
    format_release_notes(&notes) // Markdown 格式
}
```

**输出示例**：
```markdown
# v1.3.0 发布说明

## ✨ 新功能
- 添加 JWT 登录验证
- 添加用户头像上传
- 添加暗黑模式

## 🐛 修复
- 修复登录超时问题
- 修复文件上传大小限制

## 📦 下载
- Windows: yunji-v1.3.0.exe
- macOS: yunji-v1.3.0.dmg
- Linux: yunji-v1.3.0.AppImage
```

### 6.5 发布流程

```
用户输入 "/release" 或 AI 检测候选版本
  ↓
AI 汇总自上次 release 的所有快照
  ↓
AI 建议版本号 (semver) + 生成 changelog
  ↓
用户确认（或自动确认）
  ↓
并行构建多目标 (Windows/macOS/Linux/Web)
  ↓
并行分发多平台 (GitHub/Gitee/网盘/官网)
  ↓
生成 Release Notes + 更新版本号
  ↓
通知用户 + 更新时间轴标记
```

---

## 7. 三种运行模式

### 7.1 隐身模式（默认，企业最爱）

```
纯本地 TimeFlow，不碰 git，代码永不上云
  - 自动快照存本地
  - AI 用本地模型（bge-small + Ollama）
  - 适合: 政府/金融/隐私敏感企业
```

### 7.2 同步模式（开发者常用）

```
TimeFlow 自动镜像到 git 仓库
  - 每个快照 → git commit（AI 生成 msg）
  - 定时 push 到 GitHub/Gitee
  - 适合: 开源项目/个人开发者
```

### 7.3 发布模式（团队协作）

```
只把"正式版本"推到 git，WIP 留本地
  - 候选版本 → 提示用户
  - 正式版本 → 自动 tag + push + release
  - 适合: 商业软件/团队开发
```

**模式可随时切换，数据互通。**

---

## 8. 与 AgentWork 其他模块的关系

```
AgentWork
├── TimeFlow (本模块)          ← 版本控制 + 发布
│   ├── timeflow-core (Rust)   ← 存储引擎
│   ├── timeflow-ai (Rust)     ← AI 语义层
│   ├── timeflow-workflow      ← 工作流引擎
│   └── timeflow-release       ← 发布引擎
├── Agent Runtime (OpenHands)  ← AI 写代码
├── MCP Bridge                 ← 工具协议
├── Skills 市场                ← 可复用任务
└── Docker 沙箱                ← 隔离执行
```

**协作关系**：
- Agent Runtime 写代码 → TimeFlow 自动快照
- TimeFlow 检测候选版本 → 触发 Agent Runtime 跑测试
- TimeFlow 发布 → 调用 Skills 市场的"发布 Skill"

---

## 9. 开源方案参考

| 项目 | 语言 | 借鉴点 |
|------|------|--------|
| **restic** | Go | 快照+去重+加密的存储模型 |
| **Jujutsu (jj)** | Rust | 自动 commit 每次工作区变化的设计 |
| **kopia** | Go | GUI + 快照管理的 UX |
| **libgit2** | C | Git 兼容层（git2-rs 绑定） |
| **notify** | Rust | 跨平台文件监控 |
| **n8n** | TS | 工作流 DAG 执行模型 |
| **GitHub Actions** | YAML | 触发器语法设计 |

**不直接依赖任何单一项目，借鉴思想自研轻量实现。**

---

## 10. 实现优先级

| 优先级 | 模块 | 里程碑 | 周期 |
|:---:|------|:---:|:---:|
| P0 | 存储引擎核心（快照/回滚/分支） | M1 | 4 周 |
| P0 | 时间轴 UI | M1 | 2 周 |
| P1 | Git 双模兼容（libgit2） | M2 | 2 周 |
| P1 | AI commit msg 生成 | M2 | 1 周 |
| P1 | AI 变更摘要 + 语义搜索 | M3 | 2 周 |
| P2 | 工作流引擎（触发词+DAG） | M4 | 2 周 |
| P2 | 多目标构建矩阵 | M5 | 2 周 |
| P2 | 多平台分发（GitHub/Gitee/网盘） | M5 | 2 周 |
| P3 | 可运行版本标记 | M5 | 1 周 |
| P3 | Release Notes 自动生成 | M5 | 1 周 |

---

## 11. 风险与对策

| 风险 | 等级 | 对策 |
|------|:---:|------|
| 自研 VCS 数据损坏 | 🔴 高 | 内容寻址 + 校验和 + 自动备份 |
| libgit2 兼容层复杂度 | 🟡 中 | 先做单向导出，双向同步后置 |
| AI commit msg 质量不稳定 | 🟡 中 | 模板兜底 + 用户可编辑 |
| 多平台分发 API 变更 | 🟡 中 | 适配器模式 + 独立版本锁定 |
| 网盘 API 限流 | 🟡 中 | 重试 + 队列 + 多账号轮换 |
| 构建矩阵环境差异 | 🟡 中 | Docker 统一构建环境 |

---

## 12. 变更历史

| 日期 | 变更 | 作者 |
|------|------|------|
| 2026-06-17 | 初始设计文档 v1.0 | Trae |

---

> **TimeFlow 是 AgentWork 的灵魂**：让版本控制从"手动操作"变成"自动智能"，让发布从"繁琐流程"变成"一句话触发"。
