# GitFox 架构迁移指南

本文档描述从旧架构（内置 SSH）迁移到新架构（GitLab 式分层架构）的过程。

## 架构变更概览

### 旧架构 (v0.x)

```
┌─────────────────────────────────────┐
│         gitfox-omnibus              │
│                                     │
│  ┌──────────────────────────────┐   │
│  │      devops (Main App)       │   │
│  │  - HTTP API :8081            │   │
│  │  - 内置 SSH (russh) :2222    │   │
│  │  - Git 操作 (直接)           │   │
│  └──────────────────────────────┘   │
│                                     │
│  ┌──────────────────────────────┐   │
│  │      workhorse :8080         │   │
│  │  - 只代理 HTTP 请求          │   │
│  └──────────────────────────────┘   │
└─────────────────────────────────────┘
```

**问题：**
- Main App 过于耦合，承担太多职责
- 内置 SSH 不支持标准 sshd 功能（如 AuthorizedKeysCommand）
- 无法水平扩展 Git 操作

### 新架构 (v1.x)

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
                        └────────┬────────┘
                                 │
                                 ▼
                        ┌─────────────────┐
                        │  Git Repos      │
                        └─────────────────┘
```

**优势：**
- Main App 只处理业务逻辑
- Git 操作通过 GitLayer 统一处理
- 支持标准 sshd 和 AuthorizedKeysCommand
- 可独立扩展各组件

## 迁移步骤

### 自动迁移 (推荐)

```bash
# 停止旧版本
systemctl stop gitfox

# 备份数据
cp -r /var/lib/gitfox /var/lib/gitfox.backup

# 升级二进制
cp gitfox-new /usr/local/bin/gitfox

# 运行迁移向导
gitfox migrate

# 启动新版本
gitfox start
```

### 手动迁移

#### 1. 配置文件迁移

**旧配置 (gitfox.env):**
```env
# SSH 服务器配置（将被移除）
SSH_ENABLED=true
SSH_HOST=0.0.0.0
SSH_PORT=2222
SSH_HOST_KEY_PATH=/var/lib/gitfox/ssh_host_key
```

**新配置 (gitfox.env):**
```env
# SSH Clone URL 配置（仅用于前端显示）
SSH_PUBLIC_HOST=git.example.com
SSH_PUBLIC_PORT=22

# gRPC 服务配置
GRPC_ENABLED=true
GRPC_ADDRESS=[::1]:50051

# GitLayer 配置
GITLAYER_ENABLED=true
GITLAYER_ADDRESS=[::1]:50052
```

#### 2. 创建 git 用户

```bash
# 创建专用 git 用户
sudo useradd -r -m -d /var/opt/gitfox -s /bin/bash git

# 设置权限
sudo chown -R git:git /var/lib/gitfox/repos
```

#### 3. 配置 sshd

添加到 `/etc/ssh/sshd_config`:

```
Match User git
    AuthorizedKeysCommand /usr/local/bin/gitfox-shell-authorized-keys-check %u %k %t
    AuthorizedKeysCommandUser git
    PasswordAuthentication no
    AllowTcpForwarding no
    X11Forwarding no
    AllowAgentForwarding no
```

#### 4. 安装 gitfox-shell

```bash
sudo cp /var/lib/gitfox/bin/gitfox-shell /usr/local/bin/
sudo cp /var/lib/gitfox/bin/gitfox-shell-authorized-keys-check /usr/local/bin/
sudo chmod 755 /usr/local/bin/gitfox-shell*
```

#### 5. 配置 gitfox-shell

创建 `/etc/gitfox/shell.env`:

```env
GITFOX_USE_GRPC_AUTH=true
AUTH_GRPC_ADDRESS=http://[::1]:50051
# GitLayer 是必需的
GITLAYER_ADDRESS=http://[::1]:50052
GITFOX_REPOS_PATH=/var/lib/gitfox/repos
```

#### 6. 重启服务

```bash
sudo systemctl restart sshd
gitfox start
```

## 数据迁移

### 仓库数据

仓库数据 (`repos/`) 无需修改，新旧架构使用相同的裸仓库格式。

### SSH 密钥

用户 SSH 密钥存储在数据库中，无需迁移。新架构通过 `AuthorizedKeysCommand` 动态查询。

### 数据库

运行数据库迁移（如有新的 migration）：

```bash
gitfox migrate --database
```

## 回滚

如果迁移失败，可以回滚：

```bash
# 停止新版本
gitfox stop

# 恢复旧配置
cp /var/lib/gitfox.backup/gitfox.env /var/lib/gitfox/

# 恢复旧二进制
cp /usr/local/bin/gitfox.old /usr/local/bin/gitfox

# 移除 sshd 配置
# 编辑 /etc/ssh/sshd_config 移除 Match User git 块

# 重启
sudo systemctl restart sshd
gitfox start
```

## 验证迁移

### HTTP 访问

```bash
curl http://localhost:8080/api/v1/health
```

### SSH 访问

```bash
ssh -T git@localhost
# 应该返回 GitFox Shell 欢迎信息
```

### Git 操作

```bash
git clone git@localhost:user/repo.git
```

## 常见问题

### Q: 可以同时运行新旧版本吗？

不推荐。两个版本可能会冲突（数据库、仓库锁等）。

### Q: GitLayer 是可选的吗？

GitLayer 是新架构的核心组件, 对于配置 fallback 到直接 Git 操作的兼容性路径已经被完全移除。主应用并不知道如何处理 Git Smart HTTP 请求和git ssh 操作。因此，GitLayer 是必需的。
