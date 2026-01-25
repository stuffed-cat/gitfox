<template>
  <div class="file-browser">
    <!-- Header: Branch selector + path -->
    <div class="browser-header">
      <div class="ref-selector">
        <button class="ref-btn" @click="showRefDropdown = !showRefDropdown">
          <svg viewBox="0 0 16 16" fill="none" class="branch-icon">
            <circle cx="4" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="4" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/>
            <path d="M4 6v4" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          {{ currentRef || '选择分支' }}
          <svg viewBox="0 0 16 16" fill="none" class="chevron">
            <path d="M4 6l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
        
        <div v-if="showRefDropdown" class="ref-dropdown">
          <div class="dropdown-tabs">
            <button :class="{ active: refTab === 'branches' }" @click="refTab = 'branches'">分支</button>
            <button :class="{ active: refTab === 'tags' }" @click="refTab = 'tags'">标签</button>
          </div>
          <input v-model="refSearch" type="text" placeholder="搜索分支或标签..." class="ref-search" />
          <div class="ref-list">
            <template v-if="refTab === 'branches'">
              <div
                v-for="branch in filteredBranches"
                :key="branch.name"
                class="ref-item"
                :class="{ active: branch.name === currentRef }"
                @click="selectRef(branch.name)"
              >
                {{ branch.name }}
                <span v-if="branch.name === project?.default_branch" class="default-badge">默认</span>
              </div>
              <div v-if="filteredBranches.length === 0" class="no-results">无匹配分支</div>
            </template>
            <template v-else>
              <div
                v-for="tag in filteredTags"
                :key="tag.name"
                class="ref-item"
                :class="{ active: tag.name === currentRef }"
                @click="selectRef(tag.name)"
              >
                {{ tag.name }}
              </div>
              <div v-if="filteredTags.length === 0" class="no-results">无匹配标签</div>
            </template>
          </div>
        </div>
      </div>
      
      <div class="path-breadcrumb">
        <router-link :to="`/${owner}/${repo}/-/tree/${currentRef}`" class="root-link">
          {{ repo }}
        </router-link>
        <template v-for="(segment, index) in pathSegments" :key="index">
          <span class="separator">/</span>
          <router-link
            :to="`/${owner}/${repo}/-/tree/${currentRef}/${pathSegments.slice(0, index + 1).join('/')}`"
          >
            {{ segment }}
          </router-link>
        </template>
      </div>
      
      <div class="header-actions">
        <button class="btn btn-sm" @click="copyCloneUrl">
          <svg viewBox="0 0 16 16" fill="none">
            <rect x="5" y="5" width="8" height="10" rx="1" stroke="currentColor" stroke-width="1.5"/>
            <path d="M3 11V3a1 1 0 011-1h6" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          克隆
        </button>
        <router-link :to="`/${owner}/${repo}/-/tree/${currentRef}/${currentPath}`" class="btn btn-sm btn-primary">
          Web IDE
        </router-link>
      </div>
    </div>
    
    <!-- Loading state -->
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>加载中...</span>
    </div>
    
    <!-- Error state -->
    <div v-else-if="error" class="error-state">
      <svg viewBox="0 0 64 64" fill="none">
        <circle cx="32" cy="32" r="24" stroke="currentColor" stroke-width="2"/>
        <path d="M32 20v16M32 42v2" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
      <h3>无法加载文件</h3>
      <p>{{ error }}</p>
      <button class="btn btn-primary" @click="loadTree">重试</button>
    </div>
    
    <!-- Empty repository -->
    <div v-else-if="isEmpty" class="empty-repo">
      <h3>空仓库</h3>
      <p>此仓库尚无任何文件。您可以通过以下方式添加文件：</p>
      
      <div class="setup-instructions">
        <h4>命令行创建新仓库</h4>
        <pre><code>git clone {{ cloneUrl }}
cd {{ repo }}
touch README.md
git add README.md
git commit -m "add README"
git push -u origin {{ project?.default_branch || 'master' }}</code></pre>
        
        <h4>推送已有的文件夹</h4>
        <pre><code>cd existing_folder
