//! Blob service implementation

use std::pin::Pin;
use std::sync::Arc;

use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::config::Config;
use crate::git::repository::RepositoryOps;
use crate::git::blob::BlobOps;
use crate::proto::*;

pub struct BlobServiceImpl {
    config: Arc<Config>,
}

impl BlobServiceImpl {
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
impl blob_service_server::BlobService for BlobServiceImpl {
    async fn get_blob(
        &self,
        request: Request<GetBlobRequest>,
    ) -> Result<Response<GetBlobResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        match BlobOps::get_blob(&repo, &req.blob_id) {
            Ok(Some(blob)) => Ok(Response::new(GetBlobResponse {
                blob: Some(Blob {
                    id: blob.id,
                    size: blob.size,
                    data: blob.data,
                    is_binary: blob.is_binary,
                }),
                found: true,
            })),
            Ok(None) => Ok(Response::new(GetBlobResponse {
                blob: None,
                found: false,
            })),
            Err(_) => Ok(Response::new(GetBlobResponse {
                blob: None,
                found: false,
            })),
        }
    }
    
    type StreamBlobStream = Pin<Box<dyn Stream<Item = Result<BlobChunk, Status>> + Send>>;
    
    async fn stream_blob(
        &self,
        _request: Request<StreamBlobRequest>,
    ) -> Result<Response<Self::StreamBlobStream>, Status> {
        Err(Status::unimplemented("Streaming not yet implemented"))
    }
    
    async fn get_blob_size(
        &self,
        request: Request<GetBlobSizeRequest>,
    ) -> Result<Response<GetBlobSizeResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        match BlobOps::get_blob_size(&repo, &req.blob_id) {
            Ok(Some(size)) => Ok(Response::new(GetBlobSizeResponse {
                size,
                found: true,
            })),
            Ok(None) => Ok(Response::new(GetBlobSizeResponse {
                size: 0,
                found: false,
            })),
            Err(_) => Ok(Response::new(GetBlobSizeResponse {
                size: 0,
                found: false,
            })),
        }
    }
    
    async fn get_file_content(
        &self,
        request: Request<GetFileContentRequest>,
    ) -> Result<Response<GetFileContentResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        match BlobOps::get_file_content(&repo, &req.revision, &req.path) {
            Ok(Some(blob)) => Ok(Response::new(GetFileContentResponse {
                data: blob.data,
                size: blob.size,
                is_binary: blob.is_binary,
                found: true,
                blob_id: blob.id,
            })),
            Ok(None) => Ok(Response::new(GetFileContentResponse {
                data: Vec::new(),
                size: 0,
                is_binary: false,
                found: false,
                blob_id: String::new(),
            })),
            Err(_) => Ok(Response::new(GetFileContentResponse {
                data: Vec::new(),
                size: 0,
                is_binary: false,
                found: false,
                blob_id: String::new(),
            })),
        }
    }
    
    type StreamFileContentStream = Pin<Box<dyn Stream<Item = Result<BlobChunk, Status>> + Send>>;
    
    async fn stream_file_content(
        &self,
        _request: Request<StreamFileContentRequest>,
    ) -> Result<Response<Self::StreamFileContentStream>, Status> {
        Err(Status::unimplemented("Streaming not yet implemented"))
    }
    
    async fn path_exists(
        &self,
        request: Request<PathExistsRequest>,
    ) -> Result<Response<PathExistsResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        match BlobOps::path_exists(&repo, &req.revision, &req.path) {
            Ok(Some(object_type)) => Ok(Response::new(PathExistsResponse {
                exists: true,
                object_type,
            })),
            Ok(None) => Ok(Response::new(PathExistsResponse {
                exists: false,
                object_type: String::new(),
            })),
            Err(_) => Ok(Response::new(PathExistsResponse {
                exists: false,
                object_type: String::new(),
            })),
        }
    }
    
    async fn blame(
        &self,
        request: Request<BlameRequest>,
    ) -> Result<Response<BlameResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let start = if req.start_line > 0 { Some(req.start_line) } else { None };
        let end = if req.end_line > 0 { Some(req.end_line) } else { None };
        
        let lines = BlobOps::blame(&repo, &req.revision, &req.path, start, end)?;
        
        let line_protos: Vec<BlameLine> = lines.iter()
            .map(|l| BlameLine {
                line_number: l.line_number,
                commit_id: l.commit_id.clone(),
                author_name: l.author_name.clone(),
                author_email: l.author_email.clone(),
                author_date: l.author_date,
                content: l.content.clone(),
                original_path: l.original_path.clone(),
                original_line: l.original_line,
            })
            .collect();
        
        Ok(Response::new(BlameResponse { lines: line_protos }))
    }
    
    async fn get_lfs_pointers(
        &self,
        request: Request<GetLfsPointersRequest>,
    ) -> Result<Response<GetLfsPointersResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let revision = if req.revision.is_empty() { "HEAD" } else { &req.revision };
        
        // 如果指定了路径，只检查这些路径；否则扫描整个树
        let pointers = if req.paths.is_empty() {
            BlobOps::scan_lfs_pointers(&repo, revision)
                .map_err(|e| Status::internal(format!("Failed to scan LFS pointers: {}", e)))?
        } else {
            BlobOps::get_lfs_pointers(&repo, revision, &req.paths)
                .map_err(|e| Status::internal(format!("Failed to get LFS pointers: {}", e)))?
        };
        
        let pointer_protos: Vec<LfsPointer> = pointers.into_iter()
            .map(|p| LfsPointer {
                oid: p.oid,
                size: p.size,
                path: p.path,
            })
            .collect();
        
        Ok(Response::new(GetLfsPointersResponse {
            pointers: pointer_protos,
        }))
    }
}
