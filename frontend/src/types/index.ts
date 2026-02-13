export interface User {
  id: string
  username: string
  email: string
  display_name?: string
  avatar_url?: string
  role: 'admin' | 'developer' | 'viewer'
}

export interface LoginRequest {
  username: string
  password: string
}

export interface RegisterRequest {
  username: string
  email: string
  password: string
  display_name?: string
}

export interface LoginResponse {
  token: string
  user: User
}

export interface Project {
  id: string
  name: string
  description?: string
  visibility: 'public' | 'private' | 'internal'
  owner_id: string
  owner_name?: string
  owner_avatar?: string
  created_at: string
  updated_at: string
}

export interface CreateProjectRequest {
  name: string
  description?: string
  visibility?: 'public' | 'private' | 'internal'
}

export interface ProjectStats {
  commits_count: number
  branches_count: number
  tags_count: number
  merge_requests_count: number
  members_count: number
}

export interface ProjectMember {
  id: string
  project_id: string
  user_id: string
  role: 'owner' | 'maintainer' | 'developer' | 'reporter' | 'guest'
  created_at: string
}

export interface RepositoryInfo {
  default_branch: string | null
  branches: string[]
  tags: string[]
  size_kb: number
  last_commit?: CommitInfo
}

export interface FileEntry {
  name: string
  path: string
  entry_type: 'File' | 'Directory' | 'Submodule' | 'Symlink'
  size?: number
  mode: number
}

export interface FileContent {
  path: string
  content: string
  size: number
  encoding: string
  is_binary: boolean
}

export interface CommitInfo {
  sha: string
  message: string
  author_name: string
  author_email: string
  authored_date: number
  committer_name: string
  committer_email: string
  committed_date: number
}

export interface CommitDetail extends CommitInfo {
  parent_shas: string[]
  stats: CommitStats
  diffs: DiffInfo[]
}

export interface CommitStats {
  additions: number
  deletions: number
  files_changed: number
}

export interface DiffInfo {
  old_path: string
  new_path: string
  diff: string
  status: 'Added' | 'Deleted' | 'Modified' | 'Renamed' | 'Copied'
  additions: number
  deletions: number
}

export interface BranchInfo {
  name: string
  commit: CommitInfo
  is_protected: boolean
  is_default: boolean
}

// 别名类型，方便使用
export type Branch = BranchInfo & {
  last_commit_message?: string
  updated_at: string
}

export interface TagInfo {
  name: string
  commit: CommitInfo
  message?: string
  tagger_name?: string
  tagger_email?: string
  created_at: string
}

// 别名类型
export type Tag = TagInfo & {
  commit_sha?: string
}

// 别名类型
export type Commit = CommitInfo & {
  parent_sha?: string
  committed_at: string
}

export interface MergeRequest {
  id: string
  project_id: string
  iid: number
  title: string
  description?: string
  source_branch: string
  target_branch: string
  status: 'open' | 'merged' | 'closed' | 'draft'
  author_id: string
  assignee_id?: string
  merged_by?: string
  merged_at?: string
  closed_by?: string
  closed_at?: string
  created_at: string
  updated_at: string
}

export interface CreateMergeRequestRequest {
  title: string
  description?: string
  source_branch: string
  target_branch: string
  assignee_id?: string
  is_draft?: boolean
}

export interface MergeRequestComment {
  id: string
  merge_request_id: string
  author_id: string
  content: string
  line_number?: number
  file_path?: string
  parent_id?: string
  is_resolved: boolean
  created_at: string
  updated_at: string
}

export interface MergeRequestReview {
  id: string
  merge_request_id: string
  reviewer_id: string
  status: 'pending' | 'approved' | 'requestchanges' | 'commented'
  comment?: string
  created_at: string
  updated_at: string
}

export interface Pipeline {
  id: string
  project_id: string
  ref_name: string
  commit_sha: string
  status: PipelineStatus
  trigger_type: string
  triggered_by?: string
  started_at?: string
  finished_at?: string
  duration_seconds?: number
  created_at: string
  updated_at: string
}

export type PipelineStatus = 'pending' | 'running' | 'success' | 'failed' | 'canceled' | 'skipped'

export interface PipelineJob {
  id: string
  pipeline_id: string
  name: string
  stage: string
  status: PipelineStatus
  runner_id?: string
  started_at?: string
  finished_at?: string
  duration_seconds?: number
  allow_failure: boolean
  created_at: string
  updated_at: string
}

export interface Webhook {
  id: string
  project_id: string
  url: string
  secret?: string
  events: string[]
  is_active: boolean
  created_at: string
  updated_at: string
}

export interface CreateWebhookRequest {
  url: string
  secret?: string
  events: string[]
}

// Groups
export interface Group {
  id: string
  namespace_id: string
  name: string
  path: string
  description?: string
  avatar_url?: string
  visibility: 'public' | 'private' | 'internal'
  parent_id?: string
  created_at: string
  updated_at: string
}

export interface GroupMember {
  id: string
  group_id: string
  user_id: string
  access_level: number
  created_at: string
  expires_at?: string
  // joined from users table
  username?: string
  display_name?: string
  avatar_url?: string
}

export interface CreateGroupRequest {
  name: string
  path: string
  description?: string
  visibility?: 'public' | 'private' | 'internal'
  parent_id?: string
}

export interface UpdateGroupRequest {
  name?: string
  description?: string
  visibility?: 'public' | 'private' | 'internal'
  avatar_url?: string
}

export interface AddGroupMemberRequest {
  user_id: string
  access_level: number
  expires_at?: string
}

// Access levels matching backend
export const ACCESS_LEVELS = {
  GUEST: 10,
  REPORTER: 20,
  DEVELOPER: 30,
  MAINTAINER: 40,
  OWNER: 50,
} as const

export const ACCESS_LEVEL_LABELS: Record<number, string> = {
  10: '访客',
  20: '报告者',
  30: '开发者',
  40: '维护者',
  50: '所有者',
}
