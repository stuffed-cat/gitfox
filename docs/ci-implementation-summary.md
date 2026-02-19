# CI/CD 自动触发功能实现总结

## 完成的功能

✅ **在 git push 后自动生成 pipeline 和 job**

### 实现内容

1. **修改了 [src/handlers/internal.rs](../src/handlers/internal.rs)**
   - 更新 `post_receive` handler，在接收到 push 通知时触发 CI 流程
   - 新增 `try_trigger_pipeline` 辅助函数，负责：
     - 从 git 仓库读取 `.gitfox/ci/*.yml` 配置文件
     - 解析并合并多个 CI 配置文件
     - 创建 pipeline 记录
     - 根据 `only`/`except` 规则过滤并创建 jobs
     - 如果没有匹配的 jobs，标记 pipeline 为 `skipped`

2. **创建了完整的 CI 配置示例**
   - [.gitfox/ci/config.yml](../.gitfox/ci/config.yml) - 主配置文件，包含完整的 job 定义示例
   - [.gitfox/ci/docker.yml](../.gitfox/ci/docker.yml) - Docker 相关任务示例

3. **创建了详细的文档**
   - [docs/ci-auto-trigger.md](ci-auto-trigger.md) - 完整的实现说明和使用指南

## 工作流程

```
          ┌─────────┐
          │ User    │
          │ git push│
          └────┬────┘
               │
               ▼
       ┌───────────────┐
       │ gitfox-shell  │
       │ post-receive  │
       └───────┬───────┘
               │
               ▼
   ┌───────────────────────┐
   │ /api/internal/        │
   │ post-receive          │
   └─────┬─────────────────┘
         │
         ▼
   ┌──────────────────┐
   │ 读取 CI 配置     │
   │ .gitfox/ci/*.yml │
   └─────┬────────────┘
         │
         ▼
   ┌──────────────────┐           ┌─────────────────┐
   │ 创建 Pipeline    │           │ Pipeline        │
   │ (status=pending) ├──────────>│ - id            │
   └─────┬────────────┘           │ - ref_name      │
         │                        │ - commit_sha    │
         │                        │ - status        │
         ▼                        └─────────────────┘
   ┌──────────────────┐
   │ 创建 Jobs        │           ┌─────────────────┐
   │ - 过滤 only/     │           │ Job             │
   │   except 规则    ├──────────>│ - name          │
   │ - 保存配置到 JSON│           │ - stage         │
   └──────────────────┘           │ - script        │
                                  │ - status=pending│
                                  └─────────────────┘
                                           │
                                           ▼
                                  ┌─────────────────┐
                                  │ Runner 轮询     │
                                  │ 并执行 jobs     │
                                  └─────────────────┘
```

## 支持的功能

### 1. 多文件配置
可以在 `.gitfox/ci/` 目录下创建多个 yml 文件，系统会自动合并

### 2. Job 过滤规则

#### only（包含）
```yaml
job:
  only:
    - main      # 只在 main 分支
    - branches  # 所有分支
    - tags      # 所有标签
```

#### except（排除）
```yaml
job:
  except:
    - staging   # 排除 staging 分支
    - tags      # 排除标签推送
```

### 3. 完整的 Job 配置
- `stage` - 所属阶段
- `script` - 执行脚本
- `before_script` / `after_script` - 前后置脚本
- `variables` - 环境变量
- `artifacts` - 产物配置
- `cache` - 缓存配置
- `allow_failure` - 允许失败
- `when` - 执行条件
- `tags` - Runner 标签
- `needs` - 依赖关系

## 测试方法

1. **创建测试仓库**
   ```bash
   mkdir test-ci && cd test-ci
   git init
   mkdir -p .gitfox/ci
   ```

2. **添加 CI 配置**
   ```bash
   cat > .gitfox/ci/config.yml << 'EOF'
   stages:
     - test

   test_job:
     stage: test
     script:
       - echo "Hello CI/CD!"
     only:
       - branches
   EOF
   ```

3. **推送到 GitFox**
   ```bash
   git add .
   git commit -m "Add CI configuration"
   git remote add origin ssh://git@localhost:2222/namespace/project.git
   git push -u origin main
   ```

4. **检查结果**
   - 查看数据库 `pipelines` 表
   - 查看 `pipeline_jobs` 表
   - 观察 runner 日志

## 配置示例

参考文件：
- [.gitfox/ci/config.yml](../.gitfox/ci/config.yml) - 完整的构建、测试、部署流程
- [.gitfox/ci/docker.yml](../.gitfox/ci/docker.yml) - Docker 相关任务
- [docs/gitfox-ci-config.md](gitfox-ci-config.md) - 更多配置示例

## 注意事项

1. ✅ **不会阻塞 git push**：即使 CI 配置有错误，push 操作仍会成功
2. ✅ **自动跳过无 CI 配置的仓库**：如果没有 `.gitfox/ci/` 目录，不会创建 pipeline
3. ✅ **智能过滤**：根据 `only`/`except` 规则自动判断是否运行 job
4. ✅ **支持删除操作**：删除分支或标签时不会触发 CI
5. ✅ **Runner 自动发现**：Runner 通过轮询自动获取待执行的 jobs

## 下一步

可以考虑的后续优化：
- 支持单文件 `.gitfox-ci.yml` 配置（向后兼容）
- 支持 `include` 指令引用其他配置
- 支持 `extends` 继承机制
- 添加配置验证 API
- Pipeline 可视化界面
- WebSocket 实时推送 job 状态
