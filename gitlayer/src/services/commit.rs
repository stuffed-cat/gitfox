//! Commit service implementation

use std::pin::Pin;
use std::sync::Arc;

use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::config::Config;
use crate::git::repository::RepositoryOps;
use crate::git::commit::CommitOps;
use crate::proto::*;

pub struct CommitServiceImpl {
    config: Arc<Config>,
}

impl CommitServiceImpl {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
    
    fn get_repo_path(&self, repo: Option<&Repository>) -> Result<String, Status> {
        let repo = repo.ok_or_else(|| Status::invalid_argument("Repository required"))?;
        
        if !repo.storage_path.is_empty() {
            Ok(repo.storage_path.clone())
        } else if !repo.relative_path.is_empty() {
            Ok(self.config.repo_path(&repo.relative_path))
        } else {
            Err(Status::invalid_argument("Repository path required"))
        }
    }
    
    fn convert_commit(c: &crate::git::commit::CommitInfo) -> Commit {
        // Extract signature info if present
        let (signature, signature_status) = if let Some(ref gpg_sig) = c.gpg_signature {
            // Signature is present - status will be determined by the consumer
            // who can call the GPG verification service
            (gpg_sig.signature.clone(), "unverified".to_string())
        } else {
            (String::new(), String::new())
        };
        
        Commit {
            id: c.id.clone(),
            tree_id: c.tree_id.clone(),
            parent_ids: c.parent_ids.clone(),
            author: Some(Signature {
                name: c.author.name.clone(),
                email: c.author.email.clone(),
                timestamp: c.author.timestamp,
                timezone: c.author.timezone.clone(),
            }),
            committer: Some(Signature {
                name: c.committer.name.clone(),
                email: c.committer.email.clone(),
                timestamp: c.committer.timestamp,
                timezone: c.committer.timezone.clone(),
            }),
            message: c.message.clone(),
            short_message: c.short_message.clone(),
            signature,
            signature_status,
        }
    }
}

