use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;
use std::path::Path;

/// GitFox CI 配置解析器
/// 支持 .gitfox/ci/ 目录风格
pub struct CiConfigParser;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiConfig {
    pub stages: Vec<String>,
    pub jobs: HashMap<String, JobDefinition>,
    pub variables: Option<HashMap<String, String>>,
    pub before_script: Option<Vec<String>>,
    pub after_script: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDefinition {
    pub stage: String,
    pub script: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_script: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_script: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<ArtifactsDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<CacheDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
    #[serde(default)]
    pub allow_failure: bool,
    #[serde(default = "default_when")]
    pub when: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub except: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs: Option<Vec<String>>,
}

fn default_when() -> String {
    "on_success".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactsDefinition {
    pub paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_in: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheDefinition {
    pub paths: Vec<String>,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RetryDefinition {
    Simple(i32),
    Complex { max: i32, when: Vec<String> },
}

impl CiConfigParser {
    /// 从 git 仓库的指定 commit 解析 CI 配置
    pub async fn parse_from_repo(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        commit_sha: &str,
    ) -> AppResult<CiConfig> {
        use crate::services::GitService;
        
        // 尝试读取 .gitfox/ci/ 目录下的所有 yml/yaml 文件
        let ci_dir_path = ".gitfox/ci";
        
        // 浏览树获取文件列表
        let entries = match GitService::browse_tree(config, owner_name, project_name, commit_sha, Some(ci_dir_path)).await {
            Ok(entries) => entries,
            Err(_) => {
                return Err(AppError::BadRequest(
                    "No CI configuration found. Please create .gitfox/ci/*.yml files.".to_string()
                ));
            }
        };
        
        // 收集所有 yaml 文件
        let mut yaml_files: Vec<(String, String)> = Vec::new();
        for entry in entries {
            if entry.entry_type == crate::models::FileEntryType::File {
                let path = entry.path.to_lowercase();
                if path.ends_with(".yml") || path.ends_with(".yaml") {
                    // entry.path already contains full path like ".gitfox/ci/build.yml"
                    match GitService::get_file_content(config, owner_name, project_name, commit_sha, &entry.path).await {
                        Ok(file_content) => {
                            if !file_content.is_binary {
                                yaml_files.push((entry.name.clone(), file_content.content));
                            }
                        }
                        Err(e) => {
                            return Err(AppError::BadRequest(
                                format!("Failed to read CI file {}: {}", entry.path, e)
                            ));
                        }
                    }
                }
            }
        }
        
        if yaml_files.is_empty() {
            return Err(AppError::BadRequest(
                "No valid CI configuration files found in .gitfox/ci/".to_string()
            ));
        }
        
        // 排序保证顺序
        yaml_files.sort_by(|a, b| a.0.cmp(&b.0));
        
        let mut merged_config = CiConfig {
            stages: Vec::new(),
            jobs: HashMap::new(),
            variables: None,
            before_script: None,
            after_script: None,
        };
        
        // 解析并合并所有配置文件
        for (filename, content) in yaml_files {
            let config: Value = serde_yaml::from_str(&content)
                .map_err(|e| AppError::BadRequest(
                    format!("Invalid YAML syntax in {}: {}", filename, e)
                ))?;
            
            Self::merge_config(&mut merged_config, config)
                .map_err(|e| AppError::BadRequest(
                    format!("Failed to parse CI config in {}: {}", filename, e)
                ))?;
        }
        
        // 如果没有定义 stages，从 jobs 中推断
        if merged_config.stages.is_empty() {
            let mut stages: Vec<String> = merged_config
                .jobs
                .values()
                .map(|job| job.stage.clone())
                .collect();
            stages.sort();
            stages.dedup();
            if stages.is_empty() {
                return Err(AppError::BadRequest(
                    "No jobs defined in CI configuration".to_string()
                ));
            }
            merged_config.stages = stages;
        }
        
        Ok(merged_config)
    }
    
    /// 从 .gitfox/ci/ 目录解析所有 CI 配置
    pub async fn parse_from_directory<P: AsRef<Path>>(ci_dir: P) -> AppResult<CiConfig> {
        let ci_dir = ci_dir.as_ref();
        
        if !ci_dir.exists() {
            return Err(crate::error::AppError::NotFound(
                "CI directory not found".to_string(),
            ));
        }

        let mut merged_config = CiConfig {
            stages: Vec::new(),
            jobs: HashMap::new(),
            variables: None,
            before_script: None,
            after_script: None,
        };

        // 读取所有 .yml 和 .yaml 文件
        let mut entries = tokio::fs::read_dir(ci_dir).await?;
        let mut yaml_files = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "yml" || ext == "yaml" {
                        yaml_files.push(path);
                    }
                }
            }
        }

        // 排序以保证确定性顺序
        yaml_files.sort();

        // 解析并合并所有配置文件
        for yaml_file in yaml_files {
            let content = tokio::fs::read_to_string(&yaml_file).await?;
            let config: Value = serde_yaml::from_str(&content)?;

            Self::merge_config(&mut merged_config, config)?;
        }

        // 如果没有定义 stages，从 jobs 中推断
        if merged_config.stages.is_empty() {
            let mut stages: Vec<String> = merged_config
                .jobs
                .values()
                .map(|job| job.stage.clone())
                .collect();
            stages.sort();
            stages.dedup();
            merged_config.stages = stages;
        }

        Ok(merged_config)
    }

    fn merge_config(target: &mut CiConfig, source: Value) -> AppResult<()> {
        if let Value::Mapping(map) = source {
            for (key, value) in map {
                if let Some(key_str) = key.as_str() {
                    match key_str {
                        "stages" => {
                            if let Value::Sequence(stages) = value {
                                for stage in stages {
                                    if let Some(s) = stage.as_str() {
                                        if !target.stages.contains(&s.to_string()) {
                                            target.stages.push(s.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        "variables" => {
                            if let Ok(vars) = serde_yaml::from_value::<HashMap<String, String>>(value) {
                                target.variables.get_or_insert_with(HashMap::new).extend(vars);
                            }
                        }
                        "before_script" => {
                            if let Ok(script) = serde_yaml::from_value::<Vec<String>>(value) {
                                target.before_script = Some(script);
                            }
                        }
                        "after_script" => {
                            if let Ok(script) = serde_yaml::from_value::<Vec<String>>(value) {
                                target.after_script = Some(script);
                            }
                        }
                        // 其他键作为 job 定义
                        _ => {
                            if !key_str.starts_with('.') {
                                // 跳过隐藏的锚点定义
                                if let Ok(mut job) = serde_yaml::from_value::<JobDefinition>(value.clone()) {
                                    // 如果 job 没有指定 stage，使用默认值
                                    if job.stage.is_empty() {
                                        job.stage = "test".to_string();
                                    }
                                    target.jobs.insert(key_str.to_string(), job);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 检查 job 是否应该在指定的 ref 上运行
    pub fn should_run_job(job: &JobDefinition, ref_name: &str) -> bool {
        // 检查 only 规则
        if let Some(only) = &job.only {
            if !Self::matches_ref_patterns(ref_name, only) {
                return false;
            }
        }

        // 检查 except 规则
        if let Some(except) = &job.except {
            if Self::matches_ref_patterns(ref_name, except) {
                return false;
            }
        }

        true
    }

    fn matches_ref_patterns(ref_name: &str, patterns: &[String]) -> bool {
        for pattern in patterns {
            match pattern.as_str() {
                "branches" => {
                    // Assume it's a branch (tags usually have version-like names)
                    // For now, just check if it doesn't look like a semantic version
                    if !ref_name.chars().next().map_or(false, |c| c.is_numeric()) {
                        return true;
                    }
                }
                "tags" => {
                    // Assume it's a tag if it starts with a number (like v1.0.0 or 1.0.0)
                    if ref_name.chars().next().map_or(false, |c| c.is_numeric() || c == 'v') {
                        return true;
                    }
                }
                _ => {
                    if ref_name.contains(pattern) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_filtering() {
        let job = JobDefinition {
            stage: "test".to_string(),
            script: vec!["echo test".to_string()],
            before_script: None,
            after_script: None,
            variables: None,
            artifacts: None,
            cache: None,
            retry: None,
            timeout: None,
            allow_failure: false,
            when: "on_success".to_string(),
            only: Some(vec!["main".to_string()]),
            except: None,
            tags: None,
            needs: None,
        };

        assert!(CiConfigParser::should_run_job(&job, "refs/heads/main"));
        assert!(!CiConfigParser::should_run_job(&job, "refs/heads/develop"));
    }
}
