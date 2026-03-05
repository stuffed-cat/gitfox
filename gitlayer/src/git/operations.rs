//! Write operations (create commit, merge, etc.)

use std::path::Path;

use git2::{Repository, Signature as GitSignature, ObjectType, IndexAddOption, MergeOptions, build::CheckoutBuilder, Oid};
use tracing::{debug, info};

use crate::error::{GitLayerError, Result};

#[derive(Debug, Clone)]
pub struct SignatureInfo {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct FileAction {
    pub action: String, // "create", "update", "delete", "move"
    pub path: String,
    pub content: Option<Vec<u8>>,
    pub previous_path: Option<String>,
    pub mode: Option<i32>,
}

pub struct OperationOps;

impl OperationOps {
    /// Create a commit with file actions
    pub fn create_commit(
        repo: &Repository,
        branch: &str,
        author: &SignatureInfo,
        committer: &SignatureInfo,
        message: &str,
        actions: &[FileAction],
        create_branch: bool,
    ) -> Result<String> {
        info!("Creating commit on branch {} with {} actions", branch, actions.len());
        
        let author_sig = GitSignature::now(&author.name, &author.email)?;
        let committer_sig = GitSignature::now(&committer.name, &committer.email)?;
        
        // Get current HEAD of branch
        let branch_ref = format!("refs/heads/{}", branch);
        let parent = match repo.find_reference(&branch_ref) {
            Ok(r) => {
                let commit = r.peel_to_commit()?;
                Some(commit)
            }
            Err(_) => {
                if create_branch {
                    None
                } else {
                    return Err(GitLayerError::RefNotFound(format!("Branch not found: {}", branch)));
                }
            }
        };
        
        // Get tree from parent or create empty tree
        let mut index = repo.index()?;
        if let Some(ref p) = parent {
            let tree = p.tree()?;
            index.read_tree(&tree)?;
        }
        
        // Apply actions
        for action in actions {
            match action.action.as_str() {
                "create" | "update" => {
                    if let Some(content) = &action.content {
                        // Write blob
                        let oid = repo.blob(content)?;
                        let mode = action.mode.unwrap_or(0o100644) as u32;
                        index.add_frombuffer(
                            &git2::IndexEntry {
                                ctime: git2::IndexTime::new(0, 0),
                                mtime: git2::IndexTime::new(0, 0),
                                dev: 0,
                                ino: 0,
                                mode,
                                uid: 0,
                                gid: 0,
                                file_size: content.len() as u32,
                                id: oid,
                                flags: 0,
                                flags_extended: 0,
                                path: action.path.as_bytes().to_vec(),
                            },
                            content,
                        )?;
                    }
                }
                "delete" => {
                    index.remove_path(Path::new(&action.path))?;
                }
                "move" => {
                    if let Some(prev_path) = &action.previous_path {
                        // Get content from old path
                        if let Some(entry) = parent.as_ref()
                            .and_then(|p| p.tree().ok())
                            .and_then(|t| t.get_path(Path::new(prev_path)).ok())
                        {
                            let blob = repo.find_blob(entry.id())?;
                            let oid = repo.blob(blob.content())?;
                            let mode = action.mode.unwrap_or(entry.filemode()) as u32;
                            
                            index.remove_path(Path::new(prev_path))?;
                            index.add_frombuffer(
                                &git2::IndexEntry {
                                    ctime: git2::IndexTime::new(0, 0),
                                    mtime: git2::IndexTime::new(0, 0),
                                    dev: 0,
                                    ino: 0,
                                    mode,
                                    uid: 0,
                                    gid: 0,
                                    file_size: blob.size() as u32,
                                    id: oid,
                                    flags: 0,
                                    flags_extended: 0,
                                    path: action.path.as_bytes().to_vec(),
                                },
                                blob.content(),
                            )?;
                        }
                    }
                }
                _ => {
                    return Err(GitLayerError::InvalidArgument(
                        format!("Unknown action: {}", action.action)
                    ));
                }
            }
        }
        
        // Write tree
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        
        // Create commit
        let parents: Vec<&git2::Commit> = parent.as_ref().map(|p| vec![p]).unwrap_or_default();
        let commit_id = repo.commit(
            Some(&branch_ref),
            &author_sig,
            &committer_sig,
            message,
            &tree,
            &parents,
        )?;
        
        Ok(commit_id.to_string())
    }
    
