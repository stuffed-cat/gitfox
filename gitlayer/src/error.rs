//! GitLayer error types

use thiserror::Error;
use tonic::Status;

#[derive(Error, Debug)]
pub enum GitLayerError {
    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),
    
    #[error("Repository already exists: {0}")]
    RepositoryExists(String),
    
    #[error("Reference not found: {0}")]
    RefNotFound(String),
    
    #[error("Commit not found: {0}")]
    CommitNotFound(String),
    
    #[error("Object not found: {0}")]
    ObjectNotFound(String),
    
    #[error("Invalid revision: {0}")]
    InvalidRevision(String),
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Path not found: {0}")]
    PathNotFound(String),
    
    #[error("Merge conflict: {0}")]
    MergeConflict(String),
    
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

pub type Result<T> = std::result::Result<T, GitLayerError>;

impl From<GitLayerError> for Status {
    fn from(err: GitLayerError) -> Self {
        match err {
            GitLayerError::RepositoryNotFound(msg) => Status::not_found(msg),
            GitLayerError::RepositoryExists(msg) => Status::already_exists(msg),
            GitLayerError::RefNotFound(msg) => Status::not_found(msg),
            GitLayerError::CommitNotFound(msg) => Status::not_found(msg),
            GitLayerError::ObjectNotFound(msg) => Status::not_found(msg),
            GitLayerError::InvalidRevision(msg) => Status::invalid_argument(msg),
            GitLayerError::InvalidPath(msg) => Status::invalid_argument(msg),
            GitLayerError::PathNotFound(msg) => Status::not_found(msg),
            GitLayerError::MergeConflict(msg) => Status::failed_precondition(msg),
            GitLayerError::Git(e) => Status::internal(e.to_string()),
            GitLayerError::Io(e) => Status::internal(e.to_string()),
            GitLayerError::Internal(msg) => Status::internal(msg),
            GitLayerError::InvalidArgument(msg) => Status::invalid_argument(msg),
        }
    }
}
