//! Diff operations

use git2::{Repository, DiffOptions, DiffFormat, DiffFindOptions};
use tracing::debug;

use crate::error::{GitLayerError, Result};
use crate::git::commit::{CommitOps, CommitInfo};

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub prefix: String,
    pub old_line_number: i32,
    pub new_line_number: i32,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub old_start: i32,
    pub old_lines: i32,
    pub new_start: i32,
    pub new_lines: i32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct DiffFile {
    pub old_path: String,
    pub new_path: String,
    pub old_id: String,
    pub new_id: String,
    pub status: String,
    pub additions: i32,
    pub deletions: i32,
    pub is_binary: bool,
    pub hunks: Vec<DiffHunk>,
    pub similarity: i32,
}

#[derive(Debug, Clone)]
pub struct FileStats {
    pub path: String,
    pub additions: i32,
    pub deletions: i32,
}

/// 合并冲突信息
#[derive(Debug, Clone)]
pub struct ConflictFileInfo {
    pub path: String,
    pub ancestor_content: Vec<u8>,
    pub our_content: Vec<u8>,
    pub their_content: Vec<u8>,
}

pub struct DiffOps;

impl DiffOps {
    /// Get diff between two commits
    pub fn commit_diff(
        repo: &Repository,
        old_revision: Option<&str>,
        new_revision: &str,
        paths: &[String],
        context_lines: u32,
    ) -> Result<(Vec<DiffFile>, i32, i32, i32)> {
        let new_obj = repo.revparse_single(new_revision)
            .map_err(|_| GitLayerError::InvalidRevision(new_revision.to_string()))?;
        let new_commit = new_obj.peel_to_commit()?;
        let new_tree = new_commit.tree()?;
        
        let old_tree = if let Some(old_rev) = old_revision {
            let old_obj = repo.revparse_single(old_rev)
                .map_err(|_| GitLayerError::InvalidRevision(old_rev.to_string()))?;
            let old_commit = old_obj.peel_to_commit()?;
            Some(old_commit.tree()?)
        } else if new_commit.parent_count() > 0 {
            Some(new_commit.parent(0)?.tree()?)
        } else {
            None
        };
        
        let mut opts = DiffOptions::new();
        opts.context_lines(context_lines);
        
        for path in paths {
            opts.pathspec(path);
        }
        
        let mut diff = repo.diff_tree_to_tree(
            old_tree.as_ref(),
            Some(&new_tree),
            Some(&mut opts),
        )?;
        
        // Find renames/copies
        let mut find_opts = DiffFindOptions::new();
        find_opts.renames(true);
        find_opts.copies(true);
        diff.find_similar(Some(&mut find_opts))?;
        
        let stats = diff.stats()?;
        let total_additions = stats.insertions() as i32;
        let total_deletions = stats.deletions() as i32;
        let files_changed = stats.files_changed() as i32;
        
        let files = Self::parse_diff(&diff)?;
        
        Ok((files, total_additions, total_deletions, files_changed))
    }
    
    /// Get diff stats only
    pub fn diff_stats(
        repo: &Repository,
        old_revision: &str,
        new_revision: &str,
    ) -> Result<(i32, i32, i32, Vec<FileStats>)> {
        let (files, additions, deletions, files_changed) = Self::commit_diff(
            repo,
            Some(old_revision),
            new_revision,
            &[],
            0,
        )?;
        
        let file_stats: Vec<FileStats> = files.iter()
            .map(|f| FileStats {
                path: if f.new_path.is_empty() { f.old_path.clone() } else { f.new_path.clone() },
                additions: f.additions,
                deletions: f.deletions,
            })
            .collect();
        
        Ok((additions, deletions, files_changed, file_stats))
    }
    
