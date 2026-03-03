# GitFox Shell

GitFox Shell 是 GitFox 的 **内置 SSH 服务器**，类似于 GitLab Shell。它直接提供 Git over SSH 协议支持，无需依赖系统 sshd。

## 架构

```
           HTTP :8080                        SSH :22
               │                                │
               ▼                                ▼
    ┌─────────────────┐               ┌─────────────────┐
    │   Workhorse     │               │  gitfox-shell   │
    │  (HTTP 代理)    │               │  (SSH 服务器)   │
    └────────┬────────┘               └────────┬────────┘
             │                                 │
    ┌────────┴────────┐                       │
    │                 │                       │
 API/*            Git HTTP                    │
    │             *.git/*                     │
    ▼                 │        gRPC           │
┌──────────┐         │       (Auth)          │
│Main App  │◄────────┼────────────────────────┤
│  :8081   │         │       :50051          │
└──────────┘         │                       │
                    │        gRPC           │
                    │      (Git ops)        │
                    └──────────┬────────────┘
                               ▼
                    ┌─────────────────┐
                    │    GitLayer     │
                    │     :50052      │
                    │   (Git 操作)    │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │ Git Repositories│
                    │    ./repos/     │
                    └─────────────────┘
```

**GitFox Shell 是一个独立的 SSH 服务器，不依赖系统 sshd。**

请求流：
- **HTTP Git**: `客户端 → Workhorse → GitLayer`
- **SSH Git**: `客户端 → gitfox-shell → GitLayer`
- **认证**: 都通过 Main App 的 gRPC Auth 服务

## 功能特性

- **内置 SSH 服务器**: 使用 russh 库实现，无需系统 sshd
- **公钥认证**: 通过 gRPC 查询用户 SSH 公钥
- **权限控制**: 通过 gRPC 与主应用通信进行权限验证
- **Git 操作**: 支持 `git clone`, `git push`, `git pull`, `git fetch`
- **Git LFS**: 支持 Git Large File Storage 认证
- **GitLayer 集成**: 通过 gRPC 调用 GitLayer 执行 Git 操作
- **安全限制**: 禁止交互式 shell 访问，禁止端口转发

## 安装

### 1. 编译

```bash
cd gitfox-shell
cargo build --release
```

### 2. 安装

```bash
sudo cp target/release/gitfox-shell /usr/local/bin/
sudo chmod 755 /usr/local/bin/gitfox-shell
```

### 3. 配置

创建配置文件 `/etc/gitfox/shell.env`:

```env
# SSH 服务器配置
SSH_LISTEN_ADDR=0.0.0.0:22
SSH_HOST_KEY_PATH=/var/lib/gitfox/ssh_host_key

# gRPC 认证服务
GITFOX_USE_GRPC_AUTH=true
AUTH_GRPC_ADDRESS=http://[::1]:50051

# GitLayer 服务 (必需)
GITLAYER_ADDRESS=http://[::1]:50052
```

### 4. 配置环境变量

创建 `/etc/gitfox/shell.env`:

```bash
# ============================================
# SSH 服务器配置
# ============================================
# 监听地址
SSH_LISTEN_ADDR=0.0.0.0:22

# Host Key 路径 (不存在时自动生成)
SSH_HOST_KEY_PATH=/etc/gitfox/ssh_host_ed25519_key

# ============================================
# gRPC 配置
# ============================================
# Auth gRPC 服务地址（主应用提供）
AUTH_GRPC_ADDRESS=http://[::1]:50051

# GitLayer gRPC 服务地址（所有 Git 操作通过 GitLayer 执行）
GITLAYER_ADDRESS=http://[::1]:50052

# 内部 API 认证密钥
GITFOX_API_SECRET=your-secret-token-here

# ============================================
# Debug (生产环境关闭)
# ============================================
GITFOX_DEBUG=false
RUST_LOG=info
```

### 5. 启动服务

```bash
# 直接运行
gitfox-shell server --listen 0.0.0.0:22 --host-key /etc/gitfox/ssh_host_ed25519_key

# 使用 systemd
sudo systemctl enable gitfox-shell
sudo systemctl start gitfox-shell
```

