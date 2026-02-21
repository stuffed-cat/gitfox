# GitFox DevOps Platform - Copilot Instructions

## 记住以下信息：
  - 0. 比能跑重要的是在不该跑的时候不传播损坏
  - 1. 会让整个程序处于不安全状态的错误需要使用unwrap()处理
  - 2. 代码质量和安全性同样重要，不能为了追求代码质量而牺牲安全性
  - 3. 为了部分安全性而牺牲代码质量的情况需要在代码中添加注释说明原因
  - 4. 为了速度牺牲代码质量的情况需要在代码中添加注释说明原因
  
## 项目概述

GitFox 是一个类似 GitLab 的 DevSecOps 版本管理系统，采用 **Rust (Actix Web)** 后端 + **Vue 3 (TypeScript)** 前端 + **PostgreSQL/Redis** 存储的全栈架构。

## 架构关键点

### 后端 (Rust/Actix Web)

- **入口**: [src/main.rs](src/main.rs) - 初始化 HTTP 服务器、SSH 服务器、数据库连接池
- **路由配置**: [src/handlers/mod.rs](src/handlers/mod.rs) - 所有 API 路由定义在 `configure_routes()`
- **API 风格**: RESTful，路径模式为 `/api/v1/projects/{namespace}/{project}/...`
- **错误处理**: 使用 [src/error.rs](src/error.rs) 中的 `AppError` 枚举，实现 `ResponseError` trait
- **认证**: JWT Bearer Token，通过 [src/middleware/auth.rs](src/middleware/auth.rs) 的 `AuthenticatedUser` extractor

```rust
// 示例：添加新 handler
pub async fn my_handler(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    auth: AuthenticatedUser,  // 自动验证 JWT
) -> AppResult<HttpResponse> {
    // auth.user_id, auth.username 可用
}
```

### 前端 (Vue 3/TypeScript)

- **API 客户端**: [frontend/src/api/index.ts](frontend/src/api/index.ts) - 所有后端调用封装在 `ApiClient` 类
- **类型定义**: [frontend/src/types/index.ts](frontend/src/types/index.ts) - 与后端 model 对应的 TypeScript 接口
- **状态管理**: Pinia stores 在 `frontend/src/stores/` - 如 `useAuthStore()` 管理登录状态
- **路由**: [frontend/src/router/index.ts](frontend/src/router/index.ts) - 使用 `meta.requiresAuth` 控制访问

### 数据库

- **迁移文件**: `migrations/` 目录，命名格式 `YYYYMMDDHHMMSS_description.sql`
- **运行迁移**: `sqlx migrate run`（自动在启动时执行）
- **自定义类型**: 使用 PostgreSQL ENUM（如 `project_visibility`, `member_role`）

### SSH/Git 协议

- **内置 SSH 服务器**: [src/ssh/](src/ssh/) - 使用 `russh` 库，无需配置系统 sshd
- **Git HTTP Smart Protocol**: [src/handlers/git_http.rs](src/handlers/git_http.rs) - 支持 clone/push/pull
- **独立 Shell 组件**: [gitfox-shell/](gitfox-shell/) - 用于传统 sshd 集成

## 开发工作流

```bash
# 后端启动 (从项目根目录)
cargo run                        # HTTP :8080 + SSH :2222

# 前端启动
cd frontend && npm run dev       # Vite dev server :5173

# 数据库迁移
sqlx migrate run

# 类型检查
cd frontend && npm run type-check
cargo check
```

## 代码约定

### 后端

- Handler 返回类型: `AppResult<HttpResponse>` (= `Result<HttpResponse, AppError>`)
- Model 定义使用 `#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]`
- 请求验证使用 `validator` crate 的 `#[derive(Validate)]`
- 项目路径格式: `{namespace}/{project_name}.git`（namespace 可以是用户名或组路径）

### 前端

- 组件使用 `<script setup lang="ts">` 语法
- API 方法按资源分组: `api.projects.get()`, `api.auth.login()`
- 样式使用 Sass，变量在 `frontend/src/styles/variables.scss`

## 配置

关键环境变量（参见 [src/config.rs](src/config.rs)）:
- `DATABASE_URL`: PostgreSQL 连接串（必需）
- `JWT_SECRET`: JWT 签名密钥
- `GIT_REPOS_PATH`: Git 仓库存储路径（默认 `./repos`）
- `SSH_ENABLED`: 启用内置 SSH 服务器
- `GITFOX_SHELL_SECRET`: gitfox-shell 内部 API 认证

## 常见任务

### 添加新 API 端点

1. 在 `src/models/` 添加请求/响应结构
2. 在 `src/handlers/` 添加 handler 函数
3. 在 `src/handlers/mod.rs` 的 `configure_routes()` 注册路由
4. 在 `frontend/src/types/index.ts` 添加对应 TypeScript 类型
5. 在 `frontend/src/api/index.ts` 添加 API 方法

### 添加数据库迁移

1. 创建 `migrations/{timestamp}_{name}.sql`
2. 使用 `sqlx migrate run` 应用
3. 更新对应的 `src/models/*.rs`
