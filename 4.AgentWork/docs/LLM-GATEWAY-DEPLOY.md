# LLM Gateway 部署指南

> **目的**：在宝塔服务器上部署 LLM 反代网关，统一管理 Qwen / GLM / MiniMax 等 LLM API。
> **适用**：4.AgentWork M4.0 D7
> **服务器**：宝塔面板 / 4 核 8G / < 30G 磁盘 / 公网 IP

---

## 1. 架构概览

```
AgentWork 桌面端
       │
       ▼
  LLM Gateway（宝塔服务器）
       │
       ├── /qwen/*  → Qwen API（通义千问）
       ├── /zhipu/* → GLM API（智谱清言）
       └── /minimax/* → MiniMax API
```

### 1.1 为什么需要 Gateway？

| 问题 | 解决方案 |
|------|---------|
| API Key 散落在客户端 | Gateway 统一管理 |
| 多用户共享 Key | Gateway 限流 + 负载均衡 |
| 宝塔 8G 内存 | 单实例 + 限流保护 |
| 多 LLM 切换 | 统一入口 + 路由 |

### 1.2 资源约束

- **内存**：单实例 ≤ 500MB
- **并发**：≤ 20 并发请求
- **限流**：每用户 10 req/min
- **磁盘**：日志 ≤ 1GB（自动轮转）

---

## 2. 部署步骤

### 2.1 宝塔环境准备

```bash
# 1. 宝塔面板 → 软件商店 → 安装 Python 项目管理器
# 2. 安装 Python 3.11+
# 3. 安装 Nginx（反向代理）
```

### 2.2 部署 zhipu2api（GLM 反代）

```bash
# 1. 上传代码
cd /www/wwwroot/llm-gateway
# 将 4.AgentWork/llm-gateway/zhipu2api/ 上传到此目录

# 2. 创建虚拟环境
python3.11 -m venv venv
source venv/bin/activate

# 3. 安装依赖
pip install fastapi uvicorn httpx

# 4. 配置环境变量
cp .env.example .env
# 编辑 .env 填入智谱 API Key

# 5. 启动（宝塔 Python 项目管理器）
# 项目路径: /www/wwwroot/llm-gateway/zhipu2api
# 启动命令: uvicorn main:app --host 127.0.0.1 --port 8101
```

### 2.3 Nginx 反向代理

在宝塔 Nginx 配置中添加：

```nginx
# LLM Gateway 反向代理
location /api/llm/zhipu/ {
    proxy_pass http://127.0.0.1:8101/;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    
    # SSE 流式支持
    proxy_buffering off;
    proxy_cache off;
    proxy_read_timeout 300s;
    
    # 限流
    limit_req zone=llm burst=10 nodelay;
}

# 限流区域定义（http 块）
limit_req_zone $binary_remote_addr zone=llm:10m rate=10r/m;
```

### 2.4 qwen2api 部署（后续）

qwen2api 代码量较大（70+ 文件），后续从 1.PC 完整搬迁后部署：

```bash
# 端口 8102
cd /www/wwwroot/llm-gateway/qwen2api
uvicorn backend.main:app --host 127.0.0.1 --port 8102
```

---

## 3. AgentWork 配置

### 3.1 环境变量

在 AgentWork 桌面端设置中配置：

```
LLM_GATEWAY_URL=https://your-domain.com/api/llm
LLM_GATEWAY_TOKEN=your-gateway-token
```

### 3.2 模型路由

| 模型 ID | Gateway 路径 | 说明 |
|---------|-------------|------|
| `qwen3.7` | `/api/llm/qwen/chat/completions` | 通义千问 |
| `glm5.2` | `/api/llm/zhipu/chat/completions` | 智谱清言 |
| `minimax3` | `/api/llm/minimax/chat/completions` | MiniMax |

### 3.3 代码配置

在 `timeflow-ai` 的 `AiConfig` 中设置 `base_url` 为 Gateway URL：

```rust
let config = AiConfig {
    api_key: Some("your-gateway-token".to_string()),
    base_url: "https://your-domain.com/api/llm/zhipu".to_string(),
    model: "glm5.2".to_string(),
    ..Default::default()
};
```

---

## 4. 监控 + 限流

### 4.1 限流策略

| 维度 | 限制 | 说明 |
|------|------|------|
| 每用户 | 10 req/min | 防滥用 |
| 全局 | 100 req/min | 保护宝塔 |
| 单次请求 | 30s 超时 | 防长连接 |
| 响应体 | 4MB | 防大响应 |

### 4.2 日志

```bash
# 日志路径
/www/wwwroot/llm-gateway/logs/

# 自动轮转（宝塔日志切割）
# 每天切割 + 保留 7 天
```

### 4.3 监控指标

| 指标 | 告警阈值 | 说明 |
|------|---------|------|
| 内存 | > 1GB | OOM 风险 |
| CPU | > 80% 持续 5min | 过载 |
| 响应时间 | > 10s | 上游慢 |
| 错误率 | > 5% | 上游异常 |

---

## 5. Token 计数

Gateway 对每个请求/响应进行 token 计数：

```python
# zhipu2api/main.py 中已有 token 计数逻辑
# 记录到 SQLite：timestamp / user / model / prompt_tokens / completion_tokens
```

### 5.1 计费（未来）

| 档位 | 月度 token 限额 | 价格 |
|------|:---:|:---:|
| Free | 10 万 | ¥0 |
| Pro | 100 万 | ¥99/月 |
| Team | 500 万 | ¥299/月/席位 |
| Enterprise | 不限 | ¥999/月/席位 |

---

## 6. 安全

### 6.1 API Key 管理

- **不在客户端存储上游 API Key**
- Gateway 用环境变量管理
- 客户端只持有 Gateway Token

### 6.2 HTTPS

- 宝塔申请 Let's Encrypt 免费证书
- 所有流量走 HTTPS

### 6.3 防火墙

- 仅开放 80/443 端口
- Gateway 内部端口（8101/8102）不对外

---

## 7. 故障排查

| 问题 | 原因 | 解决 |
|------|------|------|
| 502 Bad Gateway | zhipu2api 未启动 | 检查宝塔 Python 项目状态 |
| 429 Too Many Requests | 限流触发 | 等待 1 分钟 |
| 504 Gateway Timeout | 上游 LLM 慢 | 增加 `proxy_read_timeout` |
| 内存 OOM | 并发过高 | 降低并发限制 |

---

## 8. 后续规划

| 功能 | 时间 | 说明 |
|------|------|------|
| qwen2api 完整搬迁 | M4.1 | 70+ 文件 |
| MiniMax 反代 | M4.1 | 新增 |
| Token 计费系统 | M5 | Beta 前完成 |
| 多 Key 负载均衡 | M5 | KeyPool 已有基础 |
| 监控仪表盘 | GA 后 | 详见 FUTURE-PLANS.md |
