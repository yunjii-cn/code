# 云集智能 - AGENTS.md（4 代架构契约）

> 目的：所有 AI 协作必须遵守的项目契约。  
> 生效日期：2026-06-24  
> 取代：2026-06-10 的 3 代架构契约。

## 0. 产品线契约

本仓库是 **云集智能**（Yunji）的产品矩阵仓库，当前包含 4 条产品线和 2 个官网站点：

| 代 | 目录 | 产品 | 档位 | 状态 |
|:---:|:---|:---|:---:|:---|
| 1 | `1.PC/` | 云集 PC 版 | Free | 完整可跑 |
| 2 | `2.WEB/` | 云集 Web 版 | Pro | 半成品 |
| 3 | `3.dev/`（迁移目标：`3.Team/`） | 云集团队版 | Business | 继续维护 |
| 4 | `4.AgentWork/` | 云集智能体工作台 / AgentWork | Enterprise | 新主力 |
| - | `work.yunjii.cn/` | 云集工作台门户 | - | 待开发 |
| - | `aw.yunjii.cn/` | AgentWork 官网 | - | 待开发 |

所有 agent 必须：

1. 明确自己工作在哪个范围：`1.PC` / `2.WEB` / `3.dev` / `4.AgentWork` / `work.yunjii.cn` / `aw.yunjii.cn` / 跨产品。
2. 明确自己工作在哪个层：产品、API、UI、平台层、企业服务、文档、官网。
3. 新的 AI 员工、AI 团队、行业模板、自进化能力优先在 `4.AgentWork/` 落地。
4. `1.PC/` 是独立产品，不依赖 `3.dev/` 或 `4.AgentWork/`。
5. `2.WEB/` 和 `3.dev/` 代码独立维护，可以复制思想，不能形成隐式强耦合。
6. 根级 `doc/` 只放全局文档；产品专属文档必须放回对应产品目录。

## 1. 路径契约

```
云集智能编程工作站/
├── 1.PC/                 第 1 代：云集 PC 版
├── 2.WEB/                第 2 代：云集 Web 版
├── 3.dev/                第 3 代：云集团队版（迁移目标：3.Team/）
├── 4.AgentWork/          第 4 代：云集智能体工作台 / AgentWork
├── work.yunjii.cn/       云集工作台门户站点
├── aw.yunjii.cn/         AgentWork 旗舰产品站
├── doc/                  根级全局文档
├── BAK/                  历史归档
├── AGENTS.md             本文件
└── README.md             项目入口
```

旧路径 `dev/app/`、`dev/dev/app/`、`dev/dev/web/` 已废弃。旧文档若仍引用这些路径，应标注为历史参考，不能作为当前实现依据。

## 2. 文档契约

文档归属规则见 [doc/文档治理规范.md](doc/文档治理规范.md)。产品命名和品牌表达见 [doc/品牌与命名规范.md](doc/品牌与命名规范.md)。

- 根级 `doc/`：产品矩阵、品牌、商业模式、全局路线、站点计划、跨产品架构。
- `1.PC/doc/`：PyQt6、本地桌面、打包、进程控制、单实例、便携环境。
- `2.WEB/doc/`：Web 化、多端、PWA、轻量 Pro 版。
- `3.dev/doc/`：云集团队版、Platformkit、团队协作、旧 3 代规范。
- `4.AgentWork/docs/`：AgentWork 架构、AI 员工、AI 团队、行业模板、Tauri/Rust。
- `BAK/`：无维护价值或低频查阅的历史计划、原始对话、临时旧文件。

## 3. 产品工作流

### 3.1 修改 `1.PC/`

- 保持独立可运行。
- 不引入 `3.dev/` 或 `4.AgentWork/` 依赖。
- PyQt6、QWebEngineView、便携 Python/uv/Node/Bun 相关文档放 `1.PC/doc/`。

### 3.2 修改 `2.WEB/`

- 面向低价、轻量、多端使用场景。
- 从 `3.dev/` 借鉴时复制设计，不直接引用代码。
- 去掉复杂企业能力，优先保持简单可用。

### 3.3 修改 `3.dev/`

- 面向 Business 档中小企业团队。
- 维护 Python/FastAPI/Vue/Platformkit 路线。
- Platformkit 变动要检查 `2.WEB/` 是否需要同步思想或能力。

### 3.4 修改 `4.AgentWork/`

- 面向 Enterprise 档和 2026 下半年主力方向。
- 独立技术栈：Tauri 2 + Rust + React。
- 不依赖 `1.PC/2.WEB/3.dev` 的运行时代码。
- 必须优先考虑 AI 员工、行业模板、任务驱动 + 互动机制双引擎。

### 3.5 修改官网站点

- `work.yunjii.cn/` 是矩阵门户和统一身份入口，不承载具体产品重业务。
- `aw.yunjii.cn/` 是 AgentWork 产品站，不自建独立用户数据库。
- 两个站点可以在当前仓库孵化，但目录、构建和部署从第一天保持可拆分。

## 4. 构建与测试

| 范围 | 命令 |
|---|---|
| 1.PC 开发 | `cd 1.PC\app && uv\uv.exe run main.py` |
| 1.PC 测试 | `cd 1.PC\app && pytest tests/` |
| 2.WEB API | `cd 2.WEB\api && uv run api_main.py` |
| 2.WEB 测试 | `cd 2.WEB\api && pytest tests/` |
| 3.dev API | `cd 3.dev\api && uv run api_main.py --flagship` |
| 3.dev API 测试 | `cd 3.dev\api && pytest tests/` |
| 3.dev UI 测试 | `cd 3.dev\ui && npm test` |
| 4.AgentWork 桌面 | `cd 4.AgentWork\apps\desktop && pnpm tauri dev` |
| 4.AgentWork Rust | `cd 4.AgentWork && cargo test` |
| 4.AgentWork 前端 | `cd 4.AgentWork && pnpm test` |

## 5. Git 契约

可提交：

- `1.PC/app/`、`1.PC/doc/`
- `2.WEB/api/`、`2.WEB/ui/`、`2.WEB/doc/`
- `3.dev/api/`、`3.dev/ui/`、`3.dev/platformkit/`、`3.dev/doc/`
- `4.AgentWork/`
- `work.yunjii.cn/`、`aw.yunjii.cn/`
- `doc/`

不提交：

- `dist/`、`build_artifacts/`、`node_modules/`
- `.pytest_cache/`、`__pycache__/`
- 临时文件、下载缓存、生成 EXE、运行时数据

## 6. 变更历史

| 日期 | 变更 |
|---|---|
| 2026-06-10 | 根目录拆分为 1.PC / 2.WEB / 3.dev 三代产品线 |
| 2026-06-17 | 新增 4.AgentWork 第 4 代产品线 |
| 2026-06-24 | 根级契约升级为 4 代 + 双站点；文档按全局/产品/归档重新分层 |
