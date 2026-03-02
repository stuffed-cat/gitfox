//! Tree (directory) operations

use std::path::Path;

use git2::{Repository, ObjectType};
use tracing::debug;

use crate::error::{GitLayerError, Result};
use crate::git::commit::{CommitOps, CommitInfo};

#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub name: String,
    pub path: String,
    pub entry_type: String,
    pub id: String,
    pub mode: i32,
    pub size: i64,
    pub submodule_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TreeEntryWithCommit {
    pub entry: TreeEntry,
    pub last_commit: Option<CommitInfo>,
}

pub struct TreeOps;

impl TreeOps {
    /// Get tree entries (directory listing)
    pub fn get_tree(
        repo: &Repository,
        revision: &str,
        path: &str,
        include_sizes: bool,
    ) -> Result<Option<Vec<TreeEntry>>> {
        let obj = repo.revparse_single(revision)
            .map_err(|_| GitLayerError::InvalidRevision(revision.to_string()))?;
        
        let commit = obj.peel_to_commit()
            .map_err(|_| GitLayerError::InvalidRevision(format!("{} is not a commit", revision)))?;
        
        let tree = commit.tree()?;
        
        // Navigate to the requested path
        let target_tree = if path.is_empty() || path == "." || path == "/" {
            tree
        } else {
            let entry = match tree.get_path(Path::new(path)) {
                Ok(e) => e,
                Err(_) => return Ok(None),
            };
            
            if entry.kind() != Some(ObjectType::Tree) {
                return Ok(None);
            }
            
            repo.find_tree(entry.id())?
        };
        
        let mut entries = Vec::new();
        
        for entry in target_tree.iter() {
            let name = entry.name().unwrap_or("").to_string();
            let entry_path = if path.is_empty() || path == "." || path == "/" {
                name.clone()
            } else {
                format!("{}/{}", path, name)
            };
            
            let entry_type = match entry.kind() {
                Some(ObjectType::Blob) => "blob",
                Some(ObjectType::Tree) => "tree",
                Some(ObjectType::Commit) => "commit", // submodule
                _ => "unknown",
            };
            
            let mut size = 0i64;
            if include_sizes && entry.kind() == Some(ObjectType::Blob) {
                if let Ok(blob) = repo.find_blob(entry.id()) {
                    size = blob.size() as i64;
                }
            }
            
            // Get submodule URL if it's a submodule
            let submodule_url = if entry.kind() == Some(ObjectType::Commit) {
                repo.find_submodule(&name)
                    .ok()
                    .and_then(|s| s.url().map(|u| u.to_string()))
            } else {
                None
            };
            
            entries.push(TreeEntry {
                name,
                path: entry_path,
                entry_type: entry_type.to_string(),
                id: entry.id().to_string(),
                mode: entry.filemode() as i32,
                size,
                submodule_url,
            });
        }
        
        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| {
            let a_is_tree = a.entry_type == "tree";
            let b_is_tree = b.entry_type == "tree";
            
            match (a_is_tree, b_is_tree) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });
        
