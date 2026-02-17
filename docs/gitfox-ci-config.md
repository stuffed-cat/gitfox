# GitFox CI/CD 配置示例

本目录包含 GitFox CI/CD 配置文件，采用目录风格（类似 GitLab 15.7+ 的目录 CI）。

## 目录结构

```
.gitfox/
└── ci/
    ├── build.yml       # 构建阶段
    ├── test.yml        # 测试阶段
    ├── deploy.yml      # 部署阶段
    └── ...             # 其他配置文件
```

## 全局配置

全局变量、stages 等可以在任意文件中定义，会自动合并：

```yaml
# .gitfox/ci/config.yml
stages:
  - build
  - test
  - deploy

variables:
  CARGO_HOME: ${CI_PROJECT_DIR}/.cargo
  RUST_BACKTRACE: 1

before_script:
  - echo "Starting job..."

after_script:
  - echo "Job finished"
```

## Job 定义

每个 YAML 文件可以定义一个或多个 job：

```yaml
# .gitfox/ci/build.yml
build:rust:
  stage: build
  script:
    - cargo build --release
  artifacts:
    paths:
      - target/release/
    expire_in: 1 day
  cache:
    key: cargo-cache
    paths:
      - .cargo/
      - target/
  tags:
    - rust

build:frontend:
  stage: build
  script:
    - cd frontend
    - npm ci
    - npm run build
  artifacts:
    paths:
      - frontend/dist/
  cache:
    key: npm-cache
    paths:
      - frontend/node_modules/
```

```yaml
# .gitfox/ci/test.yml
test:unit:
  stage: test
  script:
    - cargo test
  needs:
    - build:rust
  only:
    - branches
  tags:
    - rust

test:integration:
  stage: test
  script:
    - ./run_integration_tests.sh
  allow_failure: true
  needs:
    - build:rust
```

```yaml
# .gitfox/ci/deploy.yml
deploy:staging:
  stage: deploy
  script:
    - ./deploy.sh staging
  when: manual
  only:
    - develop
  environment:
    name: staging
    url: https://staging.example.com

deploy:production:
  stage: deploy
  script:
    - ./deploy.sh production
  only:
    - main
  when: manual
  environment:
    name: production
    url: https://example.com
```

## Job 配置选项

### 基本选项

- `stage`: Job 所属的阶段
- `script`: 要执行的命令列表（必需）
- `before_script`: 在 script 之前执行的命令
- `after_script`: 在 script 之后执行的命令

### 条件执行

- `only`: 仅在指定的分支/标签上运行
  - `branches`: 所有分支
  - `tags`: 所有标签
  - 具体分支名：`main`, `develop`
- `except`: 排除指定的分支/标签
- `when`: 何时运行
  - `on_success`: 前置 job 成功时（默认）
  - `on_failure`: 前置 job 失败时
  - `always`: 总是运行
  - `manual`: 手动触发

### 依赖关系

- `needs`: 指定依赖的 job（不需要等待整个 stage 完成）

### Runner 选择

- `tags`: 指定 runner 标签

### 错误处理

- `allow_failure`: 允许失败（不影响 pipeline 状态）
- `retry`: 重试配置
  ```yaml
  retry: 2  # 简单模式
  # 或
  retry:
    max: 2
    when:
      - runner_system_failure
      - stuck_or_timeout_failure
  ```

### 制品与缓存

- `artifacts`: 保存文件供后续 job 使用
  ```yaml
  artifacts:
    paths:
      - target/release/
      - dist/
    expire_in: 1 week
    name: my-artifacts
  ```

- `cache`: 缓存依赖加速构建
  ```yaml
  cache:
    key: ${CI_COMMIT_REF_SLUG}
    paths:
      - .cargo/
      - node_modules/
  ```

### 超时

- `timeout`: Job 超时时间（秒）

### 变量

- `variables`: Job 级别的环境变量
  ```yaml
  variables:
    DATABASE_URL: postgres://localhost/test
    NODE_ENV: test
  ```

## 内置变量

GitFox CI 提供以下内置环境变量：

- `CI_COMMIT_SHA`: 当前 commit 的 SHA
- `CI_COMMIT_REF_NAME`: 分支或标签名
- `CI_COMMIT_REF_SLUG`: 分支或标签名（slug 格式）
- `CI_PROJECT_ID`: 项目 ID
- `CI_PROJECT_NAME`: 项目名称
- `CI_PROJECT_PATH`: 项目完整路径
- `CI_PROJECT_DIR`: 项目工作目录
- `CI_PIPELINE_ID`: Pipeline ID
- `CI_JOB_ID`: Job ID
- `CI_JOB_NAME`: Job 名称
- `CI_JOB_STAGE`: Job 所属 stage

## 完整示例

```
.gitfox/ci/
├── config.yml
├── build.yml
├── test.yml
├── lint.yml
├── security.yml
└── deploy.yml
```

```yaml
# config.yml
stages:
  - build
  - test
  - security
  - deploy

variables:
  RUST_VERSION: "1.75"
  NODE_VERSION: "20"
```

```yaml
# build.yml
build:backend:
  stage: build
  image: rust:${RUST_VERSION}
  script:
    - cargo build --release
  artifacts:
    paths:
      - target/release/
  cache:
    key: rust-${CI_COMMIT_REF_SLUG}
    paths:
      - target/
      - .cargo/
  tags:
    - rust
    - docker

build:frontend:
  stage: build
  image: node:${NODE_VERSION}
  script:
    - npm ci
    - npm run build
  artifacts:
    paths:
      - dist/
  cache:
    key: npm-${CI_COMMIT_REF_SLUG}
    paths:
      - node_modules/
  tags:
    - node
    - docker
```

```yaml
# test.yml
test:unit:
  stage: test
  script:
    - cargo test --all
  needs:
    - build:backend
  coverage: '/\d+\.\d+% coverage/'

test:integration:
  stage: test
  script:
    - ./scripts/integration-tests.sh
  needs:
    - build:backend
  services:
    - postgres:15
    - redis:7
```

```yaml
# lint.yml
lint:rust:
  stage: test
  script:
    - cargo clippy -- -D warnings
    - cargo fmt -- --check
  allow_failure: true

lint:frontend:
  stage: test
  script:
    - npm run lint
    - npm run type-check
  allow_failure: true
```

```yaml
# security.yml
security:audit:
  stage: security
  script:
    - cargo audit
  allow_failure: true

security:dependency-check:
  stage: security
  script:
    - npm audit
  allow_failure: true
```

```yaml
# deploy.yml
deploy:staging:
  stage: deploy
  script:
    - ./scripts/deploy.sh staging
  only:
    - develop
  when: manual
  environment:
    name: staging
    url: https://staging.example.com

deploy:production:
  stage: deploy
  script:
    - ./scripts/deploy.sh production
  only:
    - main
  when: manual
  environment:
    name: production
    url: https://example.com
  needs:
    - test:unit
    - test:integration
    - security:audit
```
