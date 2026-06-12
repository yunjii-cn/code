---
name: "yunji-dev-guide"
description: "云集智能 3 代产品线（1.PC / 2.WEB / 3.dev）项目开发规范。Invoke when working on this project: coding, building EXE, modifying UI, fixing bugs, or any development task across all 3 generations."
---

# 云集智能 - 3 代产品线开发规范

> **取代** 旧 "云集智能文件清理专家 + CustomTkinter" 规范
> **生效日期**：2026-06-10
> **完整路径契约**：项目根 `AGENTS.md`

## 项目核心信息

- **品牌**：云集智能 (Yunji)
- **3 代产品线**：
  - `1.PC/` 云集桌面（Free / PyQt6）✅ 完整
  - `2.WEB/` 云集 Web（Pro / FastAPI+Vue 3）🟡 半成品
  - `3.dev/` 云集旗舰（Business / 全 Platformkit）🟢 全力开发
- **打包工具**：
  - 产品 1：PyInstaller `--onedir`（**禁止** `--onefile`，Conda DLL 会失败）
  - 产品 2/3：PyInstaller `--onefile` + Vite `build`
- **EXE 输出**：
  - 产品 1 → `1.PC/dist/云集桌面-vYYYY.MM.DD.HHMM/`
  - 产品 2 → `2.WEB/dist/云集Web-vYYYY.MM.DD.HHMM/`
  - 产品 3 → `3.dev/dist/云集旗舰-vYYYY.MM.DD.HHMM/`

## 3 代产品线核心规则

### 规则 1：明确工作在哪个代

- 在 `1.PC/` → PyQt6 独立栈，**不动** platformkit
- 在 `2.WEB/` → 简化版，**借鉴 3.dev** 但不强同步
- 在 `3.dev/` → 主力开发，**新功能先在这里**
- 跨代 → 先在 3.dev 落地，2.WEB 后续同步

### 规则 2：路径契约（**绝对路径**，2026-06-10 生效）

**旧路径已废弃**：`dev/app/`、`dev/dev/app/`、`dev/dev/web/`、根目录散落 `*.bat`

```
云集智能/
├── 1.PC/
│   ├── app/             PyQt6 启动器（main.py 8000+ 行）
│   │   ├── main.py      <-- 入口
│   │   ├── backend.py
│   │   ├── services/
│   │   ├── api/         qwen2api / zhipu2api 子应用
│   │   ├── platformkit/ 桌面版平台层（**与 3.dev 独立**）
│   │   ├── routes/
│   │   ├── scripts/     install-env.ps1
│   │   └── uv/ python/ nodejs/ bun/  便携版环境
│   └── build/ data/ _internal/ dist/ ver/ temp/
├── 2.WEB/
│   ├── api/             FastAPI 后端（结构照搬 3.dev）
│   └── ui/              Vue 3 前端
├── 3.dev/
│   ├── platformkit/     <-- 共享层（3 代技术精华，最大最全）
│   │   ├── shared/      12+ 核心模块（ai/knowledge/responsive/team/yjs/...）
│   │   └── daemon/      独立 daemon 进程
│   ├── api/             FastAPI 后端（最全）
│   ├── ui/              Vue 3 前端（最全）
│   └── enterprise-services/  私有化 / 授权 / 支持
├── BAK/  doc/  启动*.bat  一键环境部署.bat  AGENTS.md  README.md
```

### 规则 3：1.PC 是独立产品

- 1.PC 的 platformkit / routes 与 3.dev 独立维护
- 1.PC 的 import 路径**不需修改**（结构 1:1 照搬原 dev/app/）
- PyQt6 + QWebEngineView 自带 Chromium（**~200MB**）
- 任何修改要保证 1.PC 仍是"自包含"独立 EXE

### 规则 4：2.WEB 借鉴 3.dev

- 从 3.dev 借鉴时**复制而非引用**（避免强耦合）
- 简化版优先（去掉企业特性：多租户 / SSO / 审计 / 私有化）
- 慢慢补完，**不强同步**

