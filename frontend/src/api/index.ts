import axios, { type AxiosInstance } from 'axios'
import type {
  User,
  LoginRequest,
  LoginResponse,
  RegisterRequest,
  Project,
  CreateProjectRequest,
  ProjectStats,
  ProjectMember,
  RepositoryInfo,
  FileEntry,
  FileContent,
  CommitInfo,
  CommitDetail,
  BranchInfo,
  TagInfo,
  MergeRequest,
  CreateMergeRequestRequest,
  MergeRequestComment,
  MergeRequestReview,
  Pipeline,
  PipelineJob,
  Webhook,
  CreateWebhookRequest,
  Group,
  CreateGroupRequest,
  UpdateGroupRequest,
  GroupMember,
  AddGroupMemberRequest,
  NamespaceOption,
  SystemStats,
  AdminUserListResponse,
  AdminUserInfo,
  AdminUpdateUserRequest,
  // PAT types
  PersonalAccessToken,
  CreatePatRequest,
  CreatePatResponse,
  PatScopeInfo,
  // OAuth types
  OAuthProviderInfo,
  OAuthApplication,
  OAuthApplicationWithSecret,
  CreateOAuthApplicationRequest,
  UpdateOAuthApplicationRequest,
  OAuthIdentity,
  // Admin OAuth types
  OAuthProviderAdmin,
  CreateOAuthProviderRequest,
  UpdateOAuthProviderRequest,
} from '@/types'

// 的项目路径
type ProjectPath = {
  namespace: string
  project: string
}

class ApiClient {
  public client: AxiosInstance

  constructor() {
    this.client = axios.create({
      baseURL: '/api/v1',
      headers: {
        'Content-Type': 'application/json'
      }
    })
  }

  setAuthToken(token: string | null) {
    if (token) {
      this.client.defaults.headers.common['Authorization'] = `Bearer ${token}`
    } else {
      delete this.client.defaults.headers.common['Authorization']
    }
  }

  // Helper to build project path
  private projectPath(path: ProjectPath): string {
    return `/projects/${path.namespace}/${path.project}`
  }

  // Auth
  auth = {
    login: async (data: LoginRequest): Promise<LoginResponse> => {
      const response = await this.client.post('/auth/login', data)
      return response.data
    },
    register: async (data: RegisterRequest): Promise<User> => {
      const response = await this.client.post('/auth/register', data)
      return response.data
    },
    me: async (): Promise<User> => {
      const response = await this.client.get('/auth/me')
      return response.data
    },
    // Email confirmation
    confirmEmail: async (token: string): Promise<{ success: boolean; message: string; user: User }> => {
      const response = await this.client.post('/auth/confirm-email', { token })
      return response.data
    },
    resendConfirmation: async (): Promise<{ success: boolean; message: string }> => {
      const response = await this.client.post('/auth/resend-confirmation')
      return response.data
    },
    // Password reset
    forgotPassword: async (email: string): Promise<{ success: boolean; message: string }> => {
      const response = await this.client.post('/auth/forgot-password', { email })
      return response.data
    },
    verifyResetToken: async (token: string): Promise<{ valid: boolean; username: string }> => {
      const response = await this.client.post('/auth/verify-reset-token', { token })
      return response.data
    },
    resetPassword: async (token: string, new_password: string): Promise<{ success: boolean; message: string; user: User }> => {
      const response = await this.client.post('/auth/reset-password', { token, new_password })
      return response.data
    }
  }

  // Server config
  config = {
    get: async (): Promise<{ ssh_enabled: boolean; ssh_clone_url_prefix: string; http_clone_url_prefix: string }> => {
      const response = await this.client.get('/config')
      return response.data
    }
  }

  // Users
  users = {
    list: async (page = 1, perPage = 20): Promise<User[]> => {
      const response = await this.client.get('/users', { params: { page, per_page: perPage } })
      return response.data
    },
    get: async (username: string): Promise<User> => {
      const response = await this.client.get(`/users/${username}`)
      return response.data
    },
    getAvatarsByEmails: async (emails: string[]): Promise<{ email: string; avatar_url: string | null; display_name: string | null }[]> => {
      const response = await this.client.post('/users/avatars', { emails })
      return response.data
    }
  }