git init
git remote add origin {{ cloneUrl }}
git add .
git commit -m "Initial commit"
git push -u origin {{ project?.default_branch || 'master' }}</code></pre>
        
        <h4>推送已有的 Git 仓库</h4>
        <pre><code>cd existing_repo
git remote rename origin old-origin
git remote add origin {{ cloneUrl }}
git push -u origin --all
git push -u origin --tags</code></pre>
      </div>
    </div>
    
    <!-- File content view -->
    <div v-else-if="viewingFile" class="file-view">
      <div class="file-header">
        <div class="file-info">
          <span class="file-name">{{ currentFileName }}</span>
          <span class="file-size">{{ formatSize(fileSize) }}</span>
          <span class="file-lines" v-if="fileLines">{{ fileLines }} 行</span>
        </div>
        <div class="file-actions">
          <button class="btn btn-sm" @click="copyFileContent">复制</button>
          <button class="btn btn-sm" @click="downloadFile">下载</button>
          <a :href="rawFileUrl" class="btn btn-sm" target="_blank">原始</a>
        </div>
      </div>
      <div class="file-content">
        <pre class="code-block"><code :class="fileLanguage">{{ fileContent }}</code></pre>
      </div>
    </div>
    
    <!-- Tree view -->
    <div v-else class="tree-view">
      <!-- Last commit info -->
      <div v-if="lastCommit" class="last-commit">
        <div class="commit-author">
          <div class="avatar">{{ lastCommit.author?.charAt(0)?.toUpperCase() || '?' }}</div>
          <span class="author-name">{{ lastCommit.author }}</span>
        </div>
        <router-link :to="`/${owner}/${repo}/-/commit/${lastCommit.sha}`" class="commit-message">
          {{ lastCommit.message }}
        </router-link>
        <router-link :to="`/${owner}/${repo}/-/commit/${lastCommit.sha}`" class="commit-sha">
          {{ lastCommit.sha?.substring(0, 8) }}
        </router-link>
        <span class="commit-time">{{ formatDate(lastCommit.date) }}</span>
      </div>
      
      <!-- File table -->
      <table class="file-table">
        <thead>
          <tr>
            <th class="col-name">名称</th>
            <th class="col-commit">最后提交</th>
            <th class="col-time">更新时间</th>
          </tr>
        </thead>
        <tbody>
          <!-- Parent directory -->
          <tr v-if="currentPath" class="tree-item" @click="goToParent">
            <td class="item-name">
              <span class="item-icon">📁</span>
              <span>..</span>
            </td>
            <td></td>
            <td></td>
          </tr>
          
          <!-- Tree items -->
          <tr
            v-for="item in sortedTreeItems"
            :key="item.path"
            class="tree-item"
            @click="handleItemClick(item)"
          >
            <td class="item-name">
              <span class="item-icon">{{ item.type === 'tree' ? '📁' : getFileIcon(item.name) }}</span>
              <span>{{ item.name }}</span>
            </td>
            <td class="item-commit">{{ item.last_commit_message || '-' }}</td>
            <td class="item-time">{{ item.last_commit_time ? formatDate(item.last_commit_time) : '-' }}</td>
          </tr>
        </tbody>
      </table>
      
      <!-- README preview -->
      <div v-if="readmeContent" class="readme-preview">
        <div class="readme-header">
          <svg viewBox="0 0 16 16" fill="none">
            <rect x="2" y="2" width="12" height="12" rx="2" stroke="currentColor" stroke-width="1.5"/>
            <path d="M5 5h6M5 8h6M5 11h4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          README.md
        </div>
        <div class="readme-content" v-html="readmeContent"></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import apiClient from '@/api'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import type { Project } from '@/types'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

interface TreeItem {
  name: string
  type: 'tree' | 'blob'
  path: string
  mode?: string
  last_commit_message?: string
  last_commit_time?: string
}

