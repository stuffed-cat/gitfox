# GitLayer

GitLayer 是 GitFox 的 Git 操作 RPC 服务，类似于 GitLab 的 Gitaly 组件。它通过 gRPC 提供所有底层 Git 操作，供主应用、gitfox-shell 和 gitfox-workhorse 调用。

## 架构

GitLayer 是所有 Git 操作的统一处理点，类似 GitLab Gitaly：

```
                    HTTP 请求                    SSH 请求
                        │                             │
                        ▼                             ▼
              ┌─────────────────────┐        ┌─────────────────────┐
              │ gitfox-workhorse    │        │  gitfox-shell       │
              └────────┬────────────┘        └────────┬────────────┘
                       │                              │
            ┌──────────┴──────────┐                   │
            │                     │                   │
       API请求                Git HTTP                │
            │                     │                   │
            ▼                     ▼                   ▼
   ┌─────────────────┐    ┌───────────────────────────────────────┐
   │    Main App     │    │              GitLayer                 │  :50052
   │     :8081       │◄───│           (Git 操作 RPC)              │
   │   (业务逻辑)    │gRPC└────────────────┬──────────────────────┘
   └─────────────────┘    (Auth)           │
                                           ▼
                               ┌───────────────────────┐
                               │   Git Repositories    │
                               │       ./repos/        │
                               └───────────────────────┘
```

**请求流：**
- **HTTP Git (clone/push)**: Workhorse → GitLayer
- **SSH Git (clone/push)**: Shell → GitLayer
- **认证**: Workhorse/Shell → Main App gRPC Auth → (验证后) → GitLayer

## gRPC 服务

GitLayer 提供以下 gRPC 服务：

| 服务 | Proto 文件 | 描述 |
|------|-----------|------|
| Repository | repository.proto | 仓库创建、删除、信息查询 |
| Ref | ref.proto | 分支、标签操作 |
| Commit | commit.proto | 提交查询、历史遍历 |
| Blob | blob.proto | 文件内容读取 |
| Tree | tree.proto | 目录树操作 |
| Diff | diff.proto | 差异比较 |
| Operations | operations.proto | 用户操作（合并、创建分支等） |
| SmartHTTP | smarthttp.proto | Git Smart HTTP 协议支持 |
| Health | health.proto | 健康检查 |

## 配置

GitLayer 会自动从以下位置加载 `.env` 文件（按优先级）：

1. 当前目录的 `.env`
2. `/etc/gitfox/gitlayer.env`
3. `~/.config/gitfox/gitlayer.env`

配置示例参见 [.env.example](.env.example)

### 环境变量

| 环境变量 | 默认值 | 描述 |
|---------|--------|------|
| `GITLAYER_LISTEN_ADDR` | `0.0.0.0:9999` | gRPC 监听地址 |
| `GITLAYER_STORAGE_PATH` | `./repos` | Git 仓库存储路径 |
| `GIT_REPOS_PATH` | `./repos` | 备选仓库路径变量 |
| `GITLAYER_GIT_BIN` | `git` | Git 二进制路径 |
| `GITLAYER_MAX_CONCURRENT_OPS` | `10` | 每仓库最大并发操作数 |
| `GITLAYER_ENABLE_CACHE` | `true` | 启用仓库缓存 |
| `GITLAYER_CACHE_TTL` | `60` | 缓存 TTL（秒） |

## 运行

```bash
# 开发环境
cd gitlayer
cargo run

# 生产环境
export GITLAYER_LISTEN_ADDR=0.0.0.0:50052
export GITLAYER_STORAGE_PATH=/var/opt/gitfox/repos
./gitlayer
```

## 与其他组件集成

### gitfox-shell

```env
# GitLayer 是必需的
GITLAYER_ADDRESS=http://[::1]:50052
```

### gitfox-workhorse

```toml
# GitLayer 是必需的
gitlayer_address = "http://[::1]:50052"
```

## 开发

```bash
# 编译 proto 文件
cargo build

# 运行测试
cargo test

# 检查类型
cargo check
```
