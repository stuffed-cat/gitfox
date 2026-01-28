//! SSH session handling for Git operations

use std::collections::HashMap;
use std::sync::Arc;

use log::{debug, info, warn};
use russh::server::Session;
use russh::ChannelId;
use sqlx::PgPool;
use tokio::sync::Mutex;

use crate::config::Config;

/// Represents an authenticated Git session
#[derive(Clone)]
pub struct GitSession {
    /// User ID of the authenticated user
    pub user_id: Option<i64>,
    /// Username of the authenticated user
    pub username: Option<String>,
    /// SSH key ID used for authentication
    pub key_id: Option<i64>,
    /// Whether the user has been authenticated
    pub authenticated: bool,
    /// Active channels and their state
    pub channels: Arc<Mutex<HashMap<ChannelId, ChannelState>>>,
    /// Database pool
    pub pool: Arc<PgPool>,
    /// Application config
    pub config: Arc<Config>,
}

/// State of a channel
#[derive(Clone, Default)]
pub struct ChannelState {
    /// The Git command being executed
    pub command: Option<String>,
    /// Repository path
    pub repo_path: Option<String>,
    /// Whether we're waiting for data
    pub waiting_for_data: bool,
    /// Collected data for the command
    pub data_buffer: Vec<u8>,
}

impl GitSession {
    /// Create a new unauthenticated session
    pub fn new(config: Arc<Config>, pool: Arc<PgPool>) -> Self {
        Self {
            user_id: None,
            username: None,
            key_id: None,
            authenticated: false,
            channels: Arc::new(Mutex::new(HashMap::new())),
            pool,
            config,
        }
    }

    /// Authenticate the session with a key
    pub fn authenticate(&mut self, user_id: i64, username: String, key_id: i64) {
        self.user_id = Some(user_id);
        self.username = Some(username);
        self.key_id = Some(key_id);
        self.authenticated = true;
    }

    /// Check if session is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Get the username
    pub fn get_username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Parse a Git command from the exec request
    pub fn parse_git_command(command: &str) -> Option<(GitAction, String)> {
        let command = command.trim();
        
        // Handle both "git-upload-pack" and "git upload-pack" formats
        let normalized = command
            .replace("git upload-pack", "git-upload-pack")
            .replace("git receive-pack", "git-receive-pack")
            .replace("git upload-archive", "git-upload-archive");

        let parts: Vec<&str> = normalized.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        let action = match parts[0] {
            "git-upload-pack" => GitAction::UploadPack,
            "git-receive-pack" => GitAction::ReceivePack,
            "git-upload-archive" => GitAction::UploadArchive,
            _ => return None,
        };

        // Extract repository path, removing quotes
        let repo_path = parts[1]
            .trim_matches('\'')
            .trim_matches('"')
            .trim_start_matches('/')
            .trim_end_matches(".git")
            .to_string();

        Some((action, repo_path))
    }
}

/// Git actions that can be performed over SSH
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitAction {
    /// git-upload-pack (clone, fetch, pull)
    UploadPack,
    /// git-receive-pack (push)
    ReceivePack,
    /// git-upload-archive
    UploadArchive,
}

impl GitAction {
    /// Check if this action requires write access
    pub fn requires_write(&self) -> bool {
        matches!(self, GitAction::ReceivePack)
    }

    /// Get the git binary name
    pub fn binary_name(&self) -> &'static str {
        match self {
            GitAction::UploadPack => "git-upload-pack",
            GitAction::ReceivePack => "git-receive-pack",
            GitAction::UploadArchive => "git-upload-archive",
        }
    }
}