### 6. Systemd 服务单元

创建 `/etc/systemd/system/gitfox-shell.service`:

```ini
[Unit]
Description=GitFox Shell SSH Server
After=network.target gitfox.service

[Service]
Type=simple
User=git
Group=git
EnvironmentFile=/etc/gitfox/shell.env
ExecStart=/usr/bin/gitfox-shell server --listen ${SSH_LISTEN_ADDR} --host-key ${SSH_HOST_KEY_PATH}
Restart=always
RestartSec=5

# 安全设置
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=/etc/gitfox
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

## 环境变量

gitfox-shell 会从以下位置按优先级加载配置（优先级从高到低）：
1. 当前目录的 `.env` 文件
2. `/etc/gitfox/shell.env`
3. `~/.gitfox/shell.env`

后加载的文件不会覆盖已设置的环境变量。

### SSH 服务器配置

| 变量名 | 必需 | 默认值 | 描述 |
|--------|------|--------|------|
| `SSH_LISTEN_ADDR` | 否 | `0.0.0.0:22` | SSH 服务监听地址 |
| `SSH_HOST_KEY_PATH` | 否 | `./ssh_host_key` | Host Key 文件路径 |

### gRPC 配置

| 变量名 | 必需 | 默认值 | 描述 |
|--------|------|--------|------|
| `AUTH_GRPC_ADDRESS` | **是** | - | Auth gRPC 服务地址 |
| `GITLAYER_ADDRESS` | **是** | - | GitLayer gRPC 服务地址 |

### 通用配置

| 变量名 | 必需 | 默认值 | 描述 |
|--------|------|--------|------|
| `GITFOX_API_SECRET` | **是** | - | 内部 API 认证密钥（与主应用保持一致）|
| `GITFOX_DEBUG` | 否 | `false` | 启用调试日志 |
| `RUST_LOG` | 否 | `info` | 日志级别 (debug, info, warn, error) |

**注意**: gitfox-shell 通过 GitLayer 执行所有 Git 操作，不直接访问仓库目录。仓库路径只需要在 GitLayer 中配置。

## gRPC 接口

当使用 gRPC 模式时，gitfox-shell 调用主应用的 Auth gRPC 服务。

### CheckSSHAccess

检查用户是否有权限访问仓库。

**请求 (SSHAccessRequest):**
```protobuf
message SSHAccessRequest {
    string key_id = 1;      // SSH密钥ID (格式: "key-123")
    string repo_path = 2;   // 仓库路径 (格式: "namespace/project")
    string action = 3;      // Git操作: "git-upload-pack" 或 "git-receive-pack"
    string protocol = 4;    // 协议: "ssh"
}
```

**响应 (SSHAccessResponse):**
```protobuf
message SSHAccessResponse {
    bool status = 1;              // 是否允许访问
    string message = 2;           // 拒绝原因
    int64 user_id = 3;           // 用户ID
    string username = 4;          // 用户名
    bool can_write = 5;          // 是否有写权限
    int64 project_id = 6;        // 项目ID
    string repository_path = 7;  // 仓库在磁盘上的路径
    string gitlayer_address = 8; // GitLayer gRPC地址
}
```

### FindSSHKey

根据 SSH 公钥指纹查找用户。

**请求 (FindSSHKeyRequest):**
```protobuf
message FindSSHKeyRequest {
    string fingerprint = 1;  // SSH密钥指纹 (SHA256:xxx)
}
```

**响应 (FindSSHKeyResponse):**
```protobuf
message FindSSHKeyResponse {
    bool found = 1;
    SSHKeyInfo key = 2;
}