    /// Merge branches
    pub fn merge(
        repo: &Repository,
        source_branch: &str,
        target_branch: &str,
        author: &SignatureInfo,
        message: &str,
        strategy: &str,
    ) -> Result<(String, bool, Vec<String>)> {
        info!("Merging {} into {} (strategy: {})", source_branch, target_branch, strategy);
        
        // Find source commit
        let source_obj = repo.revparse_single(source_branch)
            .map_err(|_| GitLayerError::RefNotFound(source_branch.to_string()))?;
        let source_commit = source_obj.peel_to_commit()?;
        
        // Find target commit
        let target_ref = format!("refs/heads/{}", target_branch);
        let target_obj = repo.revparse_single(target_branch)
            .map_err(|_| GitLayerError::RefNotFound(target_branch.to_string()))?;
        let target_commit = target_obj.peel_to_commit()?;
        
        let sig = GitSignature::now(&author.name, &author.email)?;
        
        // Check if fast-forward is possible
        let annotated_commit = repo.find_annotated_commit(source_commit.id())?;
        let analysis = repo.merge_analysis(&[&annotated_commit])?;
        
        if strategy == "fast_forward" || (analysis.0.is_fast_forward() && strategy != "merge") {
            // Fast-forward
            let mut target_ref_obj = repo.find_reference(&target_ref)?;
            target_ref_obj.set_target(source_commit.id(), "fast-forward merge")?;
            return Ok((source_commit.id().to_string(), false, Vec::new()));
        }
        
        // Perform actual merge
        let mut merge_opts = MergeOptions::new();
        let ancestor_commit = repo.merge_base(target_commit.id(), source_commit.id())?;
        let ancestor = repo.find_commit(ancestor_commit)?;
        
        let mut index = repo.merge_trees(
            &ancestor.tree()?,
            &target_commit.tree()?,
            &source_commit.tree()?,
            Some(&mut merge_opts),
        )?;
        
        // Check for conflicts
        if index.has_conflicts() {
            let mut conflict_files = Vec::new();
            for conflict in index.conflicts()? {
                if let Ok(c) = conflict {
                    if let Some(our) = c.our {
                        let path = String::from_utf8_lossy(&our.path).to_string();
                        conflict_files.push(path);
                    }
                }
            }
            return Ok((String::new(), true, conflict_files));
        }
        
        // Write tree
        let tree_id = index.write_tree_to(repo)?;
        let tree = repo.find_tree(tree_id)?;
        
        // Create merge commit
        let commit_id = repo.commit(
            Some(&target_ref),
            &sig,
            &sig,
            message,
            &tree,
            &[&target_commit, &source_commit],
        )?;
        
        Ok((commit_id.to_string(), false, Vec::new()))
    }
    
    /// Revert a commit
    pub fn revert(
        repo: &Repository,
        branch: &str,
        commit_id: &str,
        author: &SignatureInfo,
        message: &str,
    ) -> Result<String> {
        info!("Reverting commit {} on branch {}", commit_id, branch);
        
        let commit_oid = Oid::from_str(commit_id)
            .map_err(|_| GitLayerError::InvalidRevision(commit_id.to_string()))?;
        let commit = repo.find_commit(commit_oid)?;
        
        let sig = GitSignature::now(&author.name, &author.email)?;
        
        // Get current HEAD of branch
        let branch_ref = format!("refs/heads/{}", branch);
        let head_ref = repo.find_reference(&branch_ref)?;
        let head_commit = head_ref.peel_to_commit()?;
        
        // Revert the commit
        let mut revert_index = repo.revert_commit(&commit, &head_commit, 0, None)?;
        
        if revert_index.has_conflicts() {
            return Err(GitLayerError::MergeConflict(
                "Revert would cause conflicts".to_string()
            ));
        }
        
        let tree_id = revert_index.write_tree_to(repo)?;
        let tree = repo.find_tree(tree_id)?;
        
        let new_commit = repo.commit(
            Some(&branch_ref),
            &sig,
            &sig,
            message,
            &tree,
            &[&head_commit],
        )?;
        
        Ok(new_commit.to_string())
    }
    
    /// Cherry-pick commits
    pub fn cherry_pick(
        repo: &Repository,
        branch: &str,
        commit_ids: &[String],
        author: &SignatureInfo,
    ) -> Result<Vec<String>> {
        info!("Cherry-picking {} commits onto {}", commit_ids.len(), branch);
        
        let sig = GitSignature::now(&author.name, &author.email)?;
        let branch_ref = format!("refs/heads/{}", branch);
        
        let mut new_commits = Vec::new();
        let mut current_head = repo.find_reference(&branch_ref)?.peel_to_commit()?;
        
        for commit_id in commit_ids {
            let commit_oid = Oid::from_str(commit_id)
                .map_err(|_| GitLayerError::InvalidRevision(commit_id.to_string()))?;
            let commit = repo.find_commit(commit_oid)?;
            
            // Cherry-pick
            let mut index = repo.cherrypick_commit(&commit, &current_head, 0, None)?;
            
            if index.has_conflicts() {
                return Err(GitLayerError::MergeConflict(
                    format!("Cherry-pick of {} would cause conflicts", commit_id)
                ));
            }
            
            let tree_id = index.write_tree_to(repo)?;
            let tree = repo.find_tree(tree_id)?;
            
            let message = commit.message().unwrap_or("cherry-pick");
            let new_commit = repo.commit(
                Some(&branch_ref),
                &sig,
                &sig,
                message,
                &tree,
                &[&current_head],
            )?;
            
            new_commits.push(new_commit.to_string());
            current_head = repo.find_commit(new_commit)?;
        }
        
        Ok(new_commits)
    }
    
