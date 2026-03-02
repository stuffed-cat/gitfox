//! Repository operations using git2

use std::fs;
use std::path::Path;

use git2::{Repository, RepositoryInitOptions};
use tracing::{debug, info};
use walkdir::WalkDir;

use crate::error::{GitLayerError, Result};

pub struct RepositoryOps;

impl RepositoryOps {
    /// Create a new bare repository
    pub fn create(path: &str, default_branch: &str) -> Result<Repository> {
        let path = Path::new(path);
        
        if path.exists() {
            return Err(GitLayerError::RepositoryExists(path.display().to_string()));
        }
        
        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        info!("Creating bare repository at: {}", path.display());
        
        let mut opts = RepositoryInitOptions::new();
        opts.bare(true);
        opts.initial_head(default_branch);
        
        let repo = Repository::init_opts(path, &opts)?;
        
        debug!("Repository created successfully");
        Ok(repo)
    }
    
    /// Open an existing repository
    pub fn open(path: &str) -> Result<Repository> {
        let path = Path::new(path);
        
        if !path.exists() {
            return Err(GitLayerError::RepositoryNotFound(path.display().to_string()));
        }
        
        Repository::open(path).map_err(|e| {
            GitLayerError::RepositoryNotFound(format!("{}: {}", path.display(), e))
        })
    }
    
    /// Check if a repository exists
    pub fn exists(path: &str) -> bool {
        let path = Path::new(path);
        path.exists() && Repository::open(path).is_ok()
    }
    
    /// Delete a repository
    pub fn delete(path: &str) -> Result<()> {
        let path = Path::new(path);
        
        if !path.exists() {
            return Err(GitLayerError::RepositoryNotFound(path.display().to_string()));
        }
        
        info!("Deleting repository at: {}", path.display());
        fs::remove_dir_all(path)?;
        Ok(())
    }
    
    /// Get repository size in bytes
    pub fn size(path: &str) -> Result<u64> {
        let path = Path::new(path);
        
        if !path.exists() {
            return Err(GitLayerError::RepositoryNotFound(path.display().to_string()));
        }
        
        let mut total_size = 0u64;
        
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                total_size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
        
        Ok(total_size)
    }
    
    /// Check if repository is empty (no commits)
    pub fn is_empty(repo: &Repository) -> bool {
        repo.is_empty().unwrap_or(true)
    }
    
    /// Get the default branch name
    pub fn default_branch(repo: &Repository) -> Result<String> {
        let head = repo.head().map_err(|_| {
            GitLayerError::RefNotFound("HEAD".to_string())
        })?;
        
        if head.is_branch() {
            let name = head.shorthand().unwrap_or("main");
            Ok(name.to_string())
        } else {
            // Try to find the default branch from config
            if let Ok(config) = repo.config() {
                if let Ok(default) = config.get_string("init.defaultBranch") {
                    return Ok(default);
                }
            }
            Ok("main".to_string())
        }
    }
    
    /// Set HEAD to point to a branch
    pub fn set_head(repo: &Repository, branch: &str) -> Result<()> {
        let refname = if branch.starts_with("refs/") {
            branch.to_string()
        } else {
            format!("refs/heads/{}", branch)
        };
        
        repo.set_head(&refname)?;
        Ok(())
    }
    
    /// Fork/clone a repository
    pub fn fork(source_path: &str, dest_path: &str) -> Result<Repository> {
        let source = Path::new(source_path);
        let dest = Path::new(dest_path);
        
        if !source.exists() {
            return Err(GitLayerError::RepositoryNotFound(source.display().to_string()));
        }
        
        if dest.exists() {
            return Err(GitLayerError::RepositoryExists(dest.display().to_string()));
        }
        
        info!("Forking repository from {} to {}", source.display(), dest.display());
        
        // Create parent directories
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Clone with mirror mode
        let repo = Repository::clone(
            &format!("file://{}", source.display()),
            dest,
        )?;
        
        Ok(repo)
    }
    
    /// Get object count in repository
    pub fn object_count(repo: &Repository) -> Result<u64> {
        let odb = repo.odb()?;
        let mut count = 0u64;
        
        odb.foreach(|_oid| {
            count += 1;
            true
        })?;
        
        Ok(count)
    }
    
    /// Run garbage collection
    pub fn gc(path: &str, prune: bool) -> Result<()> {
        use std::process::Command;
        
        let mut cmd = Command::new("git");
        cmd.current_dir(path);
        cmd.arg("gc");
        
        if prune {
            cmd.arg("--prune=now");
        }
        
        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitLayerError::Internal(format!("git gc failed: {}", stderr)));
        }
        
        Ok(())
    }
    
    /// Set git config value
    pub fn set_config(repo: &Repository, key: &str, value: &str) -> Result<()> {
        let mut config = repo.config()?;
        config.set_str(key, value)?;
        Ok(())
    }
    
    /// Get git config value
    pub fn get_config(repo: &Repository, key: &str) -> Result<Option<String>> {
        let config = repo.config()?;
        match config.get_string(key) {
            Ok(value) => Ok(Some(value)),
            Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
