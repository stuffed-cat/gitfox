# GitFox Workhorse

GitFox Workhorse 是一个智能反向代理服务器，类似于 GitLab Workhorse，用于在生产环境中提供静态资源和代理后端 API。

## 功能特性

- **静态文件服务**: 提供前端 Vue SPA、WebIDE 和用户上传的 assets
- **反向代理**: 将 API 请求代理到后端服务器
- **Git HTTP 协议**: 支持 Git clone/push/pull 操作
- **SPA 路由**: 自动 fallback 到 index.html
- **流式传输**: 高效处理大文件上传和下载
- **健康检查**: 内置健康检查端点
- **CORS 支持**: 可配置的跨域资源共享
- **压缩**: 自动 gzip/brotli 压缩

## 架构

```
客户端
   ↓
GitFox Workhorse (端口 8080)
   ↓
   ├─→ 静态文件 (/, /-/ide/*)
   ├─→ Assets (/assets/*)
   └─→ 后端代理 (/api/*, /oauth/*, Git HTTP)
       ↓
   GitFox Backend (端口 8081)
```

## 路由规则

遵循 Vite 配置中定义的路由规则：

### 前端 SPA (frontend/vite.config.ts)
- **基础路径**: `/`
- **构建输出**: `frontend/dist/`
- **Fallback**: 未匹配的路由返回 `index.html`

### WebIDE (webide/vite.config.ts)
- **基础路径**: `/-/ide/`
- **构建输出**: `webide/dist/`
- **静态文件**: `/vscode/*` 或 `/-/ide/vscode/*`
- **Fallback**: WebIDE 路由返回 WebIDE 的 `index.html`

### 后端代理
- **API**: `/api/*` → 后端
- **OAuth**: `/oauth/*` → 后端
- **Assets**: `/assets/*` → 本地 assets 目录
- **Git HTTP**: `/{namespace}/{project}.git/*` → 后端

## 配置

### 环境变量

```bash
# Workhorse 监听地址
WORKHORSE_LISTEN_ADDR=0.0.0.0
WORKHORSE_LISTEN_PORT=8080

# 后端服务器地址
WORKHORSE_BACKEND_URL=http://127.0.0.1:8081

# 静态文件路径
WORKHORSE_FRONTEND_DIST=./frontend/dist
WORKHORSE_WEBIDE_DIST=./webide/dist
WORKHORSE_ASSETS_PATH=./assets
WORKHORSE_GIT_REPOS_PATH=./repos

# 上传限制 (字节)
WORKHORSE_MAX_UPLOAD_SIZE=104857600  # 100MB

# WebSocket 超时 (秒)
WORKHORSE_WEBSOCKET_TIMEOUT=3600

# 静态文件缓存控制
WORKHORSE_STATIC_CACHE_CONTROL="public, max-age=31536000, immutable"
```

### 配置文件 (config.toml)

```toml
listen_addr = "0.0.0.0"
listen_port = 8080
backend_url = "http://127.0.0.1:8081"
frontend_dist_path = "./frontend/dist"
webide_dist_path = "./webide/dist"
assets_path = "./assets"
git_repos_path = "./repos"
enable_request_logging = true
enable_cors = true
max_upload_size = 104857600
websocket_timeout = 3600
static_cache_control = "public, max-age=31536000, immutable"
```

使用配置文件：

```bash
export WORKHORSE_CONFIG=config.toml
./gitfox-workhorse
```

## 构建

### 开发模式

```bash
cargo build
cargo run
```

### 生产模式

```bash
cargo build --release
./target/release/gitfox-workhorse
```

## 部署

### 1. 构建前端资源

```bash
# 构建主前端 SPA
cd frontend
npm run build
cd ..

# 构建 WebIDE
cd webide
npm run build
cd ..
```

### 2. 启动后端服务

```bash
# 确保后端运行在 8081 端口
export SERVER_PORT=8081
cargo run --bin devops
```

### 3. 启动 Workhorse

```bash
cd gitfox-workhorse

# 使用环境变量
export WORKHORSE_BACKEND_URL=http://127.0.0.1:8081
export WORKHORSE_FRONTEND_DIST=../frontend/dist
export WORKHORSE_WEBIDE_DIST=../webide/dist
export WORKHORSE_ASSETS_PATH=../assets
cargo run --release

# 或使用配置文件
export WORKHORSE_CONFIG=config.toml
cargo run --release
```

### 4. 访问应用

打开浏览器访问: http://localhost:8080

## 健康检查

Workhorse 提供两个健康检查端点：

```bash
curl http://localhost:8080/-/health
curl http://localhost:8080/-/workhorse/health
```

响应示例：

```json
{
  "status": "healthy",
  "service": "gitfox-workhorse",
  "version": "0.1.0"
}
```

## 日志

使用 `RUST_LOG` 环境变量控制日志级别：

```bash
# 默认：info 级别
RUST_LOG=info cargo run

# 调试模式
RUST_LOG=debug cargo run

# 仅显示 Workhorse 日志
RUST_LOG=gitfox_workhorse=debug cargo run
```

## 性能优化

### 1. 静态文件缓存

Workhorse 自动为静态文件添加缓存头：

- **Last-Modified** / **ETag**: 支持条件请求
- **Cache-Control**: 可配置的缓存策略
- **immutable**: 对于带 hash 的文件（如 Vite 构建的 assets）

### 2. 压缩

自动对响应进行 gzip/brotli 压缩，减少传输大小。

### 3. 流式传输

对于大文件（Git packfile、文件上传）使用流式传输，避免内存溢出。

### 4. 多核并发

自动使用所有可用 CPU 核心（通过 `num_cpus`）。

## 与 GitLab Workhorse 的区别

GitFox Workhorse 借鉴了 GitLab Workhorse 的设计理念，但更简化：

- ✅ **静态文件服务**: 支持 SPA、WebIDE
- ✅ **API 反向代理**: 透明代理到后端
- ✅ **Git HTTP 协议**: 完整支持
- ✅ **流式传输**: 高效处理大文件
- ❌ **LFS 支持**: 暂不支持（可后续添加）
- ❌ **缓存**: 暂不支持 Git archive 缓存（可后续添加）

## 故障排查

### 问题: 404 Not Found

检查路径和构建输出：

```bash
# 检查前端构建
ls -la frontend/dist/

# 检查 WebIDE 构建
ls -la webide/dist/

# 检查 assets
ls -la assets/
```

### 问题: Backend service unavailable

检查后端是否运行：

```bash
curl http://localhost:8081/-/health
```

### 问题: Git 操作失败

检查 Git 仓库路径：

```bash
ls -la repos/
```

查看 Workhorse 日志：

```bash
RUST_LOG=debug cargo run
```

## 许可证

与 GitFox 主项目相同。
