import axios from 'axios'
import type { FileTreeNode, Repository, Branch, CommitRequest } from '@/types'

const http = axios.create({
  baseURL: '/api/v1',
  timeout: 30000,
  headers: {
    'Content-Type': 'application/json'
  }
})

// Add auth token to requests
http.interceptors.request.use((config) => {
  const token = localStorage.getItem('token')
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

// API Client
export const api = {
  // Repository operations
  async getRepository(owner: string, repo: string): Promise<Repository> {
    const res = await http.get(`/projects/${owner}/${repo}`)
    return res.data
  },

  async getBranches(owner: string, repo: string): Promise<Branch[]> {
    const res = await http.get(`/projects/${owner}/${repo}/repository/branches`)
    return res.data
  },

  // File operations
  async getFileTree(owner: string, repo: string, ref: string, path = ''): Promise<FileTreeNode[]> {
    const res = await http.get(`/projects/${owner}/${repo}/repository/tree`, {
      params: { ref_name: ref, path, recursive: true }
    })
    return buildTreeFromList(res.data)
  },

  async getFileContent(owner: string, repo: string, path: string, ref: string): Promise<string> {
    const res = await http.get(`/projects/${owner}/${repo}/repository/files/${encodeURIComponent(path)}`, {
      params: { ref_name: ref }
    })
    // Decode base64 content
    if (res.data.encoding === 'base64') {
      return atob(res.data.content)
    }
    return res.data.content
  },

  async updateFile(owner: string, repo: string, path: string, content: string, branch: string, message?: string): Promise<void> {
    await http.put(`/projects/${owner}/${repo}/repository/files/${encodeURIComponent(path)}`, {
      branch,
      content,
      commit_message: message || `Update ${path}`
    })
  },

  async createFile(owner: string, repo: string, path: string, content: string, branch: string, message?: string): Promise<void> {
    await http.post(`/projects/${owner}/${repo}/repository/files/${encodeURIComponent(path)}`, {
      branch,
      content,
      commit_message: message || `Create ${path}`
    })
  },

  async deleteFile(owner: string, repo: string, path: string, branch: string, message?: string): Promise<void> {
    await http.delete(`/projects/${owner}/${repo}/repository/files/${encodeURIComponent(path)}`, {
      data: {
        branch,
        commit_message: message || `Delete ${path}`
      }
    })
  },

  async batchCommit(owner: string, repo: string, commit: CommitRequest): Promise<void> {
    await http.post(`/projects/${owner}/${repo}/repository/commits`, commit)
  },

  // Extension management (user-level isolation)
  async getUserExtensions(userId: string): Promise<string[]> {
    const stored = localStorage.getItem(`gitfox-extensions-${userId}`)
    return stored ? JSON.parse(stored) : []
  },

  async saveUserExtensions(userId: string, extensions: string[]): Promise<void> {
    localStorage.setItem(`gitfox-extensions-${userId}`, JSON.stringify(extensions))
  },

  async getExtensionSettings(userId: string, extensionId: string): Promise<Record<string, unknown>> {
    const stored = localStorage.getItem(`gitfox-ext-settings-${userId}-${extensionId}`)
    return stored ? JSON.parse(stored) : {}
  },

  async saveExtensionSettings(userId: string, extensionId: string, settings: Record<string, unknown>): Promise<void> {
    localStorage.setItem(`gitfox-ext-settings-${userId}-${extensionId}`, JSON.stringify(settings))
  }
}

// Helper: Convert flat file list to tree structure
function buildTreeFromList(items: Array<{ path: string; type: string; mode?: string; size?: number }>): FileTreeNode[] {
  const root: FileTreeNode[] = []
  const pathMap = new Map<string, FileTreeNode>()

  // Sort so directories come before files, then alphabetically
  items.sort((a, b) => {
    if (a.type !== b.type) {
      return a.type === 'tree' ? -1 : 1
    }
    return a.path.localeCompare(b.path)
  })

  for (const item of items) {
    const parts = item.path.split('/')
    const name = parts[parts.length - 1]
    
    const node: FileTreeNode = {
      name,
      path: item.path,
      type: item.type === 'tree' ? 'directory' : 'file',
      size: item.size,
      mode: item.mode
    }

    if (node.type === 'directory') {
      node.children = []
    }

    pathMap.set(item.path, node)

    if (parts.length === 1) {
      root.push(node)
    } else {
      const parentPath = parts.slice(0, -1).join('/')
      const parent = pathMap.get(parentPath)
      if (parent && parent.children) {
        parent.children.push(node)
      }
    }
  }

  return root
}

export default api