interface Branch {
  name: string
  commit?: string
}

interface Tag {
  name: string
}

interface CommitInfo {
  sha: string
  message: string
  author: string
  date: string
}

const props = defineProps<{
  project?: Project
}>()

const route = useRoute()

// Route params
const owner = computed(() => route.params.owner as string)
const repo = computed(() => route.params.repo as string)

// State
const loading = ref(false)
const error = ref('')
const isEmpty = ref(false)
const branches = ref<Branch[]>([])
const tags = ref<Tag[]>([])
const currentRef = ref('')
const currentPath = ref('')
const treeItems = ref<TreeItem[]>([])
const fileContent = ref<string | null>(null)
const fileSize = ref(0)
const viewingFile = ref(false)
const lastCommit = ref<CommitInfo | null>(null)
const readmeContent = ref('')

// Ref dropdown
const showRefDropdown = ref(false)
const refTab = ref<'branches' | 'tags'>('branches')
const refSearch = ref('')

// Computed
const project = computed(() => props.project)
const cloneUrl = computed(() => `${window.location.origin}/${owner.value}/${repo.value}.git`)

const pathSegments = computed(() => {
  return currentPath.value ? currentPath.value.split('/').filter(Boolean) : []
})

const currentFileName = computed(() => {
  const segments = pathSegments.value
  return segments.length > 0 ? segments[segments.length - 1] : ''
})

const fileLines = computed(() => {
  if (!fileContent.value) return 0
  return fileContent.value.split('\n').length
})

const fileLanguage = computed(() => {
  const ext = currentFileName.value.split('.').pop()?.toLowerCase()
  const langMap: Record<string, string> = {
    js: 'javascript', ts: 'typescript', py: 'python', rb: 'ruby',
    go: 'go', rs: 'rust', java: 'java', c: 'c', cpp: 'cpp',
    h: 'c', hpp: 'cpp', cs: 'csharp', php: 'php', swift: 'swift',
    kt: 'kotlin', scala: 'scala', html: 'html', css: 'css',
    scss: 'scss', less: 'less', json: 'json', xml: 'xml',
    yaml: 'yaml', yml: 'yaml', md: 'markdown', sh: 'bash',
    sql: 'sql', vue: 'vue', svelte: 'svelte'
  }
  return langMap[ext || ''] || 'plaintext'
})

const rawFileUrl = computed(() => {
  return `/api/v1/projects/${repo.value}/repository/files?path=${currentPath.value}&ref_name=${currentRef.value}&raw=true`
})

const filteredBranches = computed(() => {
  if (!refSearch.value) return branches.value
  const search = refSearch.value.toLowerCase()
  return branches.value.filter(b => b.name.toLowerCase().includes(search))
})

const filteredTags = computed(() => {
  if (!refSearch.value) return tags.value
  const search = refSearch.value.toLowerCase()
  return tags.value.filter(t => t.name.toLowerCase().includes(search))
})

const sortedTreeItems = computed(() => {
  // Sort: directories first, then files, alphabetically
  return [...treeItems.value].sort((a, b) => {
    if (a.type === 'tree' && b.type !== 'tree') return -1
    if (a.type !== 'tree' && b.type === 'tree') return 1
    return a.name.localeCompare(b.name)
  })
})

// Methods
function formatDate(date?: string) {
  if (!date) return '-'
  return dayjs(date).fromNow()
}

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

function getFileIcon(name: string) {
  const ext = name.split('.').pop()?.toLowerCase()
  const iconMap: Record<string, string> = {
    js: '📜', ts: '📘', py: '🐍', rb: '💎', go: '🔵', rs: '🦀',
    java: '☕', md: '📝', json: '📋', yaml: '📋', yml: '📋',
    html: '🌐', css: '🎨', scss: '🎨', vue: '💚', svg: '🖼️',
    png: '🖼️', jpg: '🖼️', gif: '🖼️', pdf: '📕', zip: '📦',
    lock: '🔒', gitignore: '👁️', dockerfile: '🐳'
  }
  return iconMap[ext || ''] || '📄'
}

