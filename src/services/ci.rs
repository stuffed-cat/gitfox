//! CI/CD Pipeline 服务
//!
//! 负责完整的 CI/CD pipeline 触发流程：
//! 1. 解析 CI 配置文件 (.gitfox/ci/*.yml)
//! 2. 创建 pipeline 记录
//! 3. 创建 jobs 记录
//!
//! 被以下组件调用：
//! - gRPC auth_service (HTTP Git push)
//! - handlers/internal.rs (SSH Git push)

use chrono::Utc;
use log::{debug, info, warn};
use sqlx::PgPool;
use std::sync::Arc;

use crate::config::Config;
use crate::error::AppResult;
use crate::models::{Pipeline, PipelineStatus, PipelineTriggerType};
use crate::services::{CiConfigParser, GitService};

/// CI 服务
pub struct CiService {
    pool: PgPool,
    config: Arc<Config>,
}

/// Pipeline 触发结果
#[derive(Debug)]
pub struct PipelineTriggerResult {
    /// 创建的 pipeline ID
    pub pipeline_id: i64,
    /// 创建的 jobs 数量
    pub jobs_created: i32,
    /// Pipeline 状态
    pub status: PipelineStatus,
    /// 错误信息（如果有）
    pub error_message: Option<String>,
}

impl CiService {
    /// 创建新的 CI 服务实例
    pub fn new(pool: PgPool, config: Arc<Config>) -> Self {
        Self { pool, config }
    }

