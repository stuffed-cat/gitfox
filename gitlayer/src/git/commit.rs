//! Commit operations

use git2::{Repository, Oid, Revwalk, Sort};
use tracing::debug;

use crate::error::{GitLayerError, Result};

#[derive(Debug, Clone)]
pub struct SignatureInfo {
    pub name: String,
    pub email: String,
    pub timestamp: i64,
    pub timezone: String,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub id: String,
    pub tree_id: String,
    pub parent_ids: Vec<String>,
    pub author: SignatureInfo,
    pub committer: SignatureInfo,
    pub message: String,
    pub short_message: String,
}

pub struct CommitOps;

impl CommitOps {
    /// Get a single commit by revision
    pub fn get_commit(repo: &Repository, revision: &str) -> Result<Option<CommitInfo>> {
        let obj = match repo.revparse_single(revision) {
            Ok(obj) => obj,
            Err(_) => return Ok(None),
        };
        
        let commit = match obj.peel_to_commit() {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        
        Ok(Some(Self::commit_to_info(&commit)))
    }
    
    /// List commits with various filters
    pub fn list_commits(
        repo: &Repository,
        revision: &str,
        path: Option<&str>,
        limit: usize,
        offset: usize,
        include_merges: bool,
        after: Option<i64>,
        before: Option<i64>,
    ) -> Result<(Vec<CommitInfo>, bool)> {
        let obj = repo.revparse_single(revision)
            .map_err(|_| GitLayerError::InvalidRevision(revision.to_string()))?;
        
        let commit = obj.peel_to_commit()
            .map_err(|_| GitLayerError::InvalidRevision(format!("{} is not a commit", revision)))?;
        
        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(Sort::TIME)?;
        revwalk.push(commit.id())?;
        
        let mut commits = Vec::new();
        let mut skipped = 0;
        let mut collected = 0;
        let mut has_more = false;
        
        for oid_result in revwalk {
            let oid = oid_result?;
            let c = repo.find_commit(oid)?;
            
            // Filter merges
            if !include_merges && c.parent_count() > 1 {
                continue;
            }
            
            // Filter by time
            let commit_time = c.time().seconds();
            if let Some(a) = after {
                if commit_time < a {
                    continue;
                }
            }
            if let Some(b) = before {
                if commit_time > b {
                    continue;
                }
            }
            
            // Filter by path
            if let Some(p) = path {
                if !Self::commit_touches_path(repo, &c, p)? {
                    continue;
                }
            }
            
            // Apply offset
            if skipped < offset {
                skipped += 1;
                continue;
            }
            
            // Check limit
            if collected >= limit {
                has_more = true;
                break;
            }
            
            commits.push(Self::commit_to_info(&c));
            collected += 1;
        }
        
        Ok((commits, has_more))
    }
    
    /// Count commits
    pub fn count_commits(
        repo: &Repository,
        revision: &str,
        path: Option<&str>,
    ) -> Result<i64> {
        let obj = repo.revparse_single(revision)
            .map_err(|_| GitLayerError::InvalidRevision(revision.to_string()))?;
        
        let commit = obj.peel_to_commit()
            .map_err(|_| GitLayerError::InvalidRevision(format!("{} is not a commit", revision)))?;
        
        let mut revwalk = repo.revwalk()?;
        revwalk.push(commit.id())?;
        
        let mut count = 0i64;
        
        for oid_result in revwalk {
            let oid = oid_result?;
            
            if let Some(p) = path {
                let c = repo.find_commit(oid)?;
                if !Self::commit_touches_path(repo, &c, p)? {
                    continue;
                }
            }
            
            count += 1;
        }
        
        Ok(count)
    }
    
    /// Check if one commit is an ancestor of another
    pub fn is_ancestor(repo: &Repository, ancestor: &str, descendant: &str) -> Result<bool> {
        let ancestor_obj = repo.revparse_single(ancestor)
            .map_err(|_| GitLayerError::InvalidRevision(ancestor.to_string()))?;
        let descendant_obj = repo.revparse_single(descendant)
            .map_err(|_| GitLayerError::InvalidRevision(descendant.to_string()))?;
        
        let ancestor_oid = ancestor_obj.peel_to_commit()?.id();
        let descendant_oid = descendant_obj.peel_to_commit()?.id();
        
        Ok(repo.graph_descendant_of(descendant_oid, ancestor_oid)?)
    }
    
    /// Find merge base of commits
    pub fn merge_base(repo: &Repository, revisions: &[&str]) -> Result<Option<String>> {
        if revisions.len() < 2 {
            return Err(GitLayerError::InvalidArgument(
                "At least 2 revisions required for merge base".to_string()
            ));
        }
        
        let mut oids: Vec<Oid> = Vec::new();
        for rev in revisions {
            let obj = repo.revparse_single(rev)
                .map_err(|_| GitLayerError::InvalidRevision(rev.to_string()))?;
            oids.push(obj.peel_to_commit()?.id());
        }
        
        match repo.merge_base(oids[0], oids[1]) {
            Ok(oid) => Ok(Some(oid.to_string())),
            Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    
    /// Get commits between two revisions
    pub fn commits_between(
        repo: &Repository,
        from: &str,
        to: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>> {
        let from_obj = repo.revparse_single(from)
            .map_err(|_| GitLayerError::InvalidRevision(from.to_string()))?;
        let to_obj = repo.revparse_single(to)
            .map_err(|_| GitLayerError::InvalidRevision(to.to_string()))?;
        
        let from_oid = from_obj.peel_to_commit()?.id();
        let to_oid = to_obj.peel_to_commit()?.id();
        
        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(Sort::TIME)?;
        revwalk.push(to_oid)?;
        revwalk.hide(from_oid)?;
        
        let mut commits = Vec::new();
        
        for oid_result in revwalk {
            if commits.len() >= limit {
                break;
            }
            
            let oid = oid_result?;
            let c = repo.find_commit(oid)?;
            commits.push(Self::commit_to_info(&c));
        }
        
        Ok(commits)
    }
    
    /// Check if a commit touches a specific path
    fn commit_touches_path(repo: &Repository, commit: &git2::Commit, path: &str) -> Result<bool> {
        let tree = commit.tree()?;
        
        // Check if path exists in this commit
        let exists_in_commit = tree.get_path(std::path::Path::new(path)).is_ok();
        
        if commit.parent_count() == 0 {
            return Ok(exists_in_commit);
        }
        
        // Compare with parent
        let parent = commit.parent(0)?;
        let parent_tree = parent.tree()?;
        let exists_in_parent = parent_tree.get_path(std::path::Path::new(path)).is_ok();
        
        if exists_in_commit != exists_in_parent {
            return Ok(true);
        }
        
        if !exists_in_commit {
            return Ok(false);
        }
        
        // Compare blob IDs
        let commit_entry = tree.get_path(std::path::Path::new(path))?;
        let parent_entry = parent_tree.get_path(std::path::Path::new(path))?;
        
        Ok(commit_entry.id() != parent_entry.id())
    }
    
    /// Convert git2 commit to CommitInfo
    fn commit_to_info(commit: &git2::Commit) -> CommitInfo {
        let message = commit.message().unwrap_or("").to_string();
        let short_message = commit.summary().unwrap_or("").to_string();
        
        CommitInfo {
            id: commit.id().to_string(),
            tree_id: commit.tree_id().to_string(),
            parent_ids: commit.parent_ids().map(|id| id.to_string()).collect(),
            author: SignatureInfo {
                name: commit.author().name().unwrap_or("").to_string(),
                email: commit.author().email().unwrap_or("").to_string(),
                timestamp: commit.author().when().seconds(),
                timezone: format!("{:+05}", commit.author().when().offset_minutes() / 60 * 100
                    + commit.author().when().offset_minutes() % 60),
            },
            committer: SignatureInfo {
                name: commit.committer().name().unwrap_or("").to_string(),
                email: commit.committer().email().unwrap_or("").to_string(),
                timestamp: commit.committer().when().seconds(),
                timezone: format!("{:+05}", commit.committer().when().offset_minutes() / 60 * 100
                    + commit.committer().when().offset_minutes() % 60),
            },
            message,
            short_message,
        }
    }
}
