# 云集智能 - AGENTS.md（3 代架构契约）

> **目的**：所有 AI 协作（Trae / Cursor / Claude Code / 任意 agent）必须遵守的项目契约。
> **生效日期**：2026-06-10
> **取代**：旧 `dev/AGENTS.md`（路径已变更）

---

## 0. 3 代产品线契约

本项目是 **云集智能**（Yunji）—— 一家做 **AI 原生编程工作空间** 的公司，旗下 3 个产品线：

| 代 | 目录 | 产品 | 档位 | 价格 | 状态 |
|:---:|:---|------|:---:|:---:|------|
| 1 | `1.PC/` | 云集桌面 | Free | ¥0 | ✅ 完整可跑 |
| 2 | `2.WEB/` | 云集 Web | Pro | ¥9.9/月 | 🟡 半成品 |
| 3 | `3.dev/` | 云集旗舰 | Business | ¥99/月 | 🟢 全力开发 |

**所有 agent 必须**：

1. **明确自己工作在哪个代**（1.PC / 2.WEB / 3.dev / 跨代）
2. **明确自己工作在哪个层**（platformkit / api / ui / enterprise-services）
3. **新功能先在 3.dev 落地**，再考虑是否同步到 2.WEB
4. **1.PC 是独立产品**，不动 platformkit / 不依赖 3.dev
5. **企业服务**（私有化 / 授权 / 培训）独立议价

---

## 1. 路径契约（2026-06-10 重写）

> **旧路径已废弃**：`dev/app/`、`dev/dev/app/`、`dev/dev/web/`、根目录 `*.bat` 散落

### 1.1 项目根

```
云集智能/                          <-- 项目根
├── 1.PC/                          第 1 代：云集桌面
├── 2.WEB/                         第 2 代：云集 Web
├── 3.dev/                         第 3 代：云集旗舰
├── BAK/                           历史归档（保留参照）
├── doc/                           根级共享文档
├── 启动.bat                       ⭐ 总入口
├── 启动-云集桌面.bat
├── 启动-云集Web.bat
├── 启动-云集旗舰.bat
├── 一键环境部署.bat
└── AGENTS.md                      本文件
```

### 1.2 1.PC/ 结构

```
1.PC/
├── app/                <-- PyQt6 启动器（**代码与原 dev/app/ 1:1 一致**）
│   ├── main.py
│   ├── backend.py
│   ├── services/
│   ├── api/            qwen2api / zhipu2api 子应用
│   ├── platformkit/    桌面版平台层（与 3.dev 独立）
│   ├── routes/
│   ├── scripts/        install-env.ps1
│   ├── uv/             便携版 uv
│   ├── python/         便携版 Python
│   ├── nodejs/         便携版 Node.js
│   ├── bun/            便携版 Bun
│   └── icon.ico / icon.png
├── build/              PyInstaller 脚本
├── data/  _internal/  dist/  ver/  temp/
└── AGENTS.md  tier.yaml  README.md
```

**重要**：1.PC 内部结构**与原 dev/app/ 1:1 一致**，**所有 import 路径不需修改**。

### 1.3 2.WEB/ 结构

```
2.WEB/
├── api/                FastAPI 后端（**结构照搬 3.dev/api/**）
│   ├── api_main.py
│   ├── backend.py
│   ├── services/
│   ├── routes/         17 个 API（与 3.dev 同步）
│   └── tests/
├── ui/                 Vue 3 + Vite 前端（**结构照搬 3.dev/ui/**）
│   ├── src/views/  src/components/  src/stores/  src/platform/
│   └── package.json
├── build/
└── tier.yaml  README.md
```

### 1.4 3.dev/ 结构

```
3.dev/
├── platformkit/        <-- 共享层（**3 代技术精华，最大最全**）
│   ├── shared/         12+ 核心模块
│   │   ├── ai_engine.py
│   │   ├── knowledge_core.py
│   │   ├── responsive_core.py
│   │   ├── team_core.py
│   │   ├── yjs_server.py
│   │   ├── auth_core.py
│   │   ├── durable_store.py
│   │   ├── model_router.py
│   │   ├── github_core.py
│   │   ├── whisper_core.py
│   │   ├── skill_market.py
│   │   ├── tailscale_core.py
│   │   ├── billing_core.py          🆕
│   │   ├── tenant_core.py           🆕
│   │   └── audit_core.py            🆕
│   └── daemon/         独立 daemon 进程
├── api/                FastAPI 后端（**最全**）
│   ├── api_main.py
│   ├── backend.py
│   ├── services/
│   ├── routes/         全部 17 个 API
│   └── tests/          250+ pytest
├── ui/                 Vue 3 + Vite 前端（**最全**）
│   ├── src/features/   15 features
│   ├── src/shared/     设计系统 5 组件
│   └── src/views/
├── enterprise-services/  <-- 🟣 企业服务
│   ├── deployment/     私有化部署
│   ├── licensing/      商业授权
│   └── support/        SLA / 培训
├── .github/workflows/  CI
└── tier.yaml  README.md
```