    /// 触发 CI/CD Pipeline
    ///
    /// 完整流程：
    /// 1. 获取项目信息
    /// 2. 打开 Git 仓库
    /// 3. 解析 CI 配置
    /// 4. 创建 pipeline
    /// 5. 创建 jobs
    ///
    /// # Arguments
    /// * `project_id` - 项目 ID
    /// * `user_id` - 触发用户 ID
    /// * `ref_name` - 分支/标签名（如 "main" 或 "refs/heads/main"）
    /// * `commit_sha` - 提交 SHA
    /// * `trigger_type` - 触发类型
    ///
    /// # Returns
    /// * `Ok(Some(result))` - 成功创建 pipeline
    /// * `Ok(None)` - 不需要触发（如删除分支）
    /// * `Err(e)` - 发生错误
    pub async fn trigger_pipeline(
        &self,
        project_id: i64,
        user_id: i64,
        ref_name: &str,
        commit_sha: &str,
        trigger_type: PipelineTriggerType,
    ) -> AppResult<Option<PipelineTriggerResult>> {
        let now = Utc::now();

        // 跳过删除操作
        if commit_sha == "0000000000000000000000000000000000000000" {
            debug!("Ref {} deleted, skipping CI", ref_name);
            return Ok(None);
        }

        // 标准化 ref_name（移除 refs/heads/ 前缀）
        let branch_name = ref_name
            .strip_prefix("refs/heads/")
            .or_else(|| ref_name.strip_prefix("refs/tags/"))
            .unwrap_or(ref_name);

        // 获取项目信息
        let project = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT n.path, p.name
            FROM projects p
            JOIN namespaces n ON p.namespace_id = n.id
            WHERE p.id = $1
            "#,
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?;

        let (namespace_path, project_name) = match project {
            Some(p) => p,
            None => {
                warn!("Project {} not found", project_id);
                return Ok(None);
            }
        };

        // 解析 CI 配置
        let ci_config = match CiConfigParser::parse_from_repo(&self.config, &namespace_path, &project_name, commit_sha).await {
            Ok(config) => config,
            Err(e) => {
                let error_msg = format!("CI configuration error: {}", e);
                warn!("{}", error_msg);

                let pipeline = self
                    .create_failed_pipeline(
                        project_id,
                        user_id,
                        branch_name,
                        commit_sha,
                        trigger_type,
                        &error_msg,
                        now,
                    )
                    .await?;

                return Ok(Some(PipelineTriggerResult {
                    pipeline_id: pipeline.id,
                    jobs_created: 0,
                    status: PipelineStatus::Failed,
                    error_message: Some(error_msg),
                }));
            }
        };

        // 检查是否有 jobs
        if ci_config.jobs.is_empty() {
            debug!("No jobs defined in CI config");

            let pipeline = self
                .create_skipped_pipeline(
                    project_id,
                    user_id,
                    branch_name,
                    commit_sha,
                    trigger_type,
                    now,
                )
                .await?;

            return Ok(Some(PipelineTriggerResult {
                pipeline_id: pipeline.id,
                jobs_created: 0,
                status: PipelineStatus::Skipped,
                error_message: None,
            }));
        }

        // 创建 pipeline
        let pipeline = sqlx::query_as::<_, Pipeline>(
            r#"
            INSERT INTO pipelines
            (project_id, ref_name, commit_sha, status, trigger_type, triggered_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
            RETURNING *
            "#,
        )
        .bind(project_id)
        .bind(branch_name)
        .bind(commit_sha)
        .bind(PipelineStatus::Pending)
        .bind(trigger_type)
        .bind(user_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        info!(
            "Created pipeline {} for project {} on {}",
            pipeline.id, project_id, branch_name
        );

        // 创建 jobs
        let mut jobs_created = 0;
        for (job_name, job_def) in &ci_config.jobs {
            // 检查 job 是否应该在这个 ref 上运行
            if !CiConfigParser::should_run_job(job_def, branch_name) {
                debug!("Job {} skipped due to only/except rules", job_name);
                continue;
            }

            // 构建 job 配置 JSON
            let job_config = serde_json::json!({
                "script": job_def.script,
                "before_script": job_def.before_script.as_ref().or(ci_config.before_script.as_ref()),
                "after_script": job_def.after_script.as_ref().or(ci_config.after_script.as_ref()),
                "variables": job_def.variables.as_ref().or(ci_config.variables.as_ref()),
                "artifacts": job_def.artifacts,
                "cache": job_def.cache,
                "retry": job_def.retry,
                "timeout": job_def.timeout,
                "tags": job_def.tags,
                "needs": job_def.needs,
            });

            // 插入 job
            sqlx::query(
                r#"
                INSERT INTO jobs
                (pipeline_id, project_id, name, stage, status, config, allow_failure, when_condition, created_at, updated_at)
                VALUES ($1, $2, $3, $4, 'pending', $5, $6, $7, $8, $8)
                "#,
            )
            .bind(pipeline.id)
            .bind(project_id)
            .bind(job_name)
            .bind(&job_def.stage)
            .bind(&job_config)
            .bind(job_def.allow_failure)
            .bind(&job_def.when)
            .bind(now)
            .execute(&self.pool)
            .await?;

            jobs_created += 1;
        }

        info!(
            "Created {} jobs for pipeline {}",
            jobs_created, pipeline.id
        );

        // 如果没有创建任何 jobs，标记为 skipped
        let final_status = if jobs_created == 0 {
            sqlx::query(
                r#"
                UPDATE pipelines
                SET status = 'skipped', updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(pipeline.id)
            .execute(&self.pool)
            .await?;

            debug!(
                "Pipeline {} marked as skipped (no matching jobs)",
                pipeline.id
            );
            PipelineStatus::Skipped
        } else {
            PipelineStatus::Pending
        };

        Ok(Some(PipelineTriggerResult {
            pipeline_id: pipeline.id,
            jobs_created,
            status: final_status,
            error_message: None,
        }))
    }

    /// 创建失败状态的 pipeline
    async fn create_failed_pipeline(
        &self,
        project_id: i64,
        user_id: i64,
        ref_name: &str,
        commit_sha: &str,
        trigger_type: PipelineTriggerType,
        error_msg: &str,
        now: chrono::DateTime<Utc>,
    ) -> AppResult<Pipeline> {
        let pipeline = sqlx::query_as::<_, Pipeline>(
            r#"
            INSERT INTO pipelines
            (project_id, ref_name, commit_sha, status, trigger_type, triggered_by, error_message, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
            RETURNING *
            "#,
        )
        .bind(project_id)
        .bind(ref_name)
        .bind(commit_sha)
        .bind(PipelineStatus::Failed)
        .bind(trigger_type)
        .bind(user_id)
        .bind(error_msg)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        info!(
            "Created failed pipeline {} with error: {}",
            pipeline.id, error_msg
        );
        Ok(pipeline)
    }

    /// 创建跳过状态的 pipeline
    async fn create_skipped_pipeline(
        &self,
        project_id: i64,
        user_id: i64,
        ref_name: &str,
        commit_sha: &str,
        trigger_type: PipelineTriggerType,
        now: chrono::DateTime<Utc>,
    ) -> AppResult<Pipeline> {
        let pipeline = sqlx::query_as::<_, Pipeline>(
            r#"
            INSERT INTO pipelines
            (project_id, ref_name, commit_sha, status, trigger_type, triggered_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
            RETURNING *
            "#,
        )
        .bind(project_id)
        .bind(ref_name)
        .bind(commit_sha)
        .bind(PipelineStatus::Skipped)
        .bind(trigger_type)
        .bind(user_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        info!(
            "Created skipped pipeline {} (no jobs defined)",
            pipeline.id
        );
        Ok(pipeline)
    }

    /// 通过项目路径触发 pipeline（用于 internal API）
    ///
    /// # Arguments
    /// * `namespace_path` - Namespace 路径
    /// * `project_name` - 项目名称
    /// * `user_id` - 触发用户 ID
    /// * `ref_name` - 分支/标签名
    /// * `commit_sha` - 提交 SHA
    pub async fn trigger_pipeline_by_path(
        &self,
        namespace_path: &str,
        project_name: &str,
        user_id: i64,
        ref_name: &str,
        commit_sha: &str,
    ) -> AppResult<Option<PipelineTriggerResult>> {
        // 查找项目 ID
        let project_id = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT p.id
            FROM projects p
            JOIN namespaces n ON p.namespace_id = n.id
            WHERE n.path = $1 AND p.name = $2
            "#,
        )
        .bind(namespace_path)
        .bind(project_name)
        .fetch_optional(&self.pool)
        .await?;

        match project_id {
            Some(id) => {
                self.trigger_pipeline(id, user_id, ref_name, commit_sha, PipelineTriggerType::Push)
                    .await
            }
            None => {
                warn!(
                    "Project not found: {}/{}",
                    namespace_path, project_name
                );
                Ok(None)
            }
        }
    }

    /// 通过仓库路径触发 pipeline（用于 gRPC）
    ///
    /// # Arguments
    /// * `repository` - 仓库路径（如 "namespace/project" 或 "namespace/project.git"）
    /// * `user_id` - 触发用户 ID
    /// * `ref_name` - 分支/标签名
    /// * `commit_sha` - 提交 SHA
    pub async fn trigger_pipeline_by_repo(
        &self,
        repository: &str,
        user_id: i64,
        ref_name: &str,
        commit_sha: &str,
    ) -> AppResult<Option<PipelineTriggerResult>> {
        // 解析仓库路径
        let repo_path = repository.trim_end_matches(".git");
        let parts: Vec<&str> = repo_path.rsplitn(2, '/').collect();

        if parts.len() != 2 {
            warn!("Invalid repository path: {}", repository);
            return Ok(None);
        }

        let (project_name, namespace_path) = (parts[0], parts[1]);
        self.trigger_pipeline_by_path(namespace_path, project_name, user_id, ref_name, commit_sha)
            .await
    }
}