  // Projects - GET /projects/:namespace/:project
  projects = {
    list: async (page = 1, perPage = 20): Promise<Project[]> => {
      const response = await this.client.get('/projects', { params: { page, per_page: perPage } })
      return response.data
    },
    get: async (path: ProjectPath): Promise<Project> => {
      const response = await this.client.get(this.projectPath(path))
      return response.data
    },
    getStats: async (path: ProjectPath): Promise<ProjectStats> => {
      const response = await this.client.get(`${this.projectPath(path)}/stats`)
      return response.data
    },
    create: async (data: CreateProjectRequest): Promise<Project> => {
      const response = await this.client.post('/projects', data)
      return response.data
    },
    update: async (path: ProjectPath, data: Partial<CreateProjectRequest>): Promise<Project> => {
      const response = await this.client.put(this.projectPath(path), data)
      return response.data
    },
    delete: async (path: ProjectPath): Promise<void> => {
      await this.client.delete(this.projectPath(path))
    },
    getMembers: async (path: ProjectPath): Promise<ProjectMember[]> => {
      const response = await this.client.get(`${this.projectPath(path)}/members`)
      return response.data
    },
    addMember: async (path: ProjectPath, data: { username: string; role: string }): Promise<ProjectMember> => {
      const response = await this.client.post(`${this.projectPath(path)}/members`, data)
      return response.data
    },
    removeMember: async (path: ProjectPath, userId: string): Promise<void> => {
      await this.client.delete(`${this.projectPath(path)}/members/${userId}`)
    },
    // Star APIs
    checkStarred: async (path: ProjectPath): Promise<{ starred: boolean }> => {
      const response = await this.client.get(`${this.projectPath(path)}/starred`)
      return response.data
    },
    star: async (path: ProjectPath): Promise<{ starred: boolean; stars_count: number }> => {
      const response = await this.client.post(`${this.projectPath(path)}/star`)
      return response.data
    },
    unstar: async (path: ProjectPath): Promise<{ starred: boolean; stars_count: number }> => {
      const response = await this.client.delete(`${this.projectPath(path)}/star`)
      return response.data
    },
    // Fork APIs
    fork: async (path: ProjectPath, data?: { namespace_id?: number; name?: string; description?: string; visibility?: string; branches?: string }): Promise<Project> => {
      const response = await this.client.post(`${this.projectPath(path)}/fork`, data || {})
      return response.data
    },
    listForks: async (path: ProjectPath): Promise<{ forks_count: number; forks: Project[] }> => {
      const response = await this.client.get(`${this.projectPath(path)}/forks`)
      return response.data
    }
  }

  // Repository - 
  repository = {
    getInfo: async (path: ProjectPath): Promise<RepositoryInfo> => {
      const response = await this.client.get(`${this.projectPath(path)}/repository`)
      return response.data
    },
    browseTree: async (path: ProjectPath, treePath?: string, refName?: string): Promise<FileEntry[]> => {
      const response = await this.client.get(`${this.projectPath(path)}/repository/tree`, {
        params: { path: treePath, ref_name: refName }
      })
      return response.data
    },
    getFile: async (path: ProjectPath, filePath: string, refName?: string): Promise<FileContent> => {
      const response = await this.client.get(`${this.projectPath(path)}/repository/files/${encodeURIComponent(filePath)}`, {
        params: { ref_name: refName }
      })
      return response.data
    }
  }

  // Branches - 
  branches = {
    list: async (path: ProjectPath): Promise<BranchInfo[]> => {
      const response = await this.client.get(`${this.projectPath(path)}/repository/branches`)
      return response.data
    },
    create: async (path: ProjectPath, name: string, refName: string): Promise<void> => {
      await this.client.post(`${this.projectPath(path)}/repository/branches`, { name, ref_name: refName })
    },
    get: async (path: ProjectPath, branchName: string): Promise<BranchInfo> => {
      const response = await this.client.get(`${this.projectPath(path)}/repository/branches/${encodeURIComponent(branchName)}`)
      return response.data
    },
    delete: async (path: ProjectPath, name: string): Promise<void> => {
      await this.client.delete(`${this.projectPath(path)}/repository/branches/${encodeURIComponent(name)}`)
    }
  }

