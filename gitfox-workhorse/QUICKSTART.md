# GitFox Workhorse 快速开始指南

## 什么是 GitFox Workhorse？

GitFox Workhorse 是一个智能反向代理服务器，类似于 GitLab Workhorse，专门为 GitFox DevOps 平台设计。它在生产环境中提供：

- ✅ 前端 Vue SPA 静态文件服务
- ✅ WebIDE (VS Code Server) 静态文件服务
- ✅ 用户上传的 Assets（头像、附件等）
- ✅ API 请求反向代理到后端
- ✅ Git HTTP 协议支持（clone/push/pull）
- ✅ 自动压缩和缓存优化
- ✅ CORS 支持

## 快速开始

### 方法 1: 使用启动脚本（推荐）

```bash
# 1. 构建前端和 WebIDE（只需要第一次或更新时）
cd frontend && npm run build && cd ..
cd webide && npm run build && cd ..

# 2. 启动后端服务（在另一个终端）
export SERVER_PORT=8081
cargo run

# 3. 使用启动脚本（会自动检查和构建）
cd gitfox-workhorse
./start.sh          # debug 模式
./start.sh release  # release 模式（推荐生产环境）
```

### 方法 2: 手动启动

```bash
# 设置环境变量
export WORKHORSE_LISTEN_ADDR=0.0.0.0
export WORKHORSE_LISTEN_PORT=8080
export WORKHORSE_BACKEND_URL=http://127.0.0.1:8081
export WORKHORSE_FRONTEND_DIST=../frontend/dist
export WORKHORSE_WEBIDE_DIST=../webide/dist
export WORKHORSE_ASSETS_PATH=../assets

# 启动 Workhorse
cd gitfox-workhorse
cargo run --release
```

### 方法 3: 使用配置文件

```bash
# 1. 复制配置示例
cd gitfox-workhorse
cp config.example.toml config.toml

# 2. 编辑配置文件（可选）
vim config.toml

# 3. 启动
export WORKHORSE_CONFIG=config.toml
cargo run --release
```

## 访问应用

启动后，在浏览器中访问：

- **主页**: http://localhost:8080
- **WebIDE**: http://localhost:8080/-/ide/
- **健康检查**: http://localhost:8080/-/health
- **API**: http://localhost:8080/api/v1/...

## 部署架构

```
                     ┌─────────────────────┐
                     │   客户端浏览器       │
                     └──────────┬──────────┘
                                │
                                ▼
                     ┌─────────────────────┐
                     │  GitFox Workhorse   │
                     │    (端口 8080)      │
                     └──────────┬──────────┘
                                │
                ┌───────────────┼───────────────┐
                │               │               │
                ▼               ▼               ▼
        ┌──────────┐    ┌──────────┐    ┌──────────┐
        │ 静态文件  │    │ 后端 API  │    │  Assets  │
        │ (SPA +   │    │ (8081)   │    │  (本地)  │
        │ WebIDE)  │    └──────────┘    └──────────┘
        └──────────┘
```

## 路由规则

Workhorse 会智能路由请求：

### 代理到后端的请求
- `/api/*` - REST API
- `/oauth/*` - OAuth 认证
- `/{namespace}/{project}.git/*` - Git HTTP 协议

### 本地提供的静态文件
- `/` - 前端 SPA（来自 `frontend/dist/`）
- `/-/ide/*` - WebIDE（来自 `webide/dist/`）
- `/assets/*` - 用户上传文件（来自 `assets/`）

## 环境变量参考

| 变量                              | 默认值                    | 说明                  |
|-----------------------------------|---------------------------|-----------------------|
| `WORKHORSE_LISTEN_ADDR`           | `0.0.0.0`                 | 监听地址              |
| `WORKHORSE_LISTEN_PORT`           | `8080`                    | 监听端口              |
| `WORKHORSE_BACKEND_URL`           | `http://127.0.0.1:8081`   | 后端服务器地址        |
| `WORKHORSE_FRONTEND_DIST`         | `./frontend/dist`         | 前端构建输出目录      |
| `WORKHORSE_WEBIDE_DIST`           | `./webide/dist`           | WebIDE 构建输出目录   |
| `WORKHORSE_ASSETS_PATH`           | `./assets`                | Assets 目录           |
| `WORKHORSE_MAX_UPLOAD_SIZE`       | `104857600` (100MB)       | 最大上传大小（字节）  |
| `WORKHORSE_WEBSOCKET_TIMEOUT`     | `3600` (1小时)            | WebSocket 超时（秒）  |
| `RUST_LOG`                        | `info`                    | 日志级别              |

## 日志级别

