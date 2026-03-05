//! Operations service implementation

use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::info;

use crate::config::Config;
use crate::git::repository::RepositoryOps;
use crate::git::operations::{OperationOps, SignatureInfo, FileAction};
use crate::proto::*;

pub struct OperationServiceImpl {
    config: Arc<Config>,
}

impl OperationServiceImpl {
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
    
    fn convert_signature(sig: Option<&Signature>) -> SignatureInfo {
        sig.map(|s| SignatureInfo {
            name: s.name.clone(),
            email: s.email.clone(),
        }).unwrap_or_else(|| SignatureInfo {
            name: "Unknown".to_string(),
            email: "unknown@example.com".to_string(),
        })
    }
}

#[tonic::async_trait]
impl operation_service_server::OperationService for OperationServiceImpl {
    async fn create_commit(
        &self,
        request: Request<CreateCommitRequest>,
    ) -> Result<Response<CreateCommitResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let author = Self::convert_signature(req.author.as_ref());
        let committer = Self::convert_signature(req.committer.as_ref());
        
        let actions: Vec<FileAction> = req.actions.iter()
            .map(|a| FileAction {
                action: a.action.clone(),
                path: a.path.clone(),
                content: if a.content.is_empty() { None } else { Some(a.content.clone()) },
                previous_path: if a.previous_path.is_empty() { None } else { Some(a.previous_path.clone()) },
                mode: if a.mode > 0 { Some(a.mode) } else { None },
            })
            .collect();
        
        // Use GPG signing if private key is provided
        let gpg_key = if req.gpg_private_key.is_empty() {
            None
        } else {
            Some(req.gpg_private_key.as_str())
        };
        
        match OperationOps::create_commit_with_signature(&repo, &req.branch, &author, &committer, &req.message, &actions, req.create_branch, gpg_key) {
            Ok(commit_id) => Ok(Response::new(CreateCommitResponse {
                success: true,
                commit_id,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(CreateCommitResponse {
                success: false,
                commit_id: String::new(),
                message: e.to_string(),
            })),
        }
    }
    
    async fn merge(
        &self,
        request: Request<MergeRequest>,
    ) -> Result<Response<MergeResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let author = Self::convert_signature(req.author.as_ref());
        
        let strategy = match req.strategy() {
            MergeStrategy::Merge => "merge",
            MergeStrategy::Squash => "squash",
            MergeStrategy::FastForward => "fast_forward",
            MergeStrategy::Rebase => "rebase",
            _ => "merge",
        };
        
        match OperationOps::merge(&repo, &req.source_branch, &req.target_branch, &author, &req.message, strategy) {
            Ok((commit_id, has_conflicts, conflict_files)) => Ok(Response::new(MergeResponse {
                success: !has_conflicts && !commit_id.is_empty(),
                commit_id,
                message: String::new(),
                has_conflicts,
                conflict_files,
            })),
            Err(e) => Ok(Response::new(MergeResponse {
                success: false,
                commit_id: String::new(),
                message: e.to_string(),
                has_conflicts: false,
                conflict_files: Vec::new(),
            })),
        }
    }
    
    async fn revert(
        &self,
        request: Request<RevertRequest>,
    ) -> Result<Response<RevertResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let author = Self::convert_signature(req.author.as_ref());
        
        match OperationOps::revert(&repo, &req.branch, &req.commit_id, &author, &req.message) {
            Ok(commit_id) => Ok(Response::new(RevertResponse {
                success: true,
                commit_id,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(RevertResponse {
                success: false,
                commit_id: String::new(),
                message: e.to_string(),
            })),
        }
    }
    
    async fn cherry_pick(
        &self,
        request: Request<CherryPickRequest>,
    ) -> Result<Response<CherryPickResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let author = Self::convert_signature(req.author.as_ref());
        
        match OperationOps::cherry_pick(&repo, &req.branch, &req.commit_ids, &author) {
            Ok(new_commits) => Ok(Response::new(CherryPickResponse {
                success: true,
                new_commit_ids: new_commits,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(CherryPickResponse {
                success: false,
                new_commit_ids: Vec::new(),
                message: e.to_string(),
            })),
        }
    }
    
    async fn squash(
        &self,
        request: Request<SquashRequest>,
    ) -> Result<Response<SquashResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let author = Self::convert_signature(req.author.as_ref());
        
        match OperationOps::squash(&repo, &req.branch, &req.start_commit, &req.end_commit, &author, &req.message) {
            Ok(commit_id) => Ok(Response::new(SquashResponse {
                success: true,
                commit_id,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(SquashResponse {
                success: false,
                commit_id: String::new(),
                message: e.to_string(),
            })),
        }
    }
    
    async fn write_file(
        &self,
        request: Request<WriteFileRequest>,
    ) -> Result<Response<WriteFileResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let author = Self::convert_signature(req.author.as_ref());
        let committer = Self::convert_signature(req.committer.as_ref());
        let gpg_key = if req.gpg_private_key.is_empty() { None } else { Some(req.gpg_private_key.as_str()) };
        
        match OperationOps::write_file(&repo, &req.branch, &req.path, &req.content, &author, &committer, &req.message, req.create_branch, gpg_key) {
            Ok((commit_id, blob_id)) => Ok(Response::new(WriteFileResponse {
                success: true,
                commit_id,
                blob_id,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(WriteFileResponse {
                success: false,
                commit_id: String::new(),
                blob_id: String::new(),
                message: e.to_string(),
            })),
        }
    }
    
    async fn delete_file(
        &self,
        request: Request<DeleteFileRequest>,
    ) -> Result<Response<DeleteFileResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let author = Self::convert_signature(req.author.as_ref());
        let committer = Self::convert_signature(req.committer.as_ref());
        let gpg_key = if req.gpg_private_key.is_empty() { None } else { Some(req.gpg_private_key.as_str()) };
        
        match OperationOps::delete_file(&repo, &req.branch, &req.path, &author, &committer, &req.message, gpg_key) {
            Ok(commit_id) => Ok(Response::new(DeleteFileResponse {
                success: true,
                commit_id,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(DeleteFileResponse {
                success: false,
                commit_id: String::new(),
                message: e.to_string(),
            })),
        }
    }
    
    async fn move_file(
        &self,
        request: Request<MoveFileRequest>,
    ) -> Result<Response<MoveFileResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let author = Self::convert_signature(req.author.as_ref());
        let committer = Self::convert_signature(req.committer.as_ref());
        let gpg_key = if req.gpg_private_key.is_empty() { None } else { Some(req.gpg_private_key.as_str()) };
        
        match OperationOps::move_file(&repo, &req.branch, &req.old_path, &req.new_path, &author, &committer, &req.message, gpg_key) {
            Ok(commit_id) => Ok(Response::new(MoveFileResponse {
                success: true,
                commit_id,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(MoveFileResponse {
                success: false,
                commit_id: String::new(),
                message: e.to_string(),
            })),
        }
    }
}
