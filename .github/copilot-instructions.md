# GitFox DevOps Platform - Copilot Instructions

## 核心原则

  - 0. 比能跑重要的是在不该跑的时候不传播损坏
  - 1. 会让整个程序处于不安全状态的错误需要使用unwrap()处理
  - 2. 代码质量和安全性同样重要，不能为了追求代码质量而牺牲安全性
  - 3. 为了部分安全性而牺牲代码质量的情况需要在代码中添加注释说明原因
  - 4. 为了速度牺牲代码质量的情况需要在代码中添加注释说明原因
  - 5. 必须完全完整实现功能, 不能有任何stub, hack, todo, 简化, 不完全实现
## 项目概述

GitFox 是一个类似 GitLab 的 DevSecOps 平台，采用 **微服务架构**：
- **Backend (devops)**: Actix Web API + gRPC Auth 服务 `:8081`
- **GitLayer**: Git 操作 gRPC 服务（类似 Gitaly）`:50052`
- **Workhorse**: HTTP 反向代理 + Git HTTP + LFS `:8080`
- **Shell**: 独立 SSH 服务器（使用 russh）`:2222`
- **Runner**: WebSocket CI/CD 执行器
- **Omnibus**: 打包工具（生成单一超级二进制）
- **Frontend**: Vue 3 + TypeScript SPA
- **WebIDE**: VS Code Web（基于 openvscode-server）

## 系统架构

```
HTTP :8080              SSH :2222              CI/CD Runner (WS)
    │                       │                          │
    ▼                       ▼                          │
┌─────────────────┐  ┌─────────────────┐             │
│ gitfox-workhorse│  │  gitfox-shell   │             │
│  (反向代理/LFS/包管理)|  │ (独立SSH服务器)  │             │
└────────┬────────┘  └─────────┬───────┘             │
         │                     │                      │
    ┌────┼─────┬───────────────┼──gRPC Auth───────────┤
    │    │     │               │   :50051             │
  Static API/* Git HTTP         │                     │
  Files  OAuth *.git/*          │                     │
    │    │     │                │                     │
    │    ▼     │                │                     │
    │  ┌───────────────────┐   │                     │
    │  │   Main App :8081  │◄──┤                     │
    │  │  (API + gRPC Auth)│───┼─── WebSocket ───────┘
    │  └─────────┬─────────┘   │   /api/v1/runner/ws
    │            │              │
    │            ▼              ▼
    │      ┌──────────────────────────┐
    └─────►│  GitLayer :50052         │
           │  (Git 操作 gRPC)          │
           │  - Repository/Ref/Commit │
           │  - Blob/Tree/Diff        │
           │  - SmartHTTP/SSH/LFS     │
           └──────────┬───────────────┘
                      ▼
              ┌─────────────────┐
              │ Git Repositories│     ┌──────────────┐
              │    ./repos/     │     │ PostgreSQL   │
              └─────────────────┘     │ Redis        │
                                      └──────────────┘
```

**请求流示例**:
- **HTTP Git clone**: Client → Workhorse → GitLayer (SmartHttpService)
- **SSH Git push**: Client → Shell → GitLayer (SshService)
- **API 调用**: Client → Workhorse → Main App → PostgreSQL
- **CI/CD Job**: Runner (WS) ↔ Main App → GitLayer (clone repo)

## 组件详解

### 1. Main App (devops) - [src/](src/)