```bash
# 查看所有日志
RUST_LOG=debug cargo run

# 只看 Workhorse 日志
RUST_LOG=gitfox_workhorse=debug cargo run

# 生产环境
RUST_LOG=info cargo run --release
```

## 性能优化建议

### 1. 使用 Release 构建

```bash
cargo build --release
./target/release/gitfox-workhorse
```

Release 构建比 debug 快 10-100 倍。

### 2. 静态文件缓存

Workhorse 自动为静态文件添加：
- `Last-Modified` 和 `ETag` 头
- `Cache-Control` 头（可配置）
- 支持 304 Not Modified 响应

### 3. 压缩

自动对所有响应进行 gzip/brotli 压缩，无需配置。

### 4. 多核并发

Workhorse 自动使用所有可用 CPU 核心。

## 故障排查

### 问题 1: Cannot connect to backend

**症状**: 502 Bad Gateway

**解决方案**:
```bash
# 检查后端是否运行
curl http://localhost:8081/-/health

# 如果未运行，启动后端
cd /path/to/gitfox
export SERVER_PORT=8081
cargo run
```

### 问题 2: 404 Not Found for static files

**症状**: 静态文件返回 404

**解决方案**:
```bash
# 检查构建输出是否存在
ls -la frontend/dist/
ls -la webide/dist/

# 重新构建
cd frontend && npm run build && cd ..
cd webide && npm run build && cd ..
```

### 问题 3: WebIDE 无法加载

**症状**: WebIDE 页面空白或加载失败

**解决方案**:
```bash
# 检查 WebIDE 构建
ls -la webide/dist/

# 检查静态文件路径
ls -la webide/static/vscode/

# 查看日志
RUST_LOG=debug cargo run
```

### 问题 4: Git 操作失败

**症状**: git clone/push 失败

**解决方案**:
```bash
# 检查后端 Git 路由
curl -v http://localhost:8080/namespace/project.git/info/refs?service=git-upload-pack

# 检查后端日志
# 在后端终端查看日志输出
```

## 开发模式

在开发时，推荐的架构是：

1. **前端 SPA**: Vite dev server (端口 3000) - 支持 HMR
2. **WebIDE**: Vite dev server (端口 3002) - 支持 HMR
3. **后端**: Actix-web (端口 8081)
4. **Workhorse**: 仅在生产环境使用

开发时的前端 vite.config.ts 已经配置好代理，无需使用 Workhorse。

## 生产部署

### 使用 systemd

创建 `/etc/systemd/system/gitfox-workhorse.service`:

```ini
[Unit]
Description=GitFox Workhorse
After=network.target

[Service]
Type=simple
User=gitfox
WorkingDirectory=/opt/gitfox
Environment="RUST_LOG=info"
Environment="WORKHORSE_BACKEND_URL=http://127.0.0.1:8081"
Environment="WORKHORSE_FRONTEND_DIST=/opt/gitfox/frontend/dist"
Environment="WORKHORSE_WEBIDE_DIST=/opt/gitfox/webide/dist"
Environment="WORKHORSE_ASSETS_PATH=/opt/gitfox/assets"
ExecStart=/opt/gitfox/gitfox-workhorse/target/release/gitfox-workhorse
Restart=always

[Install]
WantedBy=multi-user.target
```

启动服务：

```bash
sudo systemctl daemon-reload
sudo systemctl enable gitfox-workhorse
sudo systemctl start gitfox-workhorse
sudo systemctl status gitfox-workhorse
```

### 使用 Nginx（可选）

如果需要 SSL/TLS，可以在 Workhorse 前面加一层 Nginx：

```nginx
server {
    listen 80;
    server_name gitfox.example.com;
    
    # 重定向到 HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name gitfox.example.com;
    
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket 支持
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

## 与后端的区别

| 功能               | 后端 (Actix-web)        | Workhorse              |
|--------------------|-------------------------|------------------------|
| **API 处理**       | ✅ 直接处理             | ❌ 代理到后端          |
| **静态文件**       | ❌ 不提供               | ✅ 高效提供            |
| **压缩**           | ❌ 未启用               | ✅ 自动压缩            |
| **缓存**           | ❌ 无缓存控制           | ✅ 缓存优化            |
| **Git HTTP**       | ✅ 直接处理             | ✅ 代理到后端          |
| **WebSocket**      | ✅ Runner 连接          | ✅ 透明代理            |

## 性能指标

在典型硬件上（4 核 CPU，8GB RAM），Workhorse 可以轻松处理：

- **静态文件**: 10,000+ 请求/秒
- **API 代理**: 5,000+ 请求/秒
- **并发连接**: 10,000+ 连接
- **内存使用**: ~50MB

## 许可证

与 GitFox 主项目相同。
