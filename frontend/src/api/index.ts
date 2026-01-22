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
  CreateWebhookRequest
} from '@/types'

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

  // Users
  users = {
    list: async (page = 1, perPage = 20): Promise<User[]> => {
      const response = await this.client.get('/users', { params: { page, per_page: perPage } })
      return response.data
    },
    get: async (id: string): Promise<User> => {
      const response = await this.client.get(`/users/${id}`)
      return response.data
    }
  }

  // Projects
  projects = {
    list: async (page = 1, perPage = 20): Promise<Project[]> => {
      const response = await this.client.get('/projects', { params: { page, per_page: perPage } })
      return response.data
    },
    get: async (slug: string): Promise<Project> => {
      const response = await this.client.get(`/projects/${slug}`)
      return response.data
    },
    create: async (data: CreateProjectRequest): Promise<Project> => {
      const response = await this.client.post('/projects', data)
      return response.data
    },
    update: async (slug: string, data: Partial<CreateProjectRequest>): Promise<Project> => {
      const response = await this.client.put(`/projects/${slug}`, data)
      return response.data
    },
    delete: async (slug: string): Promise<void> => {
      await this.client.delete(`/projects/${slug}`)
    },
    getStats: async (slug: string): Promise<ProjectStats> => {
      const response = await this.client.get(`/projects/${slug}/stats`)
      return response.data
    },
    getMembers: async (slug: string): Promise<ProjectMember[]> => {
      const response = await this.client.get(`/projects/${slug}/members`)
      return response.data
    }
  }

  // Repository
  repository = {
    getInfo: async (slug: string): Promise<RepositoryInfo> => {
      const response = await this.client.get(`/projects/${slug}/repository`)
      return response.data
    },
    browseTree: async (slug: string, path?: string, refName?: string): Promise<FileEntry[]> => {
      const response = await this.client.get(`/projects/${slug}/repository/tree`, {
        params: { path, ref_name: refName }
      })
      return response.data
    },
    getFile: async (slug: string, path: string, refName?: string): Promise<FileContent> => {
      const response = await this.client.get(`/projects/${slug}/repository/files`, {
        params: { path, ref_name: refName }
      })
      return response.data
    }
  }

  // Branches
  branches = {
    list: async (slug: string): Promise<BranchInfo[]> => {
      const response = await this.client.get(`/projects/${slug}/branches`)
      return response.data
    },
    create: async (slug: string, name: string, refName: string): Promise<void> => {
      await this.client.post(`/projects/${slug}/branches`, { name, ref_name: refName })
    },
    delete: async (slug: string, name: string): Promise<void> => {
      await this.client.delete(`/projects/${slug}/branches/${name}`)
    }
  }

  // Commits
  commits = {
    list: async (slug: string, refName?: string, path?: string, page = 1, perPage = 20): Promise<CommitInfo[]> => {
      const response = await this.client.get(`/projects/${slug}/commits`, {
        params: { ref_name: refName, path, page, per_page: perPage }
      })
      return response.data
    },
    get: async (slug: string, sha: string): Promise<CommitDetail> => {
      const response = await this.client.get(`/projects/${slug}/commits/${sha}`)
      return response.data
    },
    compare: async (slug: string, from: string, to: string): Promise<CommitInfo[]> => {
      const response = await this.client.get(`/projects/${slug}/compare`, {
        params: { from, to }
      })
      return response.data
    }
  }

  // Tags
  tags = {
    list: async (slug: string): Promise<TagInfo[]> => {
      const response = await this.client.get(`/projects/${slug}/tags`)
      return response.data
    },
    create: async (slug: string, name: string, refName: string, message?: string): Promise<void> => {
      await this.client.post(`/projects/${slug}/tags`, { name, ref_name: refName, message })
    },
    delete: async (slug: string, name: string): Promise<void> => {
      await this.client.delete(`/projects/${slug}/tags/${name}`)
    }
  }

  // Merge Requests
  mergeRequests = {
    list: async (slug: string, status?: string, page = 1, perPage = 20): Promise<MergeRequest[]> => {
      const response = await this.client.get(`/projects/${slug}/merge-requests`, {
        params: { status, page, per_page: perPage }
      })
      return response.data
    },
    get: async (slug: string, iid: number): Promise<{ merge_request: MergeRequest; comments: MergeRequestComment[]; reviews: MergeRequestReview[]; can_merge: boolean; has_conflicts: boolean }> => {
      const response = await this.client.get(`/projects/${slug}/merge-requests/${iid}`)
      return response.data
    },
    create: async (slug: string, data: CreateMergeRequestRequest): Promise<MergeRequest> => {
      const response = await this.client.post(`/projects/${slug}/merge-requests`, data)
      return response.data
    },
    update: async (slug: string, iid: number, data: Partial<CreateMergeRequestRequest>): Promise<MergeRequest> => {
      const response = await this.client.put(`/projects/${slug}/merge-requests/${iid}`, data)
      return response.data
    },
    merge: async (slug: string, iid: number, options?: { squash?: boolean; delete_source_branch?: boolean }): Promise<MergeRequest> => {
      const response = await this.client.post(`/projects/${slug}/merge-requests/${iid}/merge`, options)
      return response.data
    },
    close: async (slug: string, iid: number): Promise<MergeRequest> => {
      const response = await this.client.post(`/projects/${slug}/merge-requests/${iid}/close`)
      return response.data
    },
    addComment: async (slug: string, iid: number, content: string, filePath?: string, lineNumber?: number): Promise<MergeRequestComment> => {
      const response = await this.client.post(`/projects/${slug}/merge-requests/${iid}/comments`, {
        content, file_path: filePath, line_number: lineNumber
      })
      return response.data
    },
    addReview: async (slug: string, iid: number, status: string, comment?: string): Promise<MergeRequestReview> => {
      const response = await this.client.post(`/projects/${slug}/merge-requests/${iid}/reviews`, { status, comment })
      return response.data
    }
  }

  // Pipelines
  pipelines = {
    list: async (slug: string, status?: string, page = 1, perPage = 20): Promise<Pipeline[]> => {
      const response = await this.client.get(`/projects/${slug}/pipelines`, {
        params: { status, page, per_page: perPage }
      })
      return response.data
    },
    get: async (slug: string, id: string): Promise<{ pipeline: Pipeline; jobs: PipelineJob[] }> => {
      const response = await this.client.get(`/projects/${slug}/pipelines/${id}`)
      return response.data
    },
    trigger: async (slug: string, refName: string): Promise<Pipeline> => {
      const response = await this.client.post(`/projects/${slug}/pipelines`, { ref_name: refName })
      return response.data
    },
    cancel: async (slug: string, id: string): Promise<Pipeline> => {
      const response = await this.client.post(`/projects/${slug}/pipelines/${id}/cancel`)
      return response.data
    },
    retry: async (slug: string, id: string): Promise<Pipeline> => {
      const response = await this.client.post(`/projects/${slug}/pipelines/${id}/retry`)
      return response.data
    },
    getJobLog: async (slug: string, pipelineId: string, jobId: string): Promise<{ job_id: string; log: string }> => {
      const response = await this.client.get(`/projects/${slug}/pipelines/${pipelineId}/jobs/${jobId}/log`)
      return response.data
    }
  }

  // Webhooks
  webhooks = {
    list: async (slug: string): Promise<Webhook[]> => {
      const response = await this.client.get(`/projects/${slug}/webhooks`)
      return response.data
    },
    create: async (slug: string, data: CreateWebhookRequest): Promise<Webhook> => {
      const response = await this.client.post(`/projects/${slug}/webhooks`, data)
      return response.data
    },
    update: async (slug: string, id: string, data: Partial<CreateWebhookRequest>): Promise<Webhook> => {
      const response = await this.client.put(`/projects/${slug}/webhooks/${id}`, data)
      return response.data
    },
    delete: async (slug: string, id: string): Promise<void> => {
      await this.client.delete(`/projects/${slug}/webhooks/${id}`)
    },
    test: async (slug: string, id: string): Promise<{ message: string; delivery_id: string }> => {
      const response = await this.client.post(`/projects/${slug}/webhooks/${id}/test`)
      return response.data
    }
  }
}