    /// Compare two branches/refs
    pub fn compare(
        repo: &Repository,
        from: &str,
        to: &str,
        straight: bool,
        limit: usize,
    ) -> Result<(Vec<CommitInfo>, Vec<DiffFile>, String, i32, i32)> {
        let from_obj = repo.revparse_single(from)
            .map_err(|_| GitLayerError::InvalidRevision(from.to_string()))?;
        let to_obj = repo.revparse_single(to)
            .map_err(|_| GitLayerError::InvalidRevision(to.to_string()))?;
        
        let from_commit = from_obj.peel_to_commit()?;
        let to_commit = to_obj.peel_to_commit()?;
        
        // Find merge base
        let merge_base = repo.merge_base(from_commit.id(), to_commit.id())
            .map(|oid| oid.to_string())
            .unwrap_or_default();
        
        // Get diff base
        let diff_base = if straight {
            from.to_string()
        } else {
            merge_base.clone()
        };
        
        // Get commits
        let commits = if diff_base.is_empty() {
            Vec::new()
        } else {
            CommitOps::commits_between(repo, &diff_base, to, limit)?
        };
        
        // Get diff
        let (diffs, _, _, _) = if diff_base.is_empty() {
            (Vec::new(), 0, 0, 0)
        } else {
            Self::commit_diff(repo, Some(&diff_base), to, &[], 3)?
        };
        
        // Calculate ahead/behind
        let ahead = commits.len() as i32;
        let behind = if !merge_base.is_empty() && merge_base != from_commit.id().to_string() {
            CommitOps::commits_between(repo, &merge_base, from, 1000)?
                .len() as i32
        } else {
            0
        };
        
        Ok((commits, diffs, merge_base, ahead, behind))
    }
    
