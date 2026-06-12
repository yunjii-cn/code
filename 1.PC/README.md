# 云集桌面 (Yunji Desktop)

> **代际**：第 1 代 | **档位**：Free | **价格**：¥0（永久免费）

## 一句话

**永久免费的本地 AI 编程工具**。双击即用，断网能用，Ollama 一键对接，**数据不离开你的电脑**。

## 核心卖点

- 🟢 **永久免费**，无功能阉割
- 🟢 **离线可用**，无网也能跑（Ollama 本地）
- 🟢 **隐私第一**，代码 / AI 对话 / 知识 **100% 留本地**
- 🟢 **Ollama 原生**，装好 Ollama 自动识别所有本地模型
- 🟢 **零注册**，双击 EXE 即可使用
- 🟢 **PyQt6 原生窗口**，比 Web 版响应更快、占资源更少

## 技术栈

- **桌面壳**：PyQt6 + QWebEngineView（自带 Chromium）
- **后端**：Python 3.12 + FastAPI（内嵌）
- **前端**：Vue 3 + Vite + Vant（PyQt6 渲染）
- **IPC**：QWebChannel
- **打包**：PyInstaller --onedir

## 目录结构

```
1.PC/
├── app/                PyQt6 启动器（8000+ 行）
│   ├── main.py
│   ├── backend.py
│   ├── services/       Ollama 代理 / CLI 管理 / 配置
│   ├── api/            qwen2api / zhipu2api 子应用
│   ├── platformkit/    桌面版平台层
│   ├── routes/         桌面版 API
│   ├── scripts/        install-env.ps1 + start-app.ps1
│   ├── uv/             便携版 uv
│   ├── python/         便携版 Python
│   ├── nodejs/         便携版 Node.js
│   ├── bun/            便携版 Bun
│   └── icon.ico / icon.png
├── build/              PyInstaller 脚本（onedir）
├── data/               运行时数据
├── _internal/          PyInstaller 运行时
├── dist/               分发目录
├── 云集智能编程工作站-v*/  旧 EXE 构建产物
├── debug_build*/       调试构建
├── AGENTS.md           1.PC 专属 AGENTS
├── tier.yaml           功能差异定义
└── README.md           本文件
```

## 快速开始

### 方式 1：双击启动（推荐）

双击 `启动.bat` → 选 `1` → 启动云集桌面

### 方式 2：手动启动

```powershell
cd 1.PC\app
uv\uv.exe run main.py
```

### 首次运行

1. 启动 `一键环境部署.bat`（如果 uv 还没装）
2. 装好 Ollama：<https://ollama.com/download>
3. 启动云集桌面，**自动识别** Ollama 模型

## 商业模式

- **软件**：永久免费（GitHub 开源）
- **算力**：用户自备 Ollama / 本地模型 → **零成本**
- **云端模型**（未来）：按 token 付费给模型厂商
- **不做**：团队 / 多端同步 / 强制登录 / 遥测

## 引用

- 产品线矩阵：[../doc/产品线矩阵.md](../doc/产品线矩阵.md)
- 原始代码：[原 `dev/app/`，1:1 镜像保留](../BAK/)
