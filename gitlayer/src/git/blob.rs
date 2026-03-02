//! Blob (file content) operations

use std::path::Path;

use git2::{Repository, ObjectType};
use tracing::debug;

use crate::error::{GitLayerError, Result};

#[derive(Debug, Clone)]
pub struct BlobInfo {
    pub id: String,
    pub size: i64,
    pub data: Vec<u8>,
    pub is_binary: bool,
}

#[derive(Debug, Clone)]
pub struct BlameLine {
    pub line_number: i32,
    pub commit_id: String,
    pub author_name: String,
    pub author_email: String,
    pub author_date: i64,
    pub content: String,
    pub original_path: String,
    pub original_line: i32,
}

pub struct BlobOps;

impl BlobOps {
    /// Get a blob by ID
    pub fn get_blob(repo: &Repository, blob_id: &str) -> Result<Option<BlobInfo>> {
        let oid = git2::Oid::from_str(blob_id)
            .map_err(|_| GitLayerError::InvalidArgument(format!("Invalid blob ID: {}", blob_id)))?;
        
        let blob = match repo.find_blob(oid) {
            Ok(b) => b,
            Err(_) => return Ok(None),
        };
        
        let data = blob.content().to_vec();
        let is_binary = blob.is_binary();
        
        Ok(Some(BlobInfo {
            id: blob_id.to_string(),
            size: data.len() as i64,
            data,
            is_binary,
        }))
    }
    
    /// Get blob size without loading content
    pub fn get_blob_size(repo: &Repository, blob_id: &str) -> Result<Option<i64>> {
        let oid = git2::Oid::from_str(blob_id)
            .map_err(|_| GitLayerError::InvalidArgument(format!("Invalid blob ID: {}", blob_id)))?;
        
        let blob = match repo.find_blob(oid) {
            Ok(b) => b,
            Err(_) => return Ok(None),
        };
        
        Ok(Some(blob.size() as i64))
    }
    
    /// Get file content at a specific revision
    pub fn get_file_content(
        repo: &Repository,
        revision: &str,
        path: &str,
    ) -> Result<Option<BlobInfo>> {
        let obj = repo.revparse_single(revision)
            .map_err(|_| GitLayerError::InvalidRevision(revision.to_string()))?;
        
        let commit = obj.peel_to_commit()
            .map_err(|_| GitLayerError::InvalidRevision(format!("{} is not a commit", revision)))?;
        
        let tree = commit.tree()?;
        
        let entry = match tree.get_path(Path::new(path)) {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };
        
        if entry.kind() != Some(ObjectType::Blob) {
            return Ok(None);
        }
        
        let blob = repo.find_blob(entry.id())?;
        let data = blob.content().to_vec();
        let is_binary = blob.is_binary();
        
        Ok(Some(BlobInfo {
            id: entry.id().to_string(),
            size: data.len() as i64,
            data,
            is_binary,
        }))
    }
    
    /// Check if a path exists at a revision
    pub fn path_exists(
        repo: &Repository,
        revision: &str,
        path: &str,
    ) -> Result<Option<String>> {
        let obj = repo.revparse_single(revision)
            .map_err(|_| GitLayerError::InvalidRevision(revision.to_string()))?;
        
        let commit = obj.peel_to_commit()
            .map_err(|_| GitLayerError::InvalidRevision(format!("{} is not a commit", revision)))?;
        
        let tree = commit.tree()?;
        
        match tree.get_path(Path::new(path)) {
            Ok(entry) => {
                let object_type = match entry.kind() {
                    Some(ObjectType::Blob) => "blob",
                    Some(ObjectType::Tree) => "tree",
                    Some(ObjectType::Commit) => "commit", // submodule
                    _ => "unknown",
                };
                Ok(Some(object_type.to_string()))
            }
            Err(_) => Ok(None),
        }
    }
    
    /// Get file blame (line-by-line authorship)
    pub fn blame(
        repo: &Repository,
        revision: &str,
        path: &str,
        start_line: Option<i32>,
        end_line: Option<i32>,
    ) -> Result<Vec<BlameLine>> {
        let obj = repo.revparse_single(revision)
            .map_err(|_| GitLayerError::InvalidRevision(revision.to_string()))?;
        
        let commit = obj.peel_to_commit()
            .map_err(|_| GitLayerError::InvalidRevision(format!("{} is not a commit", revision)))?;
        
        // Get the file content to extract lines
        let tree = commit.tree()?;
        let entry = tree.get_path(Path::new(path))
            .map_err(|_| GitLayerError::PathNotFound(path.to_string()))?;
        
        let blob = repo.find_blob(entry.id())?;
        let content = String::from_utf8_lossy(blob.content());
        let lines: Vec<&str> = content.lines().collect();
        
        // Create blame
        let mut opts = git2::BlameOptions::new();
        opts.newest_commit(commit.id());
        
        if let (Some(start), Some(end)) = (start_line, end_line) {
            opts.min_line(start as usize);
            opts.max_line(end as usize);
        }
        
        let blame = repo.blame_file(Path::new(path), Some(&mut opts))?;
        
        let mut result = Vec::new();
        
        for hunk in blame.iter() {
            let line_start = hunk.final_start_line();
            let line_count = hunk.lines_in_hunk();
            let sig = hunk.final_signature();
            let commit_id = hunk.final_commit_id().to_string();
            // git2 0.18 doesn't expose orig_path, use current path as fallback
            let orig_path = path.to_string();
            let orig_start = hunk.orig_start_line();
            
            for i in 0..line_count {
                let line_num = (line_start + i) as i32;
                let line_idx = (line_start + i - 1) as usize;
                let line_content = lines.get(line_idx).unwrap_or(&"").to_string();
                
                result.push(BlameLine {
                    line_number: line_num,
                    commit_id: commit_id.clone(),
                    author_name: sig.name().unwrap_or("").to_string(),
                    author_email: sig.email().unwrap_or("").to_string(),
                    author_date: sig.when().seconds(),
                    content: line_content,
                    original_path: orig_path.clone(),
                    original_line: (orig_start + i) as i32,
                });
            }
        }
        
        Ok(result)
    }
    
    /// Check if content is binary
    pub fn is_binary(data: &[u8]) -> bool {
        // Check for null bytes in the first 8000 bytes (like Git does)
        let check_len = std::cmp::min(8000, data.len());
        data[..check_len].contains(&0)
    }
}
