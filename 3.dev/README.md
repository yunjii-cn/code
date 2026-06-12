# 云集旗舰 (Yunji Flagship)

> **代际**：第 3 代 | **档位**：Business | **价格**：**¥99 / 月**（企业打包 **¥199**）
> **状态**：🟢 全力开发

## 一句话

**99 元/月，全功能的 AI 编程团队工作空间**。**团队协作 / 知识演化 / 主动感知 / 多端跨平台**全开，面向商业团队。

## 核心卖点

- 🔵 **¥99 全功能**（对标 Cursor Business ¥290/月便宜 **66%**）
- 🔵 **¥199 企业打包**（中小企业首选，含私有化咨询 + 优先支持）
- 🔵 **5 角色团队模式**（主管 / 架构 / 开发 / 测试 / 文档，**边界检查 + 交叉审查**）
- 🔵 **Yjs 实时协作**（多用户同时改文件，CRDT 不丢数据）
- 🔵 **4 层知识引擎**（纠正 / 模式 / 事实 / 偏好 + **强度演化**）
- 🔵 **4 类主动感知**（文件变更 / 代码质量 / 依赖漏洞 / 未完成任务）
- 🔵 **Daemon + Tailscale**（远程编码，手机写代码）
- 🔵 **SSO + 审计 + 多租户**（企业合规）
- 🔵 **私有化部署可选**（数据不出域）

## 技术栈

**= 产品 2 全部 + 以下增量：**

- **Platformkit 共享层**（**3 代技术精华，最大最全**）
  - 12+ 核心模块
- **Yjs CRDT** 实时协作
- **Daemon** 独立进程
- **Tailscale** 远程访问
- **SSO + 审计**（企业合规）
- **多租户**（私有部署）

## 目录结构

```
3.dev/                  <-- 🔵 第 3 代：云集旗舰（Business）—— 全力开发
├── platformkit/        <-- 共享层（最大最全，3 代技术精华）
│   ├── shared/         12+ 核心模块
│   │   ├── knowledge_core.py       4 层知识 + 强度演化
│   │   ├── responsive_core.py      4 类主动感知
│   │   ├── ai_engine.py            多供应商 AI
│   │   ├── team_core.py            5 角色团队
│   │   ├── yjs_server.py           实时协作
│   │   ├── auth_core.py            JWT 鉴权
│   │   ├── durable_store.py        .yunji/ 持久化
│   │   ├── model_router.py         智能模型路由
│   │   ├── github_core.py          GitHub 集成
│   │   ├── whisper_core.py         听写
│   │   ├── skill_market.py         技能市场
│   │   └── tailscale_core.py       Tailscale 远程
│   ├── daemon/         独立 daemon 进程
│   │   ├── daemon_core.py
│   │   └── daemon_main.py
│   └── README.md
├── api/                FastAPI 后端（**最全**）
│   ├── api_main.py
│   ├── backend.py
│   ├── services/
│   ├── routes/         全部 17 个 API
│   └── tests/          250+ pytest
├── ui/                 Vue 3 + Vite 前端（**最全**）
│   ├── src/
│   │   ├── features/   15 features
│   │   ├── shared/     设计系统 5 组件
│   │   └── views/
│   └── package.json
├── enterprise-services/   <-- 🟣 企业服务（不卖产品，卖服务）
│   ├── deployment/     私有化部署
│   ├── licensing/      商业授权
│   └── support/        SLA / 培训
├── .github/workflows/  CI
├── tests/
├── tier.yaml           "Business / Enterprise"功能矩阵
└── README.md           本文件
```

## 快速开始

### 双击启动

双击 `启动.bat` → 选 `3` → 启动云集旗舰

### 手动启动

```powershell
# 终端 1：后端（含 Platformkit 全部模块）
cd 3.dev\api
uv run api_main.py --port 18099 --flagship

# 终端 2：前端（含协作 + 5 角色 + 知识界面）
cd 3.dev\ui
npm install   # 首次
npm run dev
```

访问 <http://127.0.0.1:5173>

## Platformkit 核心模块

| 模块 | 状态 | 描述 |
|------|------|------|
| `ai_engine` | ✅ | 多供应商 AI（Anthropic / OpenRouter / Ollama / 智谱） |
| `knowledge_core` | ✅ | 4 层知识 + 强度演化 |
| `responsive_core` | ✅ | 4 类主动感知 |
| `durable_store` | ✅ | `.yunji/` 工程化持久化 |
| `team_core` | ✅ | 5 角色团队 + 边界检查 |
| `yjs_server` | ✅ | Yjs 实时协作 |
| `auth_core` | ✅ | JWT 鉴权 + 角色权限 |
| `daemon_core` | ✅ | Daemon 独立进程 |
| `tailscale_core` | ✅ | Tailscale 远程访问 |
| `github_core` | ✅ | gh CLI 深度集成 |
| `whisper_core` | ✅ | Whisper 听写 |
| `skill_market` | ✅ | 技能市场 |
| `model_router` | ✅ | 智能模型路由 |
| `billing_core` | 🆕 | 订阅 / 配额 / 支付（**产品 2/3 必需**） |
| `tenant_core` | 🆕 | 多租户 / SSO（**产品 3 必需**） |
| `audit_core` | 🆕 | 审计日志（**产品 3 必需**） |

## 商业模式

| 维度 | 内容 |
|------|------|
| **价格** | ¥99 / 月 / 用户（对标 GitHub Copilot Business $19/月） |
| **企业打包** | ¥199 / 月 / 用户（含 5 人起 + 私有化咨询 + 优先支持） |
| **额度** | ¥150 等值算力 / 用户 / 月 |
| **企业定制** | 单独议价（私有化部署 ¥20 万起 / 定制开发 ¥3 万 / 人月） |
| **变现** | 订阅 + 算力 + 企业服务 + 私有化部署 |

## 引用

- 产品线矩阵：[../doc/产品线矩阵.md](../doc/产品线矩阵.md)
- 企业服务：[enterprise-services/README.md](enterprise-services/README.md)
- 全面超越计划：[../doc/全面超越-参考CodexMonitor改造计划.md](../doc/全面超越-参考CodexMonitor改造计划.md)