message SSHKeyInfo {
    int64 id = 1;
    int64 user_id = 2;
    string username = 3;
    string key_type = 4;      // ssh-rsa, ssh-ed25519 等
    string public_key = 5;
}
```

## HTTP API 接口 (备选)

### POST /api/internal/allowed

检查用户是否有权限访问仓库。

**请求:**
```json
{
    "key_id": "key-123",
    "repo_path": "owner/repo",
    "action": "git-upload-pack",
    "protocol": "ssh"
}
```

**响应:**
```json
{
    "status": true,
    "user_id": 1,
    "username": "admin",
    "can_write": true,
    "project_id": 42,
    "repository_id": 100
}
```

### POST /api/internal/keys/find

根据 SSH 公钥指纹查找用户。

**请求:**
```json
{
    "fingerprint": "SHA256:..."
}
```

**响应:**
```json
{
    "id": 123,
    "user_id": 1,
    "username": "admin",
    "key_type": "ssh-ed25519",
    "key": "ssh-ed25519 AAAA..."
}
```

## Git Hooks

GitFox Shell 在执行 Git 命令时会设置以下环境变量，供 Git hooks 使用：

| 变量名 | 描述 |
|--------|------|
| `GL_ID` | 用户标识符 (格式: `user-{id}`) |
| `GL_USERNAME` | 用户名 |
| `GL_REPOSITORY` | 仓库路径 |
| `GL_PROTOCOL` | 协议 (`ssh`) |
| `GL_PROJECT_PATH` | 项目路径 |
| `GITFOX_USER_ID` | 用户 ID |
| `GITFOX_USERNAME` | 用户名 |
| `GITFOX_REPO_PATH` | 仓库路径 |
| `GITFOX_PROJECT_ID` | 项目 ID |

## 使用方式

用户配置 SSH 后，可以这样使用：

```bash
# Clone
git clone git@your-server.com:owner/repo.git

# Push
git push origin main

# Pull
git pull origin main
```

## 安全特性

1. **无交互式 Shell**: 用户无法获得交互式 shell 访问
2. **命令白名单**: 只允许执行特定的 Git 命令 (`git-upload-pack`, `git-receive-pack`)
3. **路径验证**: 防止路径遍历攻击
4. **gRPC 认证**: 所有认证通过 gRPC 内部通信
5. **公钥认证**: 只支持 SSH 公钥认证，禁用密码
6. **禁用危险功能**: 禁用端口转发、X11 转发、代理转发

## 故障排除

### 查看日志

```bash
# 使用 systemd
sudo journalctl -u gitfox-shell -f

# 启用调试模式
RUST_LOG=debug gitfox-shell server --listen 0.0.0.0:2222
```

### 常见问题

**1. Permission denied (publickey)**
- 检查 SSH 公钥是否已添加到 GitFox 账户
- 验证公钥指纹是否匹配
- 检查 Auth gRPC 服务是否正常运行

**2. Connection refused**
- 确认 gitfox-shell 服务正在运行
- 检查防火墙设置（端口 22）
- 验证监听地址配置

**3. Repository not found**
- 确认仓库路径正确
- 检查用户是否有访问权限

**4. gRPC connection failed**
- 检查 `AUTH_GRPC_ADDRESS` 和 `GITLAYER_ADDRESS` 配置
- 验证 GitFox 主服务和 GitLayer 是否运行
- 检查网络连接

## 开发

```bash
# 运行测试
cargo test

# 构建调试版本
cargo build

# 运行 SSH 服务器 (开发模式，使用非特权端口)
RUST_LOG=debug \
AUTH_GRPC_ADDRESS=http://localhost:50051 \
GITLAYER_ADDRESS=http://localhost:50052 \
cargo run -- server --listen 127.0.0.1:2222

# 测试 SSH 连接
ssh -p 2222 -T git@localhost
```

## CLI 命令

### server 模式 (主模式)

启动独立 SSH 服务器：

```bash
gitfox-shell server [OPTIONS]

Options:
  -l, --listen <ADDR>      监听地址 [default: 0.0.0.0:22]
  -k, --host-key <PATH>    Host Key 文件路径 (不存在时自动生成 Ed25519 密钥)
  -h, --help               显示帮助
```

### session 模式 (兼容模式)

用于传统 sshd 集成（不推荐）：

```bash
gitfox-shell session <KEY_ID>

# 或直接传递 key_id
gitfox-shell <KEY_ID>
```

## License

MIT
