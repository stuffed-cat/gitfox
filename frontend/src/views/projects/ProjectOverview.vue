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
            <button class="btn btn-default" :class="{ 'is-starred': isStarred }" @click="toggleStar" :disabled="starLoading">
              <svg class="btn-icon" viewBox="0 0 16 16" :fill="isStarred ? 'currentColor' : 'none'">
                <path d="M8 2l1.8 3.6 4 .6-2.9 2.8.7 4L8 11.3 4.4 13l.7-4L2.2 6.2l4-.6L8 2z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
              </svg>
              {{ isStarred ? '取消星标' : '星标' }}
              <span class="btn-count">{{ starsCount }}</span>
            </button>
            <button class="btn btn-default" @click="handleFork" :disabled="forkLoading">
              <svg class="btn-icon" viewBox="0 0 16 16" fill="none">
                <circle cx="5" cy="3" r="2" stroke="currentColor" stroke-width="1.5"/>
                <circle cx="11" cy="3" r="2" stroke="currentColor" stroke-width="1.5"/>
                <circle cx="8" cy="13" r="2" stroke="currentColor" stroke-width="1.5"/>
                <path d="M5 5v2a3 3 0 003 3m3-5v2a3 3 0 01-3 3" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              {{ forkLoading ? '派生中...' : '派生' }}
              <span class="btn-count">{{ forksCount }}</span>
            </button>
          </div>
        </div>

        <!-- Fork关系提示 -->
        <div v-if="project?.forked_from_namespace && project?.forked_from_name" class="fork-info">
          <svg class="fork-icon" viewBox="0 0 16 16" fill="none">
            <circle cx="5" cy="3" r="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="11" cy="3" r="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="8" cy="13" r="2" stroke="currentColor" stroke-width="1.5"/>
            <path d="M5 5v2a3 3 0 003 3m3-5v2a3 3 0 01-3 3" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          <span>
            派生自 
            <router-link 
              :to="`/${project.forked_from_namespace}/${project.forked_from_name}`" 
              class="fork-link"
            >
              {{ project.forked_from_namespace }} / {{ project.forked_from_name }}
            </router-link>
          </span>
          
          <!-- Fork Divergence Status -->
          <div v-if="!divergenceLoading && forkDivergence" class="fork-status">
            <!-- In sync -->
            <span v-if="forkDivergence.ahead === 0 && forkDivergence.behind === 0" class="sync-status">
              <svg viewBox="0 0 16 16" fill="none">
                <path d="M13 5l-1.5-1.5M13 5l-1.5 1.5M13 5H5m-2 6l1.5 1.5M3 11l1.5-1.5M3 11h8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              与上游代码库保持同步
            </span>
            
            <!-- Behind upstream - show update button -->
            <template v-else-if="forkDivergence.behind > 0">
              <span class="divergence-info">
                落后 {{ forkDivergence.behind }} 个提交
                <span v-if="forkDivergence.ahead > 0">，领先 {{ forkDivergence.ahead }} 个提交</span>
              </span>
              <button class="btn btn-sm btn-primary" @click="handleUpdate">
                <svg viewBox="0 0 16 16" fill="none">
                  <path d="M13 8a5 5 0 11-10 0" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                  <path d="M13 3v5h-5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                更新
              </button>
            </template>
            
            <!-- Ahead of upstream - show create MR button -->
            <template v-else-if="forkDivergence.ahead > 0">
              <span class="divergence-info">
                领先 {{ forkDivergence.ahead }} 个提交
              </span>
              <button class="btn btn-sm btn-confirm" @click="handleCreateMR">
                <svg viewBox="0 0 16 16" fill="none">
                  <circle cx="3" cy="3" r="2" stroke="currentColor" stroke-width="1.5"/>
                  <circle cx="13" cy="13" r="2" stroke="currentColor" stroke-width="1.5"/>
                  <path d="M3 5v6m10-6v-2m0 8V9m-10 0h10" stroke="currentColor" stroke-width="1.5"/>
                </svg>
                创建合并请求
              </button>
            </template>
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
          <div class="commit-avatar">
            <img v-if="authorAvatarUrl" :src="authorAvatarUrl" alt="avatar" />
            <span v-else class="avatar-initial">{{ lastCommit.author?.charAt(0)?.toUpperCase() || '?' }}</span>
          </div>
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
          <div class="readme-content rendered-markdown" v-html="renderedReadme"></div>
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
import type { Project, ProjectStats, ForkDivergence } from '@/types'

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
interface CommitInfo { sha: string; message: string; author: string; author_email: string; date: string }

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
const authorAvatarUrl = ref<string | null>(null)
const readmeContent = ref('')
const hasReadme = ref(false)
const repoSize = ref(0)

