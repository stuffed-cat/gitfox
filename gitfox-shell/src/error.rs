//! Error types for GitFox Shell

use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShellError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Repository not found: {0}")]
    RepoNotFound(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Git command failed: {0}")]
    GitExecution(String),

    #[error("GitLayer connection error: {0}")]
    GitLayerConnection(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl ShellError {
    /// Returns a user-friendly message
    pub fn user_message(&self) -> String {
        match self {
            ShellError::Config(_) => {
                "Internal configuration error. Please contact your administrator.".to_string()
            }
            ShellError::InvalidCommand(msg) => format!("Invalid command: {}", msg),
            ShellError::AccessDenied(msg) => format!("Access denied: {}", msg),
            ShellError::RepoNotFound(repo) => {
                format!(
                    "Repository '{}' not found. Please make sure the repository exists and you have access.",
                    repo
                )
            }
            ShellError::Api(_) => {
                "Unable to verify access. Please try again later.".to_string()
            }
            ShellError::Auth(msg) => format!("Authentication failed: {}", msg),
            ShellError::GitExecution(msg) => format!("Git operation failed: {}", msg),
            ShellError::Io(_) => "An I/O error occurred.".to_string(),
            ShellError::Http(_) => "Network error. Please try again later.".to_string(),
            ShellError::Json(_) => "Internal error processing response.".to_string(),
            ShellError::GitLayerConnection(msg) => format!("GitLayer connection error: {}", msg),
        }
    }

    /// Check if this error should be logged at error level
    pub fn is_user_error(&self) -> bool {
        matches!(
            self,
            ShellError::InvalidCommand(_)
                | ShellError::AccessDenied(_)
                | ShellError::RepoNotFound(_)
        )
    }
}
