//! GitLayer - Git operations service
//!
//! GitLayer is similar to GitLab's Gitaly, providing Git operations
//! through gRPC interface using the git2 library.

pub mod config;
pub mod error;
pub mod git;
pub mod services;

// Re-export proto definitions
pub mod proto {
    tonic::include_proto!("gitlayer");
}

pub use config::Config;
pub use error::{GitLayerError, Result};
