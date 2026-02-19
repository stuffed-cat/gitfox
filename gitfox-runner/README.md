# GitFox Runner

GitFox CI/CD Runner - 使用 WebSocket 实时通信的 CI/CD 执行器

## 特性

- ✅ WebSocket 实时通信
- ✅ Shell 执行器（带安全限制）
- ✅ Docker 执行器（完全隔离）
- ✅ 实时日志流
- ✅ 自动重连
- ✅ 支持标签过滤
- ✅ 工作目录清理和大小限制
- ✅ 危险命令检测

## 安全特性

### Shell Executor
- **✅ 隔离工作目录**: 使用配置的 `builds_dir` 而非系统 `/tmp`
- **✅ 敏感路径隐藏**: 不在日志中暴露实际工作目录路径
- **✅ 大小限制**: 防止单个 job 消耗过多磁盘空间（默认 10GB）
- **✅ 危险命令检测**: 警告 `rm -rf /`、fork bomb 等危险操作
- **✅ 自动清理**: Job 完成后自动删除工作目录
- **⚠️ 权限限制**: 以 runner 进程用户运行，建议使用非 root 用户

### Docker Executor（推荐用于生产环境）
- **✅ 完整容器隔离**: 每个 job 在独立 Docker 容器中运行
- **✅ 文件系统隔离**: 容器无法访问宿主机文件系统
- **✅ 网络隔离**: 可选的网络隔离配置
- **✅ 资源限制**: 继承 Docker 的 CPU/内存限制能力
- **✅ 自动清理**: 容器运行完毕自动删除（`--rm`）

### 生产环境建议

1. **使用 Docker executor** 运行不受信任的代码
2. **非 root 运行**: 创建专用 `gitfox-runner` 用户
   ```bash
   sudo useradd -m -s /bin/bash gitfox-runner
   sudo -u gitfox-runner gitfox-runner run
   ```
3. **配置工作目录**:
   ```toml
   builds_dir = "/var/lib/gitfox-runner/builds"
   max_work_dir_size_mb = 20480  # 20 GB
   clean_builds = true
   ```
4. **使用独立磁盘或分区** 挂载到 `builds_dir` 防止系统盘被填满
5. **限制并发**: 根据机器资源设置 `concurrent_jobs`

## 安装

```bash
cargo build --release
```

## 使用

### 注册 Runner

```bash
# 交互式注册
gitfox-runner register \
  --url http://localhost:8080 \
  --token <REGISTRATION_TOKEN> \
  --name my-runner \
  --tags rust,docker \
  --executor docker

# 配置文件将保存到 ~/.config/gitfox-runner/config.toml
```

### 运行 Runner

```bash
# 使用默认配置文件
gitfox-runner run

# 使用自定义配置文件
gitfox-runner run --config /path/to/config.toml
```

### 配置文件示例

复制 `config.example.toml` 并根据需要修改：

```toml
server_url = "ws://localhost:8080/api/v1/runner/ws"
token = "your-runner-token"
name = "my-runner"
tags = ["docker", "linux"]
executor = "docker"

# 工作目录配置
builds_dir = "./builds"
max_work_dir_size_mb = 10240  # 10 GB
clean_builds = true

# 执行配置
concurrent_jobs = 1
default_docker_image = "alpine:latest"
```

## CI 配置格式

在项目中创建 `.gitfox/ci/` 目录：

```yaml
# .gitfox/ci/build.yml
build:
  stage: build
  script:
    - cargo build --release
  artifacts:
    paths:
      - target/release/

# .gitfox/ci/test.yml
test:
  stage: test
  script:
    - cargo test
  needs:
    - build

# .gitfox/ci/deploy.yml
deploy:
  stage: deploy
  script:
    - ./deploy.sh
  when: manual
  only:
    - main
```

## WebSocket 协议

### Runner → Server 消息

```json
{
  "type": "register",
  "token": "runner-token",
  "name": "runner-1",
  "tags": ["rust", "docker"],
  "executor": "shell"
}

{
  "type": "job_update",
  "job_id": 123,
  "status": "running",
  "exit_code": null
}

{
  "type": "job_log",
  "job_id": 123,
  "output": "Building project...\n"
}

{
  "type": "heartbeat"
}
```

### Server → Runner 消息

```json
{
  "type": "registered",
  "runner_id": 1
}

{
  "type": "job_assigned",
  "job": {
    "id": 123,
    "pipeline_id": 45,
    "name": "build",
    "script": ["cargo build"],
    ...
  }
}

{
  "type": "no_jobs"
}
```