const apiClient = new ApiClient()

// 导出便捷方法供组件使用
export const api = {
  ...apiClient.auth,
  ...apiClient.users,
  // Projects
  getProjects: apiClient.projects.list,
  getProject: apiClient.projects.get,
  createProject: apiClient.projects.create,
  updateProject: (id: string, data: Partial<CreateProjectRequest>) => apiClient.projects.update(id, data),
  deleteProject: apiClient.projects.delete,
  getProjectStats: apiClient.projects.getStats,
  getProjectMembers: apiClient.projects.getMembers,
  addProjectMember: (slug: string, data: { username: string; role: string }) => 
    apiClient.client.post(`/projects/${slug}/members`, data),
  updateProjectMember: (slug: string, userId: string, data: { role: string }) => 
    apiClient.client.put(`/projects/${slug}/members/${userId}`, data),
  removeProjectMember: (slug: string, userId: string) => 
    apiClient.client.delete(`/projects/${slug}/members/${userId}`),
  
  // Repository
  getRepositoryInfo: apiClient.repository.getInfo,
  getTree: (id: string, params: { ref?: string; path?: string }) => 
    apiClient.repository.browseTree(id, params.path, params.ref),
  getFileContent: (id: string, ref: string, path: string) => 
    apiClient.repository.getFile(id, path, ref),
  
  // Branches
  getBranches: apiClient.branches.list,
  createBranch: (id: string, data: { name: string; source_branch: string }) => 
    apiClient.branches.create(id, data.name, data.source_branch),
  deleteBranch: apiClient.branches.delete,
  
  // Commits  
  getCommits: (id: string, params: { ref?: string; page?: number; per_page?: number }) => 
    apiClient.commits.list(id, params.ref, undefined, params.page, params.per_page),
  getCommit: apiClient.commits.get,
  getDiff: (id: string, sha: string) => apiClient.commits.get(id, sha).then(r => r.diffs || []),
  
  // Tags
  getTags: apiClient.tags.list,
  createTag: (id: string, data: { name: string; ref: string; message?: string }) => 
    apiClient.tags.create(id, data.name, data.ref, data.message),
  deleteTag: apiClient.tags.delete,
  
  // Merge Requests
  getMergeRequests: apiClient.mergeRequests.list,
  getMergeRequest: apiClient.mergeRequests.get,
  createMergeRequest: apiClient.mergeRequests.create,
  getMergeRequestComments: (id: string, iid: number) => 
    apiClient.mergeRequests.get(id, iid).then(r => r.comments),
  getMergeRequestDiff: (id: string, iid: number) => 
    apiClient.client.get(`/projects/${id}/merge-requests/${iid}/diff`),
  createMergeRequestComment: (id: string, iid: number, data: { content: string }) => 
    apiClient.mergeRequests.addComment(id, iid, data.content),
  mergeMergeRequest: (id: string, iid: number) => apiClient.mergeRequests.merge(id, iid),
  closeMergeRequest: (id: string, iid: number) => apiClient.mergeRequests.close(id, iid),
  
  // Pipelines
  getPipelines: apiClient.pipelines.list,
  getPipeline: apiClient.pipelines.get,
  getPipelineJobs: (id: string, pipelineId: string) => 
    apiClient.pipelines.get(id, pipelineId).then(r => r.jobs),
  getPipelineJobLogs: (id: string, pipelineId: string, jobId: string) =>
    apiClient.pipelines.getJobLog(id, pipelineId, jobId).then(r => r.log),
  triggerPipeline: (id: string, data: { ref: string }) => 
    apiClient.pipelines.trigger(id, data.ref),
  cancelPipeline: apiClient.pipelines.cancel,
  retryPipeline: apiClient.pipelines.retry,
  
  // Webhooks
  getWebhooks: apiClient.webhooks.list,
  createWebhook: apiClient.webhooks.create,
  updateWebhook: (id: string, webhookId: string, data: { is_active?: boolean }) => 
    apiClient.webhooks.update(id, webhookId, data as any),
  deleteWebhook: apiClient.webhooks.delete,
}

export default apiClient