### 规则 5：3.dev 全力开发

- 新功能先在 3.dev/platformkit 落地
- 然后**按需**同步到 2.WEB
- 25% 测试覆盖率（产品 1 自由发挥）

## 启动 / 构建 / 测试

### 启动

| 产品 | 命令 |
|------|------|
| 总入口 | 双击 `启动.bat`（菜单式） |
| 1.PC | 双击 `启动-云集桌面.bat` |
| 2.WEB | 双击 `启动-云集Web.bat` |
| 3.dev | 双击 `启动-云集旗舰.bat` |
| 环境部署 | 双击 `一键环境部署.bat` |

### 构建（EXE）

| 产品 | 命令 | 输出 |
|------|------|------|
| 1.PC | `cd 1.PC\build && python build.py` | `1.PC/dist/云集桌面-v*/` |
| 2.WEB | `cd 2.WEB\build && python build.py` | `2.WEB/dist/云集Web-v*/` |
| 3.dev | `cd 3.dev\build && python build.py` | `3.dev/dist/云集旗舰-v*/` |

### 测试

| 产品 | 命令 |
|------|------|
| 1.PC | `cd 1.PC\app && pytest tests/` |
| 2.WEB | `cd 2.WEB\api && pytest tests/` |
| 3.dev | `cd 3.dev\api && pytest tests/` |
| 3.dev UI | `cd 3.dev\ui && npm test` |

## Git 契约

- **提交**：`1.PC/app/`、`2.WEB/api/`、`2.WEB/ui/`、`3.dev/api/`、`3.dev/ui/`、`3.dev/platformkit/`、`doc/`
- **不提交**：EXE、`_internal/`、`build/`、`dist/`、`node_modules/`、`.pytest_cache/`、`__pycache__/`

## 常见陷阱

| 问题 | 原因 | 解决方案 |
|------|------|----------|
| 1.PC EXE DLL 加载失败 | 用了 `--onefile` | 必须 `--onedir` |
| 1.PC 启动 ValueError | QWebEngineView 参数错 | 构建前用 `import main` 测试 |
| 1.PC 扫描时 UI 假死 | 主线程 IO | 移至后台线程 |
| 2.WEB 找不到 platformkit | 没从 3.dev 同步 | 复制 platformkit/ 到 2.WEB/api/ |
| 3.dev 端口冲突 | 18080/18099 被占 | 改 `--port` 参数 |
| 3.dev Platformkit import 错 | 路径变了 | 用相对 import `from platformkit.shared.xxx` |
| pywebview 启动白屏 | 端口未起 / 前端未 build | 先启后端，等 3 秒再启前端 |
| Vue 启动报 5173 占用 | Vite 默认端口被占 | 改 `vite.config.ts` 的 server.port |

## 关键设计决策

### 为什么 1.PC 是 PyQt6 而不是 pywebview？

- 1.PC 是"永久免费 + 离线 + Ollama"产品
- PyQt6 + QWebEngineView 自带 Chromium，**不依赖系统 WebView2**
- pywebview 需要 Windows 10+ 自带 WebView2，新机器可能没装
- 离线场景下 PyQt6 100% 自包含，更可靠

### 为什么 2.WEB 不用 Electron？

- Electron 体积大（~150MB）
- pywebview 复用系统 WebView2（**~1MB**）
- 跨端统一：pywebview 桌面 + Capacitor 移动 + 浏览器

### 为什么 3.dev 是"全 Platformkit"？

- 12+ 核心模块（知识/感知/AI/协作/...）是**真正护城河**
- 产品 1 不需要（永久免费本地）
- 产品 2 部分需要（知识/感知）
- 产品 3 全需要（+ 多租户/SSO/审计）

## 引用

- 项目根 `AGENTS.md`：完整架构契约
- `doc/产品线矩阵.md`：3 产品线商业 + 技术定义
- `README.md`：项目入口
