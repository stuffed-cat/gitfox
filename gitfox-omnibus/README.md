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
│    5. 生成 stub 程序源码                                          │
│    6. 编译 stub → 最终超级二进制                                   │
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
│                                                                 │
│  运行时行为:                                                     │
│    1. 首次启动解压到 /var/lib/gitfox/                            │
│    2. 启动 GitLayer (Git 操作 RPC)                              │
│    3. 启动 devops 后端 (API + gRPC Auth)                        │
│    4. 启动 workhorse (HTTP 代理)                                │
│    5. 处理信号，优雅关闭                                         │
│                                                                 │
│  SSH 需要单独配置系统 sshd (运行 gitfox setup-ssh)               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 架构 (v1.x)

```
                 HTTP :8080                   SSH :22
                     │                           │
                     ▼                           ▼
          ┌─────────────────┐           ┌─────────────────┐
          │   Workhorse     │           │     sshd        │
          └────────┬────────┘           └────────┬────────┘
                   │                             │
          ┌────────┴────────┐                    ▼
          │                 │           ┌─────────────────┐
       API/*            Git HTTP        │  gitfox-shell   │
          │             *.git/*         └────────┬────────┘
          ▼                 │                    │
    ┌──────────┐           │     gRPC           │
    │Main App  │◄──────────┼─────(Auth)─────────┤
    │  :8081   │           │    :50051          │
    └──────────┘           │                    │
                          │     gRPC           │
                          └──────┬─────────────┘
                                 │ (Git ops)
                                 ▼
                        ┌─────────────────┐
                        │    GitLayer     │
                        │     :50052      │
                        └─────────────────┘
```

## ✨ 核心特性

### 🎯 交互式配置向导
- **TUI 界面**：运行 `gitfox init` 启动友好的交互式配置向导
- **智能默认值**：自动从 URL 提取域名作为 WebAuthn RP ID
- **即时反馈**：实时验证输入，提供上下文帮助

### 🔐 自动安全配置
- **密钥自动生成**：JWT_SECRET、GITFOX_SHELL_SECRET 128字符随机密钥
- **随机管理员密码**：16字符强密码，避免易混淆字符
- **零配置启动**：内部组件自动协调（Workhorse ↔ Backend ↔ GitLayer）

### 📦 单一二进制
- **完全静态链接**：无运行时依赖，仅需 Linux 内核
- **嵌入式资源**：前端、WebIDE、迁移文件全部打包
- **一键部署**：复制二进制即可运行

### 🚀 快速启动
- **首次解压** ~3秒（解压嵌入的资源）
- **后续启动** ~1秒（直接启动服务）
- **优雅关闭**：正确处理 SIGTERM/SIGINT

### 🔄 迁移工具
- **自动升级**：`gitfox upgrade` 从旧版本迁移配置
- **SSH 设置**：`gitfox setup-ssh` 生成 sshd 集成脚本

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

# 保留临时文件（调试用）
./target/release/gitfox-omnibus build \
    --keep-temp \
    --output ./gitfox
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
#   - gitfox.env      # 主配置文件（JWT、密钥、数据库等）
#   - workhorse.toml  # HTTP 反向代理配置

# 2. 编辑配置文件
vim /var/lib/gitfox/gitfox.env

# 必须修改：
#   DATABASE_URL=postgres://user:pass@localhost/gitfox
#   REDIS_URL=redis://127.0.0.1:6379
#
# 已自动生成（无需修改）：
#   JWT_SECRET=<128字符随机密钥>
#   GITFOX_SHELL_SECRET=<128字符随机密钥>
#   INITIAL_ADMIN_PASSWORD=<随机密码>
#   GITFOX_BASE_URL=<你输入的公开地址>
#   WEBAUTHN_RP_ID=<你输入的域名>
#   WEBAUTHN_ORIGIN=<你输入的公开地址>
#
# 如果未启用SMTP但是后续启用，还需配置：
#   SMTP_HOST=smtp.gmail.com
#   SMTP_PORT=587
#   SMTP_USERNAME=your-email@gmail.com
#   SMTP_PASSWORD=your-app-password

# 3. 启动 GitFox
./gitfox start
```

**使用自定义数据目录**：

```bash
# 初始化到自定义目录
./gitfox --data-dir /opt/gitfox init

# 编辑配置
vim /opt/gitfox/gitfox.env

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

# 配置系统 sshd 集成
./gitfox setup-ssh

# 或者预览 sshd 配置（不实际修改）
./gitfox setup-ssh --dry-run
```

### SSH 配置 (新架构)

新版本使用系统 sshd 而非内置 SSH 服务器。初次安装或升级后需要配置：

```bash
# 1. 升级配置（如果从旧版本迁移）
./gitfox upgrade --yes

# 2. 配置 sshd 集成
./gitfox setup-ssh

# 3. 运行生成的设置脚本
sudo bash /var/lib/gitfox/setup-ssh.sh

# 4. 编辑 /etc/ssh/sshd_config 添加输出的配置
# 5. 重启 sshd
sudo systemctl restart sshd
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
        --keep-temp           保留临时文件
```

### gitfox (生成的超级二进制)

```
USAGE:
    gitfox [OPTIONS] [COMMAND]

COMMANDS:
    init        初始化配置 (生成配置文件模板)
    start       启动所有服务 (默认)
    migrate     只运行数据库迁移
    extract     解压嵌入的资源
    list        列出嵌入的资源
    version     显示版本信息
    upgrade     从旧版本升级配置
    setup-ssh   配置系统 sshd 集成

