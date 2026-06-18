# 快速开始（5 分钟跑通）

> **目标**：5 分钟内启动 AgentWork 桌面端，跑通第一个 AI 员工任务。

---

## 1. 环境检查（1 分钟）

### 必需

- **Rust** 1.88+ — [rustup.rs](https://rustup.rs)
- **Node.js** 20+ + **pnpm** 9+
- **Tauri 系统依赖**（[官方文档](https://tauri.app/start/prerequisites/)）

### 检查命令

```bash
rustc --version    # 应 ≥ 1.88.0
node --version     # 应 ≥ 20.0.0
pnpm --version     # 应 ≥ 9.0.0
```

---

## 2. 克隆 + 安装（2 分钟）

```bash
git clone https://github.com/yunji/agentwork.git
cd agentwork
git checkout agentwork

# 安装前端依赖
pnpm install
```

---

## 3. 启动桌面端（1 分钟）

### 方式一：启动脚本（Windows）

```bat
:: 双击根目录
启动.bat

:: 选择 [1] Desktop Dev
```

### 方式二：命令行

```bash
cd apps/desktop
pnpm tauri dev
```

启动后看到桌面窗口 = 成功。

---

## 4. 跑通第一个任务（1 分钟）

1. 应用启动后默认进入 **AI 互动面板**
2. 在输入框输入：`创建一个电商客服员工`
3. 按 Enter 发送
4. 观察 AI 思考过程（流式输出）
5. 点击侧边栏 **任务看板** 查看 DAG

---

## 5. 常用功能

| 功能 | 入口 | 说明 |
|------|------|------|
| AI 互动 | 侧边栏第 1 个图标 | 默认首页，自然语言交互 |
| 任务看板 | 侧边栏第 6 个图标 | DAG 可视化 + 调度控制 |
| 交付物预览 | `/delivery` 路由 | diff + 测试 + commit |
| 团队协作 | 侧边栏第 5 个图标 | 团队模板 + 模型配置 |
| 设置 | 侧边栏最后 | AI 配置 / Git 模式 |

---

## 6. 常见问题

**Q: `cargo` 命令找不到？**
A: 运行 `$env:USERPROFILE\.cargo\bin\cargo.exe` 或将 `~/.cargo/bin` 加入 PATH。

**Q: `pnpm tauri dev` 报错？**
A: 检查 Tauri 系统依赖是否安装（Windows 需要 WebView2）。

**Q: 启动后白屏？**
A: 等待 Vite 编译完成（首次约 10 秒），查看终端日志。

---

## 7. 下一步

- [AI 员工定义指南](EMPLOYEE-GUIDE.md) — 自定义 AI 员工
- [用户指南](USER-GUIDE.md) — 详细功能教程
- [完善+增强计划](IMPROVEMENT-PLAN.md) — 路线图
