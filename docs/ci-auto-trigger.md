# CI/CD 自动触发实现说明

## 功能概述

实现了在 git push 操作后自动根据 `.gitfox/ci/*.yml` 配置文件生成 pipeline 和 job 的功能。

## 工作流程

### 1. Git Push 触发

当用户通过 SSH 或 HTTP 进行 git push 操作时：

```
用户 push → gitfox-shell → post-receive hook → /api/internal/post-receive
```

### 2. Post-Receive 处理

内部 API `/api/internal/post-receive` 接收到推送通知后：

1. **解析推送信息**
   - 提取 project_id、user_id、ref_name、commit_sha
   - 判断是否为分支或标签的推送
   - 跳过删除操作（new_sha 为全零）

2. **读取 CI 配置**
   - 从仓库的指定 commit 读取 `.gitfox/ci/` 目录下的所有 `.yml`/`.yaml` 文件
   - 按文件名排序并合并配置
   - 解析全局配置（stages、variables、before_script、after_script）
   - 解析所有 job 定义

3. **创建 Pipeline**
   - 在 `pipelines` 表创建记录
   - 设置状态为 `pending`
   - 触发类型为 `push`
   - 记录触发用户和 commit SHA

4. **创建 Jobs**
   - 遍历所有 job 定义
   - 根据 `only`/`except` 规则判断是否应该运行
   - 为符合条件的 job 在 `pipeline_jobs` 表创建记录
   - 保存完整的 job 配置到 JSON 字段

### 3. Job 过滤规则

#### only 规则（包含）

```yaml
build:backend:
  script:
    - cargo build
  only:
    - main       # 只在 main 分支运行
    - develop    # 或 develop 分支
    - branches   # 或任意分支（不包括 tags）
    - tags       # 或任意 tag
```

#### except 规则（排除）

```yaml
test:unit:
  script:
    - cargo test
  except:
    - tags       # 不在 tag push 时运行
    - staging    # 不在 staging 分支运行
```

### 4. Runner 执行

Runner 通过 WebSocket 连接定期请求待处理的 jobs：

```
Runner → RequestJob → 服务器返回 pending 状态的 job → 执行
```

## 配置文件格式

### 目录结构

```
.gitfox/
└── ci/
    ├── config.yml    # 主配置文件
    ├── build.yml     # 构建任务
    ├── test.yml      # 测试任务
    ├── deploy.yml    # 部署任务
    └── docker.yml    # Docker 相关任务
```

### 配置示例

#### 全局配置

```yaml
stages:
  - build
  - test
  - deploy

variables:
  CARGO_HOME: ${CI_PROJECT_DIR}/.cargo
  NODE_ENV: production

before_script:
  - echo "Preparing environment..."

after_script:
  - echo "Cleanup..."
```

#### Job 定义

```yaml
job_name:
  stage: build
  script:
    - echo "Building..."
    - cargo build --release
  
  # 前置脚本（可覆盖全局配置）
  before_script:
    - echo "Job-specific setup"
  
  # 后置脚本
  after_script:
    - echo "Job-specific cleanup"
  
  # Job 级别变量
  variables:
    CUSTOM_VAR: "value"
  
  # 产物保存
  artifacts:
    paths:
      - target/release/
    expire_in: 1 day
    name: build-artifacts
  
  # 缓存
  cache:
    key: cargo-cache
    paths:
      - .cargo/
      - target/
  
  # 重试配置
  retry:
    max: 2
    when:
      - script_failure
      - runner_system_failure
  
  # 超时（秒）
  timeout: 3600
  
  # 允许失败（不影响 pipeline 状态）
  allow_failure: true
  
  # 执行条件
  when: on_success  # on_success, on_failure, always, manual
  
  # 运行条件
  only:
    - branches  # 或 tags、具体分支名
  except:
    - staging
  
  # Runner 标签匹配
  tags:
    - docker
    - linux
  
  # 依赖关系（DAG）
  needs:
    - build:backend
    - build:frontend
```

