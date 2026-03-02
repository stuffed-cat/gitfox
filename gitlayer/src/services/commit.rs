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
            signature: String::new(),
            signature_status: String::new(),
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
        
        let commit_protos: Vec<Commit> = commits.iter()
            .map(|c| Self::convert_commit(c))
            .collect();
        
        Ok(Response::new(ListCommitsResponse {
            commits: commit_protos,
            total_count: 0, // TODO: implement
            has_more,
        }))
    }
    
    type StreamCommitsStream = Pin<Box<dyn Stream<Item = Result<Commit, Status>> + Send>>;
    
    async fn stream_commits(
        &self,
        _request: Request<StreamCommitsRequest>,
    ) -> Result<Response<Self::StreamCommitsStream>, Status> {
        // TODO: Implement streaming
        Err(Status::unimplemented("Streaming not yet implemented"))
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
        _request: Request<FindCommitsRequest>,
    ) -> Result<Response<FindCommitsResponse>, Status> {
        // TODO: Implement search
        Err(Status::unimplemented("Search not yet implemented"))
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
