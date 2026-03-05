export interface User {
  id: string
  username: string
  email: string
  display_name?: string
  avatar_url?: string
  role: 'admin' | 'developer' | 'viewer'
  is_active: boolean
  email_confirmed?: boolean
  status_emoji?: string
  status_message?: string
  busy?: boolean
  status_set_at?: string
  status_clear_at?: string
  is_pro?: boolean
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
  stars_count?: number
  forks_count?: number
  forked_from_id?: string
  forked_from_namespace?: string
  forked_from_name?: string
}

export interface ForkDivergence {
  ahead: number
  behind: number
  fork_branch: string
  upstream_branch: string
}

export interface CreateProjectRequest {
  name: string
  description?: string
  visibility?: 'public' | 'private' | 'internal'
  /** Optional namespace_id (group's namespace). If not provided, uses user's namespace. */
  namespace_id?: number
}

/** Request for updating project settings (matches backend UpdateProjectRequest) */
export interface UpdateProjectRequest {
  name?: string
  description?: string
  visibility?: 'public' | 'private' | 'internal'
}

/** Namespace option for project creation */
export interface NamespaceOption {
  id: number
  name: string
  path: string
  namespace_type: 'user' | 'group'
  avatar_url?: string
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
  gpg_verification?: GpgVerificationInfo
}

export interface GpgVerificationInfo {
  status: 'verified' | 'unverified' | 'bad_email' | 'unknown_key' | 'bad_signature' | 'expired_key' | 'revoked_key' | 'no_signature'
  message?: string
  key_id?: string
  signer_user_id?: number
  signer_username?: string
  verified: boolean
}

export interface CommitDetail extends CommitInfo {
  parent_shas: string[]
  stats: CommitStats
  diffs: DiffInfo[]
  gpg_verification?: GpgVerificationInfo
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
  original_content?: string
  modified_content?: string
  is_truncated: boolean
  total_lines?: number
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
  project_id: string  // target project (where MR is created)
  source_project_id: string  // source project (can be same as project_id or a fork)
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
  source_project_id?: string  // If omitted, defaults to target project (same-repo MR)
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
  error_message?: string
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
  error_message?: string
  created_at: string
  updated_at: string
}

// CI/CD Runners
export interface Runner {
  id: string
  name: string
  description?: string
  scope: 'system' | 'user' | 'namespace' | 'project'
  user_id?: string
  namespace_id?: string
  project_id?: string
  token_preview: string
  is_active: boolean
  status: 'idle' | 'running' | 'offline'
  last_contact_at?: string
  tags: string[]
  run_untagged: boolean
  locked: boolean
  maximum_timeout?: number
  version?: string
  platform?: string
  architecture?: string
  created_at: string
  updated_at: string
}

export interface CreateRunnerRequest {
  name: string
  description?: string
  tags?: string[]
  run_untagged?: boolean
  locked?: boolean
  maximum_timeout?: number
}

export interface UpdateRunnerRequest {
  name?: string
  description?: string
  tags?: string[]
  is_active?: boolean
  run_untagged?: boolean
  locked?: boolean
  maximum_timeout?: number
}