// Star/Fork state
const isStarred = ref(false)
const starLoading = ref(false)
const starsCount = ref(0)
const forkLoading = ref(false)
const forksCount = ref(0)

// Fork divergence state
const forkDivergence = ref<ForkDivergence | null>(null)
const divergenceLoading = ref(false)
const syncing = ref(false)
const syncError = ref('')

const showRefDropdown = ref(false)
const showCodeDropdown = ref(false)
const refTab = ref<'branches' | 'tags'>('branches')
const refSearch = ref('')
const cloneType = ref<'http' | 'ssh'>('http')
const copied = ref(false)

const projectPath = computed(() => props.project?.owner_name ? `/${props.project.owner_name}/${props.project.name}` : '')

const cloneConfig = ref<{ ssh_enabled: boolean; ssh_clone_url_prefix: string; http_clone_url_prefix: string } | null>(null)

const currentCloneUrl = computed(() => {
  if (!props.project) return ''
  const repoPath = `${props.project.owner_name}/${props.project.name}.git`
  if (cloneType.value === 'ssh' && cloneConfig.value?.ssh_enabled) {
    return `${cloneConfig.value.ssh_clone_url_prefix}${repoPath}`
  }
  if (cloneConfig.value?.http_clone_url_prefix) {
    return `${cloneConfig.value.http_clone_url_prefix}${repoPath}`
  }
  return `${window.location.origin}/${repoPath}`
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

// Star/Fork methods
async function checkStarredStatus() {
  if (!props.project?.owner_name || !props.project?.name) return
  try {
    const result = await apiClient.projects.checkStarred({ 
      namespace: props.project.owner_name, 
      project: props.project.name 
    })
    isStarred.value = result.starred
  } catch (err) {
    console.error('Failed to check starred status:', err)
  }
}

async function toggleStar() {
  if (!props.project?.owner_name || !props.project?.name) return
  starLoading.value = true
  try {
    const path = { namespace: props.project.owner_name, project: props.project.name }
    if (isStarred.value) {
      const result = await apiClient.projects.unstar(path)
      isStarred.value = result.starred
      starsCount.value = result.stars_count
    } else {
      const result = await apiClient.projects.star(path)
      isStarred.value = result.starred
      starsCount.value = result.stars_count
    }
  } catch (err: any) {
    console.error('Failed to toggle star:', err)
    alert(err.response?.data?.message || '操作失败')
  } finally {
    starLoading.value = false
  }
}

function handleFork() {
  if (!props.project?.owner_name || !props.project?.name) return
  // Navigate to fork page like GitLab
  router.push(`/${props.project.owner_name}/${props.project.name}/-/forks/new`)
}

async function loadForkDivergence() {
  if (!props.project?.name || !props.project?.owner_name) return
  // Only load divergence if this is a fork
  if (!props.project.forked_from_id) return
  
  divergenceLoading.value = true
  try {
    const divergence = await apiClient.projects.getForkDivergence({
      namespace: props.project.owner_name,
      project: props.project.name
    })
    forkDivergence.value = divergence
  } catch (e) {
    console.warn('Failed to load fork divergence:', e)
    forkDivergence.value = null
  } finally {
    divergenceLoading.value = false
  }
}

function handleCreateMR() {
  if (!props.project?.forked_from_namespace || !props.project?.forked_from_name) return
  // Navigate to create MR page targeting upstream
  router.push(`/${props.project.forked_from_namespace}/${props.project.forked_from_name}/-/merge_requests/new?source=${props.project.owner_name}/${props.project.name}`)
}

async function handleUpdate() {
  if (!props.project?.owner_name || !props.project?.name) return
  if (!props.project.forked_from_namespace || !props.project.forked_from_name) return
  
  if (!confirm('确定要从上游仓库同步最新代码吗？这将会获取上游仓库的所有更新。')) {
    return
  }
  
  syncing.value = true
  syncError.value = ''
  
  try {
    const result = await apiClient.projects.syncFork({
      namespace: props.project.owner_name,
      project: props.project.name
    })
    
    if (result.success) {
      // 显示成功消息
      const message = result.updated_refs > 0 
        ? `同步成功！更新了 ${result.updated_refs} 个引用。` 
        : '同步成功！仓库已是最新状态。'
      alert(message)
      
      // 刷新分叉差异状态
      await loadForkDivergence()
      // 刷新分支列表
      await loadBranches()
    } else {
      syncError.value = result.message || '同步失败'
      alert(`同步失败: ${syncError.value}`)
    }
  } catch (err: any) {
    syncError.value = err.response?.data?.message || err.message || '同步失败'
    alert(`同步失败: ${syncError.value}`)
  } finally {
    syncing.value = false
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
    if (data?.content) readmeContent.value = data.content
  } catch { readmeContent.value = '' }
}

const renderedReadme = computed(() => {
  const text = readmeContent.value
  if (!text) return ''
  // 提取代码块
  const blocks: string[] = []
  let s = text.replace(/```(\w*)\n([\s\S]*?)```/g, (_: string, lang: string, code: string) => {
    const escaped = code.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')
    blocks.push(`<pre><code class="language-${lang}">${escaped}</code></pre>`)
    return `\x00BLOCK${blocks.length - 1}\x00`
  })
  // 按行解析
  const lines = s.split('\n')
  const out: string[] = []
  let inUl = false
  let inOl = false
  for (const line of lines) {
    const t = line.trim()
    if (t.startsWith('### ')) {
      if (inUl) { out.push('</ul>'); inUl = false }
      if (inOl) { out.push('</ol>'); inOl = false }
      out.push(`<h3>${inline(t.slice(4))}</h3>`)
    } else if (t.startsWith('## ')) {
      if (inUl) { out.push('</ul>'); inUl = false }
      if (inOl) { out.push('</ol>'); inOl = false }
      out.push(`<h2>${inline(t.slice(3))}</h2>`)
    } else if (t.startsWith('# ')) {
      if (inUl) { out.push('</ul>'); inUl = false }
      if (inOl) { out.push('</ol>'); inOl = false }
      out.push(`<h1>${inline(t.slice(2))}</h1>`)
    } else if (t.startsWith('- ')) {
      if (inOl) { out.push('</ol>'); inOl = false }
      if (!inUl) { out.push('<ul>'); inUl = true }
      out.push(`<li>${inline(t.slice(2))}</li>`)
    } else if (/^\d+\.\s/.test(t)) {
      if (inUl) { out.push('</ul>'); inUl = false }
      if (!inOl) { out.push('<ol>'); inOl = true }
      out.push(`<li>${inline(t.replace(/^\d+\.\s/, ''))}</li>`)
    } else if (t === '') {
      if (inUl) { out.push('</ul>'); inUl = false }
      if (inOl) { out.push('</ol>'); inOl = false }
    } else if (t.startsWith('\x00BLOCK')) {
      if (inUl) { out.push('</ul>'); inUl = false }
      if (inOl) { out.push('</ol>'); inOl = false }
      const idx = parseInt(t.replace(/\x00/g, '').replace('BLOCK', ''))
      out.push(blocks[idx])
    } else {
      if (inUl) { out.push('</ul>'); inUl = false }
      if (inOl) { out.push('</ol>'); inOl = false }
      out.push(`<p>${inline(t)}</p>`)
    }
  }
  if (inUl) out.push('</ul>')
  if (inOl) out.push('</ol>')
  return out.join('\n')
})

function inline(s: string): string {
  return s
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.+?)\*/g, '<em>$1</em>')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>')
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
      lastCommit.value = { 
        sha: c.sha, 
        message: c.message, 
        author: c.author_name, 
        author_email: c.author_email,
        date: new Date(c.authored_date * 1000).toISOString() 
      }
      // Fetch author avatar by email
      if (c.author_email) {
        const avatars = await apiClient.users.getAvatarsByEmails([c.author_email])
        if (avatars?.[0]?.avatar_url) {
          authorAvatarUrl.value = avatars[0].avatar_url
        } else {
          authorAvatarUrl.value = null
        }
      }
    }
  } catch {}
}

