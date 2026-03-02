//! Reference operations (branches, tags, refs)

use git2::{BranchType, ObjectType, Repository, Signature as GitSignature};
use tracing::debug;

use crate::error::{GitLayerError, Result};

pub struct RefOps;

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub commit_id: String,
    pub is_head: bool,
}

#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub target_id: String,
    pub message: Option<String>,
    pub tagger_name: Option<String>,
    pub tagger_email: Option<String>,
    pub tagger_time: Option<i64>,
    pub is_annotated: bool,
}

#[derive(Debug, Clone)]
pub struct RefInfo {
    pub name: String,
    pub target: String,
    pub symbolic_target: Option<String>,
    pub is_symbolic: bool,
}

impl RefOps {
    /// List all branches
    pub fn list_branches(
        repo: &Repository,
        pattern: Option<&str>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<BranchInfo>> {
        let mut branches = Vec::new();
        let head = repo.head().ok();
        let head_branch = head.as_ref()
            .and_then(|h| h.shorthand())
            .map(|s| s.to_string());
        
        for branch_result in repo.branches(Some(BranchType::Local))? {
            let (branch, _) = branch_result?;
            let name = branch.name()?.unwrap_or("").to_string();
            
            // Apply pattern filter
            if let Some(p) = pattern {
                if !name.contains(p) {
                    continue;
                }
            }
            
            let commit_id = branch.get()
                .peel_to_commit()
                .map(|c| c.id().to_string())
                .unwrap_or_default();
            
            let is_head = head_branch.as_ref().map(|h| h == &name).unwrap_or(false);
            
            branches.push(BranchInfo {
                name,
                commit_id,
                is_head,
            });
        }
        
        // Sort by name
        branches.sort_by(|a, b| a.name.cmp(&b.name));
        
        // Apply pagination
        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(branches.len());
        
        Ok(branches.into_iter().skip(offset).take(limit).collect())
    }
    
    /// Create a new branch
    pub fn create_branch(
        repo: &Repository,
        name: &str,
        start_point: &str,
    ) -> Result<BranchInfo> {
        debug!("Creating branch {} from {}", name, start_point);
        
        // Resolve start point to commit
        let commit = Self::resolve_to_commit(repo, start_point)?;
        
        // Create branch
        let branch = repo.branch(name, &commit, false)?;
        let commit_id = commit.id().to_string();
        
        Ok(BranchInfo {
            name: name.to_string(),
            commit_id,
            is_head: false,
        })
    }
    
    /// Delete a branch
    pub fn delete_branch(repo: &Repository, name: &str, force: bool) -> Result<()> {
        debug!("Deleting branch {} (force={})", name, force);
        
        let mut branch = repo.find_branch(name, BranchType::Local)
            .map_err(|_| GitLayerError::RefNotFound(format!("Branch not found: {}", name)))?;
        
        if !force {
            // Check if branch is HEAD
            if let Ok(head) = repo.head() {
                if head.shorthand() == Some(name) {
                    return Err(GitLayerError::InvalidArgument(
                        "Cannot delete the currently checked out branch".to_string()
                    ));
                }
            }
        }
        
        branch.delete()?;
        Ok(())
    }
    
    /// List all tags
    pub fn list_tags(
        repo: &Repository,
        pattern: Option<&str>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<TagInfo>> {
        let mut tags = Vec::new();
        
        repo.tag_foreach(|oid, name_bytes| {
            let name = String::from_utf8_lossy(name_bytes);
            let name = name.strip_prefix("refs/tags/").unwrap_or(&name).to_string();
            
            // Apply pattern filter
            if let Some(p) = pattern {
                if !name.contains(p) {
                    return true;
                }
            }
            
            // Try to get tag object (annotated tag)
            let tag_info = if let Ok(tag) = repo.find_tag(oid) {
                TagInfo {
                    name: name.clone(),
                    target_id: tag.target_id().to_string(),
                    message: tag.message().map(|m| m.to_string()),
                    tagger_name: tag.tagger().map(|t| t.name().unwrap_or("").to_string()),
                    tagger_email: tag.tagger().map(|t| t.email().unwrap_or("").to_string()),
                    tagger_time: tag.tagger().map(|t| t.when().seconds()),
                    is_annotated: true,
                }
            } else {
                // Lightweight tag
                TagInfo {
                    name: name.clone(),
                    target_id: oid.to_string(),
                    message: None,
                    tagger_name: None,
                    tagger_email: None,
                    tagger_time: None,
                    is_annotated: false,
                }
            };
            
            tags.push(tag_info);
            true
        })?;
        
        // Sort by name
        tags.sort_by(|a, b| a.name.cmp(&b.name));
        
        // Apply pagination
        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(tags.len());
        
        Ok(tags.into_iter().skip(offset).take(limit).collect())
    }
    
    /// Create a tag
    pub fn create_tag(
        repo: &Repository,
        name: &str,
        target: &str,
        message: Option<&str>,
        tagger_name: Option<&str>,
        tagger_email: Option<&str>,
    ) -> Result<TagInfo> {
        debug!("Creating tag {} on {}", name, target);
        
        let commit = Self::resolve_to_commit(repo, target)?;
        let target_id = commit.id().to_string();
        
        if let (Some(msg), Some(tagger_name), Some(tagger_email)) = (message, tagger_name, tagger_email) {
            // Create annotated tag
            let tagger = GitSignature::now(tagger_name, tagger_email)?;
            repo.tag(name, commit.as_object(), &tagger, msg, false)?;
            
            Ok(TagInfo {
                name: name.to_string(),
                target_id,
                message: Some(msg.to_string()),
                tagger_name: Some(tagger_name.to_string()),
                tagger_email: Some(tagger_email.to_string()),
                tagger_time: Some(tagger.when().seconds()),
                is_annotated: true,
            })
        } else {
            // Create lightweight tag
            repo.tag_lightweight(name, commit.as_object(), false)?;
            
            Ok(TagInfo {
                name: name.to_string(),
                target_id,
                message: None,
                tagger_name: None,
                tagger_email: None,
                tagger_time: None,
                is_annotated: false,
            })
        }
    }
    
    /// Delete a tag
    pub fn delete_tag(repo: &Repository, name: &str) -> Result<()> {
        debug!("Deleting tag {}", name);
        
        let refname = format!("refs/tags/{}", name);
        let mut reference = repo.find_reference(&refname)
            .map_err(|_| GitLayerError::RefNotFound(format!("Tag not found: {}", name)))?;
        
        reference.delete()?;
        Ok(())
    }
    
    /// Find a specific reference
    pub fn find_ref(repo: &Repository, ref_name: &str) -> Result<Option<RefInfo>> {
        let refname = if ref_name.starts_with("refs/") {
            ref_name.to_string()
        } else if ref_name == "HEAD" {
            "HEAD".to_string()
        } else {
            // Try branches first, then tags
            let branch_ref = format!("refs/heads/{}", ref_name);
            if repo.find_reference(&branch_ref).is_ok() {
                branch_ref
            } else {
                format!("refs/tags/{}", ref_name)
            }
        };
        
        match repo.find_reference(&refname) {
            Ok(reference) => {
                let is_symbolic = reference.kind() == Some(git2::ReferenceType::Symbolic);
                let symbolic_target = if is_symbolic {
                    reference.symbolic_target().map(|s| s.to_string())
                } else {
                    None
                };
                
                let target = reference.resolve()
                    .ok()
                    .and_then(|r| r.target())
                    .map(|oid| oid.to_string())
                    .unwrap_or_default();
                
                Ok(Some(RefInfo {
                    name: refname,
                    target,
                    symbolic_target,
                    is_symbolic,
                }))
            }
            Err(_) => Ok(None),
        }
    }
    
    /// Update a reference
    pub fn update_ref(
        repo: &Repository,
        ref_name: &str,
        old_value: Option<&str>,
        new_value: &str,
    ) -> Result<()> {
        debug!("Updating ref {} to {}", ref_name, new_value);
        
        let new_oid = git2::Oid::from_str(new_value)
            .map_err(|_| GitLayerError::InvalidRevision(new_value.to_string()))?;
        
        if let Some(old) = old_value {
            let old_oid = git2::Oid::from_str(old)
                .map_err(|_| GitLayerError::InvalidRevision(old.to_string()))?;
            
            // Atomic compare-and-swap
            let reference = repo.find_reference(ref_name);
            if let Ok(r) = reference {
                if let Some(current_oid) = r.target() {
                    if current_oid != old_oid {
                        return Err(GitLayerError::InvalidArgument(
                            "Reference has been modified".to_string()
                        ));
                    }
                }
            }
        }
        
        repo.reference(ref_name, new_oid, true, "gitlayer ref update")?;
        Ok(())
    }
    
    /// List all references matching a pattern
    pub fn list_refs(repo: &Repository, pattern: Option<&str>) -> Result<Vec<RefInfo>> {
        let mut refs = Vec::new();
        
        let references = if let Some(p) = pattern {
            repo.references_glob(p)?
        } else {
            repo.references()?
        };
        
        for reference_result in references {
            let reference = reference_result?;
            let name = reference.name().unwrap_or("").to_string();
            
            let is_symbolic = reference.kind() == Some(git2::ReferenceType::Symbolic);
            let symbolic_target = if is_symbolic {
                reference.symbolic_target().map(|s| s.to_string())
            } else {
                None
            };
            
            let target = reference.resolve()
                .ok()
                .and_then(|r| r.target())
                .map(|oid| oid.to_string())
                .unwrap_or_default();
            
            refs.push(RefInfo {
                name,
                target,
                symbolic_target,
                is_symbolic,
            });
        }
        
        Ok(refs)
    }
    
    /// Helper: Resolve a revision to a commit
    fn resolve_to_commit<'a>(repo: &'a Repository, revision: &str) -> Result<git2::Commit<'a>> {
        let obj = repo.revparse_single(revision)
            .map_err(|_| GitLayerError::InvalidRevision(revision.to_string()))?;
        
        obj.peel_to_commit()
            .map_err(|_| GitLayerError::InvalidRevision(format!("{} is not a commit", revision)))
    }
}
