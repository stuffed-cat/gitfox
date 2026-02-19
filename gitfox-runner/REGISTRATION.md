# GitFox Runner 注册和使用指南

## 概述

GitFox Runner 现在支持通过 HTTP API 进行注册，提供了更安全和标准的注册流程。

## 工作流程

1. **管理员创建 Runner Token** - 在 GitFox Web 界面中创建 runner 并获取注册 token
2. **注册 Runner** - 使用 `register` 命令向服务器注册
3. **运行 Runner** - 使用 `run` 命令启动 runner 服务

## 使用步骤

### 1. 在 GitFox 中创建 Runner

访问 GitFox 管理界面或项目设置，创建一个新的 Runner，系统会生成一个注册 token（格式：`glrt-xxxxxxxx`）。

### 2. 注册 Runner

在目标机器上使用以下命令注册 runner：

```bash
gitfox-runner register \
  --url http://localhost:8081 \
  --token glrt-eba076401723409ab26b2a8cb3686552 \
  --name "my-runner" \
  --description "Production runner" \
  --tags docker,linux \
  --executor shell
```

**参数说明：**
- `--url`: GitFox 服务器地址（HTTP/HTTPS）
- `--token`: 从 GitFox 获取的注册 token
- `--name`: Runner 名称（可选，默认为主机名）
- `--description`: Runner 描述（可选）
- `--tags`: Runner 标签，用逗号分隔（可选）
- `--executor`: 执行器类型，`shell` 或 `docker`（默认：shell）
- `--config`: 配置文件路径（可选，默认：`~/.gitfox-runner/config.toml`）

### 3. 启动 Runner

注册成功后，配置会自动保存到 `~/.gitfox-runner/config.toml`。

启动 runner：

```bash
gitfox-runner run
```

或指定自定义配置文件：

```bash
gitfox-runner run --config /path/to/config.toml
```

## 配置文件

注册后，配置文件会保存在 `~/.gitfox-runner/config.toml`，内容示例：

```toml
server_url = "ws://localhost:8081/api/v1/runner/connect"
token = "glrt-auth-xxxxxxxxxxxxxxxx"
name = "my-runner"
tags = ["docker", "linux"]
executor = "shell"
```

**注意：**
- `token` 字段是认证 token（格式：`glrt-auth-xxx`），不是注册 token
- 配置文件包含敏感信息，请妥善保管
- `server_url` 会自动从 HTTP URL 转换为 WebSocket URL

## 架构说明

### 注册流程

```
┌─────────┐      HTTP POST       ┌────────────┐
│ Runner  │ ───────────────────> │  GitFox    │
│         │  /api/v1/runner/     │  Server    │
│         │      register        │            │
│         │                      │            │
│         │ <─────────────────── │            │
│         │  {runner_id,         │            │
│         │   auth_token,        │            │
│         │   websocket_url}     │            │
└─────────┘                      └────────────┘
```

注册 API 返回：
- `runner_id`: Runner 的唯一 ID
- `auth_token`: 用于 WebSocket 连接的认证 token
- `websocket_url`: WebSocket 连接地址

### 运行时流程

```
┌─────────┐   WebSocket Connect   ┌────────────┐
│ Runner  │ ───────────────────> │  GitFox    │
│         │  + Register message   │  Server    │
│         │  (with auth_token)    │            │
│         │                       │            │
│         │ <─────────────────── │            │
│         │  Job assignments      │            │
│         │                       │            │
│         │ ─────────────────> │            │
│         │  Job updates & logs   │            │
└─────────┘                       └────────────┘
```

1. Runner 使用 WebSocket 连接到服务器
2. 发送 Register 消息（包含认证 token）
3. 服务器验证 token 并激活 runner
4. 开始接收和执行任务

## 进阶选项

### 多配置管理

你可以为不同的环境创建多个配置文件：

```bash
# 注册生产环境 runner
gitfox-runner register \
  --url https://gitfox.example.com \
  --token glrt-xxx \
  --config ~/.gitfox-runner/production.toml

# 注册开发环境 runner
gitfox-runner register \
  --url http://localhost:8081 \
  --token glrt-yyy \
  --config ~/.gitfox-runner/development.toml

# 运行指定环境
gitfox-runner run --config ~/.gitfox-runner/production.toml
```

### 系统服务

创建 systemd 服务文件 `/etc/systemd/system/gitfox-runner.service`：

```ini
[Unit]
Description=GitFox CI/CD Runner
After=network.target

[Service]
Type=simple
User=gitlab-runner
WorkingDirectory=/home/gitlab-runner
ExecStart=/usr/local/bin/gitfox-runner run
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

启用并启动服务：

```bash
sudo systemctl daemon-reload
sudo systemctl enable gitfox-runner
sudo systemctl start gitfox-runner
sudo systemctl status gitfox-runner
```

## 故障排除

### 注册失败

如果注册失败，请检查：
1. 服务器 URL 是否正确
2. 注册 token 是否有效（未过期或被撤销）
3. 网络连接是否正常
4. 服务器日志中的错误信息

### 连接失败

如果 runner 无法连接到服务器：
1. 检查配置文件中的 `server_url` 是否正确
2. 确认服务器的 WebSocket 端口是否开放
3. 查看认证 token 是否有效
4. 检查防火墙设置

### 查看日志

启用详细日志：

```bash
RUST_LOG=debug gitfox-runner run
```

## API 端点

### 注册 API

**POST** `/api/v1/runner/register`

请求体：
```json
{
  "token": "glrt-xxxxxxxx",
  "name": "my-runner",
  "description": "Production runner",
  "tags": ["docker", "linux"],
  "executor": "shell"
}
```

响应：
```json
{
  "runner_id": 1,
  "auth_token": "glrt-auth-xxxxxxxxxxxxxxxx",
  "websocket_url": "ws://localhost:8081/api/v1/runner/connect"
}
```

### WebSocket 连接

**GET** `/api/v1/runner/connect`

连接后发送注册消息：
```json
{
  "type": "register",
  "token": "glrt-auth-xxxxxxxxxxxxxxxx",
  "name": "my-runner",
  "tags": ["docker", "linux"],
  "executor": "shell"
}
```

## 安全建议

1. **保护配置文件**：设置适当的文件权限
   ```bash
   chmod 600 ~/.gitfox-runner/config.toml
   ```

2. **定期轮换 token**：定期重新注册 runner 以更新认证 token

3. **使用 HTTPS/WSS**：在生产环境中使用加密连接

4. **限制 runner 权限**：使用专用用户运行 runner 服务

5. **监控日志**：定期检查 runner 日志以发现异常行为
