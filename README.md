# DevSecOps 版本管理系统

基于 Git 的 DevSecOps 版本管理系统，使用 Actix Web + Vue.js + TypeScript + Sass + PostgreSQL + Redis 构建。

## 技术栈

### 后端
- **Rust + Actix Web 4.x**: 高性能 Web 框架
- **SQLx 0.7**: 异步数据库驱动 (PostgreSQL)
- **Redis**: 缓存和消息队列
- **git2**: Git 操作库
- **JWT**: 用户认证

### 前端
- **Vue 3.4**: 渐进式 JavaScript 框架
- **TypeScript 5.x**: 类型安全
- **Vite 5.x**: 下一代构建工具
- **Pinia**: Vue 状态管理
- **Sass**: CSS 预处理器
- **Axios**: HTTP 客户端

### 数据库
- **PostgreSQL**: 主数据库
- **Redis**: 缓存、会话管理和消息队列

## 功能特性

- 🔐 用户认证和权限管理
- 📁 项目管理（创建、设置、成员管理）
- 🌿 分支管理（创建、删除、保护分支）
- 📝 提交历史浏览和比较
- 🏷️ 标签/发布管理
- 🔀 合并请求（创建、评论、审查、合并）
- ⚡ CI/CD 流水线
- 🔔 Webhook 集成
- 📂 代码浏览器

## 快速开始

### 前提条件

- Rust 1.70+
- Node.js 18+
- PostgreSQL 14+
- Redis 6+

### 配置

1. 复制环境配置文件：
```bash
cp .env.example .env
```

2. 编辑 `.env` 文件，配置数据库和 Redis 连接信息。

### 后端设置

1. 安装 SQLx CLI（如果未安装）：
```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

2. 运行数据库迁移：
```bash
sqlx migrate run
```

3. 启动后端服务：
```bash
cargo run
```

服务将在 `http://localhost:8080` 启动。

### 前端设置

1. 进入前端目录：
```bash
cd frontend
```

2. 安装依赖：
```bash
npm install
```

3. 启动开发服务器：
```bash
npm run dev
```

前端将在 `http://localhost:5173` 启动。

### 生产构建

```bash
# 前端构建
cd frontend
npm run build

# 后端构建
cargo build --release
```

## 项目结构

```
devops/
├── src/                    # 后端源码
│   ├── main.rs            # 入口文件
│   ├── config.rs          # 配置管理
│   ├── db.rs              # 数据库连接
│   ├── error.rs           # 错误处理
│   ├── queue.rs           # 消息队列
│   ├── models/            # 数据模型
│   ├── services/          # 业务逻辑
│   ├── handlers/          # HTTP 处理器
│   └── middleware/        # 中间件
├── migrations/            # 数据库迁移
├── frontend/              # 前端源码
│   ├── src/
│   │   ├── api/          # API 客户端
│   │   ├── components/   # Vue 组件
│   │   ├── views/        # 页面视图
│   │   ├── stores/       # Pinia 状态
│   │   ├── types/        # TypeScript 类型
│   │   ├── router/       # 路由配置
│   │   └── styles/       # 样式文件
│   ├── package.json
│   └── vite.config.ts
├── webide/                # WebIDE (VS Code Server)
│   ├── packages/
│   ├── static/
│   └── vite.config.ts
├── gitfox-workhorse/      # 反向代理服务器（生产环境）
│   ├── src/
│   │   ├── main.rs       # 入口
│   │   ├── config.rs     # 配置
│   │   ├── proxy.rs      # 反向代理
│   │   └── static_files.rs # 静态文件服务
│   ├── Cargo.toml
│   ├── README.md
│   ├── QUICKSTART.md
│   └── start.sh          # 启动脚本
├── gitfox-runner/         # CI/CD Runner
├── gitfox-shell/          # Git SSH Shell
├── Cargo.toml
└── .env.example
```

## 生产部署

### 使用 GitFox Workhorse（推荐）

GitFox Workhorse 是一个高性能的反向代理服务器，专为生产环境设计：

```bash
# 1. 构建前端和 WebIDE
cd frontend && npm run build && cd ..
cd webide && npm run build && cd ..

# 2. 启动后端（在另一个终端）
export SERVER_PORT=8081
cargo run --release

# 3. 启动 Workhorse
cd gitfox-workhorse
./start.sh release
```

Workhorse 提供：
- ✅ 静态文件服务（前端 SPA + WebIDE）
- ✅ API 反向代理到后端
- ✅ Git HTTP 协议支持
- ✅ 自动压缩和缓存
- ✅ CORS 支持
- ✅ WebSocket 支持

访问 `http://localhost:8080` 即可使用完整的 GitFox 平台。

详细文档：[gitfox-workhorse/README.md](gitfox-workhorse/README.md)

### 部署架构

```
客户端浏览器
    ↓
GitFox Workhorse (端口 8080)
    ├─→ 静态文件 (/, /-/ide/*)
    ├─→ Assets (/assets/*)
    └─→ 后端代理 (/api/*, /oauth/*, Git HTTP)
        ↓
   GitFox Backend (端口 8081)
        ↓
   PostgreSQL + Redis
```

## API 文档

### 认证
- `POST /api/v1/auth/register` - 用户注册
- `POST /api/v1/auth/login` - 用户登录
- `GET /api/v1/auth/me` - 获取当前用户
## 许可证

MIT License
