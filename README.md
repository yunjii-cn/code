# 云集智能 (Yunji)

> **AI 原生编程工作空间** | 3 代产品线 / 1 个共享层 / 企业服务另议

---

## 3 代产品线矩阵

| 代 | 目录 | 产品 | 档位 | 价格 | 一句话 |
|:---:|:---|------|:---:|:---:|------|
| 1️⃣ | `1.PC/` | **云集桌面** | Free | **¥0** | 永久免费 / 离线 / Ollama |
| 2️⃣ | `2.WEB/` | **云集 Web** | Pro | **¥9.9/月** | 跨端 / 同步 / 新人送 ¥30 |
| 3️⃣ | `3.dev/` | **云集旗舰** | Business | **¥99/月** | 全功能 / 团队 5 角色 / 协作 |
| — | `3.dev/enterprise-services/` | **企业服务** | — | 议价 | 私有化 / 定制 / 培训 |

> **企业服务不绑产品**——产品 1 / 2 / 3 都可加购私有化、定制、培训、咨询。

---

## 🚀 快速开始

### 1️⃣ 双击 `启动.bat`（推荐）

```
云集智能/
├── 启动.bat                  ← ⭐ 总入口（菜单式）
├── 启动-云集桌面.bat         ← Free 桌面版
├── 启动-云集Web.bat          ← Pro 跨端版
├── 启动-云集旗舰.bat         ← Business 旗舰版
└── 一键环境部署.bat          ← 安装 Python / uv / Node / Bun
```

### 2️⃣ 手动启动

```powershell
# 产品 1：云集桌面（PyQt6）
cd 1.PC\app
uv\uv.exe run main.py

# 产品 2：云集 Web（FastAPI + Vue）
cd 2.WEB\api
uv run api_main.py --port 18080
# 另一个终端：
cd 2.WEB\ui
npm run dev

# 产品 3：云集旗舰（含 Platformkit 全模块）
cd 3.dev\api
uv run api_main.py --port 18099 --flagship
# 另一个终端：
cd 3.dev\ui
npm run dev
```

---

## 📂 目录结构

```
云集智能/                            <-- 项目根
├── 1.PC/                    <-- 🟢 第 1 代：云集桌面（Free）
│   ├── app/                 PyQt6 启动器
│   ├── build/               PyInstaller
│   ├── data/  _internal/  dist/  ver/  temp/
│   ├── AGENTS.md  tier.yaml  README.md
│
├── 2.WEB/                   <-- 🟡 第 2 代：云集 Web（Pro）
│   ├── api/                 FastAPI 后端
│   ├── ui/                  Vue 3 前端
│   ├── build/  doc/
│   ├── tier.yaml  README.md
│
├── 3.dev/                   <-- 🔵 第 3 代：云集旗舰（Business）—— **全力开发**
│   ├── platformkit/         共享层（12+ 核心模块）
│   ├── api/                 FastAPI 后端（最全）
│   ├── ui/                  Vue 3 前端（最全）
│   ├── enterprise-services/ 私有化 / 授权 / 支持
│   ├── data/  temp/  ver/  .github/  tests/
│   ├── tier.yaml  README.md
│
├── BAK/                     <-- 历史归档
│
├── doc/                     <-- 根级共享文档
│   ├── 产品线矩阵.md
│   ├── 全面超越-参考CodexMonitor改造计划.md
│   ├── Web化跨平台改造规划.md
│   └── 智能编程工作站产品规划.md
│
├── 启动.bat                 <-- ⭐ 总入口
├── 启动-云集桌面.bat
├── 启动-云集Web.bat
├── 启动-云集旗舰.bat
├── 一键环境部署.bat
├── AGENTS.md                <-- 3 代架构契约
└── README.md                <-- 本文件
```

---

## 💰 定价对照

| 档位 | 云集 | Cursor | GitHub Copilot | 云集优势 |
|------|------|--------|---------------|----------|
| 免费 | **¥0** | $0 | $0 | 永久免费 + Ollama |
| Pro | **¥9.9** | $20/月 | $10/月 | 比 Cursor 便宜 **93%** |
| Business | **¥99** | $40/月 | $19/月 | 比 Cursor 便宜 **66%** |
| 企业 | **¥199** | 议价 | $39/月 | 比 Cursor 便宜 **40%+** |

**云集独家**：
- 🟢 唯一**永久免费** + **Ollama 原生**（Cursor / Copilot 都没有）
- 🟡 唯一**¥9.9 起**带云端 AI
- 🔵 唯一**内置团队 5 角色 + 知识演化**

---

## 🎯 3 个产品的关系

```
云集智能（Yunji）              ← 品牌母体
       │
 ┌─────┼─────────┐
 │     │         │
1.PC   2.WEB     3.dev
 Free   Pro      Business
 PyQt6  Vue+API  Vue+API+协作
 │      │        │
 │      │     platformkit/   ← 共享层（**只在 3.dev/**）
 │      │     知识/感知/...
 │      │        │
 │      └────────┘            ← 2.WEB 从 3.dev 借鉴补完
 │                │
 └──── 1.PC 独立 ─┘            ← 1.PC 完全独立
```

---

## 📖 文档索引

| 文档 | 说明 |
|------|------|
| [doc/产品线矩阵.md](doc/产品线矩阵.md) | **3 产品线完整定义 + 定价 + 商业模式** |
| [doc/全面超越-参考CodexMonitor改造计划.md](doc/全面超越-参考CodexMonitor改造计划.md) | 35 TASK 路线图（已完成） |
| [doc/Web化跨平台改造规划.md](doc/Web化跨平台改造规划.md) | Web 化 + 跨端技术决策 |
| [doc/智能编程工作站产品规划.md](doc/智能编程工作站产品规划.md) | 原始产品愿景 |
| [1.PC/README.md](1.PC/README.md) | 云集桌面 - 永久免费 |
| [2.WEB/README.md](2.WEB/README.md) | 云集 Web - ¥9.9/月 |
| [3.dev/README.md](3.dev/README.md) | 云集旗舰 - ¥99/月 |
| [3.dev/enterprise-services/README.md](3.dev/enterprise-services/README.md) | 企业服务 |

---

## 🛠️ 技术栈总览

| 层 | 技术 |
|----|------|
| 桌面壳（产品 1） | PyQt6 + QWebEngineView |
| 桌面壳（产品 2） | pywebview（系统 WebView2） |
| 移动壳 | Capacitor（Android / iOS） |
| 后端 | FastAPI + uvicorn |
| 前端 | Vue 3 + Vite + TypeScript + Vant |
| 实时协作 | Yjs + WebSocket |
| AI 引擎 | 多供应商（Anthropic / OpenRouter / Ollama / 智谱） |
| 远程 | Tailscale + LAN Token |
| 打包 | PyInstaller（桌面）+ Vite build（Web） |
| 私有部署 | Docker + 内部 K8s |
| CI | GitHub Actions |

---

> **架构原则**：3 代产品分离，**1.PC 独立**（PyQt6 自包含），**2.WEB 借鉴 3.dev**（共享思想不强制同步），**3.dev 全力开发**（新功能先在这里落地）。
