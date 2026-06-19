# 云端备份与多设备同步指南（SYNC-GUIDE）

> **目的**：教你如何配置云端备份，实现培训数据的多设备同步。
> **适用版本**：M4.1 D6+（agent-team v0.4.0+）
> **最后更新**：2026-06-19

---

## 1. 概述

云集智能支持将本地培训数据（员工定义 / 知识库 / 规则 / 话术 / 评估用例）加密上传到宝塔服务器，实现：

- **多设备同步**：在 A 设备培训，到 B 设备继续使用
- **数据备份**：防止本地数据丢失
- **团队共享**：同一账号下的设备共享培训数据

### 1.1 技术架构

```
[设备 A]                    [宝塔服务器]                [设备 B]
  .yunji/                     /yunji-sync/                 .yunji/
  ├── employees/    ──HTTPS──>  ├── users/user1/  ──HTTPS──>  ├── employees/
  ├── knowledge/    加密上传    │   ├── data.enc   加密下载    ├── knowledge/
  ├── rules/                    │   └── manifest.yml          ├── rules/
  ├── speeches/                 └── ...                      ├── speeches/
  └── evals/                                                 └── evals/
```

### 1.2 安全机制

- **传输加密**：HTTPS（TLS 1.2+）
- **内容加密**：用户密码派生密钥 + XOR 加密（MVP，生产环境将升级为 AES-256-GCM）
- **密码隔离**：密码不存储到服务器，仅本地保存
- **用户隔离**：每个 user_id 的数据相互隔离

---

## 2. 服务器配置（宝塔）

### 2.1 Nginx 配置

在宝塔面板添加站点，配置 `/yunji-sync` 路径：

```nginx
server {
    listen 443 ssl;
    server_name yunji.example.com;

    ssl_certificate /www/server/ssl/yunji.crt;
    ssl_certificate_key /www/server/ssl/yunji.key;

    # 同步 API
    location /yunji-sync/api/ {
        proxy_pass http://127.0.0.1:8088/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # 限制请求体大小（培训数据通常 < 50MB）
        client_max_body_size 100m;

        # 限流（防止滥用）
        limit_req zone=sync burst=10 nodelay;
    }

    # 静态资源（可选）
    location /yunji-sync/static/ {
        alias /www/wwwroot/yunji-sync/static/;
    }
}

# 限流区域定义（http 块）
limit_req_zone $binary_remote_addr zone=sync rate=10r/m;
```

### 2.2 后端 API（示例）

```python
# /www/wwwroot/yunji-sync/api/main.py
from fastapi import FastAPI, UploadFile, File, HTTPException
from pathlib import Path

app = FastAPI()
DATA_DIR = Path("/www/wwwroot/yunji-sync/data")

@app.post("/api/users/{user_id}/data")
async def upload_data(user_id: str, file: UploadFile = File(...)):
    user_dir = DATA_DIR / user_id
    user_dir.mkdir(parents=True, exist_ok=True)
    # 保存加密文件
    content = await file.read()
    (user_dir / "data.enc").write_bytes(content)
    return {"success": True, "size": len(content)}

@app.get("/api/users/{user_id}/data")
async def download_data(user_id: str):
    user_dir = DATA_DIR / user_id
    enc_file = user_dir / "data.enc"
    if not enc_file.exists():
        raise HTTPException(404, "无备份数据")
    return FileResponse(enc_file)
```

### 2.3 磁盘监控

宝塔 30G 磁盘需监控使用率：

```bash
# /etc/cron.daily/disk-alert.sh
#!/bin/bash
USAGE=$(df /www | tail -1 | awk '{print $5}' | tr -d '%')
if [ $USAGE -gt 80 ]; then
    # 通过宝塔 API 发送告警
    curl -X POST https://api.bt.cn/notify -d "msg=磁盘使用率 ${USAGE}%"
fi
```

---

## 3. 客户端配置

### 3.1 创建同步配置

```rust
use agent_team::{SyncConfig, SyncClient};

let config = SyncConfig::new(
    "https://yunji.example.com/yunji-sync",  // 服务器地址
    "user1",                                  // 用户 ID
    "pc-001",                                 // 设备 ID（每台设备唯一）
    "my_secure_password",                     // 加密密码（≥ 6 位）
);

// 验证配置
config.validate()?;

// 保存配置到文件（不含密码）
config.save_to_file(Path::new(".yunji/sync-config.yml"))?;
```

### 3.2 上传备份

```rust
let client = SyncClient::new(config);
let result = client.upload().await?;
if result.success {
    println!("✅ {} ({} 个文件)", result.message, result.files_transferred);
}
```

### 3.3 下载恢复

```rust
let package = client.download().await?;
let result = client.unpack_to_local(&package)?;
println!("✅ 已恢复 {} 个文件", result.files_transferred);
```

