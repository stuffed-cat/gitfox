# sshd + gitfox-shell 集成配置

本文档说明如何配置系统 sshd 与 gitfox-shell 集成，实现 GitLab 式的 SSH Git 访问架构。

## 架构概览

GitFox 采用与 GitLab 类似的分层架构：

```
         HTTP 请求                          SSH 请求
             │                                   │
             ▼                                   ▼
    ┌─────────────────┐                   ┌───────────────┐
    │ gitfox-workhorse│                   │     sshd      │
    │      :8080      │                   │     :22       │
    └────────┬────────┘                   └───────┬───────┘
             │                                   │
    ┌────────┴────────┐                          ▼
    │                 │                   ┌───────────────┐
   API/*          Git HTTP                │ gitfox-shell  │
    │             *.git/*                 └───────┬───────┘
    ▼                 │                          │
┌─────────┐          │                   gRPC   │
│Main App │          │                  ┌──────┴───────┐
│ :8081   │◄────────┼── gRPC Auth ───────►│ (认证)       │
└─────────┘          │  :50051              └───────┬───────┘
                      │                            │
                      │      gRPC (Git操作)        │
                      └───────────┬────────────────┘
                                ▼
                       ┌─────────────────┐
                       │    GitLayer     │
                       │     :50052      │
                       └──────┬──────────┘
                              │
                              ▼
                       ┌─────────────────┐
                       │ Git Repositories│
                       └─────────────────┘
```

**请求流：**
- **HTTP Git**: Workhorse → (gRPC Auth) → GitLayer
- **HTTP API**: Workhorse → Main App
- **SSH Git**: Shell → (gRPC Auth) → GitLayer

## 创建 git 用户

```bash
# 创建专用的 git 系统用户
sudo useradd -r -m -d /var/opt/gitfox -s /bin/bash git

# 创建必要目录
sudo mkdir -p /var/opt/gitfox/repos
sudo mkdir -p /var/opt/gitfox/.ssh
sudo chown -R git:git /var/opt/gitfox

# 设置 .ssh 权限
sudo chmod 700 /var/opt/gitfox/.ssh
```

## 安装 gitfox-shell

```bash
# 编译
cd gitfox-shell
cargo build --release

# 安装到系统路径
sudo cp target/release/gitfox-shell /usr/local/bin/
sudo chmod 755 /usr/local/bin/gitfox-shell

# 安装 authorized_keys_check
sudo cp target/release/authorized_keys_check /usr/local/bin/
sudo chmod 755 /usr/local/bin/authorized_keys_check
```

## 配置环境变量

创建 `/etc/gitfox/shell.env`:

```bash
sudo mkdir -p /etc/gitfox
sudo tee /etc/gitfox/shell.env << 'EOF'
# gRPC 认证（推荐）
GITFOX_USE_GRPC_AUTH=true
AUTH_GRPC_ADDRESS=http://[::1]:50051

# GitLayer（推荐）
GITFOX_USE_GITLAYER=true
GITLAYER_ADDRESS=http://[::1]:50052

# 仓库路径
GITFOX_REPOS_PATH=/var/opt/gitfox/repos
EOF
```

## 配置 sshd

编辑 `/etc/ssh/sshd_config`，添加以下配置：

```sshd_config
# ============================================
# GitFox Git User Configuration
# ============================================

# 匹配 git 用户的 SSH 连接
Match User git
    # 禁用密码认证，只允许公钥
    PasswordAuthentication no
    
    # 使用 AuthorizedKeysCommand 动态查询公钥
    AuthorizedKeysCommand /usr/local/bin/authorized_keys_check %u %k %t
    AuthorizedKeysCommandUser git
    
    # 禁用不需要的功能
    AllowTcpForwarding no
    AllowAgentForwarding no
    X11Forwarding no
    PermitTTY no
```

## 配置 git 用户的 shell

设置 git 用户的默认 shell 为 gitfox-shell：

```bash
# 方法 1: 修改用户 shell
sudo usermod -s /usr/local/bin/gitfox-shell git

# 方法 2: 使用 ForceCommand（在 sshd_config 的 Match User git 块中）
# ForceCommand /usr/local/bin/gitfox-shell
```

## 重启 sshd

```bash
# 测试配置
sudo sshd -t

# 重启服务
sudo systemctl restart sshd
```

## 验证安装

### 测试 authorized_keys_check

```bash
# 模拟 sshd 查询公钥
sudo -u git /usr/local/bin/authorized_keys_check git "ssh-rsa" "AAAAB3NzaC1yc2E..."
```

### 测试 SSH 连接

```bash
# 应该返回 GitFox Shell 欢迎信息
ssh -T git@your-server.com
```

### 测试 Git 操作

```bash
# 克隆测试
git clone git@your-server.com:username/repo.git

# 推送测试
cd repo
echo "test" > test.txt
git add test.txt
git commit -m "test"
git push
```

## 故障排查

### 查看 sshd 日志

```bash
sudo journalctl -u sshd -f
```

### 测试 gRPC 连接

```bash
# 检查 Auth 服务
grpcurl -plaintext [::1]:50051 list

# 检查 GitLayer 服务
grpcurl -plaintext [::1]:50052 list
```

### 常见问题

1. **Permission denied (publickey)**
   - 检查公钥是否已添加到用户账户
   - 检查 AuthorizedKeysCommand 权限
   - 查看 `/var/log/auth.log`

2. **Connection closed by remote host**
   - 检查 gitfox-shell 是否正确安装
   - 检查 gRPC 服务是否运行

3. **Repository not found**
   - 检查仓库路径配置
   - 检查用户访问权限

## 安全建议

1. **限制 git 用户权限**
   - git 用户应该只能执行 git 操作
   - 禁用交互式 shell 访问

2. **使用防火墙**
   - gRPC 端口（50051, 50052）只监听localhost
   - SSH 端口可根据需要限制

3. **启用审计日志**
   - 记录所有 git 操作
   - 监控异常访问模式
