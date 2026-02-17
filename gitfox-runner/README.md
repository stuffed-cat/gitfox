# GitFox Runner

GitFox CI/CD Runner - 使用 WebSocket 实时通信的 CI/CD 执行器

## 特性

- ✅ WebSocket 实时通信
- ✅ Shell 执行器
- ✅ 实时日志流
- ✅ 自动重连
- ✅ 支持标签过滤
- 🚧 Docker 执行器（开发中）

## 安装

```bash
cargo build --release
```

## 使用

```bash
# 基本使用
gitfox-runner \
  --token <RUNNER_TOKEN> \
  --server-url ws://localhost:8081 \
  --name my-runner \
  --tags rust,docker \
  --executor shell

# 使用环境变量
export GITFOX_RUNNER_TOKEN=your-token
export GITFOX_SERVER_URL=ws://localhost:8081
export GITFOX_RUNNER_NAME=my-runner
export GITFOX_RUNNER_TAGS=rust,docker
export GITFOX_EXECUTOR=shell

gitfox-runner
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
