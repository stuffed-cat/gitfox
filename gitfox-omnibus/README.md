# GitFox Omnibus

GitFox Omnibus 是一个**打包工具**，用于将 GitFox 的所有组件打包成一个自包含的超级二进制。

## 概念

```
┌─────────────────────────────────────────────────────────────────┐
│                      gitfox-omnibus (打包工具)                   │
│                                                                 │
│  cargo build -p gitfox-omnibus                                  │
│  ./gitfox-omnibus build --output ./gitfox                       │
│                                                                 │
│  执行：                                                          │
│    1. npm run build (frontend)                                  │
│    2. npm run build (webide)                                    │
│    3. cargo build --target musl (devops, workhorse, shell)      │
│    4. 收集 migrations/*.sql                                      │
│    5. 构建内置依赖 (PostgreSQL, Redis, Nginx)                     │
│    6. 生成 stub 程序源码                                          │
│    7. 编译 stub → 最终超级二进制                                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    gitfox (超级二进制)                           │
│                                                                 │
│  内嵌 (rust-embed, 压缩):                                        │
│    - devops (后端 API + gRPC Auth)                              │
│    - gitfox-workhorse (HTTP 反向代理)                            │
│    - gitfox-shell (SSH 处理)                                    │
│    - gitlayer (Git 操作 RPC 服务)                               │
│    - frontend/dist/* (Vue SPA)                                  │
│    - webide/dist/* (VS Code Web)                                │
│    - migrations/*.sql                                           │
│    - 内置依赖: PostgreSQL, Redis, Nginx                          │
│                                                                 │
│  运行时行为:                                                     │
│    1. 首次启动解压到 /var/lib/gitfox/                            │
│    2. (如启用) 启动内置 PostgreSQL                               │
│    3. (如启用) 启动内置 Redis                                    │
│    4. 启动 GitLayer (Git 操作 RPC)                              │
│    5. 启动 devops 后端 (API + gRPC Auth)                        │
│    6. 启动 workhorse (HTTP 代理)                                │
│    7. 启动 gitfox-shell (独立 SSH 服务器)                        │
│    8. (如启用) 启动内置 Nginx (前端反向代理)                      │
│    9. 处理信号，优雅关闭                                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 架构

```
       HTTP :8080                   SSH :2222
           │                           │
           ▼                           ▼
┌─────────────────┐           ┌─────────────────┐
│   Workhorse     │           │  gitfox-shell   │
└────────┬────────┘           │  (独立 SSH 服务器│
         │                    │   使用 russh)   │
