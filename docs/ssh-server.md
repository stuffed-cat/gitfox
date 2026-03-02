# GitFox SSH 服务器配置

本文档说明如何配置 gitfox-shell SSH 服务器，提供 Git SSH 访问。

## 架构概览

GitFox 采用与 GitLab 类似的分层架构。gitfox-shell 是**独立的 SSH 服务器**，不依赖系统 sshd：

```
         HTTP 请求                          SSH 请求
             │                                   │
             ▼                                   ▼
    ┌─────────────────┐                   ┌───────────────┐
    │ gitfox-workhorse│                   │ gitfox-shell  │ ← 独立 SSH 服务器 (russh)
    │      :8080      │                   │     :22       │
    └────────┬────────┘                   └───────┬───────┘
             │                                   │
    ┌────────┴────────┐                          │
    │                 │                          │
   API/*          Git HTTP                      │
    │             *.git/*                       │
    ▼                 │                          │
┌─────────┐          │                   gRPC   │
│Main App │          │                  ┌──────┴───────┐
│ :8081   │◄────────┼── gRPC Auth ────►│ Auth 服务     │
└─────────┘          │  :50051              │:50051      │
                      │                     └───────┬───────┘
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
- **SSH Git**: gitfox-shell → (gRPC Auth) → GitLayer

## 与 GitLab 的对比

| 组件 | GitLab | GitFox |
|------|--------|--------|
| SSH 服务器 | gitlab-shell (独立) | gitfox-shell (独立) |
| Git 操作 | Gitaly | GitLayer |
| HTTP 代理 | Workhorse | gitfox-workhorse |
| 主应用 | Rails | Rust/Actix |

**注意**: 现代 GitLab 的 gitlab-shell 也是独立的 SSH 服务器，不再需要系统 sshd。

## 快速开始

### 使用 gitfox omnibus（推荐）

```bash
# 初始化并设置 SSH
gitfox init
gitfox setup-ssh --ssh-port 22

# 按照提示运行生成的脚本
sudo bash /var/lib/gitfox/setup-ssh.sh

# 启动服务
sudo systemctl enable gitfox-shell
sudo systemctl start gitfox-shell
```

### 手动安装

#### 1. 创建 git 用户

```bash
# 创建专用的 git 系统用户（无登录 shell）
sudo useradd -r -m -d /var/opt/gitfox -s /usr/sbin/nologin git

# 创建必要目录
sudo mkdir -p /var/opt/gitfox/repos
sudo chown -R git:git /var/opt/gitfox
```

#### 2. 安装 gitfox-shell

```bash
# 编译
cd gitfox-shell
cargo build --release

# 安装到系统路径
sudo cp target/release/gitfox-shell /usr/local/bin/
sudo chmod 755 /usr/local/bin/gitfox-shell
```

#### 3. 配置环境变量

创建 `/etc/gitfox/shell.env`:

```bash
# SSH 服务器配置
SSH_LISTEN_ADDR=0.0.0.0:22
SSH_HOST_KEY_PATH=/etc/gitfox/ssh_host_ed25519_key

# gRPC 服务
AUTH_GRPC_ADDRESS=http://[::1]:50051
GITLAYER_ADDRESS=http://[::1]:50052

# 内部 API 认证密钥
GITFOX_API_SECRET=your-secret-token-here

# 日志
RUST_LOG=info
```

#### 4. 创建 systemd 服务

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
ExecStart=/usr/local/bin/gitfox-shell server --listen ${SSH_LISTEN_ADDR} --host-key ${SSH_HOST_KEY_PATH}
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

#### 5. 启动服务

```bash
# 重载 systemd
sudo systemctl daemon-reload

# 启用并启动服务
sudo systemctl enable gitfox-shell
sudo systemctl start gitfox-shell

# 检查状态
sudo systemctl status gitfox-shell
```

## Host Key 管理

gitfox-shell 首次启动时会自动生成 Ed25519 host key。如果要使用现有的 key：

```bash
# 使用现有 key
sudo cp /path/to/your/host_key /etc/gitfox/ssh_host_ed25519_key
sudo chown git:git /etc/gitfox/ssh_host_ed25519_key
sudo chmod 600 /etc/gitfox/ssh_host_ed25519_key

# 或生成新 key
ssh-keygen -t ed25519 -f /etc/gitfox/ssh_host_ed25519_key -N ""
sudo chown git:git /etc/gitfox/ssh_host_ed25519_key
```

## 端口配置

### 使用标准端口 22

如果要使用端口 22，需要先停用或重新配置系统 sshd：

```bash
# 选项 1: 完全停用系统 sshd（如果不需要）
sudo systemctl stop sshd
sudo systemctl disable sshd

# 选项 2: 将系统 sshd 移到其他端口
# 编辑 /etc/ssh/sshd_config，修改 Port 为 2222
sudo systemctl restart sshd

# 然后启动 gitfox-shell
sudo systemctl start gitfox-shell
```

### 使用非标准端口

```bash
# 编辑 /etc/gitfox/shell.env
SSH_LISTEN_ADDR=0.0.0.0:2222

# 重启服务
sudo systemctl restart gitfox-shell
```

用户访问时需要指定端口：
```bash
git clone ssh://git@your-server:2222/namespace/repo.git
# 或
GIT_SSH_COMMAND="ssh -p 2222" git clone git@your-server:namespace/repo.git
```

## 防火墙配置

```bash
# UFW
sudo ufw allow 22/tcp

# firewalld
sudo firewall-cmd --permanent --add-port=22/tcp
sudo firewall-cmd --reload

# iptables
sudo iptables -A INPUT -p tcp --dport 22 -j ACCEPT
```

## 故障排除

### 查看日志

```bash
# systemd 日志
sudo journalctl -u gitfox-shell -f

# 启用调试模式
# 编辑 /etc/gitfox/shell.env
RUST_LOG=debug
sudo systemctl restart gitfox-shell
```

### 常见问题

**1. Connection refused**
- 确认服务正在运行：`sudo systemctl status gitfox-shell`
- 检查端口是否被占用：`sudo lsof -i :22`
- 检查防火墙设置

**2. Permission denied (publickey)**
- 确认公钥已添加到 GitFox 账户
- 检查 Auth gRPC 服务是否运行
- 查看调试日志获取详细信息

**3. Repository not found**
- 确认仓库路径正确
- 检查用户访问权限

**4. gRPC connection failed**
- 确认主应用和 GitLayer 服务正在运行
- 检查 `AUTH_GRPC_ADDRESS` 和 `GITLAYER_ADDRESS` 配置

## 安全特性

gitfox-shell 内置以下安全特性：

1. **无交互式 Shell**: 用户无法获得终端访问
2. **命令白名单**: 只允许 `git-upload-pack` 和 `git-receive-pack`
3. **路径验证**: 防止路径遍历攻击
4. **公钥认证**: 只支持 SSH 公钥认证
5. **gRPC TLS**: 可配置内部通信加密（生产环境推荐）

## 参考

- [gitfox-shell README](../gitfox-shell/README.md)
- [GitLayer 配置](../gitlayer/README.md)
- [系统架构](./architecture.md)
