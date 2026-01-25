# GitFox Shell

GitFox Shell 是 GitFox DevOps 平台的 SSH 访问组件，负责处理 Git over SSH 协议的认证和授权。

## 功能特性

- **SSH 认证**: 通过 `AuthorizedKeysCommand` 动态查找用户 SSH 公钥
- **权限控制**: 基于 API 的细粒度仓库访问权限验证
- **Git 操作**: 支持 `git clone`, `git push`, `git pull`, `git fetch`
- **Git LFS**: 支持 Git Large File Storage 认证
- **安全限制**: 禁止交互式 shell 访问，防止端口转发等

## 组件

### gitfox-shell

主要的 shell 程序，当用户通过 SSH 执行 Git 命令时被调用。

```
用户 → SSH → sshd → gitfox-shell → GitFox API → git-upload-pack/git-receive-pack
```

### gitfox-shell-authorized-keys-check

用于 sshd 的 `AuthorizedKeysCommand`，动态从 GitFox 数据库查找 SSH 公钥。

## 安装

### 1. 编译

```bash
cd gitfox-shell
cargo build --release
```

### 2. 安装二进制文件

```bash
sudo cp target/release/gitfox-shell /usr/bin/
sudo cp target/release/gitfox-shell-authorized-keys-check /usr/bin/
sudo chmod 755 /usr/bin/gitfox-shell
sudo chmod 755 /usr/bin/gitfox-shell-authorized-keys-check
```

### 3. 创建 git 用户

```bash
sudo useradd -r -m -d /var/opt/gitfox -s /usr/bin/gitfox-shell git
```

### 4. 配置 sshd

编辑 `/etc/ssh/sshd_config`:

```
# GitFox Shell Configuration
Match User git
    AuthorizedKeysCommand /usr/bin/gitfox-shell-authorized-keys-check %u %k %t
    AuthorizedKeysCommandUser git
    PasswordAuthentication no
    AllowTcpForwarding no
    X11Forwarding no
    AllowAgentForwarding no
```

重启 sshd:

```bash
sudo systemctl restart sshd
```

### 5. 配置环境变量

创建 `/etc/gitfox/shell.env`:

```bash
# GitFox API URL
GITFOX_API_URL=http://localhost:8080

# API Secret (用于内部通信认证)
GITFOX_API_SECRET=your-secret-token-here

# Git 仓库存储路径
GITFOX_REPOS_PATH=/var/opt/gitfox/repos

# 可选配置
GITFOX_API_TIMEOUT=30
GITFOX_DEBUG=false
```

## 环境变量

| 变量名 | 必需 | 默认值 | 描述 |
|--------|------|--------|------|
| `GITFOX_API_URL` | 否 | `http://localhost:8080` | GitFox API 服务器地址 |
| `GITFOX_API_SECRET` | **是** | - | 内部 API 认证密钥 |
| `GITFOX_REPOS_PATH` | 否 | `/var/opt/gitfox/repos` | Git 仓库存储路径 |
| `GITFOX_GIT_UPLOAD_PACK` | 否 | `git-upload-pack` | git-upload-pack 路径 |
| `GITFOX_GIT_RECEIVE_PACK` | 否 | `git-receive-pack` | git-receive-pack 路径 |
| `GITFOX_API_TIMEOUT` | 否 | `30` | API 请求超时时间（秒） |
| `GITFOX_DEBUG` | 否 | `false` | 启用调试日志 |

## API 接口

GitFox Shell 需要后端提供以下内部 API 接口：

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
2. **命令白名单**: 只允许执行特定的 Git 命令
3. **路径验证**: 防止路径遍历攻击
4. **API 认证**: 内部通信使用 secret token 认证
5. **SSH 限制**: 禁用端口转发、X11 转发、代理转发

## 故障排除

### 查看日志

```bash
# GitFox Shell 日志
sudo journalctl -u sshd | grep gitfox-shell

# 启用调试模式
export GITFOX_DEBUG=true
```

### 常见问题

**1. Permission denied (publickey)**
- 检查 SSH 公钥是否已添加到 GitFox
- 验证 `AuthorizedKeysCommand` 配置
- 检查 git 用户权限

**2. Repository not found**
- 确认仓库路径正确
- 检查用户是否有访问权限

**3. API connection failed**
- 检查 `GITFOX_API_URL` 配置
- 验证 GitFox 服务是否运行
- 检查网络连接

## 开发

```bash
# 运行测试
cargo test

# 构建调试版本
cargo build

# 运行 (测试模式)
GITFOX_API_SECRET=test ./target/debug/gitfox-shell key-1
```

## License

MIT
