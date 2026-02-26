# GitFox Workhorse 实现总结

## 概述

GitFox Workhorse 是一个高性能的反向代理服务器，类似于 GitLab Workhorse，专门为 GitFox DevOps 平台设计。它在生产环境中替代开发环境的多个 Vite dev server，提供统一的静态文件服务和 API 代理。

## 实现日期

2026年2月26日

## 核心功能

### 1. 静态文件服务

#### 前端 Vue SPA
- **路径**: `/` 及所有未匹配的路由
- **来源**: `frontend/dist/`
- **特性**:
  - SPA 模式（所有路由返回 index.html）
  - Last-Modified / ETag 支持
  - 自动缓存控制

#### WebIDE (VS Code Server)
- **路径**: `/-/ide/*`
- **来源**: `webide/dist/`
- **特性**:
  - 独立的 index.html fallback
  - VS Code 静态资源服务
  - WebSocket 支持（用于 IDE 通信）

#### Assets（用户上传）
- **路径**: `/assets/*`
- **来源**: `assets/` 目录
- **特性**:
  - 路径遍历防护
  - MIME 类型自动检测
  - 长期缓存

### 2. 反向代理

#### API 请求
- **路径**: `/api/*`
- **目标**: 后端服务器（默认 `http://127.0.0.1:8081`）
- **特性**:
  - 流式传输（处理大文件）
  - 5分钟超时（适用于长时间操作）
  - X-Forwarded-* 头添加

#### OAuth 端点
- **路径**: `/oauth/token`, `/oauth/revoke`, `/oauth/userinfo`
- **目标**: 后端服务器
- **特性**:
  - 透明代理
  - 保持会话状态

#### Git HTTP 协议
- **路径**: `/{namespace}/{project}.git/*`
- **目标**: 后端服务器
- **特性**:
  - 10分钟超时（Git 操作可能很慢）
  - 流式传输（packfile 可能很大）
  - 完整的 Git Smart HTTP 支持

### 3. 中间件

#### 压缩 (Compress)
- 自动 gzip/brotli 压缩
- 根据 Accept-Encoding 自动选择
- 减少传输大小

#### CORS
- 允许所有来源（开发友好）
- 支持凭证（cookies）
- 预检请求支持

#### 日志 (Logger)
- 请求/响应日志
- 性能指标
- 错误追踪

### 4. 健康检查

- **端点**: `/-/health`, `/-/workhorse/health`
- **响应**: JSON 格式的健康状态
- **用途**: 负载均衡器检查、监控

## 技术栈

### 核心依赖

| 依赖              | 版本  | 用途                  |
|-------------------|-------|-----------------------|
| actix-web         | 4.x   | Web 框架              |
| actix-files       | 0.6   | 静态文件服务          |
| actix-cors        | 0.7   | CORS 中间件           |
| reqwest           | 0.11  | HTTP 客户端（代理）   |
| tokio             | 1.x   | 异步运行时            |
| serde             | 1.0   | 序列化/反序列化       |
| tracing           | 0.1   | 日志框架              |
| mime_guess        | 2.0   | MIME 类型检测         |

### 编译优化

在 `Cargo.toml` 中：

```toml
[profile.release]
opt-level = 3         # 最高优化级别
lto = true            # 链接时优化
codegen-units = 1     # 单一代码生成单元（更好的优化）
```

## 架构设计

### 模块结构

```
gitfox-workhorse/
├── src/
│   ├── main.rs           # 入口，HTTP 服务器配置
│   ├── config.rs         # 配置管理（环境变量 + TOML）
│   ├── proxy.rs          # 反向代理逻辑
│   └── static_files.rs   # 静态文件处理
├── Cargo.toml            # 依赖和编译配置
├── config.example.toml   # 配置示例
├── start.sh              # 启动脚本
├── README.md             # 详细文档
└── QUICKSTART.md         # 快速开始指南
```

### 请求处理流程

```
客户端请求
    ↓
CORS 中间件
    ↓
日志中间件
    ↓
压缩中间件
    ↓
路由匹配
    ├─→ /-/health → health_check()
    ├─→ /assets/* → serve_asset()
    ├─→ /-/ide/* → Files service (WebIDE)
    ├─→ /api/* → proxy_to_backend()
    ├─→ /oauth/* → proxy_to_backend()
    ├─→ /*.git/* → proxy_git_http()
    └─→ /* → Files service (SPA) + fallback
```

### 配置优先级

1. 环境变量（最高优先级）
2. TOML 配置文件（通过 `WORKHORSE_CONFIG`）
3. 默认值

## 与 Vite Dev Server 的区别

### 开发环境

```
客户端
    ↓
前端 Vite Dev (3000)
    ├─→ 静态文件 + HMR
    ├─→ /api/* → 代理到 8081
    ├─→ /-/ide/* → 代理到 3002
    └─→ /oauth/* → 代理到 8081
    
WebIDE Vite Dev (3002)
    ├─→ WebIDE 静态文件 + HMR
    ├─→ /api/* → 代理到 8081
    └─→ /oauth/* → 代理到 8081

后端 (8081)
    └─→ API 处理
```

### 生产环境（使用 Workhorse）

```
客户端
    ↓
Workhorse (8080)
    ├─→ 静态文件（已构建）
    ├─→ /-/ide/* 静态文件
    ├─→ /api/* → 代理到 8081
    ├─→ /oauth/* → 代理到 8081
    └─→ /*.git/* → 代理到 8081
        ↓
    后端 (8081)
        └─→ API 处理
```

优势：
- ✅ 单一入口点（8080）
- ✅ 更好的性能（生产构建 + Rust）
- ✅ 自动压缩和缓存
- ✅ 真实的生产环境模拟

