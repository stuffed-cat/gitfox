//! Diff service implementation

use std::pin::Pin;
use std::sync::Arc;

use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::config::Config;
use crate::git::repository::RepositoryOps;
use crate::git::diff::DiffOps;
use crate::proto::*;

pub struct DiffServiceImpl {
    config: Arc<Config>,
}

impl DiffServiceImpl {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
    
    fn get_repo_path(&self, repo: Option<&Repository>) -> Result<String, Status> {
        let repo = repo.ok_or_else(|| Status::invalid_argument("Repository required"))?;
        
        if !repo.storage_path.is_empty() {
            Ok(repo.storage_path.clone())
        } else if !repo.relative_path.is_empty() {
            Ok(self.config.repo_path(&repo.relative_path))
        } else {
            Err(Status::invalid_argument("Repository path required"))
        }
    }
    
    fn convert_diff_file(f: &crate::git::diff::DiffFile) -> DiffFile {
        DiffFile {
            old_path: f.old_path.clone(),
            new_path: f.new_path.clone(),
            old_id: f.old_id.clone(),
            new_id: f.new_id.clone(),
            status: f.status.clone(),
            additions: f.additions,
            deletions: f.deletions,
            is_binary: f.is_binary,
            hunks: f.hunks.iter().map(|h| DiffHunk {
                old_start: h.old_start,
                old_lines: h.old_lines,
                new_start: h.new_start,
                new_lines: h.new_lines,
                header: h.header.clone(),
                lines: h.lines.iter().map(|l| DiffLine {
                    prefix: l.prefix.clone(),
                    old_line_number: l.old_line_number,
                    new_line_number: l.new_line_number,
                    content: l.content.clone(),
                }).collect(),
            }).collect(),
            similarity: f.similarity,
        }
    }
}

#[tonic::async_trait]
impl diff_service_server::DiffService for DiffServiceImpl {
    async fn commit_diff(
        &self,
        request: Request<CommitDiffRequest>,
    ) -> Result<Response<CommitDiffResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let old_rev = if req.old_revision.is_empty() { None } else { Some(req.old_revision.as_str()) };
        let context = if req.context_lines > 0 { req.context_lines as u32 } else { 3 };
        
        let (files, additions, deletions, files_changed) = DiffOps::commit_diff(
            &repo,
            old_rev,
            &req.new_revision,
            &req.paths,
            context,
        )?;
        
        let file_protos: Vec<DiffFile> = files.iter()
            .map(|f| Self::convert_diff_file(f))
            .collect();
        
        Ok(Response::new(CommitDiffResponse {
            files: file_protos,
            total_additions: additions,
            total_deletions: deletions,
            files_changed,
        }))
    }
    
    type StreamCommitDiffStream = Pin<Box<dyn Stream<Item = Result<DiffFile, Status>> + Send>>;
    
    async fn stream_commit_diff(
        &self,
        _request: Request<StreamCommitDiffRequest>,
    ) -> Result<Response<Self::StreamCommitDiffStream>, Status> {
        Err(Status::unimplemented("Streaming not yet implemented"))
    }
    
    async fn diff_stats(
        &self,
        request: Request<DiffStatsRequest>,
    ) -> Result<Response<DiffStatsResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        let (additions, deletions, files_changed, file_stats) = DiffOps::diff_stats(
            &repo,
            &req.old_revision,
            &req.new_revision,
        )?;
        
        let stats_protos: Vec<FileStats> = file_stats.iter()
            .map(|s| FileStats {
                path: s.path.clone(),
                additions: s.additions,
                deletions: s.deletions,
            })
            .collect();
        
        Ok(Response::new(DiffStatsResponse {
            additions,
            deletions,
            files_changed,
            file_stats: stats_protos,
        }))
    }
    
    async fn compare(
        &self,
        request: Request<CompareRequest>,
    ) -> Result<Response<CompareResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        let limit = if req.limit > 0 { req.limit as usize } else { 100 };
        
        let (commits, diffs, merge_base, ahead, behind) = DiffOps::compare(
            &repo,
            &req.from,
            &req.to,
            req.straight,
            limit,
        )?;
        
        let commit_protos: Vec<Commit> = commits.iter()
            .map(|c| Commit {
                id: c.id.clone(),
                tree_id: c.tree_id.clone(),
                parent_ids: c.parent_ids.clone(),
                author: Some(Signature {
                    name: c.author.name.clone(),
                    email: c.author.email.clone(),
                    timestamp: c.author.timestamp,
                    timezone: c.author.timezone.clone(),
                }),
                committer: Some(Signature {
                    name: c.committer.name.clone(),
                    email: c.committer.email.clone(),
                    timestamp: c.committer.timestamp,
                    timezone: c.committer.timezone.clone(),
                }),
                message: c.message.clone(),
                short_message: c.short_message.clone(),
                signature: String::new(),
                signature_status: String::new(),
            })
            .collect();
        
        let diff_protos: Vec<DiffFile> = diffs.iter()
            .map(|f| Self::convert_diff_file(f))
            .collect();
        
        Ok(Response::new(CompareResponse {
            commits: commit_protos,
            diffs: diff_protos,
            merge_base,
            ahead_count: ahead,
            behind_count: behind,
        }))
    }
    
    async fn raw_diff(
        &self,
        request: Request<RawDiffRequest>,
    ) -> Result<Response<RawDiffResponse>, Status> {
        let req = request.into_inner();
        let path = self.get_repo_path(req.repository.as_ref())?;
        
        let repo = RepositoryOps::open(&path)?;
        
        let data = DiffOps::raw_diff(&repo, &req.old_revision, &req.new_revision)?;
        
        Ok(Response::new(RawDiffResponse { data }))
    }
    
    async fn find_conflicts(
        &self,
        _request: Request<FindConflictsRequest>,
    ) -> Result<Response<FindConflictsResponse>, Status> {
        // TODO: Implement conflict detection
        Err(Status::unimplemented("Conflict detection not yet implemented"))
    }
}
