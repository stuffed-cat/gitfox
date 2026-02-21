import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { FileTreeNode, OpenFile } from '@/types'
import { api } from '@/api'

export const useEditorStore = defineStore('editor', () => {
  // File tree state
  const fileTree = ref<FileTreeNode[]>([])
  const expandedFolders = ref<Set<string>>(new Set())
  
  // Open files state
  const openFiles = ref<Map<string, OpenFile>>(new Map())
  const activeFilePath = ref<string | null>(null)
  
  // Repository info
  const owner = ref('')
  const repo = ref('')
  // Default to 'master' to match existing repositories
  const currentRef = ref('master')
  
  // Computed
  const activeFile = computed(() => {
    if (!activeFilePath.value) return null
    return openFiles.value.get(activeFilePath.value) || null
  })
  
  const openFilesArray = computed(() => Array.from(openFiles.value.values()))
  
  const hasUnsavedChanges = computed(() => {
    return openFilesArray.value.some(f => f.modified)
  })
  
  const unsavedFiles = computed(() => {
    return openFilesArray.value.filter(f => f.modified)
  })

  // Actions
  async function loadRepository(ownerName: string, repoName: string, refName?: string) {
    owner.value = ownerName
    repo.value = repoName
    if (refName) {
      currentRef.value = refName
    }
    
    try {
      const tree = await api.getFileTree(ownerName, repoName, currentRef.value)
      fileTree.value = tree
    } catch (error) {
      console.error('Failed to load repository:', error)
      throw error
    }
  }

  async function openFile(path: string) {
    // Check if already open
    if (openFiles.value.has(path)) {
      activeFilePath.value = path
      return
    }
    
    try {
      const content = await api.getFileContent(owner.value, repo.value, path, currentRef.value)
      const fileName = path.split('/').pop() || path
      
      openFiles.value.set(path, {
        path,
        name: fileName,
        content,
        originalContent: content,
        language: getLanguageFromPath(path),
        modified: false
      })
      
      activeFilePath.value = path
    } catch (error) {
      console.error('Failed to open file:', error)
      throw error
    }
  }

  function closeFile(path: string) {
    openFiles.value.delete(path)
    
    if (activeFilePath.value === path) {
      // Switch to another open file
      const remaining = Array.from(openFiles.value.keys())
      activeFilePath.value = remaining.length > 0 ? remaining[remaining.length - 1] : null
    }
  }

  function updateFileContent(path: string, content: string) {
    const file = openFiles.value.get(path)
    if (file) {
      file.content = content
      file.modified = content !== file.originalContent
    }
  }

  async function saveFile(path: string) {
    const file = openFiles.value.get(path)
    if (!file || !file.modified) return
    
    try {
      await api.updateFile(owner.value, repo.value, path, file.content, currentRef.value)
      file.originalContent = file.content
      file.modified = false
    } catch (error) {
      console.error('Failed to save file:', error)
      throw error
    }
  }

  async function saveAllFiles() {
    const modified = unsavedFiles.value
    for (const file of modified) {
      await saveFile(file.path)
    }
  }

  function toggleFolder(path: string) {
    // Replace the Set with a new Set to ensure Vue reactivity notices the change
    const next = new Set(expandedFolders.value)
    const opening = !next.has(path)
    if (next.has(path)) {
      next.delete(path)
    } else {
      next.add(path)
    }
    expandedFolders.value = next

    // If opening a folder and it has no children yet, fetch its immediate children
    if (opening) {
      // find node in current fileTree
      const node = findNode(path, fileTree.value)
      if (node && (!node.children || node.children.length === 0)) {
        // fetch children (non-recursive)
        api.getFileTreeChildren(owner.value, repo.value, currentRef.value, path)
          .then(items => {
            const children = items.map((it: any) => {
              const raw = (it.path || it.path_name || '').toString()
              const clean = raw.replace(/^\/+/, '')
              const parts = clean.split('/')
              const name = parts[parts.length - 1]
              const isDir = (it.type || it.entry_type || it.entryType || '').toString().toLowerCase() === 'directory' || (it.type || it.entry_type || it.entryType || '').toString().toLowerCase() === 'tree'
              const child: FileTreeNode = {
                name,
                path: clean,
                type: isDir ? 'directory' : 'file',
                size: it.size,
                mode: it.mode
              }
              if (isDir) child.children = []
              return child
            })
            // assign children reactively
            if (node) node.children = children
          })
          .catch(err => {
            console.error('Failed to load folder children:', err)
          })
      }
    }
  }

  // Find a node by path (depth-first)
  function findNode(path: string, nodes: FileTreeNode[]): FileTreeNode | null {
    for (const n of nodes) {
      if (n.path === path) return n
      if (n.children && n.children.length > 0) {
        const found = findNode(path, n.children)
        if (found) return found
      }
    }
    return null
  }

  function setActiveFile(path: string | null) {
    activeFilePath.value = path
  }

  return {
    // State
    fileTree,
    expandedFolders,
    openFiles,
    activeFilePath,
    owner,
    repo,
    currentRef,
    
    // Computed
    activeFile,
    openFilesArray,
    hasUnsavedChanges,
    unsavedFiles,
    
    // Actions
    loadRepository,
    openFile,
    closeFile,
    updateFileContent,
    saveFile,
    saveAllFiles,
    toggleFolder,
    setActiveFile
  }
})

// Helper function
function getLanguageFromPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase()
  const langMap: Record<string, string> = {
    'ts': 'typescript',
    'tsx': 'typescript',
    'js': 'javascript',
    'jsx': 'javascript',
    'vue': 'vue',
    'html': 'html',
    'css': 'css',
    'scss': 'scss',
    'less': 'less',
    'json': 'json',
    'md': 'markdown',
    'py': 'python',
    'rs': 'rust',
    'go': 'go',
    'java': 'java',
    'c': 'c',
    'cpp': 'cpp',
    'h': 'c',
    'hpp': 'cpp',
    'yaml': 'yaml',
    'yml': 'yaml',
    'toml': 'toml',
    'sql': 'sql',
    'sh': 'shell',
    'bash': 'shell',
    'dockerfile': 'dockerfile',
    'xml': 'xml',
    'svg': 'xml'
  }
  return langMap[ext || ''] || 'plaintext'
}
