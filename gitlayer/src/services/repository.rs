//! Repository service implementation

use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::{debug, info};

use crate::config::Config;
use crate::git::repository::RepositoryOps;
use crate::proto::*;

pub struct RepositoryServiceImpl {
    config: Arc<Config>,
}

impl RepositoryServiceImpl {
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
}

#[tonic::async_trait]
impl repository_service_server::RepositoryService for RepositoryServiceImpl {
    async fn create_repository(
        &self,
        request: Request<CreateRepositoryRequest>,
    ) -> Result<Response<CreateRepositoryResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        let default_branch = if req.default_branch.is_empty() {
            "main"
        } else {
            &req.default_branch
        };
        
        info!("Creating repository at: {}", path);
        
        match RepositoryOps::create(&path, default_branch) {
            Ok(_) => Ok(Response::new(CreateRepositoryResponse {
                success: true,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(CreateRepositoryResponse {
                success: false,
                message: e.to_string(),
            })),
        }
    }
    
    async fn delete_repository(
        &self,
        request: Request<DeleteRepositoryRequest>,
    ) -> Result<Response<DeleteRepositoryResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        info!("Deleting repository at: {}", path);
        
        match RepositoryOps::delete(&path) {
            Ok(_) => Ok(Response::new(DeleteRepositoryResponse {
                success: true,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(DeleteRepositoryResponse {
                success: false,
                message: e.to_string(),
            })),
        }
    }
    
    async fn repository_exists(
        &self,
        request: Request<RepositoryExistsRequest>,
    ) -> Result<Response<RepositoryExistsResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        Ok(Response::new(RepositoryExistsResponse {
            exists: RepositoryOps::exists(&path),
        }))
    }
    
    async fn get_repository_info(
        &self,
        request: Request<GetRepositoryInfoRequest>,
    ) -> Result<Response<GetRepositoryInfoResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let is_empty = RepositoryOps::is_empty(&repo);
        let default_branch = RepositoryOps::default_branch(&repo).unwrap_or_else(|_| "main".to_string());
        let size_bytes = RepositoryOps::size(&path).unwrap_or(0);
        let object_count = RepositoryOps::object_count(&repo).unwrap_or(0);
        
        Ok(Response::new(GetRepositoryInfoResponse {
            is_empty,
            default_branch,
            size_bytes,
            object_count,
            commit_count: 0, // TODO: implement
            is_bare: repo.is_bare(),
        }))
    }
    
    async fn fork_repository(
        &self,
        request: Request<ForkRepositoryRequest>,
    ) -> Result<Response<ForkRepositoryResponse>, Status> {
        let req = request.into_inner();
        let source_path = self.get_repo_path(req.source.as_ref())?;
        let dest_path = self.get_repo_path(req.destination.as_ref())?;
        
        info!("Forking repository from {} to {}", source_path, dest_path);
        
        match RepositoryOps::fork(&source_path, &dest_path) {
            Ok(_) => Ok(Response::new(ForkRepositoryResponse {
                success: true,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(ForkRepositoryResponse {
                success: false,
                message: e.to_string(),
            })),
        }
    }
    
    async fn repository_size(
        &self,
        request: Request<RepositorySizeRequest>,
    ) -> Result<Response<RepositorySizeResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        Ok(Response::new(RepositorySizeResponse {
            size_bytes: RepositoryOps::size(&path).unwrap_or(0),
        }))
    }
    
    async fn set_head(
        &self,
        request: Request<SetHeadRequest>,
    ) -> Result<Response<SetHeadResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        match RepositoryOps::set_head(&repo, &req.branch) {
            Ok(_) => Ok(Response::new(SetHeadResponse {
                success: true,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(SetHeadResponse {
                success: false,
                message: e.to_string(),
            })),
        }
    }
    
    async fn get_default_branch(
        &self,
        request: Request<GetDefaultBranchRequest>,
    ) -> Result<Response<GetDefaultBranchResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let branch = RepositoryOps::default_branch(&repo).unwrap_or_else(|_| "main".to_string());
        
        Ok(Response::new(GetDefaultBranchResponse { branch }))
    }
    
    async fn garbage_collect(
        &self,
        request: Request<GarbageCollectRequest>,
    ) -> Result<Response<GarbageCollectResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        match RepositoryOps::gc(&path, req.prune) {
            Ok(_) => Ok(Response::new(GarbageCollectResponse {
                success: true,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(GarbageCollectResponse {
                success: false,
                message: e.to_string(),
            })),
        }
    }
    
    async fn set_config(
        &self,
        request: Request<SetConfigRequest>,
    ) -> Result<Response<SetConfigResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        match RepositoryOps::set_config(&repo, &req.key, &req.value) {
            Ok(_) => Ok(Response::new(SetConfigResponse { success: true })),
            Err(_) => Ok(Response::new(SetConfigResponse { success: false })),
        }
    }
    
    async fn get_config(
        &self,
        request: Request<GetConfigRequest>,
    ) -> Result<Response<GetConfigResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        match RepositoryOps::get_config(&repo, &req.key) {
            Ok(Some(value)) => Ok(Response::new(GetConfigResponse {
                value,
                found: true,
            })),
            Ok(None) => Ok(Response::new(GetConfigResponse {
                value: String::new(),
                found: false,
            })),
            Err(_) => Ok(Response::new(GetConfigResponse {
                value: String::new(),
                found: false,
            })),
        }
    }
}