**入口**: [src/main.rs](src/main.rs#L19-L50) - 初始化数据库池、运行迁移、启动 HTTP/gRPC
**模块结构**:
```
src/
├── config.rs              # AppConfig::from_env()
├── error.rs               # AppError enum + ResponseError trait
├── db.rs                  # init_pg_pool(), init_redis_pool()
├── queue.rs               # RedisMessageQueue (CI/CD)
├── handlers/              # HTTP handlers + configure_routes()
├── middleware/auth.rs     # AuthenticatedUser extractor
├── models/                # DB models + request/response DTOs
├── services/              # 业务逻辑层
└── grpc/                  # gRPC Auth 服务实现
    └── auth_service.rs    # VerifyToken RPC (供 Shell/Workhorse 调用)
```

**认证机制** ([src/middleware/auth.rs](src/middleware/auth.rs#L1-L80)):
- **JWT**: `Authorization: Bearer <jwt_token>` → `AuthenticatedUser`
- **PAT**: `Authorization: Bearer gitfox-pat_<token>` → 自动识别并验证
- **OAuth2**: `Authorization: Bearer <oauth_token>` → SHA-256 哈希查找

**Handler 模式**:
```rust
// src/handlers/example.rs
use crate::{error::AppResult, middleware::AuthenticatedUser};

pub async fn my_handler(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,  // 自动验证 JWT/PAT/OAuth
    path: web::Path<(String, String)>,  // URL params
    body: web::Json<RequestDto>,        // JSON body
) -> AppResult<HttpResponse> {
    // auth.user_id, auth.username, auth.role, auth.scopes 可用
    // 返回 Ok(HttpResponse::Ok().json(...)) 或 Err(AppError::NotFound(...))
}
```

**路由注册** ([src/handlers/mod.rs](src/handlers/mod.rs)):
```rust
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/projects", web::get().to(projects::list))
            .route("/{namespace}/{project}", web::get().to(projects::detail))
            // ...
    );
}
```

### 2. GitLayer - [gitlayer/](gitlayer/)

**作用**: 所有 Git 操作的统一 RPC 服务（类似 GitLab Gitaly）

**gRPC 服务** ([gitlayer/proto/](gitlayer/proto/)):
| Service | Proto | 功能 |
|---------|-------|------|
| RepositoryService | [repository.proto](gitlayer/proto/repository.proto) | 创建/删除/fork 仓库 |
| RefService | ref.proto | 分支/标签操作 |
| CommitService | commit.proto | 查询提交历史 |
| BlobService | blob.proto | 读取文件内容 |
| TreeService | tree.proto | 列出目录树 |
| DiffService | diff.proto | 差异比较 |
| SmartHttpService | smarthttp.proto | Git HTTP Smart Protocol |
| SshService | smarthttp.proto | Git SSH Protocol |
| LfsService | lfs.proto | Git LFS 对象存储 |

**调用示例**:
```rust
// gitfox-shell/src/gitlayer_client.rs
let mut client = gitlayer::ssh_service_client::SshServiceClient::connect(addr).await?;
let response = client.handle_ssh_session(request).await?;
```

**环境变量**:
- `GITLAYER_LISTEN_ADDR`: gRPC 监听地址（默认 `0.0.0.0:9999`）
- `GITLAYER_STORAGE_PATH` / `GIT_REPOS_PATH`: 仓库存储路径

### 3. Workhorse - [gitfox-workhorse/](gitfox-workhorse/)

**作用**: 生产环境的统一 HTTP 入口（类似 GitLab Workhorse）

**路由优先级**:
1. `/-/ide/**` → WebIDE ([webide/dist/](webide/dist/))
2. `/{namespace}/{project}.git/**` → GitLayer (Git HTTP)
3. `/api/**`, `/oauth/**` → Main App `:8081`
4. `/assets/**` → 本地 assets 目录
5. `/**` → Frontend SPA ([frontend/dist/](frontend/dist/)) + SPA fallback

**关键模块**:
- [src/proxy.rs](gitfox-workhorse/src/proxy.rs): 反向代理到 Main App
- [src/lfs/](gitfox-workhorse/src/lfs/): Git LFS Batch API + 直接存储
- [src/gitlayer_client.rs](gitfox-workhorse/src/gitlayer_client.rs): 调用 GitLayer gRPC
- [src/auth_client.rs](gitfox-workhorse/src/auth_client.rs): 调用 Main App gRPC Auth

**启动**: `cd gitfox-workhorse && ./start.sh`（自动构建前端/WebIDE）

### 4. Shell - [gitfox-shell/](gitfox-shell/)

**作用**: 独立 SSH 服务器，处理 `git clone git@gitfox:namespace/project.git`

**实现**: 使用 [russh](https://docs.rs/russh) 库，无需系统 sshd

**流程**:
1. SSH 公钥认证 → 调用 Main App gRPC Auth 验证
2. 解析 Git 命令 → 调用 GitLayer SshService
3. 直接转发 stdin/stdout 到 Git 进程

**环境变量**:
- `GITFOX_SHELL_LISTEN_ADDR`: SSH 监听地址（默认 `0.0.0.0:2222`）
- `GITLAYER_ADDRESS`: GitLayer 地址（必需，如 `http://[::1]:50052`）

### 5. Runner - [gitfox-runner/](gitfox-runner/)

**作用**: CI/CD 执行器，通过 WebSocket 接收任务

**执行器类型**:
- **Shell**: 直接执行命令（隔离工作目录、检测危险命令）
- **Docker**: 容器隔离（推荐生产环境）

**安全特性** ([src/security.rs](gitfox-runner/src/security.rs)):
- 危险命令检测：`rm -rf /`, fork bomb
- 工作目录大小限制（默认 10GB）
- 自动清理构建目录
- 敏感路径隐藏（不在日志中暴露实际路径）

**注册**: `gitfox-runner register --url http://localhost:8080 --token <reg_token>`
**运行**: `gitfox-runner run --config ~/.config/gitfox-runner/config.toml`

### 6. Omnibus - [gitfox-omnibus/](gitfox-omnibus/)

**作用**: 打包工具，生成包含所有组件的单一二进制（类似 GitLab Omnibus）

**构建流程**:
```bash
cd gitfox-omnibus
cargo run --release -- build --output ./gitfox

# 跳过依赖构建（使用缓存）
cargo run --release -- build --skip-deps-build --output ./gitfox

# 清理构建产物（保留依赖缓存）
cargo run -- clean
```

**内部步骤**:
1. `npm run build` (frontend + webide)
2. `cargo build --release --target x86_64-unknown-linux-musl` (devops + workhorse + shell + gitlayer)
3. 收集 `migrations/*.sql`
4. 构建内置依赖 (PostgreSQL, Redis, Nginx) - 首次构建或使用缓存
5. 嵌入资源到 stub 程序（使用 `rust-embed`）
6. 编译 stub → 最终超级二进制

**依赖缓存**:
- 位置: `gitfox-omnibus/.build/deps-work/`
- 包含: PostgreSQL, Redis, Nginx, zlib, readline, openssl, icu 源码和编译产物
- 策略: 永久保留，使用 `use_cache: true` 自动复用
- 手动清理: `rm -rf gitfox-omnibus/.build/deps-work`

**运行时**:
```bash
./gitfox init           # 交互式配置向导（生成 gitfox.toml）
./gitfox start          # 启动所有服务
./gitfox stop           # 优雅关闭
./gitfox status         # 查看状态
```

## 前端架构

### Vue 3 SPA - [frontend/src/](frontend/src/)

**技术栈**: Vue 3 Composition API + TypeScript + Pinia + Vite

**目录结构**:
```
frontend/src/
├── api/index.ts           # ApiClient 类（封装所有 API 调用）
├── types/index.ts         # TypeScript 接口（对应后端 models）
├── stores/                # Pinia stores (auth, project, user...)
├── router/index.ts        # Vue Router + meta.requiresAuth
├── components/            # 可复用组件
├── views/                 # 页面组件
└── styles/                # Sass 变量和全局样式
```

**API 调用模式**:
```typescript
// frontend/src/api/index.ts
class ApiClient {
  projects = {
    list: () => this.get<Project[]>('/api/v1/projects'),
    get: (ns: string, name: string) => 
      this.get<Project>(`/api/v1/${ns}/${name}`),
  };
  auth = {
    login: (data: LoginRequest) => 
      this.post<LoginResponse>('/api/v1/auth/login', data),
  };
}

// 在组件中
import { api } from '@/api';
const projects = await api.projects.list();
```

**认证状态管理**:
```typescript
// frontend/src/stores/auth.ts
import { useAuthStore } from '@/stores/auth';

const authStore = useAuthStore();
authStore.setToken(token);  // 自动存储到 localStorage + 设置 axios header
```

### WebIDE - [webide/](webide/)

**基于**: `@gitpod/openvscode-server`（非 GitLab 的 fork）

**集成方式**:
- 主应用通过 `@gitfox/web-ide` 的 `start()` API 初始化
- 使用 OAuth2 token 调用 GitFox API
- BrowserFS 内存虚拟文件系统

**访问路径**: `http://localhost:8080/-/ide/?project=namespace/project`

## 开发工作流

### 本地开发（微服务模式）

```bash
# 终端 1: 启动后端 API + gRPC Auth
cargo run                 # :8081 (API) + :50051 (gRPC)

# 终端 2: 启动 GitLayer
cd gitlayer && cargo run  # :50052 (gRPC)

# 终端 3: 启动 Workhorse
cd gitfox-workhorse && ./start.sh  # :8080 (HTTP entry)

# 终端 4: 前端开发
cd frontend && npm run dev  # :5173 (Vite HMR)

# 终端 5 (可选): SSH 服务器
cd gitfox-shell && cargo run  # :2222 (SSH)

# 终端 6 (可选): CI Runner
cd gitfox-runner && cargo run -- run  # WebSocket 连接
```

### 生产部署（Omnibus 模式）

```bash
# 构建超级二进制
cd gitfox-omnibus
cargo run --release -- build --output /opt/gitfox/bin/gitfox

# 配置
/opt/gitfox/bin/gitfox init  # 生成 gitfox.toml

# 启动（包含所有组件）
/opt/gitfox/bin/gitfox start
```

### 数据库迁移

```bash
# 创建新迁移
# 手动创建 migrations/20240220000000_add_feature.sql

# 应用迁移（开发）
sqlx migrate run

# 生成离线元数据（用于编译时检查）
cargo sqlx prepare --workspace
```

### 测试

```bash
# Rust 单元测试
cargo test

# 前端测试
cd frontend && npm test

# 类型检查
cargo check
cd frontend && npm run type-check
```

## 代码约定

### Rust 后端
Gitness 
**错误处理** ([src/error.rs](src/error.rs)):
```rust
pub type AppResult<T> = Result<T, AppError>;

pub enum AppError {
    InternalError(String),   // 500
    NotFound(String),        // 404
    BadRequest(String),      // 400
    Unauthorized(String),    // 401
    Forbidden(String),       // 403
    Conflict(String),        // 409
    // ...
}
```

**Model 定义**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: i64,
    pub name: String,
    #[sqlx(try_from = "String")]  // PostgreSQL ENUM
    pub visibility: ProjectVisibility,
    pub created_at: DateTime<Utc>,
}
```

**验证** (使用 `validator` crate):
```rust
#[derive(Deserialize, Validate)]
pub struct CreateProjectRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(email)]
    pub email: Option<String>,
}
```

**项目路径格式**: `{namespace}/{project_name}.git`
- namespace 可以是用户名或组路径（如 `team/subteam`）

### TypeScript 前端

**组件风格**: 
```vue
<script setup lang="ts">
import { ref, computed } from 'vue';
import type { Project } from '@/types';

const props = defineProps<{ project: Project }>();
const loading = ref(false);
</script>
```

**样式**: Sass + CSS Modules（全局变量在 `frontend/src/styles/variables.scss`）

## CI/CD 配置

**配置目录**: `.gitfox/ci/*.yml` (类似 GitLab 15.7+ Component CI)

**示例** ([docs/gitfox-ci-config.md](docs/gitfox-ci-config.md#L1-L80)):
```yaml
# .gitfox/ci/build.yml
build:rust:
  stage: build
  script:
    - cargo build --release
  artifacts:
    paths: [target/release/]
  cache:
    key: cargo-cache
    paths: [.cargo/, target/]
  tags: [rust]
```

**自动触发**: 
- 创建 Runner 时自动创建 `.gitfox-ci.yml` 模板
- 每次 push 自动检测配置并触发 pipeline

## gRPC 通信

**Proto 定义**: [gitlayer/proto/](gitlayer/proto/)

**代码生成**: 在 `build.rs` 中使用 `tonic-build`:
```rust`
// gitlayer/build.rs
tonic_build::configure()
    .compile(&["proto/repository.proto"], &["proto/"])?;
```

**客户端调用示例**:
```rust
// gitfox-shell/src/gitlayer_client.rs
use gitlayer::repository_service_client::RepositoryServiceClient;

let mut client = RepositoryServiceClient::connect("http://[::1]:50052").await?;
let req = CreateRepositoryRequest { repository: Some(repo), ... };
let res = client.create_repository(req).await?.into_inner();
```

## 配置管理

### 环境变量（开发模式）

**Main App** (`.env`):
- `DATABASE_URL`: PostgreSQL 连接串
- `REDIS_URL`: Redis 连接串
- `JWT_SECRET`: JWT 签名密钥
- `GIT_REPOS_PATH`: 仓库存储路径（默认 `./repos`）
- `GRPC_AUTH_LISTEN_ADDR`: gRPC Auth 监听地址（`:50051`）

**Workhorse** (`gitfox-workhorse/config.toml`):
- `listen_addr`, `listen_port`: HTTP 监听地址
- `backend_url`: Main App 地址（`http://127.0.0.1:8081`）
- `gitlayer_url`: GitLayer 地址
- `frontend_dist`, `webide_dist`: 前端构建目录

### 统一配置（生产模式）

**Omnibus**: `gitfox.toml` (通过 `gitfox init` 生成)
- 替代分散的环境变量和配置文件
- 自动生成密钥（JWT_SECRET, GITFOX_SHELL_SECRET）
- 组件间自动协调（端口、地址）

## 常见任务速查

### 添加 API 端点

1. 定义 Model: `src/models/feature.rs`
   ```rust
   #[derive(Deserialize, Validate)]
   pub struct CreateFeatureRequest { ... }
   ```
2. 实现 Handler: `src/handlers/feature.rs`
   ```rust
   pub async fn create(auth: AuthenticatedUser, ...) -> AppResult<HttpResponse> { ... }
   ```
3. 注册路由: `src/handlers/mod.rs`
   ```rust
   .route("/features", web::post().to(feature::create))
   ```
4. 前端类型: `frontend/src/types/index.ts`
   ```typescript
   export interface Feature { ... }
   ```
5. API 方法: `frontend/src/api/index.ts`
   ```typescript
   features = { create: (data) => this.post('/api/v1/features', data) }
   ```

### 添加 gRPC 服务

1. 定义 Proto: `gitlayer/proto/myservice.proto`
2. 更新 `gitlayer/build.rs`: `tonic_build::compile(&["proto/myservice.proto"], ...)`
3. 实现服务: `gitlayer/src/services/myservice.rs`
4. 注册: `gitlayer/src/main.rs` → `Server::builder().add_service(...)`

### 添加数据库迁移

1. 创建 `migrations/20240220120000_add_feature.sql`
2. 本地应用: `sqlx migrate run`
3. 更新 Model: `src/models/*.rs`
4. 更新元数据: `cargo sqlx prepare --workspace`（用于离线编译检查）