### 1.5 文档

```
doc/                    根级共享文档
├── 产品线矩阵.md       <-- 商业 + 技术双轨入口
├── 全面超越-参考CodexMonitor改造计划.md
├── Web化跨平台改造规划.md
├── 智能编程工作站产品规划.md
└── 云集智能平台产品蓝图.md
```

---

## 2. 工作流契约

### 2.1 修改 1.PC（PyQt6）

- ✅ **不动** platformkit（与 3.dev 独立）
- ✅ **不动** routes 之外的跨代引用
- ⚠️ PyQt6 + QWebEngineView 自带 Chromium（~200MB）
- ⚠️ `dev/` 旧路径已废弃

### 2.2 修改 2.WEB

- ⚠️ 与 3.dev 共享思想，**代码独立维护**
- ✅ 从 3.dev 借鉴时，**复制而非引用**（避免强耦合）
- ✅ 简化版优先（去掉复杂的企业特性）

### 2.3 修改 3.dev

- ✅ 主力开发目录
- ✅ 新功能先在 3.dev 落地
- ✅ Platformkit 共享层变动需同步检查 2.WEB

### 2.4 修改企业服务

- 🟣 `3.dev/enterprise-services/` 独立维护
- 💰 价格单独议价，不进 tier.yaml
- 📞 涉及金额时回到产品线矩阵

---

## 3. 构建契约

### 3.1 打包命令

| 产品 | 命令 | 输出位置 |
|------|------|----------|
| 1.PC | `cd 1.PC\app && uv\uv.exe run main.py` | 开发模式 |
| 1.PC EXE | `cd 1.PC\build && python build_pc.py` | `1.PC/dist/云集智能编程工作站-v*.exe` (单EXE自部署) |
| 2.WEB | `cd 2.WEB\api && uv run api_main.py` | 开发模式 |
| 2.WEB EXE | `cd 2.WEB\build && python build.py` | `2.WEB/dist/云集Web-v*/` |
| 3.dev | `cd 3.dev\api && uv run api_main.py --flagship` | 开发模式 |
| 3.dev EXE | `cd 3.dev\build && python build.py` | `3.dev/dist/云集旗舰-v*/` |

### 3.2 EXE 输出位置

- 1.PC EXE → `1.PC/dist/云集智能编程工作站-v2026.06.10.HHMM.exe` (单EXE自部署)
- 2.WEB EXE → `2.WEB/dist/云集Web-v2026.06.10.HHMM/`
- 3.dev EXE → `3.dev/dist/云集旗舰-v2026.06.10.HHMM/`
- 统一归档到根 `build_artifacts/`（未来）

### 3.3 打包模式

- **1.PC 使用 `--onefile`**（单 EXE 自部署，首次运行自动展开目录结构，无 `_internal`）
- **2.WEB / 3.dev 使用 `--onedir`**（分散输出）
- PyInstaller + Vite 双轨

---

## 4. 测试契约

| 产品 | 测试命令 |
|------|----------|
| 1.PC | `cd 1.PC\app && pytest tests/` |
| 2.WEB | `cd 2.WEB\api && pytest tests/` |
| 3.dev | `cd 3.dev\api && pytest tests/` |
| 3.dev UI | `cd 3.dev\ui && npm test` |

---

## 5. Git 契约

- **提交**：`1.PC/app/`、`2.WEB/api/`、`2.WEB/ui/`、`3.dev/api/`、`3.dev/ui/`、`3.dev/platformkit/`、`doc/`
- **不提交**：EXE、`_internal/`、`build/`、`dist/`、`node_modules/`、`.pytest_cache/`、`__pycache__/`

---

## 6. 变更历史

| 日期 | 变更 | 负责人 |
|------|------|--------|
| 2026-06-10 | 3 代产品线从根目录分离：`dev/app/` → `1.PC/app/`、`dev/dev/` → `2.WEB/` + `3.dev/`、根 `AGENTS.md` 重写 | Trae |
| 2026-06-10 | 删除 Electron / Ink UI / Claude Code 残留 | Trae |
| 2026-06-10 | 新建 `启动.bat` / `启动-云集桌面.bat` / `启动-云集Web.bat` / `启动-云集旗舰.bat` / `一键环境部署.bat` | Trae |
| 2026-06-10 | 新建 3 个产品 README + tier.yaml | Trae |
| 2026-06-13 | 1.PC 打包模式从 `--onedir` 改为 `--onefile` 自部署，新增 `launcher.py` 入口，`build_pc.py` 重写 | Qoder |

---

> **最后**：本文件是项目宪法。所有 agent 修改前请**先读**本文件，**再行动**。
