//! Tree service implementation

use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::config::Config;
use crate::git::repository::RepositoryOps;
use crate::git::tree::TreeOps;
use crate::proto::*;
use crate::services::commit::CommitServiceImpl;

pub struct TreeServiceImpl {
    config: Arc<Config>,
}

impl TreeServiceImpl {
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
    
    fn convert_entry(e: &crate::git::tree::TreeEntry) -> TreeEntry {
        TreeEntry {
            name: e.name.clone(),
            path: e.path.clone(),
            r#type: e.entry_type.clone(),
            id: e.id.clone(),
            mode: e.mode,
            size: e.size,
            submodule_url: e.submodule_url.clone().unwrap_or_default(),
        }
    }
}

#[tonic::async_trait]
impl tree_service_server::TreeService for TreeServiceImpl {
    async fn get_tree(
        &self,
        request: Request<GetTreeRequest>,
    ) -> Result<Response<GetTreeResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let tree_path = if req.path.is_empty() { "" } else { &req.path };
        // 使用请求中的 revision，如果为空则使用 HEAD
        let revision = if req.revision.is_empty() { "HEAD" } else { &req.revision };
        
        match TreeOps::get_tree(&repo, revision, tree_path, req.include_sizes) {
            Ok(Some(entries)) => {
                let entry_protos: Vec<TreeEntry> = entries.iter()
                    .map(|e| Self::convert_entry(e))
                    .collect();
                
                Ok(Response::new(GetTreeResponse {
                    entries: entry_protos,
                    found: true,
                }))
            }
            Ok(None) => Ok(Response::new(GetTreeResponse {
                entries: Vec::new(),
                found: false,
            })),
            Err(_) => Ok(Response::new(GetTreeResponse {
                entries: Vec::new(),
                found: false,
            })),
        }
    }
    
    async fn get_tree_recursive(
        &self,
        request: Request<GetTreeRecursiveRequest>,
    ) -> Result<Response<GetTreeRecursiveResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let tree_path = if req.path.is_empty() { "" } else { &req.path };
        let revision = if req.revision.is_empty() { "HEAD" } else { &req.revision };
        
        let entries = TreeOps::get_tree_recursive(&repo, revision, tree_path, req.max_depth)?;
        
        let entry_protos: Vec<TreeEntry> = entries.iter()
            .map(|e| Self::convert_entry(e))
            .collect();
        
        Ok(Response::new(GetTreeRecursiveResponse {
            entries: entry_protos,
        }))
    }
    
    async fn find_files(
        &self,
        request: Request<FindFilesRequest>,
    ) -> Result<Response<FindFilesResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let limit = if req.limit > 0 { req.limit as usize } else { 1000 };
        
        let paths = TreeOps::find_files(&repo, &req.revision, &req.pattern, limit)?;
        
        Ok(Response::new(FindFilesResponse { paths }))
    }
    
    async fn get_tree_with_commits(
        &self,
        request: Request<GetTreeWithCommitsRequest>,
    ) -> Result<Response<GetTreeWithCommitsResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let tree_path = if req.path.is_empty() { "" } else { &req.path };
        let revision = if req.revision.is_empty() { "HEAD" } else { &req.revision };
        
        let entries = TreeOps::get_tree_with_commits(&repo, revision, tree_path)?;
        
        let entry_protos: Vec<TreeEntryWithCommit> = entries.iter()
            .map(|e| {
                let commit = e.last_commit.as_ref().map(|c| Commit {
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
                });
                
                TreeEntryWithCommit {
                    entry: Some(Self::convert_entry(&e.entry)),
                    last_commit: commit,
                }
            })
            .collect();
        
        Ok(Response::new(GetTreeWithCommitsResponse {
            entries: entry_protos,
        }))
    }
    
    async fn get_tree_size(
        &self,
        request: Request<GetTreeSizeRequest>,
    ) -> Result<Response<GetTreeSizeResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let tree_path = if req.path.is_empty() { "" } else { &req.path };
        let revision = if req.revision.is_empty() { "HEAD" } else { &req.revision };
        
        let (total_size, file_count, dir_count) = TreeOps::get_tree_size(&repo, revision, tree_path)?;
        
        Ok(Response::new(GetTreeSizeResponse {
            total_size,
            file_count,
            dir_count,
        }))
    }
}