export interface CreateRunnerResponse {
  runner: Runner
  token: string
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
  parent_id?: number
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

// Admin types
export interface SystemStats {
  total_users: number
  active_users: number
  total_projects: number
  total_groups: number
  admin_count: number
}

export interface AdminUserInfo {
  id: string
  username: string
  email: string
  display_name?: string
  avatar_url?: string
  role: 'admin' | 'developer' | 'viewer'
  is_active: boolean
  created_at: string
  updated_at: string
  projects_count: number
  is_pro: boolean
}

export interface AdminUserListResponse {
  users: AdminUserInfo[]
  total: number
  page: number
  per_page: number
}

export interface AdminUpdateUserRequest {
  role?: 'admin' | 'developer' | 'viewer'
  is_active?: boolean
  display_name?: string
  email?: string
  is_pro?: boolean
}

export type SystemConfigMap = Record<string, any>

// ─────────────────────────────────────────────────────────────────────────────
// Personal Access Token Types
// ─────────────────────────────────────────────────────────────────────────────

export type PatScope = 
  | 'read_api' 
  | 'write_api' 
  | 'read_repository' 
  | 'write_repository' 
  | 'read_user' 
  | 'write_user' 
  | 'read_registry' 
  | 'write_registry' 
  | 'admin'

export interface PatScopeInfo {
  name: PatScope
  description: string
}

export interface PersonalAccessToken {
  id: number
  name: string
  token_last_four: string  // Also known as token_prefix
  scopes: PatScope[]
  expires_at?: string
  last_used_at?: string
  revoked: boolean
  created_at: string
}

export interface CreatePatRequest {
  name: string
  scopes?: PatScope[]
  expires_in_days?: number
}

export interface CreatePatResponse {
  id: number
  name: string
  /** The raw token - only shown once! */
  token: string
  scopes: PatScope[]
  expires_at?: string
  created_at: string
}

// ─────────────────────────────────────────────────────────────────────────────
// OAuth Types
// ─────────────────────────────────────────────────────────────────────────────

export type OAuthProviderType = 'github' | 'gitlab' | 'google' | 'bitbucket' | 'azure_ad' | 'custom'

export interface OAuthProviderInfo {
  name: string
  display_name: string
  provider_type: string
  icon?: string
  authorize_url: string
  enabled?: boolean
}

export interface OAuthProvider {
  name: OAuthProviderType
  display_name: string
  authorize_url: string
}

export interface OAuthProvidersResponse {
  providers: OAuthProviderInfo[]
}

export interface OAuthApplication {
  id: number
  name: string
  uid: string  // Client ID
  redirect_uris: string[]
  scopes: string[]
  description?: string
  homepage_url?: string
  logo_url?: string
  confidential: boolean
  trusted: boolean
  created_at: string
}

export interface OAuthApplicationWithSecret extends OAuthApplication {
  secret: string  // Client Secret (only shown once)
}

export interface CreateOAuthApplicationRequest {
  name: string
  redirect_uris: string[]
  scopes?: string[]
  description?: string
  homepage_url?: string
  logo_url?: string
  confidential?: boolean
}

export interface UpdateOAuthApplicationRequest {
  name?: string
  redirect_uris?: string[]
  scopes?: string[]
  description?: string
  homepage_url?: string
  logo_url?: string
  confidential?: boolean
}

export interface OAuthIdentity {
  id: number
  provider_id: number
  provider_name: string
  provider_type: string
  provider_display_name: string
  external_uid: string
  external_username?: string
  external_email?: string
  external_avatar_url?: string
  last_sign_in_at?: string
  created_at: string
}

export interface OAuthAuthorizeParams {
  client_id: string
  redirect_uri: string
  response_type: 'code'
  scope?: string
  state?: string
  code_challenge?: string
  code_challenge_method?: 'plain' | 'S256'
}

export interface OAuthTokenResponse {
  access_token: string
  token_type: string
  expires_in?: number
  refresh_token?: string
  scope: string
  created_at?: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin OAuth Provider Types
// ─────────────────────────────────────────────────────────────────────────────

/** OAuth 提供商管理信息（管理员视图） */
export interface OAuthProviderAdmin {
  id: number
  name: string
  display_name: string
  provider_type: string
  client_id: string
  authorization_url: string
  token_url: string
  userinfo_url?: string
  scopes: string[]
  icon?: string
  enabled: boolean
  sort_order: number
  created_at: string
  updated_at: string
}

/** 创建 OAuth 提供商请求 */
export interface CreateOAuthProviderRequest {
  name: string
  display_name: string
  provider_type: string
  client_id: string
  client_secret: string
  authorization_url?: string
  token_url?: string
  userinfo_url?: string
  scopes?: string[]
  icon?: string
  enabled?: boolean
}

/** 更新 OAuth 提供商请求 */
export interface UpdateOAuthProviderRequest {
  display_name?: string
  client_id?: string
  client_secret?: string
  authorization_url?: string
  token_url?: string
  userinfo_url?: string
  scopes?: string[]
  icon?: string
  enabled?: boolean
  sort_order?: number
}

// ─── Two-Factor Authentication ──────────────────────────────────────

/** 2FA required response from login */
export interface TwoFactorRequiredResponse {
  requires_two_factor: boolean
  available_methods: string[] // ['totp', 'webauthn', 'recovery']
  temporary_token: string
}

/** Verify 2FA request */
export interface VerifyTwoFactorRequest {
  temporary_token: string
  method: string // 'totp', 'webauthn', or 'recovery'
  code?: string // For TOTP and recovery codes
  webauthn_response?: string // For WebAuthn
}

/** 2FA status */
export interface TwoFactorStatus {
  enabled: boolean
  totp_enabled: boolean
  webauthn_credentials: WebAuthnCredentialInfo[]
  recovery_codes_count: number
}

/** WebAuthn credential info */
export interface WebAuthnCredentialInfo {
  id: number
  name: string
  created_at: string
  last_used_at?: string
}

/** TOTP setup response */
export interface TotpSetupResponse {
  state_key: string // Redis key for completing setup
  secret: string
  qr_code: string // Data URL
  backup_codes: string[]
}

/** Enable TOTP request */
export interface EnableTotpRequest {
  state_key: string
  totp_code: string
}

/** Disable TOTP request */
export interface DisableTotpRequest {
  totp_code: string
}

/** Recovery codes response */
export interface RecoveryCodesResponse {
  codes: string[]
}

/** WebAuthn registration start request */
export interface WebAuthnRegisterStartRequest {
  name: string
}

/** WebAuthn registration start response */
export interface WebAuthnRegisterStartResponse {
  challenge: PublicKeyCredentialCreationOptions
  state_key: string
}

/** WebAuthn registration finish request */
export interface WebAuthnRegisterFinishRequest {
  state_key: string
  name: string
  credential: any // PublicKeyCredential
}

/** WebAuthn registration finish response */
export interface WebAuthnRegisterFinishResponse {
  message: string
  recovery_codes_generated: boolean
  recovery_codes?: string[]
}

/** WebAuthn authentication start request (for login) */
export interface WebAuthnAuthStartRequest {
  temporary_token: string
}

/** WebAuthn authentication start response */
export interface WebAuthnAuthStartResponse {
  challenge: PublicKeyCredentialRequestOptions
  state_key: string
}

/** WebAuthn authentication finish request */
export interface WebAuthnAuthFinishRequest {
  temporary_token: string
  state_key: string
  credential: any // PublicKeyCredential
}

// ─────────────────────────────────────────────────────────────────────────────
// GPG Key Types
// ─────────────────────────────────────────────────────────────────────────────

/** GPG Key information */
export interface GpgKey {
  id: number
  primary_key_id: string
  fingerprint: string
  key_algorithm: string
  key_size?: number
  emails: string[]
  can_sign: boolean
  can_encrypt: boolean
  can_certify: boolean
  key_created_at?: string
  key_expires_at?: string
  verified: boolean
  revoked: boolean
  subkeys: GpgKeySubkey[]
  last_used_at?: string
  created_at: string
}

/** GPG Key subkey information */
export interface GpgKeySubkey {
  id: number
  key_id: string
  fingerprint: string
  key_algorithm: string
  key_size?: number
  can_sign: boolean
  can_encrypt: boolean
  key_created_at?: string
  key_expires_at?: string
  revoked: boolean
}

/** Request to add a new GPG key */
export interface CreateGpgKeyRequest {
  key: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Package Registry Types
// ─────────────────────────────────────────────────────────────────────────────

export type PackageType = 'docker' | 'npm' | 'maven' | 'pypi' | 'generic'
export type PackageStatus = 'default' | 'hidden' | 'pending_destruction'

export interface Package {
  id: number
  project_id: number
  name: string
  version: string
  package_type: PackageType
  status: PackageStatus
  size?: number
  created_at: string
  updated_at?: string
  created_by_id?: number
  created_by?: {
    id: number
    username: string
    avatar_url?: string
  }
  // 包含元数据 JSON（如 npm package.json 内容或 Docker manifest）
  metadata?: Record<string, any>
}

export interface PackageFile {
  id: number
  package_id: number
  file_name: string
  size: number
  file_sha256?: string
  file_sha512?: string
  created_at: string
  download_url?: string
}

export interface DockerManifest {
  id: number
  package_id: number
  digest: string
  media_type: string
  schema_version: number
  total_size: number
  created_at: string
  config_digest?: string
  architecture?: string
  os?: string
}

export interface NpmPackageMetadata {
  id: number
  package_id: number
  description?: string
  license?: string
  homepage?: string
  repository?: string
  keywords?: string[]
  author?: string
  dependencies?: Record<string, string>
  dev_dependencies?: Record<string, string>
  peer_dependencies?: Record<string, string>
}