function handleClickOutside(e: MouseEvent) {
  const t = e.target as HTMLElement
  if (!t.closest('.ref-selector-wrapper')) showRefDropdown.value = false
  if (!t.closest('.code-dropdown-wrapper')) showCodeDropdown.value = false
}

onMounted(async () => {
  document.addEventListener('click', handleClickOutside)
  
  // Load server config for clone URLs
  try {
    cloneConfig.value = await apiClient.config.get()
  } catch (e) {
    console.warn('Failed to load server config:', e)
  }
  
  if (props.project) {
    // Initialize star/fork counts from project data
    starsCount.value = props.project.stars_count ?? 0
    forksCount.value = props.project.forks_count ?? 0
    
    // Check starred status
    checkStarredStatus()
    
    // Load fork divergence if this is a fork
    loadForkDivergence()
    
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
$blue-700: #0b5ca3;
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

// Fork 关系提示
.fork-info {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  margin-bottom: 1rem;
  background: $gray-50;
  border: 1px solid $gray-200;
  border-radius: 6px;
  font-size: 0.875rem;
  color: $gray-700;

  .fork-icon {
    width: 16px;
    height: 16px;
    color: $gray-500;
    flex-shrink: 0;
  }

  .fork-link {
    color: $blue-600;
    text-decoration: none;
    font-weight: 500;
    
    &:hover {
      color: $blue-700;
      text-decoration: underline;
    }
  }

  .fork-status {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-left: auto;
    
    .sync-status {
      display: flex;
      align-items: center;
      gap: 0.375rem;
      color: $green-500;
      font-weight: 500;
      
      svg {
        width: 16px;
        height: 16px;
      }
    }

    .divergence-info {
      color: $gray-600;
      font-size: 0.8125rem;
    }

    .btn-sm {
      padding: 0.25rem 0.625rem;
      font-size: 0.8125rem;
      
      svg {
        width: 14px;
        height: 14px;
      }
    }

    .btn-primary {
      background: $blue-600;
      color: $white;
      border-color: $blue-600;

      &:hover {
        background: $blue-700;
        border-color: $blue-700;
      }
    }
  }
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
  
  &.is-starred {
    background: #fef3c7;
    border-color: #f59e0b;
    color: #b45309;
    
    .btn-icon {
      color: #f59e0b;
    }
    
    .btn-count {
      background: #fde68a;
    }
  }
  
  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
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
  background: $gray-200;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  flex-shrink: 0;
  
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  
  .avatar-initial {
    font-size: 14px;
    font-weight: 600;
    color: $gray-600;
  }
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
}

.rendered-markdown {
  font-size: 14px;
  line-height: 1.6;
  color: $gray-900;

  :deep(h1) { font-size: 1.75em; font-weight: 600; margin: 24px 0 16px; padding-bottom: 0.3em; border-bottom: 1px solid $gray-200; &:first-child { margin-top: 0; } }
  :deep(h2) { font-size: 1.5em; font-weight: 600; margin: 24px 0 16px; padding-bottom: 0.3em; border-bottom: 1px solid $gray-200; }
  :deep(h3) { font-size: 1.25em; font-weight: 600; margin: 24px 0 16px; }
  :deep(p) { margin: 0 0 16px; }
  :deep(ul), :deep(ol) { margin: 0 0 16px; padding-left: 2em; }
  :deep(li) { margin: 4px 0; }
  :deep(strong) { font-weight: 600; }
  :deep(em) { font-style: italic; }
  :deep(a) { color: $blue-600; text-decoration: none; &:hover { text-decoration: underline; } }
  :deep(code) { padding: 0.2em 0.4em; font-size: 85%; background: rgba($gray-500, 0.15); border-radius: 4px; font-family: monospace; }
  :deep(pre) { margin: 0 0 16px; padding: 16px; background: $gray-900; border-radius: 4px; overflow-x: auto;
    code { display: block; padding: 0; background: none; color: #e5e7eb; font-size: 0.875rem; line-height: 1.5; }
  }
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
