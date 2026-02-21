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
    const params: any = { path, recursive: true }
    if (ref && ref.length > 0) params.ref_name = ref
    const res = await http.get(`/projects/${owner}/${repo}/repository/tree`, { params })
    return buildTreeFromList(res.data)
  },

  // Get immediate children of a directory (non-recursive)
  async getFileTreeChildren(owner: string, repo: string, ref: string, path = ''): Promise<any[]> {
    const params: any = { path, recursive: false }
    if (ref && ref.length > 0) params.ref_name = ref
    const res = await http.get(`/projects/${owner}/${repo}/repository/tree`, { params })
    return res.data
  },

  async getFileContent(owner: string, repo: string, path: string, ref: string): Promise<string> {
    const cleanPath = path.replace(/^\/+/, '')
    const params: any = {}
    if (ref && ref.length > 0) params.ref_name = ref
    const res = await http.get(`/projects/${owner}/${repo}/repository/files/${encodeURIComponent(cleanPath)}`, { params })
    // Decode base64 content
    if (res.data.encoding === 'base64') {
      return atob(res.data.content)
    }
    return res.data.content
  },

  async updateFile(owner: string, repo: string, path: string, content: string, branch: string, message?: string): Promise<void> {
    const cleanPath = path.replace(/^\/+/, '')
    await http.put(`/projects/${owner}/${repo}/repository/files/${encodeURIComponent(cleanPath)}`, {
      branch,
      content,
      commit_message: message || `Update ${cleanPath}`
    })
  },

  async createFile(owner: string, repo: string, path: string, content: string, branch: string, message?: string): Promise<void> {
    const cleanPath = path.replace(/^\/+/, '')
    await http.post(`/projects/${owner}/${repo}/repository/files/${encodeURIComponent(cleanPath)}`, {
      branch,
      content,
      commit_message: message || `Create ${cleanPath}`
    })
  },

  async deleteFile(owner: string, repo: string, path: string, branch: string, message?: string): Promise<void> {
    const cleanPath = path.replace(/^\/+/, '')
    await http.delete(`/projects/${owner}/${repo}/repository/files/${encodeURIComponent(cleanPath)}`, {
      data: {
        branch,
        commit_message: message || `Delete ${cleanPath}`
      }
    })
  },

  async batchCommit(owner: string, repo: string, commit: CommitRequest): Promise<void> {
    // Server expects /repository/commits/batch for batch commit operations
    await http.post(`/projects/${owner}/${repo}/repository/commits/batch`, commit)
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
function buildTreeFromList(items: Array<any>): FileTreeNode[] {
  const root: FileTreeNode[] = []
  const pathMap = new Map<string, FileTreeNode>()

  // Helper to determine if an item represents a directory
  const isDirectory = (it: any) => {
    if (!it) return false
    const t = (it.type || it.entry_type || it.entryType || '').toString().toLowerCase()
    return t === 'tree' || t === 'directory'
  }

  // Sort so directories come before files, then alphabetically
  items.sort((a: any, b: any) => {
    const aDir = isDirectory(a)
    const bDir = isDirectory(b)
    if (aDir !== bDir) return aDir ? -1 : 1
    const aPath = (a.path || '').toString()
    const bPath = (b.path || '').toString()
    return aPath.localeCompare(bPath)
  })

  for (const item of items) {
    // Normalize path: remove leading slashes to make client-side tree logic consistent
    const rawPath = (item.path || item.path_name || item.path_name_raw || '').toString()
    const cleanPath = rawPath.replace(/^\/+/, '')
    const parts = cleanPath ? cleanPath.split('/') : ['']
    const name = parts[parts.length - 1] || ''

    const node: FileTreeNode = {
      name,
      path: cleanPath,
      type: isDirectory(item) ? 'directory' : 'file',
      size: item.size,
      mode: item.mode
    }

    if (node.type === 'directory') {
      node.children = []
    }

    pathMap.set(cleanPath, node)

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