┌────────┴────────┐           └────────┬────────┘
│                 │                    │
API/*        Git HTTP                  │
│            *.git/*                   │
▼                 │     gRPC           │
┌──────────┐     │    (Auth)          │
│Main App  │◄────┼────:50051──────────┤
│  :8081   │     │                    │
└──────────┘     │     gRPC           │
                 └──────┬─────────────┘
                        │ (Git ops)
                        ▼
                ┌─────────────────┐
                │    GitLayer     │
                │     :50052      │
                └─────────────────┘
```

## ✨ 核心特性

### 🎯 交互式配置向导 (TUI)
- **友好的 TUI 界面**：运行 `gitfox init` 启动交互式配置向导，使用 `inquire` crate 实现
- **统一配置格式**：生成 `gitfox.toml` TOML 格式的统一配置文件，替代分散的 env 文件
- **智能默认值**：自动从 URL 提取域名作为 WebAuthn RP ID，推荐 Unix Socket 连接
- **实时验证**：输入验证和上下文帮助，避免配置错误

### 🔐 自动安全配置
- **密钥自动生成**：JWT_SECRET、GITFOX_SHELL_SECRET（64字符随机密钥）
- **随机管理员密码**：16字符强密码，避免易混淆字符
- **零配置启动**：内部组件自动协调（Workhorse ↔ Backend ↔ GitLayer）
- **向后兼容**：同时生成传统 `gitfox.env` 和 `workhorse.toml`（但推荐使用 `gitfox.toml`）

### 📦 单一二进制
- **完全静态链接**：无运行时依赖，仅需 Linux 内核
- **嵌入式资源**：前端、WebIDE、迁移文件全部打包
- **一键部署**：复制二进制即可运行

### 🗄️ 内置依赖 (GitLab Omnibus 风格)
- **内置 PostgreSQL**：总是打包，运行时可选启用
- **内置 Redis**：总是打包，运行时可选启用
- **内置 Nginx**：总是打包，运行时可选启用
- **灵活配置**：可在 `gitfox.toml` 中选择使用内置或外部服务
- **数据持久化**：内置服务数据存储在 data_dir 下

### �🚀 快速启动
- **首次解压** ~3秒（解压嵌入的资源）
- **后续启动** ~1秒（直接启动所有服务）
- **优雅关闭**：正确处理 SIGTERM/SIGINT
- **内置 SSH**：gitfox-shell 独立 SSH 服务器，无需系统 sshd

### 🔄 迁移工具
- **自动升级**：`gitfox upgrade` 从旧版本迁移配置

## 使用

### 1. 构建 omnibus 工具

```bash
cargo build --release -p gitfox-omnibus
```

### 2. 运行打包

```bash
# 基本用法
./target/release/gitfox-omnibus build --output ./gitfox

# 指定工作区
./target/release/gitfox-omnibus build \
    --workspace /path/to/devops \
    --output ./gitfox

# 跳过前端构建（使用已有的 dist）
./target/release/gitfox-omnibus build \
    --skip-frontend \
    --skip-webide \
    --output ./gitfox

# 跳过依赖构建（使用缓存）
./target/release/gitfox-omnibus build \
    --skip-deps-build \
    --output ./gitfox

# 清理构建产物（保留依赖缓存）
./target/release/gitfox-omnibus clean
```

### 3. 使用生成的超级二进制

**首次使用**：

```bash
# 1. 初始化配置文件（交互式）
./gitfox init

# 🚀 GitFox Configuration Wizard
#
# ? GitFox public URL (用于 OAuth 回调等): http://localhost:8080
#   例如: https://git.example.com or http://192.168.1.100:8080
#
# ? HTTP 监听端口: 8080
#   Workhorse 对外服务的端口
#
# ? 启用 SMTP 邮件服务? No
#   用于发送注册确认、密码重置等邮件
#
# ? WebAuthn RP ID (域名): localhost
#   用于 Passkey/WebAuthn 认证的域名（不含协议和端口）
#
# ============================================================
#   Initial Admin User (auto-generated)
# ============================================================
#   Username: admin
#   Email:    admin@localhost
#   Password: SZStGGWtDu7kjrnu
# ============================================================
# ⚠️  请保存此密码！
#
# 📝 Configuration Summary:
#    Public URL: http://localhost:8080
#    HTTP Port:  8080
#    SMTP:       Disabled
#    WebAuthn:   localhost

# 这会在 /var/lib/gitfox 生成：
#   - gitfox.toml           # 主配置文件（手动编辑）
#   - gitfox.env            # 后端配置（自动生成）
#   - gitlayer.env          # GitLayer 配置（自动生成）
#   - gitfox-shell.env      # SSH 服务器配置（自动生成）
#   - workhorse.toml        # Workhorse 配置（自动生成）
#
# 注意：除了 gitfox.toml，其他配置文件都是自动生成的，请勿手动编辑

# 2. 编辑配置文件
vim /var/lib/gitfox/gitfox.toml

# 必须修改：
#   [database]
#   url = "postgres://user:pass@localhost/gitfox"
#   
#   [redis]
#   url = "redis://127.0.0.1:6379"
#
# 已自动生成（无需修改）：
#   [secrets]
#   jwt = "<64字符随机密钥>"
#   internal = "<64字符随机密钥>"
#
#   [admin]
#   username = "admin"
#   email = "admin@localhost"
#   password = "<随机密码>"
#
#   [server]
#   base_url = "<你输入的公开地址>"
#
# 如果未启用SMTP但是后续启用，还需配置：
#   [smtp]
#   enabled = true
#   host = "smtp.gmail.com"
#   port = 587
#   username = "your-email@gmail.com"
#   password = "your-app-password"

# 3. 重新生成组件配置文件（修改 gitfox.toml 后必须执行）
./gitfox reconfigure

# 4. 启动 GitFox
./gitfox start
```

**使用自定义数据目录**：

```bash
# 初始化到自定义目录
./gitfox --data-dir /opt/gitfox init

# 编辑配置
vim /opt/gitfox/gitfox.toml

# 启动
./gitfox --data-dir /opt/gitfox start
```

**其他命令**：

```bash
# 只解压资源（不启动服务）
./gitfox extract

# 列出嵌入的资源
./gitfox list

# 运行迁移（迁移文件会被自动执行，无需手动运行）
./gitfox migrate

# 显示版本
./gitfox version

# 从旧版本升级配置
./gitfox upgrade

# 重新生成所有组件配置文件（从 gitfox.toml）
./gitfox reconfigure

# 验证配置
./gitfox config check
```

### SSH 配置

gitfox-shell 是内置的独立 SSH 服务器（使用 russh 库），无需配置系统 sshd。

SSH 服务器会在 `gitfox start` 时自动启动，默认监听 2222 端口。可以通过配置文件修改：

```bash
# 编辑 gitfox.toml，配置 SSH 端口
vim /var/lib/gitfox/gitfox.toml

# [server.ssh]
# port = 2222  # 修改为你需要的端口（如 22）

# 重启服务
./gitfox start
```

## 命令参考

### gitfox-omnibus (打包工具)

```
USAGE:
    gitfox-omnibus <COMMAND>

COMMANDS:
    build     构建超级二进制
    list      列出会被打包的组件
    verify    验证工作区结构

BUILD OPTIONS:
    -o, --output <PATH>       输出文件路径 [default: ./gitfox]
    -t, --target <TARGET>     Rust 编译目标 [default: x86_64-unknown-linux-musl]
    -w, --workspace <DIR>     工作区根目录
        --skip-frontend       跳过前端构建
        --skip-webide         跳过 WebIDE 构建
        --skip-rust           跳过 Rust 二进制编译
        --release             使用 release 模式 [default: true]
        --skip-deps-build     跳过依赖构建（使用缓存）

CLEAN COMMAND:
    clean                     清理构建产物（保留依赖缓存 .build/deps-work）
```

### gitfox (生成的超级二进制)

```
USAGE:
    gitfox [OPTIONS] [COMMAND]

COMMANDS:
    init        初始化配置 (生成配置文件模板)
    start       启动所有服务 (默认，包括 SSH 服务器)
    migrate     只运行数据库迁移
    extract     解压嵌入的资源
    list        列出嵌入的资源
    version     显示版本信息
    upgrade     从旧版本升级配置 (gitfox.env + workhorse.toml → gitfox.toml)
    reconfigure 重新生成所有组件配置文件 (从 gitfox.toml)
    config      验证和查看配置 (check, show, generate, migrate)

OPTIONS:
    --data-dir <DIR>  数据目录 [default: /var/lib/gitfox]
    -v, --verbose     启用调试日志

UPGRADE OPTIONS:
    --yes             自动确认所有变更
```

**配置文件**：

运行 `gitfox init` 会启动**交互式 TUI 配置向导**，让你配置：
- GitFox 公开访问地址（用于 OAuth 回调）
- HTTP 监听端口
- 后端连接方式（Unix Socket 或 TCP）
- SSH 配置（端口、公开地址等）
- 是否启用 SMTP 邮件服务
- WebAuthn RP ID（域名）

然后自动生成 **`gitfox.toml`** 统一配置文件，并**自动生成密钥**。

### gitfox.toml - 统一配置格式

`gitfox init` 生成的主配置文件，TOML 格式，包含所有组件的配置：

```toml
version = "1.0"

[database]
url = "postgres://gitfox:password@localhost:5432/gitfox"

[redis]
url = "redis://127.0.0.1:6379"

[secrets]
jwt = "83e79a75bc1c8b568e5498b9e18cfd7d..."        # 自动生成
internal = "2026c553bf708579571592d305c1a18e..."  # 自动生成

[server]
base_url = "http://localhost:8080"
http_port = 8080
max_upload_size = 1073741824  # 1GB

[server.ssh]
enabled = true
host = "0.0.0.0"
port = 2222
public_host = "localhost"
public_port = 2222

[internal]
backend_host = "127.0.0.1"       # 或使用 backend_socket
backend_port = 8081
gitlayer_port = 50052
auth_grpc_port = 50051

[paths]
repos = "/var/lib/gitfox/repos"
frontend = "/var/lib/gitfox/frontend"
webide = "/var/lib/gitfox/webide"
assets = "/var/lib/gitfox/assets"
ssh_host_key = "/var/lib/gitfox/ssh/host_key"

[admin]
username = "admin"
email = "admin@localhost"
password = "SZStGGWtDu7kjrnu"  # 自动生成，请妥善保存

[smtp]
enabled = false
host = "smtp.gmail.com"
port = 587
# ... 更多 SMTP 配置

[oauth]
# OAuth 提供商配置（可选）
[oauth.github]
client_id = ""
client_secret = ""

[logging]
level = "info"
```

**配置说明**：

**自动生成的内容**（无需修改）：
- `secrets.jwt` - JWT 令牌签名密钥（64字符随机）
- `secrets.internal` - 组件间通信密钥（64字符随机）
- `admin.password` - 初始管理员密码（16字符随机）

**通过 TUI 向导配置的内容**：
- `server.base_url` - 公开访问地址
- `server.http_port` - HTTP 端口
- `server.ssh.*` - SSH 配置
- `smtp.*` - SMTP 邮件配置（如启用）

**需要手动修改的内容**：
- `database.url` - PostgreSQL 连接串
- `redis.url` - Redis 连接串
- `oauth.*` - OAuth 提供商配置（可选）

### 配置文件生成流程

GitFox 使用分层配置架构：

```
gitfox.toml (主配置，手动编辑)
    ↓
gitfox reconfigure (自动生成)
    ↓
├── gitfox.env         (devops 后端)
├── gitlayer.env       (Git 操作服务)
├── gitfox-shell.env   (SSH 服务器)
└── workhorse.toml     (HTTP 代理)
```

**重要**：
- 只有 `gitfox.toml` 需要手动编辑
- 其他配置文件由 `gitfox reconfigure` 自动生成
- 修改 `gitfox.toml` 后必须运行 `gitfox reconfigure`
- `gitfox init` 会自动调用 `reconfigure` 生成所有配置

### 向后兼容

`gitfox init` 同时也会生成传统的 `gitfox.env` 和 `workhorse.toml` 配置文件（为了向后兼容）。但**推荐使用 `gitfox.toml`** 统一配置格式。

如果同时存在 `gitfox.toml` 和传统配置文件，GitFox 会优先使用 `gitfox.toml`。

**重要提示**：
- TUI 向导会自动推荐最佳配置（如 Unix Socket 连接）
- 管理员密码会在 init 完成后打印，请谨慎保存
- WebAuthn RP ID 会从公开地址自动提取域名作为默认值
- 所有密钥都是自动生成的，无需手动配置

## 目录结构

### 运行时数据目录 (/var/lib/gitfox)

```
/var/lib/gitfox/
├── gitfox.toml             # 主配置文件 (手动编辑)
├── backend.env             # 后端配置 (自动生成)
├── gitfox.env              # 后端配置 (自动生成)
├── gitlayer.env            # GitLayer 配置 (自动生成)
├── gitfox-shell.env        # SSH 服务器配置 (自动生成)
├── workhorse.toml          # Workhorse 配置进制
│   ├── devops
│   ├── gitfox-workhorse
│   ├── gitfox-shell
│   └── gitlayer
├── frontend/               # 解压的前端静态文件
│   ├── index.html
│   └── assets/
├── webide/                 # 解压的 WebIDE 静态文件
│   ├── index.html
│   └── ...
├── migrations/             # 解压的迁移文件
│   └── *.sql
├── repos/                  # Git 仓库存储
├── assets/                 # 用户上传文件
└── ssh/                    # SSH 密钥
```

## 前置要求

### 构建 omnibus 工具

- Rust 1.75+
- Node.js 18+
- npm 9+

### 构建超级二进制 (musl 静态链接)

```bash
# Ubuntu/Debian
sudo apt install musl-tools

# 安装 Rust musl 目标
rustup target add x86_64-unknown-linux-musl
```

**注意**: GitFox 的静态链接配置：
- **git2**: `vendored-libgit2` + `vendored-openssl` - 静态链接 libgit2 和 OpenSSL
- **webauthn-rs**: 依赖 openssl，通过在根项目添加 `openssl-sys = { features = ["vendored"] }` 解决
- **reqwest/lettre**: 使用 `rustls-tls` 而非 OpenSSL，纯 Rust 实现

所有依赖项已配置为完全静态链接，无需运行时动态库。

### 构建成功示例

完整构建输出示例：

```
2026-02-26T05:07:52.180244Z  INFO gitfox_omnibus::build: Stub program compiled successfully
2026-02-26T05:07:52.277880Z  INFO gitfox_omnibus::build: Build completed successfully!
2026-02-26T05:07:52.277892Z  INFO gitfox_omnibus::build: Output: ./gitfox
2026-02-26T05:07:52.277894Z  INFO gitfox_omnibus::build: Size: 124827680 bytes (119.04 MB)
2026-02-26T05:07:52.277898Z  INFO gitfox_omnibus::build: SHA256: 351dcf6129cfed0034afee1853d4405832ce3a09ec012298d186d08eed4de737

real    2m30.291s
```

嵌入资源统计（`./gitfox list` 输出）：

```
Binaries:
  devops (35345376 bytes / 33.7 MB)
  gitfox-shell (9556176 bytes / 9.1 MB)
  gitfox-shell-authorized-keys-check (5956112 bytes / 5.7 MB)
  gitfox-workhorse (12117704 bytes / 11.6 MB)

Frontend: 204 files (16 MB)
WebIDE: 3209 files (250 MB)
Migrations: 28 files (160 KB)

压缩后总大小: 119.04 MB (原始 ~327 MB，压缩率 36%)
```

## 部署示例

### 快速开始

```bash
# 1. 下载/构建 gitfox 二进制
./gitfox-omnibus build --output /usr/local/bin/gitfox

# 2. 创建数据目录
sudo mkdir -p /var/lib/gitfox
sudo chown git:git /var/lib/gitfox

# 3. 初始化配置
sudo -u git /usr/local/bin/gitfox init

# 4. 编辑配置
sudo -u git vim /var/lib/gitfox/gitfox.toml

# 5. 启动服务
sudo -u git /usr/local/bin/gitfox start
```

### 使用内置依赖 (All-in-One 部署)

GitFox Omnibus **总是**打包内置的 PostgreSQL、Redis、Nginx（类似 GitLab Omnibus）。
用户在**运行时**通过 `gitfox.toml` 选择使用内置服务还是外部服务。

```bash
# 1. 构建超级二进制（自动包含所有内置依赖）
./gitfox-omnibus build --output /usr/local/bin/gitfox

# 2. 初始化配置
sudo -u git /usr/local/bin/gitfox init

# 3. 编辑配置，启用内置服务
sudo -u git vim /var/lib/gitfox/gitfox.toml
```

在 `gitfox.toml` 中启用内置服务：

```toml
[bundled]
enabled = true

[bundled.postgresql]
enabled = true
port = 5432
database = "gitfox"
username = "gitfox"

[bundled.redis]
enabled = true
port = 6379

[bundled.nginx]
enabled = true
http_port = 80
https_port = 443
ssl_enabled = true
ssl_certificate = "/path/to/cert.pem"
ssl_certificate_key = "/path/to/key.pem"
```

**注意**: 
- 使用内置依赖时，`database.url` 和 `redis.url` 配置将被忽略
- 从旧版本升级时，如果已配置外部服务，内置服务会自动禁用

### 使用缓存加速构建

如果只想跳过依赖编译（使用已缓存的）：

```bash
# 使用缓存的内置依赖
./gitfox-omnibus build --skip-deps-build --output ./gitfox
```

### 单独预编译内置依赖

如果只想预编译内置依赖（用于缓存或分发）：

```bash
# 构建所有依赖
./gitfox-omnibus build-deps --all

# 或单独构建
./gitfox-omnibus build-deps --postgresql --redis
./gitfox-omnibus build-deps --nginx

# 指定输出目录
./gitfox-omnibus build-deps --all --output ./deps-cache
```

### systemd 服务

```ini
# /etc/systemd/system/gitfox.service
[Unit]
Description=GitFox DevOps Platform
After=network.target postgresql.service redis.service

[Service]
Type=simple
User=git
Group=git
WorkingDirectory=/var/lib/gitfox
ExecStart=/usr/local/bin/gitfox start
Restart=always
RestartSec=5

# 环境变量（可选，优先级高于配置文件）
# Environment="DATABASE_URL=postgres://gitfox:password@localhost/gitfox"
# Environment="REDIS_URL=redis://127.0.0.1:6379"

[Install]
WantedBy=multi-user.target
```

启动服务：

```bash  
sudo systemctl daemon-reload
sudo systemctl enable gitfox
sudo systemctl start gitfox
sudo systemctl status gitfox
```

### Docker

**Dockerfile**:

```dockerfile
FROM scratch
COPY gitfox /gitfox
WORKDIR /data
ENTRYPOINT ["/gitfox"]
CMD ["start"]
```

**docker-compose.yml**:

```yaml
version: '3'

services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_USER: gitfox
      POSTGRES_PASSWORD: password
      POSTGRES_DB: gitfox
    volumes:
      - postgres-data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    volumes:
      - redis-data:/data

  gitfox:
    image: gitfox:latest
    depends_on:
      - postgres
      - redis
    ports:
      - "80:8080"
      - "22:22"
    volumes:
      - gitfox-data:/data
    environment:
      DATABASE_URL: postgres://gitfox:password@postgres:5432/gitfox
      REDIS_URL: redis://redis:6379
      JWT_SECRET: change-me-to-random-secret
      GITFOX_BASE_URL: http://localhost
      SSH_PUBLIC_HOST: localhost
    # 或者挂载配置文件：
    # volumes:
    #   - ./gitfox.toml:/data/gitfox.toml
    #   - gitfox-data:/data

volumes:
  postgres-data:
  redis-data:
  gitfox-data:
```

启动：

```bash
# 构建镜像
docker build -t gitfox:latest .

# 初始化配置（首次运行）
docker run --rm -v gitfox-data:/data gitfox:latest init

# 编辑配置
docker run --rm -it -v gitfox-data:/data alpine sh
vi /data/gitfox.toml
exit

# 启动所有服务
docker-compose up -d
```

## 与 GitLab Omnibus 对比

| 特性 | GitFox Omnibus | GitLab Omnibus |
|------|----------------|----------------|
| 打包方式 | Rust + rust-embed | Chef + fpm |
| 输出格式 | 单一静态二进制 | .deb/.rpm 包 |
| 运行时依赖 | 仅 Linux 内核 | 系统库 |
| 资源存储 | 嵌入二进制，首次运行解压 | 解压到文件系统 |
| 配置方式 | 交互式 TUI + .env + TOML | gitlab.rb (Ruby DSL) |
| 配置管理 | `gitfox init` 交互式向导 | `gitlab-ctl reconfigure` |
| 密钥生成 | 自动生成（JWT、Shell、Admin密码） | 手动配置或chef生成 |
| 大小 | ~120MB | ~1GB |
| 启动速度 | <3s (首次解压) | ~30s |
| 服务管理 | 内置进程管理 | runit/systemd |

## 开发

### 项目结构

```
gitfox-omnibus/
├── Cargo.toml          # omnibus 工具的依赖
├── README.md
├── src/
│   ├── main.rs         # CLI 入口
│   ├── build.rs        # 打包逻辑
│   └── stub.rs         # stub 项目生成器 (复制模板并替换路径)
└── stub/               # stub 程序模板 (真正的 Rust 代码)
    ├── Cargo.toml.template
    ├── build.rs.template
    └── src/
        └── main.rs     # 超级二进制的完整源码
```

### stub/ 子目录

`stub/` 目录包含超级二进制的源代码模板：
- **src/main.rs**: 完整的 Rust 程序，有语法高亮和类型检查支持
- **Cargo.toml.template**: 依赖声明模板
- **build.rs.template**: 构建脚本模板，包含路径占位符 `{{FRONTEND_PATH}}` 等

打包时，omnibus 会复制这些文件，替换占位符，然后编译。

### 打包流程详解

1. **build_frontend**: 运行 `npm ci && npm run build`，复制 dist/
2. **build_webide**: 同上
3. **build_rust_binaries**: 使用 musl 目标编译所有 Rust 组件
4. **copy_migrations**: 复制 migrations/*.sql
5. **generate_stub_project**: 生成临时 Rust 项目，使用 rust-embed 嵌入所有资源
6. **compile_stub**: 编译临时项目，生成最终二进制

### 调试

```bash
# 查看构建目录结构 (在工作区内，避免占用 /tmp)
ls -la gitfox-omnibus/.build/

# 清理构建产物（保留依赖缓存）
./gitfox-omnibus clean
```

**注意**: 构建过程会在 `gitfox-omnibus/.build/` 目录下进行，其中:
- `assets/`: 收集的资源 (frontend, webide, binaries, migrations) - 每次构建会清理
- `stub/`: 生成的 stub 项目 (包含 target/ 目录) - 每次构建会清理
- `deps-work/`: 依赖源码和编译缓存 (PostgreSQL, Redis, Nginx 等) - **永久保留**

依赖缓存策略:
- 首次构建会克隆依赖源码并编译（可能需要 30+ 分钟）
- 后续构建会复用缓存（只需几秒）
- 手动清理依赖缓存: `rm -rf gitfox-omnibus/.build/deps-work`

不使用 `/tmp` 是因为 Cargo 编译的 target 目录非常大。
