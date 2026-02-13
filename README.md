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
├── Cargo.toml
└── .env.example
```

## API 文档

### 认证
- `POST /api/v1/auth/register` - 用户注册
- `POST /api/v1/auth/login` - 用户登录
- `GET /api/v1/auth/me` - 获取当前用户

### 项目
- `GET /api/v1/projects` - 项目列表
- `POST /api/v1/projects` - 创建项目
- `GET /api/v1/projects/:slug` - 项目详情
- `PUT /api/v1/projects/:slug` - 更新项目
- `DELETE /api/v1/projects/:slug` - 删除项目

### 仓库
- `GET /api/v1/projects/:slug/repository` - 仓库信息
- `GET /api/v1/projects/:slug/repository/tree` - 文件树
- `GET /api/v1/projects/:slug/repository/files` - 文件内容

### 分支
- `GET /api/v1/projects/:slug/branches` - 分支列表
- `POST /api/v1/projects/:slug/branches` - 创建分支
- `DELETE /api/v1/projects/:slug/branches/:name` - 删除分支

### 提交
- `GET /api/v1/projects/:slug/commits` - 提交列表
- `GET /api/v1/projects/:slug/commits/:sha` - 提交详情

### 合并请求
- `GET /api/v1/projects/:slug/merge-requests` - 合并请求列表
- `POST /api/v1/projects/:slug/merge-requests` - 创建合并请求
- `GET /api/v1/projects/:slug/merge-requests/:iid` - 合并请求详情
- `POST /api/v1/projects/:slug/merge-requests/:iid/merge` - 合并

### 流水线
- `GET /api/v1/projects/:slug/pipelines` - 流水线列表
- `POST /api/v1/projects/:slug/pipelines` - 触发流水线
- `GET /api/v1/projects/:slug/pipelines/:id` - 流水线详情

## 许可证

MIT License