    /// Write a single file
    pub fn write_file(
        repo: &Repository,
        branch: &str,
        path: &str,
        content: &[u8],
        author: &SignatureInfo,
        committer: &SignatureInfo,
        message: &str,
        create_branch: bool,
    ) -> Result<(String, String)> {
        let actions = vec![FileAction {
            action: "update".to_string(),
            path: path.to_string(),
            content: Some(content.to_vec()),
            previous_path: None,
            mode: Some(0o100644),
        }];
        
        let commit_id = Self::create_commit(repo, branch, author, committer, message, &actions, create_branch)?;
        
        // Get blob ID
        let commit = repo.find_commit(Oid::from_str(&commit_id)?)?;
        let tree = commit.tree()?;
        let entry = tree.get_path(Path::new(path))?;
        let blob_id = entry.id().to_string();
        
        Ok((commit_id, blob_id))
    }
    
    /// Delete a file
    pub fn delete_file(
        repo: &Repository,
        branch: &str,
        path: &str,
        author: &SignatureInfo,
        committer: &SignatureInfo,
        message: &str,
    ) -> Result<String> {
        let actions = vec![FileAction {
            action: "delete".to_string(),
            path: path.to_string(),
            content: None,
            previous_path: None,
            mode: None,
        }];
        
        Self::create_commit(repo, branch, author, committer, message, &actions, false)
    }
    
    /// Move/rename a file
    pub fn move_file(
        repo: &Repository,
        branch: &str,
        old_path: &str,
        new_path: &str,
        author: &SignatureInfo,
        committer: &SignatureInfo,
        message: &str,
    ) -> Result<String> {
        let actions = vec![FileAction {
            action: "move".to_string(),
            path: new_path.to_string(),
            content: None,
            previous_path: Some(old_path.to_string()),
            mode: None,
        }];
        
        Self::create_commit(repo, branch, author, committer, message, &actions, false)
    }
    
    /// Squash multiple commits into one
    /// 
    /// Takes all commits from start_commit to end_commit (inclusive) and squashes them
    /// into a single commit. The resulting commit will have:
    /// - The tree state from end_commit
    /// - The parent of start_commit as its parent
    /// - The provided message and author
    pub fn squash(
        repo: &Repository,
        branch: &str,
        start_commit: &str,
        end_commit: &str,
        author: &SignatureInfo,
        message: &str,
    ) -> Result<String> {
        info!("Squashing commits from {} to {} on branch {}", start_commit, end_commit, branch);
        
        // 解析开始和结束提交
        let start_oid = Oid::from_str(start_commit)
            .or_else(|_| repo.revparse_single(start_commit).map(|o| o.id()))
            .map_err(|_| GitLayerError::InvalidRevision(start_commit.to_string()))?;
        let end_oid = Oid::from_str(end_commit)
            .or_else(|_| repo.revparse_single(end_commit).map(|o| o.id()))
            .map_err(|_| GitLayerError::InvalidRevision(end_commit.to_string()))?;
        
        let start_commit_obj = repo.find_commit(start_oid)?;
        let end_commit_obj = repo.find_commit(end_oid)?;
        
        // 验证 end_commit 是 start_commit 的后代
        // 通过检查 start_commit 是否是 end_commit 的祖先
        if !repo.graph_descendant_of(end_oid, start_oid)? && start_oid != end_oid {
            return Err(GitLayerError::InvalidArgument(
                format!("end_commit {} is not a descendant of start_commit {}", end_commit, start_commit)
            ));
        }
        
        // 获取 start_commit 的父提交作为新提交的父提交
        let parent = if start_commit_obj.parent_count() > 0 {
            Some(start_commit_obj.parent(0)?)
        } else {
            // start_commit 是根提交，squash 后的提交也将是根提交
            None
        };
        
        // 获取 end_commit 的 tree（最终状态）
        let tree = end_commit_obj.tree()?;
        
        // 创建签名
        let sig = GitSignature::now(&author.name, &author.email)?;
        
        // 创建 squash 后的新提交
        let branch_ref = format!("refs/heads/{}", branch);
        let parents: Vec<&git2::Commit> = parent.as_ref().map(|p| vec![p]).unwrap_or_default();
        
        let new_commit_id = repo.commit(
            Some(&branch_ref),
            &sig,
            &sig,
            message,
            &tree,
            &parents,
        )?;
        
        info!("Squash complete, new commit: {}", new_commit_id);
        Ok(new_commit_id.to_string())
    }
}
