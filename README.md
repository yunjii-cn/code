# 云集智能 (Yunji)

> AI 原生工作空间产品矩阵。当前仓库同时孵化 4 条产品线、2 个官网站点和共享文档体系。

## 产品线

| 代 | 目录 | 产品 | 档位 | 当前定位 |
|:---:|:---|:---|:---:|:---|
| 1 | `1.PC/` | 云集 PC 版 | Free | 永久免费、本地优先、Ollama 原生 |
| 2 | `2.WEB/` | 云集 Web 版 | Pro | 低价多端轻量版 |
| 3 | `3.dev/`（迁移目标：`3.Team/`） | 云集团队版 | Business | 中小企业团队协作、安全与多端平台 |
| 4 | `4.AgentWork/` | 云集智能体工作台 / AgentWork | Enterprise | AI 员工、AI 团队、行业模板、自进化工作台 |

关键边界：

- `1.PC/` 是独立 PyQt6 桌面产品，不依赖 3.dev/4.AgentWork。
- `2.WEB/` 与 `3.dev/` 可以共享产品思想，但代码独立维护。
- `3.dev/` 是 Business 档团队版，保留 Python/FastAPI/Vue/Platformkit 路线，后续目录迁移目标为 `3.Team/`。
- `4.AgentWork/` 是新一代主力产品，Tauri 2 + Rust + React 独立技术栈。
- 新的行业化、AI 员工、AI 团队、自进化能力优先在 `4.AgentWork/` 落地。

## 官网站点

| 域名 | 本地目录 | 定位 |
|---|---|---|
| `work.yunjii.cn` | `work.yunjii.cn/` | 云集工作台矩阵门户、统一身份入口、生态路由中枢 |
| `aw.yunjii.cn` | `aw.yunjii.cn/` | AgentWork 旗舰产品站，负责 AW 专属叙事与企业转化 |

站点开发计划见 [doc/双站点开发计划.md](doc/双站点开发计划.md)。

## 文档入口

根目录 `doc/` 只放跨产品线、品牌、商业、站点和全局架构文档。单个产品的实现、打包、技术细节放到各自产品目录。

| 文档 | 说明 |
|---|---|
| [doc/README.md](doc/README.md) | 全局文档索引 |
| [doc/品牌与命名规范.md](doc/品牌与命名规范.md) | 产品线命名、站点命名和 AgentWork Logo 方向 |
| [doc/文档治理规范.md](doc/文档治理规范.md) | 文档分类、命名和迁移规则 |
| [doc/产品线矩阵.md](doc/产品线矩阵.md) | 4 代产品线、定价、差异和路线 |
| [doc/云集智能平台产品蓝图.md](doc/云集智能平台产品蓝图.md) | 平台级愿景 |
| [doc/产品宣发方案.md](doc/产品宣发方案.md) | 宣发素材与对外表达参考 |
| [1.PC/doc/README.md](1.PC/doc/README.md) | 云集 PC 版文档索引 |
| [2.WEB/doc/README.md](2.WEB/doc/README.md) | 云集 Web 版文档索引 |
| [3.dev/doc/README.md](3.dev/doc/README.md) | 云集团队版文档索引 |
| [4.AgentWork/docs/README.md](4.AgentWork/docs/README.md) | AgentWork 文档索引 |

## 快速启动

```powershell
# 第 1 代：云集 PC 版
cd 1.PC\app
uv\uv.exe run main.py

# 第 2 代：云集 Web 版
cd 2.WEB\api
uv run api_main.py

# 第 3 代：云集团队版
cd 3.dev\api
uv run api_main.py --flagship

# 第 4 代：AgentWork
cd 4.AgentWork
pnpm install
cd apps\desktop
pnpm tauri dev
```

也可以使用根目录 `启动.bat` 进入菜单式启动。

## 工作约束

所有协作者修改前先读 [AGENTS.md](AGENTS.md)。如果任务涉及某个产品，还要读对应产品目录下的 `AGENTS.md` 或 README。