        Ok(Some(entries))
    }
    
    /// Get tree entries recursively
    pub fn get_tree_recursive(
        repo: &Repository,
        revision: &str,
        path: &str,
        max_depth: i32,
    ) -> Result<Vec<TreeEntry>> {
        let obj = repo.revparse_single(revision)
            .map_err(|_| GitLayerError::InvalidRevision(revision.to_string()))?;
        
        let commit = obj.peel_to_commit()
            .map_err(|_| GitLayerError::InvalidRevision(format!("{} is not a commit", revision)))?;
        
        let tree = commit.tree()?;
        
        // Navigate to the requested path
        let (target_tree, base_path) = if path.is_empty() || path == "." || path == "/" {
            (tree, String::new())
        } else {
            let entry = tree.get_path(Path::new(path))
                .map_err(|_| GitLayerError::PathNotFound(path.to_string()))?;
            
            if entry.kind() != Some(ObjectType::Tree) {
                return Err(GitLayerError::InvalidPath(format!("{} is not a directory", path)));
            }
            
            (repo.find_tree(entry.id())?, path.to_string())
        };
        
        let mut entries = Vec::new();
        Self::collect_tree_entries(repo, &target_tree, &base_path, 0, max_depth, &mut entries)?;
        
        Ok(entries)
    }
    
    fn collect_tree_entries(
        repo: &Repository,
        tree: &git2::Tree,
        base_path: &str,
        current_depth: i32,
        max_depth: i32,
        entries: &mut Vec<TreeEntry>,
    ) -> Result<()> {
        if max_depth >= 0 && current_depth > max_depth {
            return Ok(());
        }
        
        for entry in tree.iter() {
            let name = entry.name().unwrap_or("").to_string();
            let entry_path = if base_path.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", base_path, name)
            };
            
            let entry_type = match entry.kind() {
                Some(ObjectType::Blob) => "blob",
                Some(ObjectType::Tree) => "tree",
                Some(ObjectType::Commit) => "commit",
                _ => "unknown",
            };
            
            entries.push(TreeEntry {
                name: name.clone(),
                path: entry_path.clone(),
                entry_type: entry_type.to_string(),
                id: entry.id().to_string(),
                mode: entry.filemode() as i32,
                size: 0,
                submodule_url: None,
            });
            
            // Recurse into subdirectories
            if entry.kind() == Some(ObjectType::Tree) {
                if let Ok(subtree) = repo.find_tree(entry.id()) {
                    Self::collect_tree_entries(
                        repo,
                        &subtree,
                        &entry_path,
                        current_depth + 1,
                        max_depth,
                        entries,
                    )?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Find files matching a pattern
    pub fn find_files(
        repo: &Repository,
        revision: &str,
        pattern: &str,
        limit: usize,
    ) -> Result<Vec<String>> {
        let entries = Self::get_tree_recursive(repo, revision, "", -1)?;
        
        let glob_pattern = glob::Pattern::new(pattern)
            .map_err(|e| GitLayerError::InvalidArgument(format!("Invalid pattern: {}", e)))?;
        
        let mut paths = Vec::new();
        
        for entry in entries {
            if entry.entry_type != "blob" {
                continue;
            }
            
            if glob_pattern.matches(&entry.path) {
                paths.push(entry.path);
                if paths.len() >= limit {
                    break;
                }
            }
        }
        
        Ok(paths)
    }
    
    /// Get tree entries with last commit info
    pub fn get_tree_with_commits(
        repo: &Repository,
        revision: &str,
        path: &str,
    ) -> Result<Vec<TreeEntryWithCommit>> {
        let entries = match Self::get_tree(repo, revision, path, true)? {
            Some(e) => e,
            None => return Ok(Vec::new()),
        };
        
        let mut result = Vec::new();
        
        for entry in entries {
            // Find last commit that touched this path
            let (commits, _) = CommitOps::list_commits(
                repo,
                revision,
                Some(&entry.path),
                1,
                0,
                true,
                None,
                None,
            )?;
            
            result.push(TreeEntryWithCommit {
                entry,
                last_commit: commits.into_iter().next(),
            });
        }
        
        Ok(result)
    }
    
    /// Get directory size
    pub fn get_tree_size(
        repo: &Repository,
        revision: &str,
        path: &str,
    ) -> Result<(i64, i32, i32)> {
        let entries = Self::get_tree_recursive(repo, revision, path, -1)?;
        
        let mut total_size = 0i64;
        let mut file_count = 0i32;
        let mut dir_count = 0i32;
        
        for entry in entries {
            match entry.entry_type.as_str() {
                "blob" => {
                    file_count += 1;
                    if let Ok(blob) = repo.find_blob(git2::Oid::from_str(&entry.id).unwrap()) {
                        total_size += blob.size() as i64;
                    }
                }
                "tree" => {
                    dir_count += 1;
                }
                _ => {}
            }
        }
        
        Ok((total_size, file_count, dir_count))
    }
}
