use git2::{
    BranchType, Commit, DiffOptions, ObjectType, Oid, Repository, Signature, Sort, Tree,
};
use std::path::Path;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::models::{
    BlobContent, BranchInfo, CommitDetail, CommitInfo, CommitStats, DiffInfo, DiffStatus, FileContent,
    FileEntry, FileEntryType, RepositoryInfo, TagInfo,
};

pub struct GitService;

impl GitService {
    /// Git仓库路径: {repos_path}/{owner_name}/{project_name}.git
    fn get_repo_path(config: &AppConfig, owner_name: &str, project_name: &str) -> String {
        format!("{}/{}/{}.git", config.git_repos_path, owner_name, project_name)
    }

    /// 获取仓库的默认分支（从HEAD引用读取）
    pub fn get_default_branch(repo: &Repository) -> AppResult<Option<String>> {
        // 先检查是否有任何分支
        let branches: Vec<String> = repo
            .branches(Some(BranchType::Local))?
            .filter_map(|b| b.ok())
            .filter_map(|(branch, _)| branch.name().ok().flatten().map(String::from))
            .collect();
        
        if branches.is_empty() {
            return Ok(None); // 空仓库
        }
        
        // 尝试从HEAD获取默认分支
        if let Ok(head) = repo.head() {
            if let Some(name) = head.shorthand() {
                return Ok(Some(name.to_string()));
            }
        }
        
        // 如果HEAD没设置，返回第一个分支
        Ok(branches.into_iter().next())
    }

    pub fn init_repository(config: &AppConfig, owner_name: &str, project_name: &str) -> AppResult<()> {
        let path = Self::get_repo_path(config, owner_name, project_name);
        // 确保父目录存在
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        Repository::init_bare(&path)?;
        
        // Install post-receive hook for CI/CD triggering
        Self::install_hooks(config, &path)?;
        
        Ok(())
    }