function selectRef(ref: string) {
  currentRef.value = ref
  showRefDropdown.value = false
  refSearch.value = ''
  loadTree()
}

function goToParent() {
  const segments = pathSegments.value
  segments.pop()
  currentPath.value = segments.join('/')
  viewingFile.value = false
  loadTree()
}

function handleItemClick(item: TreeItem) {
  if (item.type === 'tree') {
    currentPath.value = item.path
    viewingFile.value = false
    // Navigate using window.location for simplicity
    window.history.pushState({}, '', `/${owner.value}/${repo.value}/-/tree/${currentRef.value}/${item.path}`)
    loadTree()
  } else {
    currentPath.value = item.path
    window.history.pushState({}, '', `/${owner.value}/${repo.value}/-/blob/${currentRef.value}/${item.path}`)
    loadFileContent(item.path)
  }
}

async function loadBranches() {
  if (!project.value?.owner_name || !project.value?.name) return
  const path = { namespace: project.value.owner_name, project: project.value.name }
  try {
    const data = await apiClient.branches.list(path)
    branches.value = (data || []).map(b => ({ name: b.name, commit: b.commit?.sha }))
  } catch (err) {
    console.error('Failed to load branches:', err)
    branches.value = []
  }
}

async function loadTags() {
  if (!project.value?.owner_name || !project.value?.name) return
  const path = { namespace: project.value.owner_name, project: project.value.name }
  try {
    const data = await apiClient.tags.list(path)
    tags.value = (data || []).map(t => ({ name: t.name }))
  } catch (err) {
    console.error('Failed to load tags:', err)
    tags.value = []
  }
}

async function loadTree() {
  if (!project.value?.owner_name || !project.value?.name) return
  
  loading.value = true
  error.value = ''
  viewingFile.value = false
  isEmpty.value = false
  const path = { namespace: project.value.owner_name, project: project.value.name }
  
  try {
    const refToUse = currentRef.value || project.value.default_branch
    if (!refToUse) { isEmpty.value = true; loading.value = false; return }
    
    const data = await apiClient.repository.browseTree(
      path,
      currentPath.value || undefined,
      refToUse
    )
    
    if (Array.isArray(data)) {
      treeItems.value = data.map(item => ({
        name: item.name,
        type: item.entry_type === 'Directory' ? 'tree' : 'blob',
        path: item.path,
        mode: String(item.mode)
      }))
      isEmpty.value = data.length === 0 && !currentPath.value
      
      // Check for README
      const readme = treeItems.value.find((item: TreeItem) => 
        item.name.toLowerCase() === 'readme.md' && item.type === 'blob'
      )
      if (readme && !currentPath.value) {
        loadReadme(readme.path)
      } else {
        readmeContent.value = ''
      }
    } else {
      treeItems.value = []
      isEmpty.value = true
    }
  } catch (err: any) {
    console.error('Failed to load tree:', err)
    if (err.response?.status === 404) {
      isEmpty.value = true
      treeItems.value = []
    } else {
      error.value = err.response?.data?.message || '加载文件列表失败'
    }
  } finally {
    loading.value = false
  }
}

async function loadFileContent(filePath: string) {
  if (!project.value?.owner_name || !project.value?.name) return
  
  loading.value = true
  error.value = ''
  viewingFile.value = true
  const path = { namespace: project.value.owner_name, project: project.value.name }
  
  try {
    const refToUse = currentRef.value || project.value.default_branch
    if (!refToUse) { error.value = '无法加载文件，仓库为空'; loading.value = false; return }
    
    const data = await apiClient.repository.getFile(
      path,
      filePath,
      refToUse
    )
    
    fileContent.value = data.content || ''
    fileSize.value = data.size || fileContent.value?.length || 0
  } catch (err: any) {
    console.error('Failed to load file:', err)
    error.value = err.response?.data?.message || '加载文件内容失败'
    fileContent.value = null
  } finally {
    loading.value = false
  }
}