    /// Get raw diff patch
    pub fn raw_diff(
        repo: &Repository,
        old_revision: &str,
        new_revision: &str,
    ) -> Result<Vec<u8>> {
        let old_obj = repo.revparse_single(old_revision)
            .map_err(|_| GitLayerError::InvalidRevision(old_revision.to_string()))?;
        let new_obj = repo.revparse_single(new_revision)
            .map_err(|_| GitLayerError::InvalidRevision(new_revision.to_string()))?;
        
        let old_tree = old_obj.peel_to_commit()?.tree()?;
        let new_tree = new_obj.peel_to_commit()?.tree()?;
        
        let diff = repo.diff_tree_to_tree(
            Some(&old_tree),
            Some(&new_tree),
            None,
        )?;
        
        let mut patch = Vec::new();
        diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            patch.extend_from_slice(line.content());
            true
        })?;
        
        Ok(patch)
    }
    
    /// Parse git2 diff into our structure
    fn parse_diff(diff: &git2::Diff) -> Result<Vec<DiffFile>> {
        use std::cell::RefCell;
        
        let files = RefCell::new(Vec::<DiffFile>::new());
        
        diff.foreach(
            &mut |delta, _progress| {
                let status = match delta.status() {
                    git2::Delta::Added => "added",
                    git2::Delta::Deleted => "deleted",
                    git2::Delta::Modified => "modified",
                    git2::Delta::Renamed => "renamed",
                    git2::Delta::Copied => "copied",
                    _ => "unknown",
                };
                
                let old_path = delta.old_file().path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                let new_path = delta.new_file().path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                
                files.borrow_mut().push(DiffFile {
                    old_path,
                    new_path,
                    old_id: delta.old_file().id().to_string(),
                    new_id: delta.new_file().id().to_string(),
                    status: status.to_string(),
                    additions: 0,
                    deletions: 0,
                    is_binary: delta.old_file().is_binary() || delta.new_file().is_binary(),
                    hunks: Vec::new(),
                    // git2 0.18 doesn't expose similarity percentage
                    similarity: 0,
                });
                
                true
            },
            None,
            Some(&mut |_delta, hunk| {
                if let Some(file) = files.borrow_mut().last_mut() {
                    file.hunks.push(DiffHunk {
                        old_start: hunk.old_start() as i32,
                        old_lines: hunk.old_lines() as i32,
                        new_start: hunk.new_start() as i32,
                        new_lines: hunk.new_lines() as i32,
                        header: String::from_utf8_lossy(hunk.header()).to_string(),
                        lines: Vec::new(),
                    });
                }
                true
            }),
            Some(&mut |_delta, _hunk, line| {
                let mut files_mut = files.borrow_mut();
                if let Some(file) = files_mut.last_mut() {
                    if let Some(hunk) = file.hunks.last_mut() {
                        let prefix = match line.origin() {
                            '+' => {
                                file.additions += 1;
                                "+"
                            }
                            '-' => {
                                file.deletions += 1;
                                "-"
                            }
                            ' ' => " ",
                            _ => "",
                        };
                        
                        if !prefix.is_empty() {
                            hunk.lines.push(DiffLine {
                                prefix: prefix.to_string(),
                                old_line_number: line.old_lineno().map(|n| n as i32).unwrap_or(-1),
                                new_line_number: line.new_lineno().map(|n| n as i32).unwrap_or(-1),
                                content: String::from_utf8_lossy(line.content()).to_string(),
                            });
                        }
                    }
                }
                true
            }),
        )?;
        
        Ok(files.into_inner())
    }
    
    /// 检测两个分支之间的合并冲突
    /// 使用 git2 的 merge_commits 来模拟合并并检测冲突
    pub fn find_conflicts(
        repo: &Repository,
        our_revision: &str,
        their_revision: &str,
    ) -> Result<(Vec<ConflictFileInfo>, bool)> {
        debug!("Finding conflicts between {} and {}", our_revision, their_revision);
        
        // 解析提交
        let our_obj = repo.revparse_single(our_revision)
            .map_err(|_| GitLayerError::InvalidRevision(our_revision.to_string()))?;
        let their_obj = repo.revparse_single(their_revision)
            .map_err(|_| GitLayerError::InvalidRevision(their_revision.to_string()))?;
        
        let our_commit = our_obj.peel_to_commit()?;
        let their_commit = their_obj.peel_to_commit()?;
        
        // 找到合并基准
        let ancestor_oid = repo.merge_base(our_commit.id(), their_commit.id())
            .map_err(|e| GitLayerError::Internal(format!("Failed to find merge base: {}", e)))?;
        let ancestor_commit = repo.find_commit(ancestor_oid)?;
        
        // 获取三方的 tree
        let our_tree = our_commit.tree()?;
        let their_tree = their_commit.tree()?;
        let ancestor_tree = ancestor_commit.tree()?;
        
        // 执行三方合并
        let mut merge_opts = git2::MergeOptions::new();
        let index = repo.merge_trees(&ancestor_tree, &our_tree, &their_tree, Some(&mut merge_opts))?;
        
        let has_conflicts = index.has_conflicts();
        let mut conflict_files = Vec::new();
        
        if has_conflicts {
            // 遍历冲突
            let conflicts = index.conflicts()?;
            for conflict in conflicts {
                let conflict = conflict?;
                
                // 获取路径（从任一可用的条目）
                let path = conflict.ancestor.as_ref()
                    .or(conflict.our.as_ref())
                    .or(conflict.their.as_ref())
                    .and_then(|e| std::str::from_utf8(&e.path).ok())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                
                if path.is_empty() {
                    continue;
                }
                
                // 读取三方内容
                let ancestor_content = conflict.ancestor.as_ref()
                    .and_then(|e| repo.find_blob(e.id).ok())
                    .map(|b| b.content().to_vec())
                    .unwrap_or_default();
                
                let our_content = conflict.our.as_ref()
                    .and_then(|e| repo.find_blob(e.id).ok())
                    .map(|b| b.content().to_vec())
                    .unwrap_or_default();
                
                let their_content = conflict.their.as_ref()
                    .and_then(|e| repo.find_blob(e.id).ok())
                    .map(|b| b.content().to_vec())
                    .unwrap_or_default();
                
                conflict_files.push(ConflictFileInfo {
                    path,
                    ancestor_content,
                    our_content,
                    their_content,
                });
            }
        }
        
        debug!("Found {} conflicts, has_conflicts: {}", conflict_files.len(), has_conflicts);
        Ok((conflict_files, has_conflicts))
    }
}
