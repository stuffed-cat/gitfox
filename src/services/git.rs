//! Git service using GitLayer gRPC
//! 所有 Git 操作通过 GitLayer gRPC 服务完成

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::models::{
    BlobContent, BranchInfo, CommitDetail, CommitInfo, CommitStats, DiffInfo, DiffStatus,
    FileContent, FileEntry, FileEntryType, RepositoryInfo, TagInfo,
};
use crate::services::gitlayer::{
    self, BlobServiceClient, CommitServiceClient, DiffServiceClient, OperationServiceClient,
    RefServiceClient, RepositoryServiceClient, TreeServiceClient,
};

/// Helper to create a gRPC Repository message
fn make_repository(config: &AppConfig, owner_name: &str, project_name: &str) -> gitlayer::Repository {
    let relative_path = format!("{}/{}.git", owner_name, project_name);
    let storage_path = format!("{}/{}", config.git_repos_path, relative_path);
    gitlayer::Repository {
        storage_path,
        relative_path,
    }
}

/// Get GitLayer address from config or environment
fn get_gitlayer_address(config: &AppConfig) -> String {
    config
        .gitlayer_address
        .clone()
        .or_else(|| std::env::var("GITLAYER_URL").ok())
        .unwrap_or_else(|| "http://[::1]:50052".to_string())
}

/// Helper to create a gRPC Signature with current timestamp
fn make_signature(name: &str, email: &str) -> gitlayer::Signature {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    gitlayer::Signature {
        name: name.to_string(),
        email: email.to_string(),
        timestamp,
        timezone: "+0000".to_string(),
    }
}

pub struct GitService;

impl GitService {
    /// 初始化一个新的空仓库
    pub async fn init_repository(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
    ) -> AppResult<()> {
        let addr = get_gitlayer_address(config);
        let mut client = RepositoryServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::CreateRepositoryRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            default_branch: "main".to_string(),
            initialize: false,
        };

        let response = client
            .create_repository(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to create repository: {}", e)))?;

        if !response.into_inner().success {
            return Err(AppError::InternalError(
                "Failed to create repository".to_string(),
            ));
        }

        Ok(())
    }

    /// 删除一个仓库
    pub async fn delete_repository(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
    ) -> AppResult<()> {
        let addr = get_gitlayer_address(config);
        let mut client = RepositoryServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::DeleteRepositoryRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
        };

        let response = client
            .delete_repository(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete repository: {}", e)))?;

        if !response.into_inner().success {
            return Err(AppError::InternalError(
                "Failed to delete repository".to_string(),
            ));
        }

        Ok(())
    }

