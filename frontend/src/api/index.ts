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
}

const apiClient = new ApiClient()

// Named export for convenience
export const api = apiClient

export default apiClient
