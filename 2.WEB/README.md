# 云集 Web (Yunji Web)

> **代际**：第 2 代 | **档位**：Pro | **价格**：**¥9.9 / 月**（新人送 ¥30 体验金）

## 一句话

**9.9 元/月，跨端跨平台的 AI 编程工作空间**。一个账户，电脑、手机、平板无缝切换。

## 核心卖点

- 🟡 **¥9.9 起**（行业最低价，对标 Cursor Pro ¥145/月便宜 **93%**）
- 🟡 **新人送额度**（注册送 ¥30 体验金，约 3 个月免费）
- 🟡 **跨端同步**（电脑写一半，手机继续）
- 🟡 **云端 AI 一键用**（不用配 Ollama）
- 🟡 **本地也能用**（不想给数据走云，Ollama 模式）
- 🟡 **PWA / Capacitor**（手机就是 App）
- 🟡 **超额按 token**（用多少付多少，不强制订阅）

## 技术栈

- **后端**：FastAPI + uvicorn
- **桌面壳**：pywebview（系统 WebView2，仅 ~1MB）
- **前端**：Vue 3 + Vite + TypeScript + Vant UI
- **移动壳**：Capacitor（Android / iOS）
- **实时**：WebSocket + SSE
- **远程**：Tailscale + LAN Token 配对

## 目录结构

```
2.WEB/
├── api/                FastAPI 后端（结构照搬 3.dev/api/）
│   ├── api_main.py
│   ├── backend.py
│   ├── services/
│   ├── routes/         17 个 API 路由（与 3.dev 同步补完）
│   └── tests/
├── ui/                 Vue 3 + Vite 前端
│   ├── src/
│   │   ├── views/      （慢慢补完）
│   │   ├── components/
│   │   ├── stores/
│   │   └── platform/
│   └── package.json
├── build/              PyInstaller + Vite
├── doc/                跨平台指南 / PWA 接入
├── tier.yaml           功能差异
└── README.md           本文件
```

## 快速开始

### 双击启动

双击 `启动.bat` → 选 `2` → 启动云集 Web

### 手动启动

```powershell
# 终端 1：后端
cd 2.WEB\api
uv run api_main.py --port 18080

# 终端 2：前端
cd 2.WEB\ui
npm install   # 首次
npm run dev
```

访问 <http://127.0.0.1:5173>

## 当前状态

🟡 **半成品**（慢慢补完）：

- ✅ FastAPI 后端框架 + 基础路由
- ✅ Vue 3 + Vite 前端框架
- 🟡 账户系统（待完善）
- 🟡 配额系统（待完善）
- 🟡 支付集成（待对接）
- 🟡 PWA / Capacitor（待集成）

**借鉴来源**：从 3.dev/ 同步核心代码，避免重复造轮子

## 商业模式

| 维度 | 内容 |
|------|------|
| **免费层** | 本地 Ollama + 限 5 项目 + 1 设备 |
| **Pro ¥9.9/月** | 云端 AI + ¥10 等值算力 + 多端同步 |
| **新人优惠** | 注册送 ¥30 体验金（≈ 3 个月） |
| **超额** | 按 token 阶梯计费 |

## 引用

- 产品线矩阵：[../doc/产品线矩阵.md](../doc/产品线矩阵.md)
- 旗舰版（完整版）：[../3.dev/README.md](../3.dev/README.md)