    /// Fork a repository by cloning it to a new location
    pub async fn fork_repository(
        config: &AppConfig,
        source_owner: &str,
        source_name: &str,
        target_owner: &str,
        target_name: &str,
        _only_default_branch: bool,
    ) -> AppResult<()> {
        let addr = get_gitlayer_address(config);
        let mut client = RepositoryServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::ForkRepositoryRequest {
            source: Some(make_repository(config, source_owner, source_name)),
            destination: Some(make_repository(config, target_owner, target_name)),
        };

        let response = client
            .fork_repository(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to fork repository: {}", e)))?;

        if !response.into_inner().success {
            return Err(AppError::InternalError(
                "Failed to fork repository".to_string(),
            ));
        }

        Ok(())
    }

    /// 获取仓库的默认分支
    pub async fn get_default_branch(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
    ) -> AppResult<Option<String>> {
        let addr = get_gitlayer_address(config);
        let mut client = RepositoryServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::GetDefaultBranchRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
        };

        let response = client
            .get_default_branch(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to get default branch: {}", e)))?;

        let branch = response.into_inner().branch;
        if branch.is_empty() {
            Ok(None)
        } else {
            Ok(Some(branch))
        }
    }

    /// 获取仓库信息
    pub async fn get_repository_info(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
    ) -> AppResult<RepositoryInfo> {
        let addr = get_gitlayer_address(config);

        // Get repository info
        let mut repo_client = RepositoryServiceClient::connect(addr.clone())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let repo_info_request = gitlayer::GetRepositoryInfoRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
        };

        let repo_info = repo_client
            .get_repository_info(repo_info_request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to get repository info: {}", e)))?
            .into_inner();

        // Get branches
        let mut ref_client = RefServiceClient::connect(addr.clone())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let branches_request = gitlayer::ListBranchesRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            pattern: String::new(),
            limit: 0,
            offset: 0,
        };

        let branches_response = ref_client
            .list_branches(branches_request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to list branches: {}", e)))?
            .into_inner();

        let branches: Vec<String> = branches_response
            .branches
            .iter()
            .map(|b| b.name.clone())
            .collect();

        // Get tags
        let tags_request = gitlayer::ListTagsRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            pattern: String::new(),
            limit: 0,
            offset: 0,
        };

        let tags_response = ref_client
            .list_tags(tags_request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to list tags: {}", e)))?
            .into_inner();

        let tags: Vec<String> = tags_response.tags.iter().map(|t| t.name.clone()).collect();

        // Get last commit on default branch
        let default_branch = if repo_info.default_branch.is_empty() {
            None
        } else {
            Some(repo_info.default_branch.clone())
        };

        let last_commit = if let Some(ref branch) = default_branch {
            let mut commit_client = CommitServiceClient::connect(addr)
                .await
                .map_err(|e| {
                    AppError::InternalError(format!("Failed to connect to GitLayer: {}", e))
                })?;

            let commit_request = gitlayer::ListCommitsRequest {
                repository: Some(make_repository(config, owner_name, project_name)),
                revision: branch.clone(),
                path: String::new(),
                limit: 1,
                offset: 0,
                include_merges: true,
                order: "date".to_string(),
                after: 0,
                before: 0,
            };

            commit_client
                .list_commits(commit_request)
                .await
                .ok()
                .and_then(|r| r.into_inner().commits.into_iter().next())
                .map(|c| grpc_commit_to_info(&c))
        } else {
            None
        };

        Ok(RepositoryInfo {
            default_branch,
            branches,
            tags,
            size_kb: repo_info.size_bytes / 1024,
            last_commit,
        })
    }

    /// 获取分支列表
    pub async fn get_branches(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
    ) -> AppResult<Vec<BranchInfo>> {
        let addr = get_gitlayer_address(config);
        let mut client = RefServiceClient::connect(addr.clone())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::ListBranchesRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            pattern: String::new(),
            limit: 0,
            offset: 0,
        };

        let response = client
            .list_branches(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to list branches: {}", e)))?
            .into_inner();

        // Get commit info for each branch
        let mut commit_client = CommitServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let mut branches = Vec::new();
        for branch in response.branches {
            let commit_request = gitlayer::GetCommitRequest {
                repository: Some(make_repository(config, owner_name, project_name)),
                revision: branch.commit_id.clone(),
            };

            let commit_info = commit_client
                .get_commit(commit_request)
                .await
                .ok()
                .and_then(|r| {
                    let inner = r.into_inner();
                    if inner.found {
                        inner.commit.map(|c| grpc_commit_to_info(&c))
                    } else {
                        None
                    }
                });

            if let Some(commit) = commit_info {
                branches.push(BranchInfo {
                    name: branch.name,
                    commit,
                    is_protected: branch.is_protected,
                    is_default: branch.is_default,
                });
            }
        }

        Ok(branches)
    }

    /// 创建分支
    pub async fn create_branch(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        name: &str,
        ref_name: &str,
    ) -> AppResult<()> {
        let addr = get_gitlayer_address(config);
        let mut client = RefServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::CreateBranchRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            name: name.to_string(),
            start_point: ref_name.to_string(),
        };

        let response = client
            .create_branch(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to create branch: {}", e)))?;

        if !response.into_inner().success {
            return Err(AppError::InternalError(
                "Failed to create branch".to_string(),
            ));
        }

        Ok(())
    }

    /// 删除分支
    pub async fn delete_branch(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        name: &str,
    ) -> AppResult<()> {
        let addr = get_gitlayer_address(config);
        let mut client = RefServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::DeleteBranchRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            name: name.to_string(),
            force: false,
        };

        let response = client
            .delete_branch(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete branch: {}", e)))?;

        if !response.into_inner().success {
            return Err(AppError::InternalError(
                "Failed to delete branch".to_string(),
            ));
        }

        Ok(())
    }

    /// 获取标签列表
    pub async fn get_tags(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
    ) -> AppResult<Vec<TagInfo>> {
        let addr = get_gitlayer_address(config);
        let mut client = RefServiceClient::connect(addr.clone())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::ListTagsRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            pattern: String::new(),
            limit: 0,
            offset: 0,
        };

        let response = client
            .list_tags(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to list tags: {}", e)))?
            .into_inner();

        // Get commit info for each tag
        let mut commit_client = CommitServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let mut tags = Vec::new();
        for tag in response.tags {
            let commit_request = gitlayer::GetCommitRequest {
                repository: Some(make_repository(config, owner_name, project_name)),
                revision: tag.target_id.clone(),
            };

            let commit_info = commit_client
                .get_commit(commit_request)
                .await
                .ok()
                .and_then(|r| {
                    let inner = r.into_inner();
                    if inner.found {
                        inner.commit.map(|c| grpc_commit_to_info(&c))
                    } else {
                        None
                    }
                });

            if let Some(commit) = commit_info {
                tags.push(TagInfo {
                    name: tag.name,
                    commit,
                    message: if tag.message.is_empty() {
                        None
                    } else {
                        Some(tag.message)
                    },
                    tagger_name: if tag.tagger_name.is_empty() {
                        None
                    } else {
                        Some(tag.tagger_name)
                    },
                    tagger_email: if tag.tagger_email.is_empty() {
                        None
                    } else {
                        Some(tag.tagger_email)
                    },
                    created_at: chrono::DateTime::from_timestamp(tag.tagger_date, 0)
                        .unwrap_or_else(chrono::Utc::now),
                });
            }
        }

        Ok(tags)
    }

    /// 创建标签
    pub async fn create_tag(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        name: &str,
        ref_name: &str,
        message: Option<&str>,
        tagger_name: &str,
        tagger_email: &str,
    ) -> AppResult<()> {
        let addr = get_gitlayer_address(config);
        let mut client = RefServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::CreateTagRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            name: name.to_string(),
            target: ref_name.to_string(),
            message: message.unwrap_or("").to_string(),
            tagger_name: tagger_name.to_string(),
            tagger_email: tagger_email.to_string(),
        };

        let response = client
            .create_tag(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to create tag: {}", e)))?;

        if !response.into_inner().success {
            return Err(AppError::InternalError("Failed to create tag".to_string()));
        }

        Ok(())
    }

    /// 删除标签
    pub async fn delete_tag(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        name: &str,
    ) -> AppResult<()> {
        let addr = get_gitlayer_address(config);
        let mut client = RefServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::DeleteTagRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            name: name.to_string(),
        };

        let response = client
            .delete_tag(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete tag: {}", e)))?;

        if !response.into_inner().success {
            return Err(AppError::InternalError("Failed to delete tag".to_string()));
        }

        Ok(())
    }

    /// 获取提交列表
    pub async fn get_commits(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        ref_name: &str,
        path: Option<&str>,
        page: u32,
        per_page: u32,
    ) -> AppResult<Vec<CommitInfo>> {
        let addr = get_gitlayer_address(config);
        let mut client = CommitServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let offset = (page.saturating_sub(1)) * per_page;
        let request = gitlayer::ListCommitsRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            revision: ref_name.to_string(),
            path: path.unwrap_or("").to_string(),
            limit: per_page as i32,
            offset: offset as i32,
            include_merges: true,
            order: "date".to_string(),
            after: 0,
            before: 0,
        };

        let response = client
            .list_commits(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to list commits: {}", e)))?
            .into_inner();

        Ok(response.commits.iter().map(grpc_commit_to_info).collect())
    }

    /// 获取提交详情
    pub async fn get_commit_detail(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        sha: &str,
    ) -> AppResult<CommitDetail> {
        let addr = get_gitlayer_address(config);

        // Get commit info
        let mut commit_client = CommitServiceClient::connect(addr.clone())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let commit_request = gitlayer::GetCommitRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            revision: sha.to_string(),
        };

        let commit_response = commit_client
            .get_commit(commit_request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to get commit: {}", e)))?
            .into_inner();

        if !commit_response.found {
            return Err(AppError::NotFound(format!("Commit '{}' not found", sha)));
        }

        let commit = commit_response.commit.ok_or_else(|| {
            AppError::InternalError("Commit data missing from response".to_string())
        })?;

        // Get diff for this commit
        let mut diff_client = DiffServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let diff_request = gitlayer::CommitDiffRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            old_revision: String::new(), // Empty = diff with parent
            new_revision: sha.to_string(),
            paths: vec![],
            context_lines: 3,
        };

        let diff_response = diff_client
            .commit_diff(diff_request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to get commit diff: {}", e)))?
            .into_inner();

        let diffs: Vec<DiffInfo> = diff_response
            .files
            .iter()
            .map(|f| grpc_diff_file_to_info(f))
            .collect();

        Ok(CommitDetail {
            sha: commit.id,
            message: commit.message,
            author_name: commit.author.as_ref().map(|a| a.name.clone()).unwrap_or_default(),
            author_email: commit.author.as_ref().map(|a| a.email.clone()).unwrap_or_default(),
            authored_date: commit.author.as_ref().map(|a| a.timestamp).unwrap_or(0),
            committer_name: commit.committer.as_ref().map(|c| c.name.clone()).unwrap_or_default(),
            committer_email: commit.committer.as_ref().map(|c| c.email.clone()).unwrap_or_default(),
            committed_date: commit.committer.as_ref().map(|c| c.timestamp).unwrap_or(0),
            parent_shas: commit.parent_ids,
            stats: CommitStats {
                additions: diff_response.total_additions as u32,
                deletions: diff_response.total_deletions as u32,
                files_changed: diff_response.files_changed as u32,
            },
            diffs,
            gpg_verification: None,
        })
    }

    /// 获取完整文件 diff（用于 Monaco Editor）
    pub async fn get_full_file_diff(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        sha: &str,
        file_path: &str,
    ) -> AppResult<crate::handlers::commit::FullFileDiff> {
        let addr = get_gitlayer_address(config);

        // Get parent commit
        let mut commit_client = CommitServiceClient::connect(addr.clone())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let commit_request = gitlayer::GetCommitRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            revision: sha.to_string(),
        };

        let commit_response = commit_client
            .get_commit(commit_request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to get commit: {}", e)))?
            .into_inner();

        if !commit_response.found {
            return Err(AppError::NotFound(format!("Commit '{}' not found", sha)));
        }

        let commit = commit_response.commit.ok_or_else(|| {
            AppError::InternalError("Commit data missing from response".to_string())
        })?;

        let parent_sha = commit.parent_ids.first().cloned();

        // Get file content at current commit
        let mut blob_client = BlobServiceClient::connect(addr.clone())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let modified_content = {
            let request = gitlayer::GetFileContentRequest {
                repository: Some(make_repository(config, owner_name, project_name)),
                revision: sha.to_string(),
                path: file_path.to_string(),
            };

            blob_client
                .get_file_content(request)
                .await
                .ok()
                .and_then(|r| {
                    let inner = r.into_inner();
                    if inner.found && !inner.is_binary {
                        String::from_utf8(inner.data).ok()
                    } else {
                        None
                    }
                })
        };

        // Get file content at parent commit
        let original_content = if let Some(ref parent) = parent_sha {
            let request = gitlayer::GetFileContentRequest {
                repository: Some(make_repository(config, owner_name, project_name)),
                revision: parent.clone(),
                path: file_path.to_string(),
            };

            blob_client
                .get_file_content(request)
                .await
                .ok()
                .and_then(|r| {
                    let inner = r.into_inner();
                    if inner.found && !inner.is_binary {
                        String::from_utf8(inner.data).ok()
                    } else {
                        None
                    }
                })
        } else {
            None
        };

        let total_lines = modified_content
            .as_ref()
            .or(original_content.as_ref())
            .map(|c| c.lines().count() as u32)
            .unwrap_or(0);

        Ok(crate::handlers::commit::FullFileDiff {
            original_content,
            modified_content,
            total_lines,
        })
    }

    /// 浏览目录树
    pub async fn browse_tree(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        ref_name: &str,
        path: Option<&str>,
    ) -> AppResult<Vec<FileEntry>> {
        let addr = get_gitlayer_address(config);
        let mut client = TreeServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::GetTreeRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            revision: ref_name.to_string(),
            path: path.unwrap_or("").to_string(),
            include_sizes: true,
        };

        let response = client
            .get_tree(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to get tree: {}", e)))?
            .into_inner();

        if !response.found {
            return Err(AppError::NotFound(format!(
                "Path '{}' not found",
                path.unwrap_or("")
            )));
        }

        let mut entries: Vec<FileEntry> = response
            .entries
            .iter()
            .map(|e| {
                let entry_type = match e.r#type.as_str() {
                    "tree" => FileEntryType::Directory,
                    "commit" => FileEntryType::Submodule,
                    _ => FileEntryType::File,
                };
                FileEntry {
                    name: e.name.clone(),
                    path: e.path.clone(),
                    entry_type,
                    size: if e.size > 0 { Some(e.size as u64) } else { None },
                    mode: e.mode as u32,
                }
            })
            .collect();

        // Sort: directories first, then files
        entries.sort_by(|a, b| match (&a.entry_type, &b.entry_type) {
            (FileEntryType::Directory, FileEntryType::File) => std::cmp::Ordering::Less,
            (FileEntryType::File, FileEntryType::Directory) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        Ok(entries)
    }

    /// 获取文件内容
    pub async fn get_file_content(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        ref_name: &str,
        path: &str,
    ) -> AppResult<FileContent> {
        let addr = get_gitlayer_address(config);
        let mut client = BlobServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::GetFileContentRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            revision: ref_name.to_string(),
            path: path.to_string(),
        };

        let response = client
            .get_file_content(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to get file content: {}", e)))?
            .into_inner();

        if !response.found {
            return Err(AppError::NotFound(format!("File '{}' not found", path)));
        }

        let is_binary = response.is_binary;
        let content = if is_binary {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(&response.data)
        } else {
            String::from_utf8_lossy(&response.data).to_string()
        };

        Ok(FileContent {
            path: path.to_string(),
            content,
            size: response.size as u64,
            encoding: if is_binary { "base64" } else { "utf-8" }.to_string(),
            is_binary,
        })
    }

    /// 比较两个引用之间的提交
    pub async fn compare_refs(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        from: &str,
        to: &str,
    ) -> AppResult<Vec<CommitInfo>> {
        let addr = get_gitlayer_address(config);
        let mut client = CommitServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::CommitsBetweenRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            from: from.to_string(),
            to: to.to_string(),
            limit: 0,
        };

        let response = client
            .commits_between(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to compare refs: {}", e)))?
            .into_inner();

        Ok(response.commits.iter().map(grpc_commit_to_info).collect())
    }

    /// 计算两个引用之间的分歧 (ahead, behind)
    pub async fn calculate_divergence(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        local_ref: &str,
        upstream_ref: &str,
    ) -> AppResult<(usize, usize)> {
        let addr = get_gitlayer_address(config);
        let mut client = DiffServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::CompareRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            from: upstream_ref.to_string(),
            to: local_ref.to_string(),
            straight: false,
            limit: 0,
        };

        let response = client
            .compare(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to compare refs: {}", e)))?
            .into_inner();

        Ok((
            response.ahead_count as usize,
            response.behind_count as usize,
        ))
    }

    /// 获取 blob 内容
    pub async fn get_blob(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        sha: &str,
    ) -> AppResult<BlobContent> {
        let addr = get_gitlayer_address(config);
        let mut client = BlobServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::GetBlobRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            blob_id: sha.to_string(),
        };

        let response = client
            .get_blob(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to get blob: {}", e)))?
            .into_inner();

        if !response.found {
            return Err(AppError::NotFound(format!("Blob '{}' not found", sha)));
        }

        let blob = response.blob.ok_or_else(|| {
            AppError::InternalError("Blob data missing from response".to_string())
        })?;

        let is_binary = blob.is_binary;
        let content = if is_binary {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(&blob.data)
        } else {
            String::from_utf8_lossy(&blob.data).to_string()
        };

        Ok(BlobContent {
            sha: sha.to_string(),
            content,
            size: blob.size as u64,
            encoding: if is_binary { "base64" } else { "utf-8" }.to_string(),
            is_binary,
        })
    }

    /// 检查是否可以合并
    pub async fn can_merge(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        source: &str,
        target: &str,
    ) -> AppResult<bool> {
        let addr = get_gitlayer_address(config);
        let mut client = DiffServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::FindConflictsRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            our_revision: target.to_string(),
            their_revision: source.to_string(),
        };

        let response = client
            .find_conflicts(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to check conflicts: {}", e)))?
            .into_inner();

        Ok(!response.has_conflicts)
    }

    /// 执行合并
    pub async fn perform_merge(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        source_branch: &str,
        target_branch: &str,
        merge_message: &str,
        author_name: &str,
        author_email: &str,
    ) -> AppResult<String> {
        let addr = get_gitlayer_address(config);
        let mut client = OperationServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::MergeRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            source_branch: source_branch.to_string(),
            target_branch: target_branch.to_string(),
            author: Some(make_signature(author_name, author_email)),
            message: merge_message.to_string(),
            strategy: gitlayer::MergeStrategy::Merge as i32,
            allow_conflicts: false,
        };

        let response = client
            .merge(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to perform merge: {}", e)))?
            .into_inner();

        if !response.success {
            if response.has_conflicts {
                return Err(AppError::Conflict("Cannot merge due to conflicts".to_string()));
            }
            return Err(AppError::InternalError(format!(
                "Failed to merge: {}",
                response.message
            )));
        }

        Ok(response.commit_id)
    }

    /// 从 fork 仓库 fetch 并合并到目标仓库
    pub async fn fetch_and_merge_from_fork(
        config: &AppConfig,
        target_owner: &str,
        target_name: &str,
        source_owner: &str,
        source_name: &str,
        source_branch: &str,
        _target_branch: &str,
    ) -> AppResult<()> {
        let addr = get_gitlayer_address(config);
        let mut client = RepositoryServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let source_path = format!(
            "{}/{}/{}.git",
            config.git_repos_path, source_owner, source_name
        );

        let request = gitlayer::FetchFromRemoteRequest {
            repository: Some(make_repository(config, target_owner, target_name)),
            remote_path: source_path,
            remote_name: "fork-source".to_string(),
            branches: vec![source_branch.to_string()],
            prune: false,
        };

        let response = client
            .fetch_from_remote(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to fetch from fork: {}", e)))?
            .into_inner();

        if !response.success {
            return Err(AppError::InternalError(format!(
                "Failed to fetch from fork: {}",
                response.message
            )));
        }

        Ok(())
    }

    /// 提交文件更改
    pub async fn commit_file_change(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        branch: &str,
        file_path: &str,
        content: &str,
        commit_message: &str,
        author_name: &str,
        author_email: &str,
    ) -> AppResult<String> {
        let addr = get_gitlayer_address(config);
        let mut client = OperationServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::WriteFileRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            branch: branch.to_string(),
            path: file_path.to_string(),
            content: content.as_bytes().to_vec(),
            author: Some(make_signature(author_name, author_email)),
            committer: Some(make_signature(author_name, author_email)),
            message: commit_message.to_string(),
            create_branch: false,
            gpg_private_key: String::new(),
        };

        let response = client
            .write_file(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to write file: {}", e)))?
            .into_inner();

        if !response.success {
            return Err(AppError::InternalError(format!(
                "Failed to commit file: {}",
                response.message
            )));
        }

        Ok(response.commit_id)
    }

    /// 删除文件并提交
    pub async fn delete_file_commit(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        branch: &str,
        file_path: &str,
        commit_message: &str,
        author_name: &str,
        author_email: &str,
    ) -> AppResult<String> {
        let addr = get_gitlayer_address(config);
        let mut client = OperationServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let request = gitlayer::DeleteFileRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            branch: branch.to_string(),
            path: file_path.to_string(),
            author: Some(make_signature(author_name, author_email)),
            committer: Some(make_signature(author_name, author_email)),
            message: commit_message.to_string(),
            gpg_private_key: String::new(),
        };

        let response = client
            .delete_file(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to delete file: {}", e)))?
            .into_inner();

        if !response.success {
            return Err(AppError::InternalError(format!(
                "Failed to delete file: {}",
                response.message
            )));
        }

        Ok(response.commit_id)
    }

    /// 批量提交多个文件更改
    pub async fn batch_commit_changes(
        config: &AppConfig,
        owner_name: &str,
        project_name: &str,
        branch: &str,
        changes: Vec<FileChange>,
        commit_message: &str,
        author_name: &str,
        author_email: &str,
    ) -> AppResult<String> {
        let addr = get_gitlayer_address(config);
        let mut client = OperationServiceClient::connect(addr)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;

        let actions: Vec<gitlayer::FileAction> = changes
            .iter()
            .map(|c| {
                let action = match c.action {
                    FileChangeAction::Create => "create",
                    FileChangeAction::Update => "update",
                    FileChangeAction::Delete => "delete",
                };
                gitlayer::FileAction {
                    action: action.to_string(),
                    path: c.path.clone(),
                    content: c.content.as_ref().map(|s| s.as_bytes().to_vec()).unwrap_or_default(),
                    previous_path: String::new(),
                    mode: 0o100644,
                    expected_blob_id: String::new(),
                }
            })
            .collect();

        let request = gitlayer::CreateCommitRequest {
            repository: Some(make_repository(config, owner_name, project_name)),
            branch: branch.to_string(),
            author: Some(make_signature(author_name, author_email)),
            committer: Some(make_signature(author_name, author_email)),
            message: commit_message.to_string(),
            actions,
            create_branch: false,
            expected_parent: String::new(),
            gpg_private_key: String::new(),
        };

        let response = client
            .create_commit(request)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to create commit: {}", e)))?
            .into_inner();

        if !response.success {
            return Err(AppError::InternalError(format!(
                "Failed to batch commit: {}",
                response.message
            )));
        }

        Ok(response.commit_id)
    }
}

/// 文件更改
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub action: FileChangeAction,
    pub content: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileChangeAction {
    Create,
    Update,
    Delete,
}

// Helper functions to convert gRPC types to model types

fn grpc_commit_to_info(commit: &gitlayer::Commit) -> CommitInfo {
    CommitInfo {
        sha: commit.id.clone(),
        message: commit.message.clone(),
        author_name: commit.author.as_ref().map(|a| a.name.clone()).unwrap_or_default(),
        author_email: commit.author.as_ref().map(|a| a.email.clone()).unwrap_or_default(),
        authored_date: commit.author.as_ref().map(|a| a.timestamp).unwrap_or(0),
        committer_name: commit.committer.as_ref().map(|c| c.name.clone()).unwrap_or_default(),
        committer_email: commit.committer.as_ref().map(|c| c.email.clone()).unwrap_or_default(),
        committed_date: commit.committer.as_ref().map(|c| c.timestamp).unwrap_or(0),
        gpg_verification: None,
    }
}

fn grpc_diff_file_to_info(diff: &gitlayer::DiffFile) -> DiffInfo {
    let status = match diff.status.as_str() {
        "added" => DiffStatus::Added,
        "deleted" => DiffStatus::Deleted,
        "renamed" => DiffStatus::Renamed,
        "copied" => DiffStatus::Copied,
        _ => DiffStatus::Modified,
    };

    // Build diff string from hunks
    let mut diff_str = String::new();
    for hunk in &diff.hunks {
        diff_str.push_str(&hunk.header);
        diff_str.push('\n');
        for line in &hunk.lines {
            diff_str.push_str(&line.prefix);
            diff_str.push_str(&line.content);
            if !line.content.ends_with('\n') {
                diff_str.push('\n');
            }
        }
    }

    DiffInfo {
        old_path: diff.old_path.clone(),
        new_path: diff.new_path.clone(),
        diff: diff_str,
        status,
        additions: diff.additions as u32,
        deletions: diff.deletions as u32,
        original_content: None,
        modified_content: None,
        is_truncated: false,
        total_lines: None,
    }
}