### 3.4 对比差异

```rust
let local_manifest = client.scan_local()?;
let remote_manifest = client.download_manifest().await?;
let diff = client.diff_manifests(&local_manifest, &remote_manifest);

println!("需上传: {} 个文件", diff.to_upload.len());
println!("需下载: {} 个文件", diff.to_download.len());
println!("冲突: {} 个文件", diff.conflicts.len());
```

---

## 4. 多设备同步流程

### 4.1 首次同步（设备 A → 服务器）

1. 在设备 A 完成培训（员工 / 知识库 / 话术 / 评估）
2. 配置同步客户端
3. 执行 `upload()` 上传到服务器

### 4.2 二次同步（服务器 → 设备 B）

1. 在设备 B 安装云集智能
2. 配置同步客户端（相同 user_id + password，不同 device_id）
3. 执行 `download()` + `unpack_to_local()` 恢复数据

### 4.3 增量同步

1. 设备 A 修改了话术 → `upload()` 上传
2. 设备 B 执行 `diff_manifests()` 发现差异 → `download()` 拉取新版本

### 4.4 冲突处理

当两台设备同时修改同一文件时：

- **时间戳策略**：last-writer-wins（较新的覆盖较旧的）
- **手动解决**：`diff.conflicts` 列出冲突文件，用户选择保留哪个版本

---

## 5. 自动同步

### 5.1 启用自动同步

```rust
let mut config = SyncConfig::new(...);
config.auto_sync = true;
config.sync_interval_secs = 300;  // 5 分钟
```

### 5.2 后台任务

```rust
tokio::spawn(async move {
    let client = SyncClient::new(config);
    loop {
        if let Err(e) = client.upload().await {
            tracing::error!("自动同步失败: {e}");
        }
        tokio::time::sleep(Duration::from_secs(300)).await;
    }
});
```

---

## 6. 数据结构

### 6.1 同步清单（manifest.yml）

```yaml
user_id: user1
device_id: pc-001
version: 1
generated_at: "2026-06-19T10:00:00Z"
files:
  - path: employees/customer_service.yml
    size: 1024
    checksum: "a1b2c3d4e5f6"
    modified_at: "2026-06-19T09:30:00Z"
    device_id: pc-001
    version: 3
  - path: speeches/customer_service.yml
    size: 2048
    checksum: "f6e5d4c3b2a1"
    modified_at: "2026-06-19T09:35:00Z"
    device_id: pc-001
    version: 2
```

### 6.2 加密包（data.enc）

```
Base64(XOR(JSON({
    "manifest": {...},
    "files": {
        "employees/customer_service.yml": "Base64(content)",
        ...
    }
}), password_key))
```

---

## 7. 安全注意事项

### 7.1 密码管理

- **不要**将密码写入 git 仓库
- **不要**在日志中打印密码
- 建议使用密码管理器（如 1Password / Bitwarden）存储
- 密码丢失 = 数据无法解密（无法找回）

### 7.2 服务器安全

- 启用 HTTPS（Let's Encrypt 免费证书）
- 配置防火墙（仅开放 443 端口）
- 定期备份服务器数据
- 监控异常访问（同一 IP 大量请求 = 攻击）

### 7.3 MVP 加密说明

⚠️ **当前版本（v1.0）使用简化 XOR 加密，仅用于演示**。

生产环境必须升级为：
- **AES-256-GCM**（aes-gcm crate）
- **PBKDF2-HMAC-SHA256** 密钥派生（100,000 次迭代）
- **随机 IV**（每次加密生成新的初始化向量）

---

## 8. 故障排查

### Q: 上传失败提示"密码长度不能少于 6 位"

A: 密码至少 6 位，建议 12 位以上含大小写字母 + 数字 + 符号。

### Q: 下载后解密失败

A: 检查：
1. 密码是否与上传时一致
2. 文件是否在传输过程中损坏（检查 checksum）
3. 服务器是否返回了完整数据

### Q: 多设备同步出现冲突

A: 使用 `diff_manifests()` 查看冲突文件，手动选择保留版本。建议避免多设备同时编辑。

### Q: 服务器磁盘满了

A: 宝塔 30G 磁盘监控超过 80% 报警。清理旧版本备份：
```bash
# 保留每个用户最近 3 个版本
find /www/wwwroot/yunji-sync/data -name "*.enc" -mtime +30 -delete
```

---

## 9. 相关文档

- [IMPROVEMENT-PLAN.md](./IMPROVEMENT-PLAN.md) § M4.1 D6
- [EVAL-GUIDE.md](./EVAL-GUIDE.md) - 评估用例编写
- [EMPLOYEE-GUIDE.md](./EMPLOYEE-GUIDE.md) - AI 员工定义
