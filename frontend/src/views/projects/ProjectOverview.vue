<template>
  <div class="project-overview">
    <!-- 主布局：内容 + 右侧边栏 290px -->
    <div class="project-page-layout">
      <!-- 左侧主内容 -->
      <div class="project-page-content">
        <!-- 项目标题栏 -->
        <div class="project-header">
          <div class="project-avatar">
            <span class="avatar-letter">{{ (project?.name || project?.name || 'P')[0].toUpperCase() }}</span>
          </div>
          <h1 class="project-title">
            {{ project?.name || project?.name }}
            <svg v-if="project?.visibility === 'private'" class="visibility-icon" viewBox="0 0 16 16" fill="currentColor">
              <path d="M4 6V4a4 4 0 118 0v2h1a1 1 0 011 1v6a2 2 0 01-2 2H4a2 2 0 01-2-2V7a1 1 0 011-1h1zm2 0h4V4a2 2 0 10-4 0v2z"/>
            </svg>
          </h1>
          <div class="project-actions">
            <button class="btn btn-default">
              <svg class="btn-icon" viewBox="0 0 16 16" fill="none">
                <path d="M8 2l1.8 3.6 4 .6-2.9 2.8.7 4L8 11.3 4.4 13l.7-4L2.2 6.2l4-.6L8 2z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
              </svg>
              星标
              <span class="btn-count">0</span>
            </button>
            <button class="btn btn-default">
              <svg class="btn-icon" viewBox="0 0 16 16" fill="none">
                <circle cx="5" cy="3" r="2" stroke="currentColor" stroke-width="1.5"/>
                <circle cx="11" cy="3" r="2" stroke="currentColor" stroke-width="1.5"/>
                <circle cx="8" cy="13" r="2" stroke="currentColor" stroke-width="1.5"/>
                <path d="M5 5v2a3 3 0 003 3m3-5v2a3 3 0 01-3 3" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              派生
              <span class="btn-count">0</span>
            </button>
          </div>
        </div>

        <!-- 分支选择器行 -->
        <div class="ref-row">
          <div class="ref-left">
            <!-- 分支选择器 -->
            <div class="ref-selector-wrapper">
              <button class="btn btn-default ref-selector" @click="toggleRefDropdown">
                <svg class="btn-icon" viewBox="0 0 16 16" fill="none">
                  <circle cx="5" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
                  <circle cx="5" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/>
                  <path d="M5 6v4" stroke="currentColor" stroke-width="1.5"/>
                </svg>
                {{ currentRef || '选择分支' }}
                <svg class="chevron" viewBox="0 0 16 16" fill="none">
                  <path d="M4 6l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                </svg>
              </button>
              
              <!-- 下拉菜单 -->
              <div v-if="showRefDropdown" class="ref-dropdown">
                <div class="dropdown-tabs">
                  <button :class="{ active: refTab === 'branches' }" @click.stop="refTab = 'branches'">分支</button>
                  <button :class="{ active: refTab === 'tags' }" @click.stop="refTab = 'tags'">标签</button>
                </div>
                <div class="dropdown-search-wrapper">
                  <input v-model="refSearch" type="text" placeholder="搜索..." class="dropdown-search" @click.stop />
                </div>
                <ul class="dropdown-list">
                  <template v-if="refTab === 'branches'">
                    <li v-for="branch in filteredBranches" :key="branch.name" 
                        class="dropdown-item" :class="{ active: branch.name === currentRef }"
                        @click="selectRef(branch.name)">
                      {{ branch.name }}
                      <span v-if="branch.is_default" class="default-badge">默认</span>
                      <svg v-if="branch.name === currentRef" class="check-icon" viewBox="0 0 16 16"><path d="M4 8l3 3 5-6" stroke="currentColor" stroke-width="2" fill="none"/></svg>
                    </li>
                    <li v-if="filteredBranches.length === 0" class="dropdown-empty">无匹配分支</li>
                  </template>
                  <template v-else>
                    <li v-for="tag in filteredTags" :key="tag.name"
                        class="dropdown-item" :class="{ active: tag.name === currentRef }"
                        @click="selectRef(tag.name)">
                      {{ tag.name }}
                      <svg v-if="tag.name === currentRef" class="check-icon" viewBox="0 0 16 16"><path d="M4 8l3 3 5-6" stroke="currentColor" stroke-width="2" fill="none"/></svg>
                    </li>
                    <li v-if="filteredTags.length === 0" class="dropdown-empty">无匹配标签</li>
                  </template>
                </ul>
              </div>
            </div>
            
            <!-- 项目路径 -->
            <span class="project-path">{{ project?.name }}</span>
          </div>
          
          <div class="ref-right">
            <button class="btn btn-default">
              <svg class="btn-icon" viewBox="0 0 16 16" fill="none">
                <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              </svg>
            </button>
            <button class="btn btn-default">查找文件</button>
            
            <!-- 代码按钮 -->
            <div class="code-dropdown-wrapper">
              <button class="btn btn-confirm" @click="showCodeDropdown = !showCodeDropdown">
                代码
                <svg class="chevron" viewBox="0 0 16 16" fill="none">
                  <path d="M4 6l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                </svg>
              </button>
              <div v-if="showCodeDropdown" class="code-dropdown">
                <div class="clone-section">
                  <div class="clone-tabs">
                    <button :class="{ active: cloneType === 'http' }" @click="cloneType = 'http'">HTTP</button>
                    <button :class="{ active: cloneType === 'ssh' }" @click="cloneType = 'ssh'">SSH</button>
                  </div>
                  <div class="clone-url-row">
                    <input type="text" :value="currentCloneUrl" readonly class="clone-input" />
                    <button class="copy-btn" @click="copyUrl">
                      <svg v-if="!copied" viewBox="0 0 16 16" fill="none"><rect x="5" y="5" width="8" height="10" rx="1" stroke="currentColor" stroke-width="1.5"/><path d="M3 11V3a1 1 0 011-1h6" stroke="currentColor" stroke-width="1.5"/></svg>
                      <svg v-else viewBox="0 0 16 16" fill="none"><path d="M4 8l3 3 5-6" stroke="#108548" stroke-width="2"/></svg>
                    </button>
                  </div>
                </div>
                <div class="dropdown-divider"></div>
                <a class="dropdown-action" href="#">
                  <svg viewBox="0 0 16 16" fill="none"><path d="M8 12V4M5 7l3-3 3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><path d="M3 12v1a2 2 0 002 2h6a2 2 0 002-2v-1" stroke="currentColor" stroke-width="1.5"/></svg>
                  下载源代码
                </a>
              </div>
            </div>
            
            <button class="btn btn-icon-only">
              <svg viewBox="0 0 16 16" fill="currentColor"><circle cx="8" cy="3" r="1.5"/><circle cx="8" cy="8" r="1.5"/><circle cx="8" cy="13" r="1.5"/></svg>
            </button>
          </div>
        </div>

        <!-- 最后提交信息 -->
        <div v-if="lastCommit" class="commit-bar">
          <img :src="`https://www.gravatar.com/avatar/${lastCommit.author}?d=identicon&s=32`" class="commit-avatar" />
          <div class="commit-info">
            <span class="commit-message">{{ lastCommit.message?.split('\n')[0] }}</span>
          </div>
          <div class="commit-meta">
            <span class="commit-author">{{ lastCommit.author }}</span>
            <span class="commit-time">authored {{ formatTime(lastCommit.date) }}</span>
          </div>
          <div class="commit-sha-wrapper">
            <router-link :to="`${projectPath}/-/commit/${lastCommit.sha}`" class="commit-sha">{{ lastCommit.sha.slice(0, 8) }}</router-link>
            <button class="copy-btn small" @click="copyCommitSha">
              <svg viewBox="0 0 16 16" fill="none"><rect x="5" y="5" width="8" height="10" rx="1" stroke="currentColor" stroke-width="1.5"/><path d="M3 11V3a1 1 0 011-1h6" stroke="currentColor" stroke-width="1.5"/></svg>
            </button>
          </div>
          <router-link :to="`${projectPath}/-/commits/${currentRef}`" class="history-btn">历史</router-link>
        </div>

        <!-- 文件表格 -->
        <div class="file-table-wrapper" v-if="!loading && !error && !isEmpty">
          <table class="file-table">
            <thead>
              <tr>
                <th class="col-name">名称</th>
                <th class="col-commit">最后提交</th>
                <th class="col-time">最后更新</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="currentPath" class="file-row" @click="goToParent">
                <td class="col-name"><span class="file-name">..</span></td>
                <td class="col-commit"></td>
                <td class="col-time"></td>
              </tr>
              <tr v-for="item in sortedTreeItems" :key="item.path" class="file-row" @click="handleItemClick(item)">
                <td class="col-name">
                  <svg v-if="item.entry_type === 'Directory'" class="file-icon folder" viewBox="0 0 16 16"><path d="M2 4v8a2 2 0 002 2h8a2 2 0 002-2V6a2 2 0 00-2-2H8L6 2H4a2 2 0 00-2 2z" fill="currentColor"/></svg>
                  <svg v-else class="file-icon" viewBox="0 0 16 16" fill="none"><path d="M4 14h8a2 2 0 002-2V6l-4-4H4a2 2 0 00-2 2v8a2 2 0 002 2z" stroke="currentColor" stroke-width="1.5"/><path d="M10 2v4h4" stroke="currentColor" stroke-width="1.5"/></svg>
                  <span class="file-name" :class="{ folder: item.entry_type === 'Directory' }">{{ item.name }}</span>
                </td>
                <td class="col-commit">{{ item.last_commit_message || '' }}</td>
                <td class="col-time">{{ item.last_commit_time ? formatTime(item.last_commit_time) : '' }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- 加载状态 -->
        <div v-if="loading" class="loading-state">
          <div class="spinner"></div>
          <span>加载中...</span>
        </div>

        <!-- 空仓库 -->
        <div v-if="isEmpty" class="empty-state">
          <h3>仓库是空的</h3>
          <p>通过命令行初始化仓库：</p>
          <pre class="code-block"><code>git clone {{ currentCloneUrl }}
cd {{ project?.name }}
touch README.md
git add README.md
git commit -m "add README"
git push -u origin main</code></pre>
        </div>

        <!-- README 预览 -->
        <div v-if="hasReadme && readmeContent && !currentPath" class="readme-section">
          <div class="readme-header">
            <svg class="file-icon" viewBox="0 0 16 16" fill="none"><path d="M4 14h8a2 2 0 002-2V6l-4-4H4a2 2 0 00-2 2v8a2 2 0 002 2z" stroke="currentColor" stroke-width="1.5"/><path d="M10 2v4h4" stroke="currentColor" stroke-width="1.5"/></svg>
            <span>README.md</span>
          </div>
          <article class="readme-content" v-html="readmeContent"></article>
        </div>
      </div>

      <!-- 右侧边栏 290px -->
      <aside class="project-sidebar">
        <!-- 项目信息 -->
        <div class="sidebar-section">
          <h4 class="sidebar-title">项目信息</h4>
          <ul class="info-list">
            <li>
              <svg viewBox="0 0 16 16" fill="none"><circle cx="8" cy="8" r="2" stroke="currentColor" stroke-width="1.5"/><path d="M8 2v2M8 12v2M2 8h2M12 8h2" stroke="currentColor" stroke-width="1.5"/></svg>
              <span>{{ stats?.commits_count || 0 }} 次提交</span>
            </li>
            <li>
              <svg viewBox="0 0 16 16" fill="none"><circle cx="5" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/><circle cx="5" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/><path d="M5 6v4" stroke="currentColor" stroke-width="1.5"/></svg>
              <span>{{ branches.length }} 个分支</span>
            </li>
            <li>
              <svg viewBox="0 0 16 16" fill="none"><path d="M2 8V3a1 1 0 011-1h5l6 6-5 5-6-6z" stroke="currentColor" stroke-width="1.5"/><circle cx="5.5" cy="5.5" r="1" fill="currentColor"/></svg>
              <span>{{ tags.length }} 个标签</span>
            </li>
            <li>
              <svg viewBox="0 0 16 16" fill="none"><rect x="2" y="2" width="12" height="12" rx="2" stroke="currentColor" stroke-width="1.5"/><path d="M5 6h6M5 10h4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
              <span>{{ formatSize(repoSize) }} 项目存储</span>
            </li>
          </ul>
        </div>

        <!-- 快速操作 -->
        <div class="sidebar-section">
          <ul class="action-list">
            <li v-if="hasReadme">
              <svg viewBox="0 0 16 16" fill="none"><path d="M4 14h8a2 2 0 002-2V6l-4-4H4a2 2 0 00-2 2v8a2 2 0 002 2z" stroke="currentColor" stroke-width="1.5"/></svg>
              <span>自述文件</span>
            </li>
            <li>
              <svg viewBox="0 0 16 16" fill="none"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
              <span>添加LICENSE</span>
            </li>
            <li>
              <svg viewBox="0 0 16 16" fill="none"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
              <span>添加更新日志</span>
            </li>
            <li>
              <svg viewBox="0 0 16 16" fill="none"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
              <span>添加贡献信息</span>
            </li>
            <li>
              <svg viewBox="0 0 16 16" fill="none"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
              <span>启用Auto DevOps</span>
            </li>
            <li>
              <svg viewBox="0 0 16 16" fill="none"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
              <span>添加 Kubernetes 集群</span>
            </li>
            <li>
              <svg viewBox="0 0 16 16" fill="none"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
              <span>配置 CI/CD</span>
            </li>
            <li>
              <svg viewBox="0 0 16 16" fill="none"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
              <span>添加 Wiki</span>
            </li>
            <li>
              <svg viewBox="0 0 16 16" fill="none"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
              <span>配置集成</span>
            </li>
            <li>
              <svg viewBox="0 0 16 16" fill="none"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
              <span>Enable Observability</span>
            </li>
          </ul>
        </div>

        <!-- 创建时间 -->
        <div class="sidebar-section">
          <h4 class="sidebar-title">创建于</h4>
          <p class="created-date">{{ formatFullDate(project?.created_at) }}</p>
        </div>
      </aside>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import apiClient from '@/api'
import type { Project, ProjectStats } from '@/types'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

interface TreeItem {
  name: string
  entry_type: 'File' | 'Directory' | 'Submodule' | 'Symlink'
  path: string
  size?: number
  mode?: number
  last_commit_message?: string
  last_commit_time?: string
}

interface Branch { 
  name: string
  commit?: { sha: string; message: string }
  is_protected?: boolean
  is_default?: boolean
}
interface Tag { 
  name: string 
  commit?: { sha: string }
}
interface CommitInfo { sha: string; message: string; author: string; date: string }

const props = defineProps<{ project?: Project; stats?: ProjectStats }>()
const router = useRouter()

const loading = ref(false)
const error = ref('')
const isEmpty = ref(false)
const branches = ref<Branch[]>([])
const tags = ref<Tag[]>([])
const treeItems = ref<TreeItem[]>([])
const currentRef = ref('')
const currentPath = ref('')
const lastCommit = ref<CommitInfo | null>(null)
const readmeContent = ref('')
const hasReadme = ref(false)
const repoSize = ref(0)

const showRefDropdown = ref(false)
const showCodeDropdown = ref(false)
const refTab = ref<'branches' | 'tags'>('branches')
const refSearch = ref('')
const cloneType = ref<'http' | 'ssh'>('http')
const copied = ref(false)

const projectPath = computed(() => props.project?.owner_name ? `/${props.project.owner_name}/${props.project.name}` : '')

const currentCloneUrl = computed(() => {
  if (!props.project) return ''
  const base = window.location.origin
  return cloneType.value === 'ssh' 
    ? `git@${window.location.hostname}:${props.project.owner_name}/${props.project.name}.git`
    : `${base}/${props.project.owner_name}/${props.project.name}.git`
})

const filteredBranches = computed(() => {
  if (!refSearch.value) return branches.value
  return branches.value.filter(b => b.name.toLowerCase().includes(refSearch.value.toLowerCase()))
})

const filteredTags = computed(() => {
  if (!refSearch.value) return tags.value
  return tags.value.filter(t => t.name.toLowerCase().includes(refSearch.value.toLowerCase()))
})

const sortedTreeItems = computed(() => {
  return [...treeItems.value].sort((a, b) => {
    if (a.entry_type === 'Directory' && b.entry_type !== 'Directory') return -1
    if (a.entry_type !== 'Directory' && b.entry_type === 'Directory') return 1
    return a.name.localeCompare(b.name)
  })
})

function formatTime(date?: string) { return date ? dayjs(date).fromNow() : '-' }
function formatFullDate(date?: string) { return date ? dayjs(date).format('YYYY年MM月DD日') : '-' }
function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

function toggleRefDropdown() {
  showRefDropdown.value = !showRefDropdown.value
  if (showRefDropdown.value) showCodeDropdown.value = false
}

function selectRef(ref: string) {
  currentRef.value = ref
  showRefDropdown.value = false
  refSearch.value = ''
  loadTree()
}

function goToParent() {
  const segments = currentPath.value.split('/').filter(Boolean)
  segments.pop()
  currentPath.value = segments.join('/')
  loadTree()
}

function handleItemClick(item: TreeItem) {
  if (item.entry_type === 'Directory') {
    currentPath.value = item.path
    loadTree()
  } else {
    router.push(`${projectPath.value}/-/blob/${currentRef.value}/${item.path}`)
  }
}

async function copyUrl() {
  await navigator.clipboard.writeText(currentCloneUrl.value)
  copied.value = true
  setTimeout(() => { copied.value = false }, 2000)
}

async function copyCommitSha() {
  if (lastCommit.value?.sha) await navigator.clipboard.writeText(lastCommit.value.sha)
}

async function loadBranches() {
  if (!props.project?.name || !props.project?.owner_name) return
  try {
    const result = await apiClient.branches.list({ namespace: props.project.owner_name, project: props.project.name })
    branches.value = result || []
    console.log('Loaded branches:', branches.value.length, branches.value)
  } catch (err: any) {
    console.error('Failed to load branches:', err)
    error.value = '获取分支列表失败: ' + (err.response?.data?.message || err.message)
    branches.value = []
  }
}

async function loadTags() {
  if (!props.project?.name || !props.project?.owner_name) return
  try {
    const result = await apiClient.tags.list({ namespace: props.project.owner_name, project: props.project.name })
    tags.value = result || []
  } catch (err: any) {
    console.error('Failed to load tags:', err)
    tags.value = []
  }
}

async function loadTree() {
  if (!props.project?.name || !props.project?.owner_name) return
  loading.value = true
  error.value = ''
  isEmpty.value = false
  
  try {
    if (branches.value.length === 0) { isEmpty.value = true; loading.value = false; return }
    
    const refToUse = currentRef.value
    if (!refToUse) { isEmpty.value = true; loading.value = false; return }
    
    const data = await apiClient.repository.browseTree(
      { namespace: props.project.owner_name, project: props.project.name },
      currentPath.value || undefined,
      refToUse
    )
    
    if (Array.isArray(data)) {
      treeItems.value = data
      isEmpty.value = data.length === 0 && !currentPath.value
      const readme = data.find((item: TreeItem) => item.name.toLowerCase() === 'readme.md' && item.entry_type === 'File')
      if (readme && !currentPath.value) { hasReadme.value = true; loadReadme(readme.path) }
      else { hasReadme.value = !!readme; if (currentPath.value) readmeContent.value = '' }
    } else { treeItems.value = []; isEmpty.value = true }
  } catch (err: any) {
    const msg = err.response?.data?.error || err.message || ''
    if (msg.includes('not_found') || branches.value.length === 0) { isEmpty.value = true; error.value = '' }
    else error.value = msg || '加载失败'
  } finally { loading.value = false }
}

async function loadReadme(path: string) {
  if (!props.project?.name || !props.project?.owner_name) return
  const refToUse = currentRef.value
  if (!refToUse) return
  try {
    const data = await apiClient.repository.getFile(
      { namespace: props.project.owner_name, project: props.project.name },
      path, refToUse
    )
    if (data?.content) readmeContent.value = convertMarkdown(data.content)
  } catch { readmeContent.value = '' }
}

async function loadLastCommit() {
  if (!props.project?.name || !props.project?.owner_name) return
  const refToUse = currentRef.value
  if (!refToUse) return
  try {
    const commits = await apiClient.commits.list(
      { namespace: props.project.owner_name, project: props.project.name },
      refToUse, undefined, 1, 1
    )
    if (commits?.[0]) {
      const c = commits[0]
      lastCommit.value = { sha: c.sha, message: c.message, author: c.author_name, date: new Date(c.authored_date * 1000).toISOString() }
    }
  } catch {}
}

function convertMarkdown(md: string): string {
  return md.replace(/^### (.*$)/gim, '<h3>$1</h3>').replace(/^## (.*$)/gim, '<h2>$1</h2>').replace(/^# (.*$)/gim, '<h1>$1</h1>')
    .replace(/\*\*(.*)\*\*/gim, '<strong>$1</strong>').replace(/\*(.*)\*/gim, '<em>$1</em>')
    .replace(/\[(.*?)\]\((.*?)\)/gim, '<a href="$2">$1</a>').replace(/```([\s\S]*?)```/gim, '<pre><code>$1</code></pre>')
    .replace(/`(.*?)`/gim, '<code>$1</code>').replace(/\n/gim, '<br>')
}

function handleClickOutside(e: MouseEvent) {
  const t = e.target as HTMLElement
  if (!t.closest('.ref-selector-wrapper')) showRefDropdown.value = false
  if (!t.closest('.code-dropdown-wrapper')) showCodeDropdown.value = false
}

onMounted(async () => {
  document.addEventListener('click', handleClickOutside)
  if (props.project) {
    await loadBranches()
    await loadTags()
    // 根据实际分支设置 currentRef，空仓库不设置任何值
    if (branches.value.length > 0) {
      // 使用API返回的is_default标记来确定默认分支
      const defaultBranch = branches.value.find(b => b.is_default)
      currentRef.value = defaultBranch?.name || branches.value[0].name
      loadTree()
      loadLastCommit()
    } else {
      currentRef.value = ''
      isEmpty.value = true
    }
  }
})

onUnmounted(() => document.removeEventListener('click', handleClickOutside))

watch(() => props.project, async (p) => {
  if (p) {
    await loadBranches()
    await loadTags()
    if (branches.value.length > 0) {
      // 使用API返回的is_default标记来确定默认分支
      const defaultBranch = branches.value.find(b => b.is_default)
      currentRef.value = defaultBranch?.name || branches.value[0].name
      loadTree()
      loadLastCommit()
    } else {
      currentRef.value = ''
      isEmpty.value = true
    }
  }
}, { immediate: true })
</script>

<style lang="scss" scoped>
// GitLab 颜色
$white: #fff;
$gray-50: #fafafa;
$gray-100: #f0f0f2;
$gray-200: #dcdcde;
$gray-300: #bfbfc3;
$gray-500: #737278;
$gray-600: #626167;
$gray-700: #525156;
$gray-900: #1f1e24;
$blue-50: #e9f3fc;
$blue-500: #1f75cb;
$blue-600: #1068bf;
$green-500: #108548;
$orange-folder: #e9a84b;

.project-overview {
  padding: 0;
}

// 主布局：左内容 + 右侧边栏
.project-page-layout {
  display: grid;
  grid-template-columns: 1fr;
  gap: 2rem;
  
  @media (min-width: 992px) {
    grid-template-columns: 1fr 290px;
  }
}

.project-page-content {
  min-width: 0;
}

// 项目标题栏
.project-header {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-bottom: 1rem;
  padding-bottom: 1rem;
  border-bottom: 1px solid $gray-200;
}

.project-avatar {
  width: 48px;
  height: 48px;
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  
  .avatar-letter {
    color: $white;
    font-size: 1.5rem;
    font-weight: 600;
  }
}

.project-title {
  flex: 1;
  font-size: 1.25rem;
  font-weight: 600;
  color: $gray-900;
  margin: 0;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  
  .visibility-icon {
    width: 16px;
    height: 16px;
    color: $gray-500;
  }
}

.project-actions {
  display: flex;
  gap: 0.5rem;
}

// 按钮
.btn {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.375rem 0.75rem;
  font-size: 0.875rem;
  font-weight: 500;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s;
  border: 1px solid $gray-200;
  background: $white;
  color: $gray-700;
  
  &:hover {
    background: $gray-50;
    border-color: $gray-300;
  }
  
  &.btn-confirm {
    background: $blue-500;
    border-color: $blue-600;
    color: $white;
    &:hover { background: $blue-600; }
  }
  
  &.btn-icon-only {
    padding: 0.375rem;
  }
  
  .btn-icon {
    width: 16px;
    height: 16px;
  }
  
  .btn-count {
    background: $gray-100;
    padding: 0 6px;
    border-radius: 10px;
    font-size: 0.75rem;
  }
  
  .chevron {
    width: 12px;
    height: 12px;
    opacity: 0.6;
  }
}

// 分支选择器行
.ref-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 0.75rem;
  margin-bottom: 1rem;
}

.ref-left {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.ref-right {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.ref-selector-wrapper {
  position: relative;
}

.ref-selector {
  font-weight: 600;
}

.project-path {
  color: $gray-600;
  font-size: 0.875rem;
}

// 下拉菜单
.ref-dropdown, .code-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  z-index: 1000;
  min-width: 240px;
  margin-top: 4px;
  background: $white;
  border: 1px solid $gray-200;
  border-radius: 4px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.15);
}

.dropdown-tabs {
  display: flex;
  border-bottom: 1px solid $gray-200;
  
  button {
    flex: 1;
    padding: 0.75rem;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    font-size: 0.875rem;
    color: $gray-600;
    cursor: pointer;
    
    &:hover { color: $gray-900; }
    &.active { color: $blue-600; border-bottom-color: $blue-600; }
  }
}

.dropdown-search-wrapper {
  padding: 0.75rem;
  border-bottom: 1px solid $gray-200;
}

.dropdown-search {
  width: 100%;
  padding: 0.5rem 0.75rem;
  border: 1px solid $gray-200;
  border-radius: 4px;
  font-size: 0.875rem;
  &:focus { outline: none; border-color: $blue-500; }
}

.dropdown-list {
  max-height: 200px;
  overflow-y: auto;
  list-style: none;
  margin: 0;
  padding: 0.5rem 0;
}

.dropdown-item {
  display: flex;
  align-items: center;
  padding: 0.5rem 0.75rem;
  font-size: 0.875rem;
  color: $gray-700;
  cursor: pointer;
  
  &:hover { background: $blue-50; }
  &.active { background: $blue-50; color: $blue-600; }
  
  .default-badge {
    margin-left: auto;
    padding: 2px 6px;
    background: $blue-50;
    color: $blue-600;
    font-size: 0.75rem;
    border-radius: 4px;
  }
  
  .check-icon {
    margin-left: auto;
    width: 16px;
    height: 16px;
    color: $blue-600;
  }
}

.dropdown-empty {
  padding: 1rem;
  text-align: center;
  color: $gray-500;
}

// 代码下拉
.code-dropdown-wrapper {
  position: relative;
}

.code-dropdown {
  right: 0;
  left: auto;
  min-width: 320px;
}

.clone-section {
  padding: 0.75rem;
}

.clone-tabs {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
  
  button {
    flex: 1;
    padding: 0.5rem;
    background: $gray-50;
    border: 1px solid $gray-200;
    border-radius: 4px;
    font-size: 0.75rem;
    color: $gray-600;
    cursor: pointer;
    
    &:hover { background: $gray-100; }
    &.active { background: $blue-50; border-color: $blue-500; color: $blue-600; }
  }
}

.clone-url-row {
  display: flex;
  gap: 0.5rem;
}

.clone-input {
  flex: 1;
  padding: 0.5rem;
  border: 1px solid $gray-200;
  border-radius: 4px;
  font-size: 0.75rem;
  font-family: monospace;
  background: $gray-50;
}

.copy-btn {
  padding: 0.5rem;
  background: $white;
  border: 1px solid $gray-200;
  border-radius: 4px;
  cursor: pointer;
  
  &:hover { background: $gray-50; }
  &.small { padding: 0.25rem; }
  
  svg { width: 16px; height: 16px; }
}

.dropdown-divider {
  border-top: 1px solid $gray-200;
  margin: 0.5rem 0;
}

.dropdown-action {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem;
  color: $gray-700;
  text-decoration: none;
  font-size: 0.875rem;
  
  &:hover { background: $gray-50; }
  svg { width: 16px; height: 16px; }
}

// 提交信息栏
.commit-bar {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  background: $gray-50;
  border: 1px solid $gray-200;
  border-radius: 4px;
  margin-bottom: 0;
  font-size: 0.875rem;
}

.commit-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
}