## 路由配置遵循

### 来自 frontend/vite.config.ts

```typescript
proxy: {
  '/-/ide': { target: 'http://localhost:3002' },
  '/api': { target: 'http://localhost:8081' },
  '/assets': { target: 'http://localhost:8081' },
  '/oauth/token': { target: 'http://localhost:8081' },
  '/oauth/revoke': { target: 'http://localhost:8081' },
  '/oauth/userinfo': { target: 'http://localhost:8081' },
  '^/[^/]+/[^/]+\\.git': { target: 'http://localhost:8081' },
}
```

### 来自 webide/vite.config.ts

```typescript
base: '/-/ide/',
proxy: {
  '/api': { target: 'http://localhost:8081' },
  '/oauth': { target: 'http://localhost:8081' },
}
```

Workhorse 完全复刻了这些路由规则。

## 性能优化

### 1. 多核并发

```rust
.workers(num_cpus::get())
```

自动使用所有 CPU 核心处理请求。

### 2. 流式传输

```rust
let stream = backend_res.bytes_stream();
Ok(client_resp.streaming(stream))
```

避免将整个响应加载到内存，支持任意大小的文件。

### 3. 缓存头

```rust
.use_last_modified(true)
.use_etag(true)
```

浏览器可以使用 304 Not Modified 响应。

### 4. 编译优化

- LTO（链接时优化）
- 单一代码生成单元
- opt-level = 3

## 安全特性

### 1. 路径遍历防护

```rust
let canonical_assets = assets_path.canonicalize()?;
let canonical_file = file_path.canonicalize()?;

if !canonical_file.starts_with(&canonical_assets) {
    return Err(ErrorNotFound("Invalid path"));
}
```

确保请求的文件在允许的目录内。

### 2. MIME 类型检测

```rust
mime_guess::from_path(&file_path)
```

自动设置正确的 Content-Type，防止 XSS。

### 3. 头部清理

```rust
if name_str != "host" && name_str != "connection" {
    backend_req = backend_req.header(name_str, value_str);
}
```

移除可能引起问题的头部。

## 部署选项

### 选项 1: 直接运行

```bash
./gitfox-workhorse/target/release/gitfox-workhorse
```

### 选项 2: systemd 服务

```ini
[Service]
ExecStart=/opt/gitfox/gitfox-workhorse/target/release/gitfox-workhorse
```

### 选项 3: Nginx + Workhorse

```nginx
location / {
    proxy_pass http://127.0.0.1:8080;
}
```

提供 SSL/TLS 终止。

### 选项 4: Docker（未实现，可扩展）

```dockerfile
FROM rust:1.70 as builder
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/gitfox-workhorse /usr/local/bin/
CMD ["gitfox-workhorse"]
```

## 监控和日志

### 日志级别

```bash
# 生产环境
RUST_LOG=info

# 调试
RUST_LOG=debug

# 仅 Workhorse
RUST_LOG=gitfox_workhorse=debug
```

### 健康检查

```bash
curl http://localhost:8080/-/health
```

可用于：
- Kubernetes liveness/readiness probes
- 负载均衡器健康检查
- 监控系统（Prometheus, Datadog 等）

## 未来扩展

### 可能的增强

1. **LFS 支持**: Git Large File Storage
2. **缓存**: Redis 缓存常用的 Git objects
3. **速率限制**: 防止滥用
4. **Metrics**: Prometheus metrics 端点
5. **gRPC**: 与后端的高效通信
6. **热重载**: 配置文件变更时自动重载

## 与 GitLab Workhorse 的对比

| 功能                 | GitLab Workhorse | GitFox Workhorse |
|----------------------|------------------|------------------|
| **语言**             | Go               | Rust             |
| **静态文件服务**     | ✅               | ✅               |
| **API 反向代理**     | ✅               | ✅               |
| **Git HTTP**         | ✅               | ✅               |
| **流式上传**         | ✅               | ✅               |
| **LFS 支持**         | ✅               | ❌ (可扩展)      |
| **Git archive 缓存** | ✅               | ❌ (可扩展)      |
| **WebSocket**        | ✅               | ✅               |
| **多核并发**         | ✅               | ✅               |
| **配置简单性**       | 中等             | 简单             |

## 测试建议

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_from_env() {
        // 测试配置加载
    }
}
```

### 集成测试

```bash
# 1. 启动后端
cargo run &

# 2. 启动 Workhorse
cd gitfox-workhorse
cargo run --release &

# 3. 测试端点
curl http://localhost:8080/-/health
curl http://localhost:8080/api/v1/projects
curl http://localhost:8080/
```

### 性能测试

```bash
# 使用 wrk 进行负载测试
wrk -t12 -c400 -d30s http://localhost:8080/

# 使用 ab
ab -n 10000 -c 100 http://localhost:8080/
```

## 总结

GitFox Workhorse 成功实现了：

✅ **完整的静态文件服务** - 前端 SPA、WebIDE、Assets
✅ **智能反向代理** - API、OAuth、Git HTTP
✅ **高性能** - Rust + Actix + 流式传输
✅ **生产就绪** - 压缩、缓存、日志、健康检查
✅ **易于部署** - 单一二进制文件、简单配置
✅ **路由兼容** - 完全遵循 Vite 开发环境的路由规则

它为 GitFox 提供了一个类似 GitLab Workhorse 的生产级反向代理解决方案，简化了部署流程，提高了性能和稳定性。

## 参考资料

- [GitLab Workhorse 文档](https://docs.gitlab.com/ee/development/workhorse/)
- [Actix Web 文档](https://actix.rs/)
- [Rust 异步编程](https://rust-lang.github.io/async-book/)
