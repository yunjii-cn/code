# 云集桌面 - AGENTS.md

> 范围：`1.PC/` 第 1 代云集桌面。  
> 最近更新：2026-06-24。

## 定位

`1.PC/` 是云集第 1 代桌面产品：

- 永久免费。
- 本地优先。
- 离线可用。
- Ollama 原生。
- 零注册。
- PyQt6 + QWebEngineView 独立桌面应用。

本产品独立维护，不依赖 `3.dev/` 或 `4.AgentWork/`。

## 目录边界

```text
1.PC/
├── app/        PyQt6 启动器、内嵌 FastAPI、服务、路由、便携运行时
├── build/      PyInstaller 打包脚本
├── data/       运行数据
├── dist/       构建产物
├── doc/        1.PC 专属文档
├── tier.yaml
└── README.md
```

旧 `dev/app/` 路径已经废弃。旧文档中如仍出现 `dev/app/`，只作为历史路径参考。

## 工作规则

1. 修改前先确认任务只影响 `1.PC/`，还是跨产品。
2. 不把 `3.dev/platformkit` 或 `4.AgentWork` 的运行时代码引入 `1.PC/`。
3. 保持 PyQt6 桌面版可独立启动。
4. 涉及进程控制、单实例、EXE、便携环境的文档放入 `1.PC/doc/`。
5. 跨产品定位和商业策略回到根级 [../doc/产品线矩阵.md](../doc/产品线矩阵.md)。

## 常用命令

```powershell
cd 1.PC\app
uv\uv.exe run main.py
```

测试：

```powershell
cd 1.PC\app
pytest tests/
```

打包：

```powershell
cd 1.PC\build
python build_pc.py
```

## 文档

| 文档 | 说明 |
|---|---|
| [README.md](README.md) | 产品入口 |
| [doc/README.md](doc/README.md) | 1.PC 文档索引 |
| [doc/单实例控制机制-技术研究.md](doc/单实例控制机制-技术研究.md) | 单实例技术研究 |
| [doc/进程控制策略-技术文档.md](doc/进程控制策略-技术文档.md) | 进程控制策略 |
| [doc/纯净整合包目录结构说明.md](doc/纯净整合包目录结构说明.md) | 运行目录说明 |
| [../BAK/文档归档-20260624/1.PC/doc/开发指南-旧路径参考.md](../BAK/文档归档-20260624/1.PC/doc/开发指南-旧路径参考.md) | 旧路径开发指南归档，仅作历史参考 |