  // Commits - 
  commits = {
    list: async (path: ProjectPath, refName?: string, filePath?: string, page = 1, perPage = 20): Promise<CommitInfo[]> => {
      const response = await this.client.get(`${this.projectPath(path)}/repository/commits`, {
        params: { ref_name: refName, path: filePath, page, per_page: perPage }
      })
      return response.data
    },
    get: async (path: ProjectPath, sha: string): Promise<CommitDetail> => {
      const response = await this.client.get(`${this.projectPath(path)}/repository/commits/${sha}`)
      return response.data
    },
    compare: async (path: ProjectPath, from: string, to: string): Promise<CommitInfo[]> => {
      const response = await this.client.get(`${this.projectPath(path)}/repository/compare`, {
        params: { from, to }
      })
      return response.data
    }
  }

  // Tags - 
  tags = {
    list: async (path: ProjectPath): Promise<TagInfo[]> => {
      const response = await this.client.get(`${this.projectPath(path)}/repository/tags`)
      return response.data
    },
    create: async (path: ProjectPath, name: string, refName: string, message?: string): Promise<void> => {
      await this.client.post(`${this.projectPath(path)}/repository/tags`, { name, ref_name: refName, message })
    },
    get: async (path: ProjectPath, tagName: string): Promise<TagInfo> => {
      const response = await this.client.get(`${this.projectPath(path)}/repository/tags/${encodeURIComponent(tagName)}`)
      return response.data
    },
    delete: async (path: ProjectPath, name: string): Promise<void> => {
      await this.client.delete(`${this.projectPath(path)}/repository/tags/${encodeURIComponent(name)}`)
    }
  }

  // Merge Requests - 
  mergeRequests = {
    list: async (path: ProjectPath, status?: string, page = 1, perPage = 20): Promise<MergeRequest[]> => {
      const response = await this.client.get(`${this.projectPath(path)}/merge_requests`, {
        params: { status, page, per_page: perPage }
      })
      return response.data
    },
    get: async (path: ProjectPath, iid: number): Promise<{ merge_request: MergeRequest; comments: MergeRequestComment[]; reviews: MergeRequestReview[]; can_merge: boolean; has_conflicts: boolean }> => {
      const response = await this.client.get(`${this.projectPath(path)}/merge_requests/${iid}`)
      return response.data
    },
    create: async (path: ProjectPath, data: CreateMergeRequestRequest): Promise<MergeRequest> => {
      const response = await this.client.post(`${this.projectPath(path)}/merge_requests`, data)
      return response.data
    },
    update: async (path: ProjectPath, iid: number, data: Partial<CreateMergeRequestRequest>): Promise<MergeRequest> => {
      const response = await this.client.put(`${this.projectPath(path)}/merge_requests/${iid}`, data)
      return response.data
    },
    merge: async (path: ProjectPath, iid: number, options?: { squash?: boolean; delete_source_branch?: boolean }): Promise<MergeRequest> => {
      const response = await this.client.put(`${this.projectPath(path)}/merge_requests/${iid}/merge`, options)
      return response.data
    },
    addComment: async (path: ProjectPath, iid: number, content: string, filePath?: string, lineNumber?: number): Promise<MergeRequestComment> => {
      const response = await this.client.post(`${this.projectPath(path)}/merge_requests/${iid}/notes`, {
        content, file_path: filePath, line_number: lineNumber
      })
      return response.data
    }
  }

  // Pipelines - 
  pipelines = {
    list: async (path: ProjectPath, status?: string, page = 1, perPage = 20): Promise<Pipeline[]> => {
      const response = await this.client.get(`${this.projectPath(path)}/pipelines`, {
        params: { status, page, per_page: perPage }
      })
      return response.data
    },
    get: async (path: ProjectPath, id: string): Promise<{ pipeline: Pipeline; jobs: PipelineJob[] }> => {
      const response = await this.client.get(`${this.projectPath(path)}/pipelines/${id}`)
      return response.data
    },
    trigger: async (path: ProjectPath, refName: string): Promise<Pipeline> => {
      const response = await this.client.post(`${this.projectPath(path)}/pipelines`, { ref_name: refName })
      return response.data
    },
    cancel: async (path: ProjectPath, id: string): Promise<Pipeline> => {
      const response = await this.client.post(`${this.projectPath(path)}/pipelines/${id}/cancel`)
      return response.data
    },
    retry: async (path: ProjectPath, id: string): Promise<Pipeline> => {
      const response = await this.client.post(`${this.projectPath(path)}/pipelines/${id}/retry`)
      return response.data
    },
    getJobLog: async (path: ProjectPath, pipelineId: string, jobId: string): Promise<{ job_id: string; log: string }> => {
      const response = await this.client.get(`${this.projectPath(path)}/pipelines/${pipelineId}/jobs/${jobId}/log`)
      return response.data
    }
  }

