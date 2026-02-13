use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub entry_type: FileEntryType,
    pub size: Option<u64>,
    pub mode: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum FileEntryType {
    File,
    Directory,
    Submodule,
    Symlink,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub encoding: String,
    pub is_binary: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlobContent {
    pub sha: String,
    pub content: String,
    pub size: u64,
    pub encoding: String,
    pub is_binary: bool,
}

#[derive(Debug, Deserialize)]
pub struct BrowseQuery {
    pub path: Option<String>,
    pub ref_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FileQuery {
    pub ref_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RepositoryInfo {
    pub default_branch: Option<String>,
    pub branches: Vec<String>,
    pub tags: Vec<String>,
    pub size_kb: u64,
    pub last_commit: Option<CommitInfo>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CommitInfo {
    pub sha: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_date: i64,
    pub committer_name: String,
    pub committer_email: String,
    pub committed_date: i64,
}

#[derive(Debug, Serialize)]
pub struct DiffInfo {
    pub old_path: String,
    pub new_path: String,
    pub diff: String,
    pub status: DiffStatus,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum DiffStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
}

#[derive(Debug, Deserialize)]
pub struct CloneRequest {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFileRequest {
    pub path: String,
    pub content: String,
    pub branch: String,
    pub commit_message: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFileRequest {
    pub path: String,
    pub content: String,
    pub branch: String,
    pub commit_message: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteFileRequest {
    pub path: String,
    pub branch: String,
    pub commit_message: String,
}