.commit-info {
  flex: 1;
  min-width: 0;
}

.commit-message {
  color: $gray-900;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.commit-meta {
  color: $gray-500;
  font-size: 0.75rem;
  white-space: nowrap;
}

.commit-author {
  font-weight: 500;
  color: $gray-700;
}

.commit-sha-wrapper {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.commit-sha {
  padding: 2px 8px;
  background: $gray-100;
  border-radius: 4px;
  font-family: monospace;
  font-size: 0.75rem;
  color: $gray-700;
  text-decoration: none;
  
  &:hover { background: $blue-50; color: $blue-600; }
}

.history-btn {
  padding: 0.375rem 0.75rem;
  background: $white;
  border: 1px solid $gray-200;
  border-radius: 4px;
  font-size: 0.875rem;
  color: $gray-700;
  text-decoration: none;
  
  &:hover { background: $gray-50; }
}

// 文件表格
.file-table-wrapper {
  border: 1px solid $gray-200;
  border-top: none;
  border-radius: 0 0 4px 4px;
  overflow: hidden;
}

.file-table {
  width: 100%;
  border-collapse: collapse;
  
  th {
    padding: 0.75rem 1rem;
    background: $gray-50;
    font-size: 0.75rem;
    font-weight: 600;
    color: $gray-600;
    text-align: left;
    border-bottom: 1px solid $gray-200;
  }
  
  .col-name { width: 40%; }
  .col-commit { width: 40%; }
  .col-time { width: 20%; text-align: right; }
}

.file-row {
  cursor: pointer;
  transition: background 0.1s;
  
  &:hover { background: $blue-50; }
  
  td {
    padding: 0.625rem 1rem;
    font-size: 0.875rem;
    border-bottom: 1px solid $gray-200;
    vertical-align: middle;
  }
  
  &:last-child td { border-bottom: none; }
}

.col-name {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.file-icon {
  width: 16px;
  height: 16px;
  color: $gray-500;
  flex-shrink: 0;
  
  &.folder { color: $orange-folder; }
}

.file-name {
  color: $gray-900;
  &.folder { font-weight: 500; }
  &:hover { color: $blue-600; }
}

.col-commit {
  color: $gray-600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.col-time {
  color: $gray-500;
  font-size: 0.75rem;
  text-align: right;
}

// 加载状态
.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  padding: 3rem;
  color: $gray-600;
}

.spinner {
  width: 20px;
  height: 20px;
  border: 2px solid $gray-200;
  border-top-color: $blue-500;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

// 空仓库
.empty-state {
  padding: 2rem;
  text-align: center;
  
  h3 { margin-bottom: 0.5rem; color: $gray-900; }
  p { color: $gray-600; margin-bottom: 1rem; }
}

.code-block {
  text-align: left;
  padding: 1rem;
  background: $gray-900;
  border-radius: 4px;
  overflow-x: auto;
  
  code {
    font-family: monospace;
    font-size: 0.875rem;
    color: #e5e7eb;
    line-height: 1.5;
  }
}

// README
.readme-section {
  margin-top: 1rem;
  border: 1px solid $gray-200;
  border-radius: 4px;
  overflow: hidden;
}

.readme-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: $gray-50;
  border-bottom: 1px solid $gray-200;
  font-size: 0.875rem;
  font-weight: 500;
  color: $gray-700;
  
  .file-icon { width: 16px; height: 16px; }
}

.readme-content {
  padding: 1rem;
  font-size: 0.875rem;
  line-height: 1.6;
  
  h1, h2, h3 { margin: 1rem 0 0.5rem; color: $gray-900; }
  h1 { font-size: 1.5rem; }
  h2 { font-size: 1.25rem; }
  h3 { font-size: 1.1rem; }
  a { color: $blue-600; }
  code { padding: 2px 6px; background: $gray-100; border-radius: 3px; font-family: monospace; }
  pre { padding: 0.75rem; background: $gray-100; border-radius: 4px; overflow-x: auto; code { padding: 0; background: none; } }
}

// 右侧边栏
.project-sidebar {
  @media (min-width: 992px) {
    position: sticky;
    top: 1rem;
  }
}

.sidebar-section {
  padding: 1rem 0;
  border-bottom: 1px solid $gray-200;
  
  &:first-child { padding-top: 0; }
  &:last-child { border-bottom: none; }
}

.sidebar-title {
  margin: 0 0 0.75rem;
  font-size: 0.875rem;
  font-weight: 600;
  color: $gray-900;
}

.info-list {
  list-style: none;
  margin: 0;
  padding: 0;
  
  li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0;
    font-size: 0.875rem;
    color: $gray-700;
    
    svg {
      width: 16px;
      height: 16px;
      color: $gray-500;
    }
  }
}

.action-list {
  list-style: none;
  margin: 0;
  padding: 0;
  
  li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0;
    font-size: 0.875rem;
    color: $blue-600;
    cursor: pointer;
    
    &:hover { text-decoration: underline; }
    
    svg {
      width: 16px;
      height: 16px;
      color: $blue-600;
    }
  }
}

.created-date {
  margin: 0;
  font-size: 0.875rem;
  color: $gray-700;
}
</style>