OPTIONS:
    --data-dir <DIR>  数据目录 [default: /var/lib/gitfox]
    -v, --verbose     启用调试日志

UPGRADE OPTIONS:
    --yes             自动确认所有变更

SETUP-SSH OPTIONS:
    --git-user <USER>   git 用户名 [default: git]
    --git-home <PATH>   git 用户 home 目录 [default: /var/opt/gitfox]
    --dry-run           只输出配置，不实际修改
```

**配置文件**：

运行 `gitfox init` 会启动**交互式配置向导**，让你配置：
- GitFox 公开访问地址（用于 OAuth 回调）
- HTTP 监听端口
- 是否启用 SMTP 邮件服务
- WebAuthn RP ID（域名）

然后自动生成两个配置文件，并**自动生成密钥**：

1. **gitfox.env** - 主配置文件，包含所有环境变量：
   
   **通过交互式向导配置的内容**：
   ```bash
   # 公开访问地址
   GITFOX_BASE_URL=http://localhost:8080
   
   # SMTP 启用状态
   SMTP_ENABLED=false
   
   # WebAuthn 配置
   WEBAUTHN_RP_ID=localhost
   WEBAUTHN_ORIGIN=http://localhost:8080
   ```
   
   **自动生成的配置（无需手动修改）**：
   ```bash
   # JWT 密钥（128字符随机密钥）
   JWT_SECRET=83e79a75bc1c8b568e5498b9e18cfd7d...
   
   # GitFox Shell 密钥（SSH Git 操作认证）
   GITFOX_SHELL_SECRET=2026c553bf708579571592d305c1a18e...
   GITFOX_SHELL_PATH=./data/bin/gitfox-shell
   
   # 初始管理员账号（首次启动时创建）
   INITIAL_ADMIN_USERNAME=admin
   INITIAL_ADMIN_EMAIL=admin@localhost
   INITIAL_ADMIN_PASSWORD=SZStGGWtDu7kjrnu  # 随机生成，请妥善保存
   
   # 后端内部端口（Workhorse 自动代理）
   SERVER_HOST=127.0.0.1
   SERVER_PORT=8081
   ```
   
   **需要手动配置的内容**：
   ```bash
   # PostgreSQL 数据库
   DATABASE_URL=postgres://gitfox:password@localhost:5432/gitfox
   
   # Redis
   REDIS_URL=redis://127.0.0.1:6379
   
   # SSH 配置
   SSH_ENABLED=true
   SSH_HOST=0.0.0.0
   SSH_PORT=22
   
   # 如果启用 SMTP，需配置：
   # SMTP_HOST=smtp.gmail.com
   # SMTP_PORT=587
   # SMTP_USERNAME=your-email@gmail.com
   # SMTP_PASSWORD=your-app-password
   
   # OAuth 配置（可选）
   # OAUTH_GITHUB_CLIENT_ID=...
   # OAUTH_GITHUB_CLIENT_SECRET=...
   
   # ... 更多配置见模板
   ```

2. **workhorse.toml** - HTTP 反向代理配置：
   
   **通过交互式向导配置的内容**：
   ```toml
   # 外部访问端口
   listen_addr = "0.0.0.0"
   listen_port = 8080  # 从向导中获取
   ```
   
   **自动协调的内部配置（无需修改）**：
   ```toml
   # 后端内部通信（自动协调）
   backend_url = "http://127.0.0.1:8081"
   ```
   
   **用户可配置**：
   ```toml
   # 静态文件路径（自动设置）
   frontend_dist_path = "/var/lib/gitfox/frontend"
   webide_dist_path = "/var/lib/gitfox/webide"
   
   # 性能参数
   max_upload_size = 104857600  # 100MB
   websocket_timeout = 3600     # 1 hour
   ```

**重要提示**：
- init 向导会收集公开地址、端口等基础配置
- 管理员密码会在 init 时打印，请妥善保存
- JWT_SECRET 和 GITFOX_SHELL_SECRET 是自动生成的，请勿修改
- Workhorse 和后端的内部通信端口（8081）自动协调，无需手动配置
- WebAuthn RP ID 会从公开地址自动提取域名作为默认值

## 目录结构

### 运行时数据目录 (/var/lib/gitfox)

```
/var/lib/gitfox/
├── gitfox.env              # 主配置文件 (gitfox init 生成)
├── workhorse.toml          # Workhorse 配置 (gitfox init 生成)
├── bin/                    # 解压的二进制
│   ├── devops
│   ├── gitfox-workhorse
│   ├── gitfox-shell
│   └── gitfox-shell-authorized-keys-check
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
sudo -u git vim /var/lib/gitfox/gitfox.env

# 5. 启动服务
sudo -u git /usr/local/bin/gitfox start
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
    #   - ./gitfox.env:/data/gitfox.env
    #   - ./workhorse.toml:/data/workhorse.toml
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
vi /data/gitfox.env
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
# 保留构建文件查看中间产物
./gitfox-omnibus build --keep-temp --output ./gitfox

# 查看构建目录结构 (在工作区内，避免占用 /tmp)
ls -la gitfox-omnibus/.build/
```

**注意**: 构建过程会在 `gitfox-omnibus/.build/` 目录下进行，其中:
- `assets/`: 收集的资源 (frontend, webide, binaries, migrations)
- `stub/`: 生成的 stub 项目 (包含 target/ 目录，可能达到 10+ GB)

不使用 `/tmp` 是因为 Cargo 编译的 target 目录非常大。