  // Webhooks - 
  webhooks = {
    list: async (path: ProjectPath): Promise<Webhook[]> => {
      const response = await this.client.get(`${this.projectPath(path)}/webhooks`)
      return response.data
    },
    create: async (path: ProjectPath, data: CreateWebhookRequest): Promise<Webhook> => {
      const response = await this.client.post(`${this.projectPath(path)}/webhooks`, data)
      return response.data
    },
    update: async (path: ProjectPath, id: string, data: Partial<CreateWebhookRequest>): Promise<Webhook> => {
      const response = await this.client.put(`${this.projectPath(path)}/webhooks/${id}`, data)
      return response.data
    },
    delete: async (path: ProjectPath, id: string): Promise<void> => {
      await this.client.delete(`${this.projectPath(path)}/webhooks/${id}`)
    },
    test: async (path: ProjectPath, id: string): Promise<{ message: string; delivery_id: string }> => {
      const response = await this.client.post(`${this.projectPath(path)}/webhooks/${id}/test`)
      return response.data
    }
  }

  // Namespaces - for project creation
  namespaces = {
    /** Get namespaces where the current user can create projects */
    listForProjectCreation: async (): Promise<NamespaceOption[]> => {
      const response = await this.client.get('/namespaces/for-project-creation')
      return response.data
    }
  }

  // Groups
  groups = {
    list: async (): Promise<Group[]> => {
      const response = await this.client.get('/groups')
      return response.data
    },
    create: async (data: CreateGroupRequest): Promise<Group> => {
      const response = await this.client.post('/groups', data)
      return response.data
    },
    get: async (path: string): Promise<Group> => {
      const response = await this.client.get(`/groups/${path}`)
      return response.data
    },
    update: async (path: string, data: UpdateGroupRequest): Promise<Group> => {
      const response = await this.client.put(`/groups/${path}`, data)
      return response.data
    },
    delete: async (path: string): Promise<void> => {
      await this.client.delete(`/groups/${path}`)
    },
    // Group members
    listMembers: async (path: string): Promise<GroupMember[]> => {
      const response = await this.client.get(`/groups/${path}/members`)
      return response.data
    },
    addMember: async (path: string, data: AddGroupMemberRequest): Promise<GroupMember> => {
      const response = await this.client.post(`/groups/${path}/members`, data)
      return response.data
    },
    removeMember: async (path: string, userId: string): Promise<void> => {
      await this.client.delete(`/groups/${path}/members/${userId}`)
    },
    // Group projects
    listProjects: async (path: string): Promise<Project[]> => {
      const response = await this.client.get(`/groups/${path}/projects`)
      return response.data
    },
    // Subgroups
    listSubgroups: async (path: string): Promise<Group[]> => {
      const response = await this.client.get(`/groups/${path}/subgroups`)
      return response.data
    },
  }

  // Admin
  admin = {
    getDashboard: async (): Promise<SystemStats> => {
      const response = await this.client.get('/admin/dashboard')
      return response.data
    },
    listUsers: async (params?: { page?: number; per_page?: number; search?: string; role?: string; status?: string }): Promise<AdminUserListResponse> => {
      const response = await this.client.get('/admin/users', { params })
      return response.data
    },
    getUser: async (id: string): Promise<AdminUserInfo> => {
      const response = await this.client.get(`/admin/users/${id}`)
      return response.data
    },
    updateUser: async (id: string, data: AdminUpdateUserRequest): Promise<User> => {
      const response = await this.client.put(`/admin/users/${id}`, data)
      return response.data
    },
    deleteUser: async (id: string): Promise<void> => {
      await this.client.delete(`/admin/users/${id}`)
    },
    getConfigs: async (): Promise<Record<string, any>> => {
      const response = await this.client.get('/admin/settings/configs')
      return response.data
    },
    updateConfigs: async (configs: Array<{ key: string; value: any }>): Promise<Record<string, any>> => {
      const response = await this.client.put('/admin/settings/configs', { configs })
      return response.data
    },
    // SMTP settings
    getSmtpConfig: async (): Promise<{
      configured: boolean
      enabled: boolean
      host: string
      port: number
      from_email: string
      from_name: string
      use_tls: boolean
      use_ssl: boolean
    }> => {
      const response = await this.client.get('/admin/settings/smtp')
      return response.data
    },
    testSmtpConnection: async (settings: {
      enabled: boolean
      host: string
      port: number
      username: string
      password: string
      from_email: string
      from_name: string
      use_tls: boolean
      use_ssl: boolean
    }): Promise<{ success: boolean; message: string }> => {
      const response = await this.client.post('/admin/settings/smtp/test', settings)
      return response.data
    },
    sendTestEmail: async (settings: {
      enabled: boolean
      host: string
      port: number
      username: string
      password: string
      from_email: string
      from_name: string
      use_tls: boolean
      use_ssl: boolean
      test_email: string
    }): Promise<{ success: boolean; message: string }> => {
      const response = await this.client.post('/admin/settings/smtp/send-test', settings)
      return response.data
    },
  }