async function loadReadme(readmePath: string) {
  if (!project.value?.owner_name || !project.value?.name) return
  const path = { namespace: project.value.owner_name, project: project.value.name }
  
  try {
    const refToUse = currentRef.value || project.value.default_branch
    if (!refToUse) return
    
    const data = await apiClient.repository.getFile(
      path,
      readmePath,
      refToUse
    )
    // Simple markdown to HTML (basic)
    readmeContent.value = (data.content || '')
      .replace(/^### (.*$)/gim, '<h3>$1</h3>')
      .replace(/^## (.*$)/gim, '<h2>$1</h2>')
      .replace(/^# (.*$)/gim, '<h1>$1</h1>')
      .replace(/\*\*(.*)\*\*/gim, '<strong>$1</strong>')
      .replace(/\*(.*)\*/gim, '<em>$1</em>')
      .replace(/`([^`]+)`/gim, '<code>$1</code>')
      .replace(/\n/gim, '<br>')
  } catch (err) {
    console.error('Failed to load README:', err)
    readmeContent.value = ''
  }
}

async function loadLastCommit() {
  if (!project.value?.owner_name || !project.value?.name) return
  const path = { namespace: project.value.owner_name, project: project.value.name }
  
  try {
    const commits = await apiClient.commits.list(
      path,
      currentRef.value,
      undefined,
      1,
      1
    )
    if (Array.isArray(commits) && commits.length > 0) {
      const commit = commits[0]
      lastCommit.value = {
        sha: commit.sha,
        message: commit.message,
        author: commit.author_name,
        date: new Date(commit.authored_date * 1000).toISOString()
      }
    }
  } catch (err) {
    console.error('Failed to load last commit:', err)
  }
}

function copyCloneUrl() {
  navigator.clipboard.writeText(cloneUrl.value)
}

function copyFileContent() {
  if (fileContent.value) {
    navigator.clipboard.writeText(fileContent.value)
  }
}

function downloadFile() {
  if (!fileContent.value) return
  const blob = new Blob([fileContent.value], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = currentFileName.value
  a.click()
  URL.revokeObjectURL(url)
}

// Click outside to close dropdown
function handleClickOutside(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (!target.closest('.ref-selector')) {
    showRefDropdown.value = false
  }
}

// Initialize
async function initFromRoute() {
  const refParam = route.params.ref as string
  const pathParam = route.params.path as string
  
  // 先加载分支列表
  await loadBranches()
  await loadTags()
  
  if (refParam) {
    currentRef.value = refParam
  } else if (branches.value.length > 0) {
    // 有分支时，使用默认分支或第一个分支
    currentRef.value = project.value?.default_branch || branches.value[0].name
  } else {
    // 空仓库不设置任何 ref
    currentRef.value = ''
    isEmpty.value = true
  }
  
  currentPath.value = pathParam || ''
  
  // Check if viewing file (blob route)
  if (route.name === 'ProjectBlob' || route.path.includes('/-/blob/')) {
    viewingFile.value = true
    if (currentPath.value) {
      loadFileContent(currentPath.value)
    }
  } else {
    viewingFile.value = false
    loadTree()
  }
}

watch(() => props.project?.id, () => {
  if (props.project?.id) {
    initFromRoute()
    loadLastCommit()
  }
}, { immediate: true })

watch(() => route.params, () => {
  initFromRoute()
}, { deep: true })

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<style lang="scss" scoped>
.file-browser {
  padding: $spacing-4;
}

.browser-header {
  display: flex;
  align-items: center;
  gap: $spacing-4;
  margin-bottom: $spacing-4;
  flex-wrap: wrap;
}

.ref-selector {
  position: relative;
  
  .ref-btn {
    display: flex;
    align-items: center;
    gap: $spacing-2;
    padding: $spacing-2 $spacing-3;
    background: $bg-primary;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    cursor: pointer;
    font-size: $text-sm;
    
    .branch-icon {
      width: 16px;
      height: 16px;
      color: $text-secondary;
    }
    
    .chevron {
      width: 12px;
      height: 12px;
      color: $text-muted;
    }
  }
  
  .ref-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: $spacing-1;
    width: 280px;
    background: $bg-primary;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    box-shadow: $shadow-lg;
    z-index: 100;
    
    .dropdown-tabs {
      display: flex;
      border-bottom: 1px solid $border-color;
      
      button {
        flex: 1;
        padding: $spacing-2;
        background: none;
        border: none;
        cursor: pointer;
        font-size: $text-sm;
        color: $text-secondary;
        
        &.active {
          color: $color-primary;
          border-bottom: 2px solid $color-primary;
          margin-bottom: -1px;
        }
      }
    }
    
    .ref-search {
      width: 100%;
      padding: $spacing-2 $spacing-3;
      border: none;
      border-bottom: 1px solid $border-color;
      font-size: $text-sm;
      
      &:focus {
        outline: none;
      }
    }
    
    .ref-list {
      max-height: 200px;
      overflow-y: auto;
    }
    
    .ref-item {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: $spacing-2 $spacing-3;
      cursor: pointer;
      font-size: $text-sm;
      
      &:hover {
        background: $bg-secondary;
      }
      
      &.active {
        background: rgba($color-primary, 0.1);
        color: $color-primary;
      }
      
      .default-badge {
        font-size: $text-xs;
        color: $text-muted;
        background: $bg-tertiary;
        padding: 2px 6px;
        border-radius: $radius-sm;
      }
    }
    
    .no-results {
      padding: $spacing-4;
      text-align: center;
      color: $text-muted;
      font-size: $text-sm;
    }
  }
}

.path-breadcrumb {
  flex: 1;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  font-size: $text-sm;
  
  .root-link {
    font-weight: 600;
  }
  
  a {
    color: $color-primary;
    text-decoration: none;
    
    &:hover {
      text-decoration: underline;
    }
  }
  
  .separator {
    margin: 0 $spacing-1;
    color: $text-muted;
  }
}

.header-actions {
  display: flex;
  gap: $spacing-2;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-2 $spacing-3;
  border-radius: $radius-md;
  font-size: $text-sm;
  cursor: pointer;
  text-decoration: none;
  transition: all 0.2s;
  
  svg {
    width: 14px;
    height: 14px;
  }
}

.btn-sm {
  padding: $spacing-1 $spacing-2;
  font-size: $text-xs;
  background: $bg-primary;
  border: 1px solid $border-color;
  color: $text-primary;
  
  &:hover {
    background: $bg-secondary;
  }
}

.btn-primary {
  background: $color-primary;
  color: white;
  border: none;
  
  &:hover {
    background: $color-primary-dark;
  }
}

.loading-state, .error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: $spacing-12;
  text-align: center;
  
  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid $border-color;
    border-top-color: $color-primary;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: $spacing-4;
  }
  
  svg {
    width: 64px;
    height: 64px;
    color: $text-muted;
    margin-bottom: $spacing-4;
  }
  
  h3 {
    margin-bottom: $spacing-2;
  }
  
  p {
    color: $text-secondary;
    margin-bottom: $spacing-4;
  }
}

.empty-repo {
  padding: $spacing-8;
  
  h3 {
    margin-bottom: $spacing-2;
  }
  
  > p {
    color: $text-secondary;
    margin-bottom: $spacing-6;
  }
  
  .setup-instructions {
    background: $bg-secondary;
    border-radius: $radius-md;
    padding: $spacing-6;
    
    h4 {
      font-size: $text-sm;
      margin-bottom: $spacing-3;
      margin-top: $spacing-6;
      
      &:first-child {
        margin-top: 0;
      }
    }
    
    pre {
      background: #1e1e1e;
      color: #d4d4d4;
      padding: $spacing-4;
      border-radius: $radius-md;
      overflow-x: auto;
      font-size: $text-sm;
      font-family: 'JetBrains Mono', monospace;
    }
  }
}

.last-commit {
  display: flex;
  align-items: center;
  gap: $spacing-3;
  padding: $spacing-3 $spacing-4;
  background: $bg-secondary;
  border: 1px solid $border-color;
  border-bottom: none;
  border-radius: $radius-md $radius-md 0 0;
  font-size: $text-sm;
  
  .commit-author {
    display: flex;
    align-items: center;
    gap: $spacing-2;
    
    .avatar {
      width: 24px;
      height: 24px;
      border-radius: 50%;
      background: $color-primary;
      color: white;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: $text-xs;
      font-weight: 600;
    }
    
    .author-name {
      font-weight: 500;
    }
  }
  
  .commit-message {
    flex: 1;
    color: $text-primary;
    text-decoration: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    
    &:hover {
      color: $color-primary;
    }
  }
  
  .commit-sha {
    font-family: monospace;
    color: $color-primary;
    text-decoration: none;
    
    &:hover {
      text-decoration: underline;
    }
  }
  
  .commit-time {
    color: $text-muted;
  }
}

.file-table {
  width: 100%;
  border-collapse: collapse;
  border: 1px solid $border-color;
  border-radius: 0 0 $radius-md $radius-md;
  overflow: hidden;
  
  th {
    padding: $spacing-2 $spacing-4;
    text-align: left;
    font-size: $text-xs;
    font-weight: 500;
    color: $text-muted;
    background: $bg-secondary;
    border-bottom: 1px solid $border-color;
  }
  
  td {
    padding: $spacing-2 $spacing-4;
    border-bottom: 1px solid $border-color;
  }
  
  tr:last-child td {
    border-bottom: none;
  }
  
  .col-name { width: 40%; }
  .col-commit { width: 40%; }
  .col-time { width: 20%; text-align: right; }
}

.tree-item {
  cursor: pointer;
  
  &:hover {
    background: $bg-secondary;
  }
  
  .item-name {
    display: flex;
    align-items: center;
    gap: $spacing-2;
    
    .item-icon {
      font-size: 16px;
    }
  }
  
  .item-commit {
    color: $text-secondary;
    font-size: $text-sm;
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  
  .item-time {
    color: $text-muted;
    font-size: $text-sm;
    text-align: right;
  }
}

.file-view {
  border: 1px solid $border-color;
  border-radius: $radius-md;
  overflow: hidden;
  
  .file-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: $spacing-3 $spacing-4;
    background: $bg-secondary;
    border-bottom: 1px solid $border-color;
    
    .file-info {
      display: flex;
      align-items: center;
      gap: $spacing-3;
      font-size: $text-sm;
      
      .file-name {
        font-weight: 500;
      }
      
      .file-size, .file-lines {
        color: $text-muted;
      }
    }
    
    .file-actions {
      display: flex;
      gap: $spacing-2;
    }
  }
  
  .file-content {
    .code-block {
      margin: 0;
      padding: $spacing-4;
      overflow-x: auto;
      font-family: 'JetBrains Mono', monospace;
      font-size: $text-sm;
      line-height: 1.6;
      background: $bg-primary;
    }
  }
}

.readme-preview {
  margin-top: $spacing-6;
  border: 1px solid $border-color;
  border-radius: $radius-md;
  overflow: hidden;
  
  .readme-header {
    display: flex;
    align-items: center;
    gap: $spacing-2;
    padding: $spacing-3 $spacing-4;
    background: $bg-secondary;
    border-bottom: 1px solid $border-color;
    font-size: $text-sm;
    font-weight: 500;
    
    svg {
      width: 16px;
      height: 16px;
      color: $text-secondary;
    }
  }
  
  .readme-content {
    padding: $spacing-6;
    font-size: $text-sm;
    line-height: 1.6;
    
    h1, h2, h3 {
      margin-top: $spacing-4;
      margin-bottom: $spacing-2;
    }
    
    code {
      background: $bg-tertiary;
      padding: 2px 6px;
      border-radius: $radius-sm;
      font-family: monospace;
    }
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