    /// Fork a repository by cloning it to a new location
    pub fn fork_repository(
        config: &AppConfig,
        source_owner: &str,
        source_name: &str,
        target_owner: &str,
        target_name: &str,
        only_default_branch: bool,
    ) -> AppResult<()> {
        let source_path = Self::get_repo_path(config, source_owner, source_name);
        let target_path = Self::get_repo_path(config, target_owner, target_name);
        
        // Ensure target directory parent exists
        if let Some(parent) = std::path::Path::new(&target_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        if only_default_branch {
            // Clone only the default branch
            let source_repo = Repository::open_bare(&source_path)?;
            let default_branch = Self::get_default_branch(&source_repo)?.unwrap_or_else(|| "main".to_string());
            
            // Create target bare repo
            let target_repo = Repository::init_bare(&target_path)?;
            
            // Add source as remote
            let mut remote = target_repo.remote("origin", &source_path)?;
            
            // Fetch only the default branch
            let refspec = format!("+refs/heads/{0}:refs/heads/{0}", default_branch);
            remote.fetch(&[&refspec], None, None)?;
            
            // Set HEAD to the default branch
            target_repo.set_head(&format!("refs/heads/{}", default_branch))?;
            
            // Remove the remote after fetching
            target_repo.remote_delete("origin")?;
        } else {
            // Clone as a bare repository using git2's RepoBuilder (all branches)
            let mut builder = git2::build::RepoBuilder::new();
            builder.bare(true);
            builder.clone(&source_path, Path::new(&target_path))?;
        }
        
        // Install hooks for the forked repository
        Self::install_hooks(config, &target_path)?;
        
        Ok(())
    }

    pub fn open_repository(config: &AppConfig, owner_name: &str, project_name: &str) -> AppResult<Repository> {
        let path = Self::get_repo_path(config, owner_name, project_name);
        let repo = Repository::open_bare(&path)?;
        Ok(repo)
    }

    pub fn get_repository_info(repo: &Repository) -> AppResult<RepositoryInfo> {
        let default_branch = Self::get_default_branch(repo)?;
        
        let branches: Vec<String> = repo
            .branches(Some(BranchType::Local))?
            .filter_map(|b| b.ok())
            .filter_map(|(branch, _)| branch.name().ok().flatten().map(String::from))
            .collect();

        let tags: Vec<String> = repo
            .tag_names(None)?
            .iter()
            .filter_map(|t| t.map(String::from))
            .collect();

        let last_commit = if let Some(ref branch) = default_branch {
            Self::get_commit_by_ref(repo, branch).ok()
        } else {
            None
        };

        Ok(RepositoryInfo {
            default_branch,
            branches,
            tags,
            size_kb: 0,
            last_commit,
        })
    }

    pub fn get_branches(repo: &Repository) -> AppResult<Vec<BranchInfo>> {
        let default_branch = Self::get_default_branch(repo)?;
        let mut branches = Vec::new();

        for branch_result in repo.branches(Some(BranchType::Local))? {
            let (branch, _) = branch_result?;
            let name = branch.name()?.unwrap_or("").to_string();
            let reference = branch.get();
            
            if let Some(target) = reference.target() {
                let commit = repo.find_commit(target)?;
                let commit_info = Self::commit_to_info(&commit);
                
                branches.push(BranchInfo {
                    name: name.clone(),
                    commit: commit_info,
                    is_protected: false,
                    is_default: Some(&name) == default_branch.as_ref(),
                });
            }
        }

        Ok(branches)
    }

    pub fn create_branch(repo: &Repository, name: &str, ref_name: &str) -> AppResult<()> {
        let commit = Self::resolve_ref_to_commit(repo, ref_name)?;
        repo.branch(name, &commit, false)?;
        Ok(())
    }

    pub fn delete_branch(repo: &Repository, name: &str) -> AppResult<()> {
        let mut branch = repo.find_branch(name, BranchType::Local)?;
        branch.delete()?;
        Ok(())
    }

    pub fn get_tags(repo: &Repository) -> AppResult<Vec<TagInfo>> {
        let mut tags = Vec::new();

        for tag_name in repo.tag_names(None)?.iter().flatten() {
            let reference = repo.find_reference(&format!("refs/tags/{}", tag_name))?;
            let target = reference.peel(ObjectType::Commit)?;
            let commit = target.peel_to_commit()?;
            let commit_info = Self::commit_to_info(&commit);

            // Try to get annotated tag info
            let (message, tagger_name, tagger_email) = if let Ok(tag) = reference.peel_to_tag() {
                (
                    tag.message().map(String::from),
                    tag.tagger().map(|t| t.name().unwrap_or("").to_string()),
                    tag.tagger().map(|t| t.email().unwrap_or("").to_string()),
                )
            } else {
                (None, None, None)
            };

            tags.push(TagInfo {
                name: tag_name.to_string(),
                commit: commit_info,
                message,
                tagger_name,
                tagger_email,
                created_at: chrono::Utc::now(),
            });
        }

        Ok(tags)
    }

    pub fn create_tag(
        repo: &Repository,
        name: &str,
        ref_name: &str,
        message: Option<&str>,
        tagger_name: &str,
        tagger_email: &str,
    ) -> AppResult<()> {
        let commit = Self::resolve_ref_to_commit(repo, ref_name)?;
        let object = commit.as_object();

        if let Some(msg) = message {
            let signature = Signature::now(tagger_name, tagger_email)?;
            repo.tag(name, object, &signature, msg, false)?;
        } else {
            repo.tag_lightweight(name, object, false)?;
        }

        Ok(())
    }

    pub fn delete_tag(repo: &Repository, name: &str) -> AppResult<()> {
        repo.tag_delete(name)?;
        Ok(())
    }

    pub fn get_commits(
        repo: &Repository,
        ref_name: &str,
        path: Option<&str>,
        page: u32,
        per_page: u32,
    ) -> AppResult<Vec<CommitInfo>> {
        let commit = Self::resolve_ref_to_commit(repo, ref_name)?;
        let mut revwalk = repo.revwalk()?;
        revwalk.push(commit.id())?;
        revwalk.set_sorting(Sort::TIME)?;

        let skip = ((page.saturating_sub(1)) * per_page) as usize;
        let mut commits = Vec::new();

        for oid_result in revwalk.skip(skip).take(per_page as usize) {
            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;

            if let Some(p) = path {
                // Filter commits that affect the given path
                if !Self::commit_affects_path(repo, &commit, p)? {
                    continue;
                }
            }

            commits.push(Self::commit_to_info(&commit));
        }

        Ok(commits)
    }

    pub fn get_commit_detail(repo: &Repository, sha: &str) -> AppResult<CommitDetail> {
        let oid = Oid::from_str(sha)?;
        let commit = repo.find_commit(oid)?;
        
        let parent = commit.parent(0).ok();
        let diffs = Self::get_commit_diffs(repo, &commit, parent.as_ref())?;
        
        let (additions, deletions) = diffs.iter().fold((0u32, 0u32), |acc, d| {
            (acc.0 + d.additions, acc.1 + d.deletions)
        });

        let parent_shas: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();
        
        // Extract values before building the result to avoid lifetime issues
        let sha_str = commit.id().to_string();
        let message = commit.message().unwrap_or("").to_string();
        let author = commit.author();
        let author_name = author.name().unwrap_or("").to_string();
        let author_email = author.email().unwrap_or("").to_string();
        let authored_date = author.when().seconds();
        let committer = commit.committer();
        let committer_name = committer.name().unwrap_or("").to_string();
        let committer_email = committer.email().unwrap_or("").to_string();
        let committed_date = committer.when().seconds();

        Ok(CommitDetail {
            sha: sha_str,
            message,
            author_name,
            author_email,
            authored_date,
            committer_name,
            committer_email,
            committed_date,
            parent_shas,
            stats: CommitStats {
                additions,
                deletions,
                files_changed: diffs.len() as u32,
            },
            diffs,
        })
    }

    pub fn get_full_file_diff(
        repo: &Repository,
        sha: &str,
        file_path: &str,
    ) -> AppResult<crate::handlers::commit::FullFileDiff> {
        let oid = Oid::from_str(sha)?;
        let commit = repo.find_commit(oid)?;
        let parent = commit.parent(0).ok();
        
        let tree = commit.tree()?;
        let parent_tree = parent.as_ref().map(|p| p.tree()).transpose()?;
        
        // 读取原始文件内容（完整版）
        let original_content = if let Some(parent_tree) = &parent_tree {
            if let Ok(entry) = parent_tree.get_path(Path::new(file_path)) {
                if let Ok(object) = entry.to_object(repo) {
                    if let Some(blob) = object.as_blob() {
                        std::str::from_utf8(blob.content())
                            .ok()
                            .map(|s| s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        // 读取修改后文件内容（完整版）
        let modified_content = if let Ok(entry) = tree.get_path(Path::new(file_path)) {
            if let Ok(object) = entry.to_object(repo) {
                if let Some(blob) = object.as_blob() {
                    std::str::from_utf8(blob.content())
                        .ok()
                        .map(|s| s.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        let total_lines = modified_content
            .as_ref()
            .or(original_content.as_ref())
            .map(|c| c.lines().count() as u32)
            .unwrap_or(0);
        
        Ok(crate::handlers::commit::FullFileDiff {
            original_content,
            modified_content,
            total_lines,
        })
    }

    pub fn browse_tree(
        repo: &Repository,
        ref_name: &str,
        path: Option<&str>,
    ) -> AppResult<Vec<FileEntry>> {
        let commit = Self::resolve_ref_to_commit(repo, ref_name)?;
        let tree = commit.tree()?;

        let target_tree = if let Some(p) = path {
            let entry = tree.get_path(Path::new(p))?;
            repo.find_tree(entry.id())?
        } else {
            tree
        };

        let mut entries = Vec::new();

        for entry in target_tree.iter() {
            let name = entry.name().unwrap_or("").to_string();
            let entry_path = if let Some(p) = path {
                format!("{}/{}", p, name)
            } else {
                name.clone()
            };

            let entry_type = match entry.kind() {
                Some(ObjectType::Tree) => FileEntryType::Directory,
                Some(ObjectType::Blob) => FileEntryType::File,
                Some(ObjectType::Commit) => FileEntryType::Submodule,
                _ => continue,
            };

            let size = if entry_type == FileEntryType::File {
                repo.find_blob(entry.id()).ok().map(|b| b.size() as u64)
            } else {
                None
            };

            entries.push(FileEntry {
                name,
                path: entry_path,
                entry_type,
                size,
                mode: entry.filemode() as u32,
            });
        }

        // Sort: directories first, then files
        entries.sort_by(|a, b| {
            match (&a.entry_type, &b.entry_type) {
                (FileEntryType::Directory, FileEntryType::File) => std::cmp::Ordering::Less,
                (FileEntryType::File, FileEntryType::Directory) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        Ok(entries)
    }

    pub fn get_file_content(repo: &Repository, ref_name: &str, path: &str) -> AppResult<FileContent> {
        let commit = Self::resolve_ref_to_commit(repo, ref_name)?;
        let tree = commit.tree()?;
        let entry = tree.get_path(Path::new(path))?;
        let blob = repo.find_blob(entry.id())?;

        let is_binary = blob.is_binary();
        let content = if is_binary {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(blob.content())
        } else {
            String::from_utf8_lossy(blob.content()).to_string()
        };

        Ok(FileContent {
            path: path.to_string(),
            content,
            size: blob.size() as u64,
            encoding: if is_binary { "base64" } else { "utf-8" }.to_string(),
            is_binary,
        })
    }

    pub fn compare_refs(repo: &Repository, from: &str, to: &str) -> AppResult<Vec<CommitInfo>> {
        let from_commit = Self::resolve_ref_to_commit(repo, from)?;
        let to_commit = Self::resolve_ref_to_commit(repo, to)?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push(to_commit.id())?;
        revwalk.hide(from_commit.id())?;
        revwalk.set_sorting(Sort::REVERSE | Sort::TIME)?;

        let mut commits = Vec::new();
        for oid_result in revwalk {
            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;
            commits.push(Self::commit_to_info(&commit));
        }

        Ok(commits)
    }

    pub fn get_blob(repo: &Repository, sha: &str) -> AppResult<BlobContent> {
        let oid = Oid::from_str(sha)
            .map_err(|_| AppError::BadRequest(format!("Invalid blob SHA: {}", sha)))?;
        let blob = repo.find_blob(oid)
            .map_err(|_| AppError::NotFound(format!("Blob not found: {}", sha)))?;

        let is_binary = blob.is_binary();
        let content = if is_binary {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(blob.content())
        } else {
            String::from_utf8_lossy(blob.content()).to_string()
        };

        Ok(BlobContent {
            sha: sha.to_string(),
            content,
            size: blob.size() as u64,
            encoding: if is_binary { "base64" } else { "utf-8" }.to_string(),
            is_binary,
        })
    }

    pub fn can_merge(repo: &Repository, source: &str, target: &str) -> AppResult<bool> {
        let source_commit = Self::resolve_ref_to_commit(repo, source)?;
        let target_commit = Self::resolve_ref_to_commit(repo, target)?;

        let index = repo.merge_commits(&target_commit, &source_commit, None)?;
        Ok(!index.has_conflicts())
    }

    /// Perform a merge from source branch to target branch
    /// Returns the merge commit SHA
    pub fn perform_merge(
        repo: &Repository,
        source_branch: &str,
        target_branch: &str,
        merge_message: &str,
        author_name: &str,
        author_email: &str,
    ) -> AppResult<String> {
        let source_commit = Self::resolve_ref_to_commit(repo, source_branch)?;
        let target_commit = Self::resolve_ref_to_commit(repo, target_branch)?;

        // Check for conflicts first
        let mut index = repo.merge_commits(&target_commit, &source_commit, None)?;
        if index.has_conflicts() {
            return Err(AppError::Conflict("Cannot merge due to conflicts".to_string()));
        }

        // Write the merged tree
        let tree_oid = index.write_tree_to(repo)?;
        let tree = repo.find_tree(tree_oid)?;

        // Create signature
        let sig = Signature::now(author_name, author_email)?;

        // Create merge commit
        let merge_commit_oid = repo.commit(
            None, // Don't update any reference yet
            &sig,
            &sig,
            merge_message,
            &tree,
            &[&target_commit, &source_commit],
        )?;

        // Update the target branch reference to point to the new merge commit
        let target_ref_name = format!("refs/heads/{}", target_branch);
        repo.reference(
            &target_ref_name,
            merge_commit_oid,
            true, // force
            &format!("merge: {} into {}", source_branch, target_branch),
        )?;

        Ok(merge_commit_oid.to_string())
    }

    /// Delete a branch (for cleanup after merge)
    pub fn delete_branch_by_name(repo: &Repository, branch_name: &str) -> AppResult<()> {
        let mut branch = repo.find_branch(branch_name, BranchType::Local)?;
        branch.delete()?;
        Ok(())
    }

    // Helper methods

    fn resolve_ref_to_commit<'a>(repo: &'a Repository, ref_name: &str) -> AppResult<Commit<'a>> {
        // Try as branch first
        if let Ok(branch) = repo.find_branch(ref_name, BranchType::Local) {
            if let Some(target) = branch.get().target() {
                return Ok(repo.find_commit(target)?);
            }
        }

        // Try as tag
        if let Ok(reference) = repo.find_reference(&format!("refs/tags/{}", ref_name)) {
            let target = reference.peel(ObjectType::Commit)?;
            return Ok(target.peel_to_commit()?);
        }

        // Try as commit SHA
        if let Ok(oid) = Oid::from_str(ref_name) {
            return Ok(repo.find_commit(oid)?);
        }

        Err(AppError::NotFound(format!("Reference '{}' not found", ref_name)))
    }

    fn get_commit_by_ref(repo: &Repository, ref_name: &str) -> AppResult<CommitInfo> {
        let commit = Self::resolve_ref_to_commit(repo, ref_name)?;
        Ok(Self::commit_to_info(&commit))
    }

    fn commit_to_info(commit: &Commit) -> CommitInfo {
        CommitInfo {
            sha: commit.id().to_string(),
            message: commit.message().unwrap_or("").to_string(),
            author_name: commit.author().name().unwrap_or("").to_string(),
            author_email: commit.author().email().unwrap_or("").to_string(),
            authored_date: commit.author().when().seconds(),
            committer_name: commit.committer().name().unwrap_or("").to_string(),
            committer_email: commit.committer().email().unwrap_or("").to_string(),
            committed_date: commit.committer().when().seconds(),
        }
    }

    fn get_commit_diffs(
        repo: &Repository,
        commit: &Commit,
        parent: Option<&Commit>,
    ) -> AppResult<Vec<DiffInfo>> {
        const MAX_DIFF_CHANGES: usize = 500; // 单个文件最大 diff 更改行数
        
        let tree = commit.tree()?;
        let parent_tree = parent.map(|p| p.tree()).transpose()?;

        let mut diff_opts = DiffOptions::new();
        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;

        let mut diffs = Vec::new();

        // Step 1: Collect file metadata and content
        diff.foreach(
            &mut |delta, _| {
                let status = match delta.status() {
                    git2::Delta::Added => DiffStatus::Added,
                    git2::Delta::Deleted => DiffStatus::Deleted,
                    git2::Delta::Modified => DiffStatus::Modified,
                    git2::Delta::Renamed => DiffStatus::Renamed,
                    git2::Delta::Copied => DiffStatus::Copied,
                    _ => return true,
                };

                let old_path = delta.old_file().path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                let new_path = delta.new_file().path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                diffs.push(DiffInfo {
                    old_path,
                    new_path,
                    diff: String::new(),
                    status,
                    additions: 0,
                    deletions: 0,
                    original_content: None,
                    modified_content: None,
                    is_truncated: false,
                    total_lines: None,
                });

                true
            },
            None,
            None,
            None,
        )?;

        // Step 2: Generate patch for each file and count stats
        let mut file_idx = 0;
        let mut seen_first_file_header = false;
        
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            // File header marks the start of a new file
            if line.origin() == 'F' {
                if seen_first_file_header && file_idx + 1 < diffs.len() {
                    file_idx += 1;
                }
                seen_first_file_header = true;
                return true;
            }

            if file_idx >= diffs.len() {
                return true;
            }

            let diff_info = &mut diffs[file_idx];

            // Count additions and deletions
            match line.origin() {
                '+' => {
                    if !line.content().starts_with(b"+++") {
                        diff_info.additions += 1;
                    }
                }
                '-' => {
                    if !line.content().starts_with(b"---") {
                        diff_info.deletions += 1;
                    }
                }
                _ => {}
            }

            // Append line to diff string
            if matches!(line.origin(), 'F' | 'H' | '+' | '-' | ' ') {
                diff_info.diff.push(line.origin());
                if let Ok(content) = std::str::from_utf8(line.content()) {
                    diff_info.diff.push_str(content);
                }
            }

            true
        })?;

        // Step 3: 根据 diff 更改行数决定是否加载完整文件内容
        for diff_info in &mut diffs {
            let change_lines = diff_info.additions + diff_info.deletions;
            
            if change_lines as usize > MAX_DIFF_CHANGES {
                // 更改行数过多，不加载完整文件内容
                diff_info.is_truncated = true;
                diff_info.total_lines = Some(change_lines as u32);
                continue;
            }
            
            // 更改行数适中，加载完整文件内容用于 Monaco Editor
            // 读取原始文件
            if diff_info.status != DiffStatus::Added {
                if let Some(content) = Self::load_file_content(&parent_tree, &diff_info.old_path, repo) {
                    diff_info.original_content = Some(content);
                }
            }
            
            // 读取修改后文件
            if diff_info.status != DiffStatus::Deleted {
                if let Some(content) = Self::load_file_content(&Some(tree.clone()), &diff_info.new_path, repo) {
                    let line_count = content.lines().count();
                    diff_info.modified_content = Some(content);
                    diff_info.total_lines = Some(line_count as u32);
                }
            }
        }

        Ok(diffs)
    }

    /// 辅助函数：从 Git tree 加载文件内容
    fn load_file_content(tree: &Option<Tree>, path: &str, repo: &Repository) -> Option<String> {
        let tree = tree.as_ref()?;
        let entry = tree.get_path(Path::new(path)).ok()?;
        let object = entry.to_object(repo).ok()?;
        let blob = object.as_blob()?;
        std::str::from_utf8(blob.content()).ok().map(|s| s.to_string())
    }

    fn commit_affects_path(repo: &Repository, commit: &Commit, path: &str) -> AppResult<bool> {
        let tree = commit.tree()?;
        
        if let Ok(parent) = commit.parent(0) {
            let parent_tree = parent.tree()?;
            let mut diff_opts = DiffOptions::new();
            diff_opts.pathspec(path);
            
            let diff = repo.diff_tree_to_tree(
                Some(&parent_tree),
                Some(&tree),
                Some(&mut diff_opts),
            )?;
            
            Ok(diff.deltas().count() > 0)
        } else {
            // First commit - check if path exists in tree
            Ok(tree.get_path(Path::new(path)).is_ok())
        }
    }
    
    /// Install Git hooks for CI/CD triggering
    fn install_hooks(config: &AppConfig, repo_path: &str) -> AppResult<()> {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        
        let hooks_dir = format!("{}/hooks", repo_path);
        fs::create_dir_all(&hooks_dir)?;
        
        // Create post-receive hook
        let post_receive_path = format!("{}/post-receive", hooks_dir);
        let hook_content = format!(
            r#"#!/bin/bash
# GitFox post-receive hook - Auto-generated
# This hook triggers CI/CD pipelines after git push
#
# Environment variables set by gitfox-shell:
#   GL_REPOSITORY - Repository path (e.g., "hanxi-cat/project")
#   GITFOX_USER_ID - User ID
#   GITFOX_PROJECT_ID - Project ID

set -e

# Collect all ref changes into JSON array
changes="["
first=true
while read oldrev newrev refname; do
    # Skip if branch/tag is deleted
    if [ "$newrev" = "0000000000000000000000000000000000000000" ]; then
        continue
    fi
    
    if [ "$first" = true ]; then
        first=false
    else
        changes="$changes,"
    fi
    changes="$changes{{\"old_sha\":\"$oldrev\",\"new_sha\":\"$newrev\",\"ref\":\"$refname\"}}"
done
changes="$changes]"

# Notify GitFox API about the push
if [ -n "$GL_REPOSITORY" ]; then
    curl -s -X POST \
        -H "Content-Type: application/json" \
        -H "X-GitFox-Shell-Token: {}" \
        -d "{{
            \"user_id\": \"${{GITFOX_USER_ID:-0}}\",
            \"repository\": \"$GL_REPOSITORY\",
            \"project_id\": \"$GITFOX_PROJECT_ID\",
            \"changes\": $changes
        }}" \
        "{}/api/internal/post-receive" > /dev/null 2>&1 || true
fi

exit 0
"#,
            config.shell_secret,
            config.base_url.trim_end_matches('/')
        );
        
        fs::write(&post_receive_path, hook_content)?;
        
        // Make hook executable
        let metadata = fs::metadata(&post_receive_path)?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&post_receive_path, permissions)?;
        
        Ok(())
    }
}