## 实现细节

### 代码修改

1. **[src/handlers/internal.rs](src/handlers/internal.rs)**
   - 更新 `post_receive` 函数，添加 pipeline 触发逻辑
   - 新增 `try_trigger_pipeline` 辅助函数

2. **关键依赖**
   - `CiConfigParser::parse_from_repo()` - 从 git 仓库解析 CI 配置
   - `CiConfigParser::should_run_job()` - 判断 job 是否应该运行

### 数据库表

#### pipelines 表

```sql
CREATE TABLE pipelines (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL,
    ref_name VARCHAR(255) NOT NULL,  -- refs/heads/main
    commit_sha VARCHAR(40) NOT NULL,
    status VARCHAR(50) NOT NULL,     -- pending, running, success, failed, canceled, skipped
    trigger_type VARCHAR(50) NOT NULL, -- push, merge_request, schedule, manual, api, webhook
    triggered_by BIGINT,             -- user_id
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    duration_seconds INTEGER,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### pipeline_jobs 表

```sql
CREATE TABLE pipeline_jobs (
    id BIGSERIAL PRIMARY KEY,
    pipeline_id BIGINT NOT NULL,
    project_id BIGINT NOT NULL,
    runner_id BIGINT,
    name VARCHAR(255) NOT NULL,
    stage VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    config JSONB NOT NULL,           -- 完整的 job 配置
    artifacts_path TEXT,
    coverage DECIMAL(5,2),
    allow_failure BOOLEAN DEFAULT FALSE,
    when_condition VARCHAR(50) DEFAULT 'on_success',
    retry_count INTEGER DEFAULT 0,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## 测试

### 1. 创建测试仓库

```bash
mkdir test-ci && cd test-ci
git init
mkdir -p .gitfox/ci
```

### 2. 添加 CI 配置

创建 `.gitfox/ci/config.yml`:

```yaml
stages:
  - test

test:
  stage: test
  script:
    - echo "Hello CI/CD!"
  only:
    - branches
```

### 3. 推送到 GitFox

```bash
git add .
git commit -m "Add CI configuration"
git remote add origin ssh://git@localhost:2222/namespace/project.git
git push -u origin main
```

### 4. 验证

- 检查数据库 `pipelines` 表是否创建了新记录
- 检查 `pipeline_jobs` 表是否创建了对应的 job
- 查看 runner 日志，确认 job 被正确执行

## 环境变量

CI 执行时可用的预定义变量：

- `CI_PROJECT_DIR` - 项目工作目录
- `CI_COMMIT_SHA` - 当前 commit SHA
- `CI_COMMIT_REF_NAME` - 分支或 tag 名称
- `CI_PIPELINE_ID` - Pipeline ID
- `CI_JOB_ID` - Job ID
- `CI_JOB_NAME` - Job 名称
- `CI_JOB_STAGE` - Job 所在 stage

## 注意事项

1. **无 CI 配置时**
   - 如果 `.gitfox/ci/` 目录不存在或没有 yml 文件，不会创建 pipeline
   - 正常返回成功，不产生错误

2. **配置解析失败**
   - YAML 语法错误会被记录到日志
   - 不会阻塞 git push 操作

3. **没有匹配的 job**
   - 如果所有 job 都被 only/except 规则过滤，pipeline 状态设置为 `skipped`

4. **Runner 执行**
   - Runner 通过 WebSocket 主动请求 job，无需队列推送
   - 支持多个 runner 并发执行不同的 job

## 后续优化

- [ ] 支持 `.gitfox-ci.yml` 单文件配置（向后兼容）
- [ ] 支持 include 指令（引用其他配置文件）
- [ ] 支持 extends 继承机制
- [ ] 添加配置验证和语法检查 API
- [ ] 支持 pipeline schedule（定时触发）
- [ ] 支持 webhook 触发方式
- [ ] 添加 pipeline 可视化展示