  // Personal Access Tokens
  accessTokens = {
    list: async (): Promise<PersonalAccessToken[]> => {
      const response = await this.client.get('/user/access_tokens')
      return response.data
    },
    create: async (data: CreatePatRequest): Promise<CreatePatResponse> => {
      const response = await this.client.post('/user/access_tokens', data)
      return response.data
    },
    get: async (id: number): Promise<PersonalAccessToken> => {
      const response = await this.client.get(`/user/access_tokens/${id}`)
      return response.data
    },
    revoke: async (id: number): Promise<void> => {
      await this.client.delete(`/user/access_tokens/${id}`)
    },
    getScopes: async (): Promise<PatScopeInfo[]> => {
      const response = await this.client.get('/user/access_tokens/scopes')
      return response.data
    },
  }

  // OAuth Applications (GitFox as OAuth Provider)
  oauthApplications = {
    list: async (): Promise<OAuthApplication[]> => {
      const response = await this.client.get('/oauth/applications')
      return response.data
    },
    create: async (data: CreateOAuthApplicationRequest): Promise<OAuthApplicationWithSecret> => {
      const response = await this.client.post('/oauth/applications', data)
      return response.data
    },
    get: async (id: number): Promise<OAuthApplication> => {
      const response = await this.client.get(`/oauth/applications/${id}`)
      return response.data
    },
    update: async (id: number, data: UpdateOAuthApplicationRequest): Promise<OAuthApplication> => {
      const response = await this.client.put(`/oauth/applications/${id}`, data)
      return response.data
    },
    delete: async (id: number): Promise<void> => {
      await this.client.delete(`/oauth/applications/${id}`)
    },
    regenerateSecret: async (id: number): Promise<OAuthApplicationWithSecret> => {
      const response = await this.client.post(`/oauth/applications/${id}/regenerate_secret`)
      return response.data
    },
  }

  // OAuth Providers (Social Login)
  oauth = {
    getProviders: async (): Promise<OAuthProviderInfo[]> => {
      const response = await this.client.get('/oauth/providers')
      return response.data.providers
    },
    getAuthorizeUrl: (provider: string): string => {
      return `/api/v1/oauth/${provider}/authorize`
    },
  }

  // OAuth Identities (Linked Social Accounts)
  identities = {
    list: async (): Promise<OAuthIdentity[]> => {
      const response = await this.client.get('/user/identities')
      return response.data
    },
    unlink: async (id: number): Promise<void> => {
      await this.client.delete(`/user/identities/${id}`)
    },
  }

  // Admin OAuth Providers Management
  adminOAuth = {
    listProviders: async (): Promise<OAuthProviderAdmin[]> => {
      const response = await this.client.get('/admin/oauth/providers')
      return response.data
    },
    getProvider: async (id: number): Promise<OAuthProviderAdmin> => {
      const response = await this.client.get(`/admin/oauth/providers/${id}`)
      return response.data
    },
    createProvider: async (data: CreateOAuthProviderRequest): Promise<OAuthProviderAdmin> => {
      const response = await this.client.post('/admin/oauth/providers', data)
      return response.data
    },
    updateProvider: async (id: number, data: UpdateOAuthProviderRequest): Promise<OAuthProviderAdmin> => {
      const response = await this.client.put(`/admin/oauth/providers/${id}`, data)
      return response.data
    },
    deleteProvider: async (id: number): Promise<void> => {
      await this.client.delete(`/admin/oauth/providers/${id}`)
    },
  }
}

const apiClient = new ApiClient()

// Named export for convenience
export const api = apiClient

export default apiClient