#[tonic::async_trait]
impl commit_service_server::CommitService for CommitServiceImpl {
    async fn get_commit(
        &self,
        request: Request<GetCommitRequest>,
    ) -> Result<Response<GetCommitResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        match CommitOps::get_commit(&repo, &req.revision) {
            Ok(Some(c)) => Ok(Response::new(GetCommitResponse {
                commit: Some(Self::convert_commit(&c)),
                found: true,
            })),
            Ok(None) => Ok(Response::new(GetCommitResponse {
                commit: None,
                found: false,
            })),
            Err(_) => Ok(Response::new(GetCommitResponse {
                commit: None,
                found: false,
            })),
        }
    }
    
    async fn list_commits(
        &self,
        request: Request<ListCommitsRequest>,
    ) -> Result<Response<ListCommitsResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let revision = if req.revision.is_empty() { "HEAD" } else { &req.revision };
        let path_filter = if req.path.is_empty() { None } else { Some(req.path.as_str()) };
        let limit = if req.limit > 0 { req.limit as usize } else { 100 };
        let offset = req.offset as usize;
        let after = if req.after > 0 { Some(req.after) } else { None };
        let before = if req.before > 0 { Some(req.before) } else { None };
        
        let (commits, has_more) = CommitOps::list_commits(
            &repo,
            revision,
            path_filter,
            limit,
            offset,
            req.include_merges,
            after,
            before,
        )?;
        
        // 计算总数（对于大型仓库可能较慢，但对分页 UI 是必要的）
        let total_count = CommitOps::count_commits(&repo, revision, path_filter).unwrap_or(0);
        
        let commit_protos: Vec<Commit> = commits.iter()
            .map(|c| Self::convert_commit(c))
            .collect();
        
        Ok(Response::new(ListCommitsResponse {
            commits: commit_protos,
            total_count: total_count as i32,
            has_more,
        }))
    }
    
    type StreamCommitsStream = Pin<Box<dyn Stream<Item = Result<Commit, Status>> + Send>>;
    
    async fn stream_commits(
        &self,
        request: Request<StreamCommitsRequest>,
    ) -> Result<Response<Self::StreamCommitsStream>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let revision = if req.revision.is_empty() { "HEAD".to_string() } else { req.revision };
        let path_filter = if req.path.is_empty() { None } else { Some(req.path) };
        let limit = if req.limit > 0 { req.limit as usize } else { 1000 };
        
        // 获取所有提交（streaming 场景不使用分页）
        let (commits, _) = CommitOps::list_commits(
            &repo,
            &revision,
            path_filter.as_deref(),
            limit,
            0,
            true,  // include_merges
            None,  // after
            None,  // before
        ).map_err(|e| Status::internal(format!("Failed to list commits: {}", e)))?;
        
        // 转换为 proto 格式
        let commit_protos: Vec<Commit> = commits.iter()
            .map(|c| Self::convert_commit(c))
            .collect();
        
        // 创建流
        let stream = tokio_stream::iter(commit_protos.into_iter().map(Ok));
        
        Ok(Response::new(Box::pin(stream)))
    }
    
    async fn count_commits(
        &self,
        request: Request<CountCommitsRequest>,
    ) -> Result<Response<CountCommitsResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let revision = if req.revision.is_empty() { "HEAD" } else { &req.revision };
        let path_filter = if req.path.is_empty() { None } else { Some(req.path.as_str()) };
        
        let count = CommitOps::count_commits(&repo, revision, path_filter)?;
        
        Ok(Response::new(CountCommitsResponse { count }))
    }
    
    async fn find_commits(
        &self,
        request: Request<FindCommitsRequest>,
    ) -> Result<Response<FindCommitsResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        if req.query.is_empty() {
            return Err(Status::invalid_argument("Search query required"));
        }
        
        let repo = RepositoryOps::open(&path)?;
        let search_in = if req.search_in.is_empty() { "all" } else { &req.search_in };
        let limit = if req.limit > 0 { req.limit as usize } else { 50 };
        let offset = req.offset as usize;
        
        let commits = CommitOps::search_commits(
            &repo,
            &req.query,
            search_in,
            limit,
            offset,
        ).map_err(|e| Status::internal(format!("Search failed: {}", e)))?;
        
        let commit_protos: Vec<Commit> = commits.iter()
            .map(|c| Self::convert_commit(c))
            .collect();
        
        Ok(Response::new(FindCommitsResponse {
            commits: commit_protos,
        }))
    }
    
    async fn is_ancestor(
        &self,
        request: Request<IsAncestorRequest>,
    ) -> Result<Response<IsAncestorResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        let is_ancestor = CommitOps::is_ancestor(&repo, &req.ancestor, &req.descendant)?;
        
        Ok(Response::new(IsAncestorResponse { is_ancestor }))
    }
    
    async fn merge_base(
        &self,
        request: Request<MergeBaseRequest>,
    ) -> Result<Response<MergeBaseResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let revisions: Vec<&str> = req.revisions.iter().map(|s| s.as_str()).collect();
        
        match CommitOps::merge_base(&repo, &revisions) {
            Ok(Some(commit_id)) => Ok(Response::new(MergeBaseResponse {
                commit_id,
                found: true,
            })),
            Ok(None) => Ok(Response::new(MergeBaseResponse {
                commit_id: String::new(),
                found: false,
            })),
            Err(_) => Ok(Response::new(MergeBaseResponse {
                commit_id: String::new(),
                found: false,
            })),
        }
    }
    
    async fn commits_between(
        &self,
        request: Request<CommitsBetweenRequest>,
    ) -> Result<Response<CommitsBetweenResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let limit = if req.limit > 0 { req.limit as usize } else { 100 };
        
        let commits = CommitOps::commits_between(&repo, &req.from, &req.to, limit)?;
        
        let commit_protos: Vec<Commit> = commits.iter()
            .map(|c| Self::convert_commit(c))
            .collect();
        
        Ok(Response::new(CommitsBetweenResponse {
            commits: commit_protos,
        }))
    }
    
    async fn get_commit_stats(
        &self,
        request: Request<GetCommitStatsRequest>,
    ) -> Result<Response<GetCommitStatsResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        // Get diff from parent
        use crate::git::diff::DiffOps;
        let (_, additions, deletions, files_changed) = DiffOps::commit_diff(
            &repo,
            None, // will diff with parent
            &req.revision,
            &[],
            0,
        )?;
        
        Ok(Response::new(GetCommitStatsResponse {
            stats: Some(CommitStats {
                additions,
                deletions,
                files_changed,
            }),
        }))
    }
}
