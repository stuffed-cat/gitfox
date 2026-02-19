<template>
  <div class="commit-detail">
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <template v-else-if="commit">
      <div class="commit-header">
        <h2>{{ commit.message.split('\n')[0] }}</h2>
        <pre v-if="commit.message.includes('\n')" class="commit-body">{{ commit.message.split('\n').slice(1).join('\n') }}</pre>
        
        <div class="commit-info">
          <div class="author">
            <span class="avatar">{{ commit.author_name.charAt(0).toUpperCase() }}</span>
            <div class="author-info">
              <strong>{{ commit.author_name }}</strong>
              <span>{{ commit.author_email }}</span>
            </div>
          </div>
          <div class="commit-meta">
            <div class="meta-item">
              <span class="label">提交 SHA:</span>
              <code>{{ commit.sha }}</code>
            </div>
            <div class="meta-item">
              <span class="label">提交时间:</span>
              <span>{{ formatCommitDate(commit.committed_date) }}</span>
            </div>
            <div v-if="commit.parent_shas?.length" class="meta-item">
              <span class="label">父提交:</span>
              <router-link :to="`/${project?.owner_name}/${project?.name}/-/commit/${commit.parent_shas[0]}`">
                {{ commit.parent_shas[0].substring(0, 8) }}
              </router-link>
            </div>
          </div>
        </div>
      </div>
      
      <div class="diff-section">
        <div class="diff-header">
          <h3>更改的文件 ({{ diffs.length }})</h3>
          <div class="diff-actions">
            <button @click="toggleAllFiles" class="btn btn-secondary">
              <svg viewBox="0 0 16 16" width="14" height="14" style="vertical-align: middle; margin-right: 4px;">
                <path :d="allExpanded ? icons.chevronRight : icons.chevronDown" fill="currentColor" />
              </svg>
              {{ allExpanded ? '全部收起' : '全部展开' }}
            </button>
          </div>
        </div>
        
        <div v-for="(diff, index) in displayedDiffs" :key="diff.file_path" class="diff-file">
          <div class="diff-file-header" @click="toggleFile(index)">
            <span class="expand-icon">{{ expandedFiles.has(index) ? '▼' : '▶' }}</span>
            <span class="file-status" :class="diff.status">{{ statusText(diff.status) }}</span>
            <span class="file-path">{{ diff.file_path }}</span>
            <span class="diff-stats">
              <span class="additions">+{{ diff.additions }}</span>
              <span class="deletions">-{{ diff.deletions }}</span>
            </span>
          </div>
          
          <div v-if="expandedFiles.has(index)" class="diff-content-wrapper">
            <!-- 操作按钮组 -->
            <div v-if="diff.is_truncated" class="diff-actions-bar">
              <span class="truncate-hint">
                已截断 (更改行数: {{ diff.additions + diff.deletions }} 行)
              </span>
            </div>

            <!-- GitLab 风格的 Unified Diff -->
            <UnifiedDiffViewer
              v-if="!isBinaryFile(diff) && diff.diff"
              :diff="diff.diff"
              :language="getLanguageFromPath(diff.file_path)"
            />
            
            <div v-else-if="isBinaryFile(diff)" class="no-diff">
              二进制文件，无法显示差异
            </div>
            <div v-else class="no-diff">无可显示的更改</div>
          </div>
        </div>

        <!-- 分页 -->
        <div v-if="totalPages > 1" class="pagination">
          <button 
            @click="currentPage--" 
            :disabled="currentPage === 1"
            class="btn btn-secondary"
          >
            上一页
          </button>
          <span class="page-info">第 {{ currentPage }} / {{ totalPages }} 页</span>
          <button 
            @click="currentPage++" 
            :disabled="currentPage === totalPages"
            class="btn btn-secondary"
          >
            下一页
          </button>
        </div>
      </div>
    </template>
    
    <div v-else class="empty-state">
      <h3>提交不存在</h3>
      <router-link :to="`/${project?.owner_name}/${project?.name}/-/commits`" class="btn btn-primary">
        返回提交列表
      </router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useRoute } from 'vue-router'
import api from '@/api'
import dayjs from 'dayjs'
import UnifiedDiffViewer from '@/components/diff/UnifiedDiffViewer.vue'
import { navIcons } from '@/navigation/icons'
import type { Project, CommitDetail } from '@/types'

const icons = navIcons

interface DiffFile {
  file_path: string
  status: string
  additions: number
  deletions: number
  diff: string
  original_content?: string
  modified_content?: string
  is_truncated: boolean
  total_lines?: number
}

const MAX_FILES_PER_PAGE = 20
const MAX_FILES_AUTO_EXPAND = 10

const props = defineProps<{
  project?: Project
}>()

const route = useRoute()

const loading = ref(false)
const commit = ref<CommitDetail | null>(null)
const diffs = ref<DiffFile[]>([])
const expandedFiles = ref<Set<number>>(new Set())
const currentPage = ref(1)

const totalPages = computed(() => Math.ceil(diffs.value.length / MAX_FILES_PER_PAGE))
const displayedDiffs = computed(() => {
  const start = (currentPage.value - 1) * MAX_FILES_PER_PAGE
  const end = start + MAX_FILES_PER_PAGE
  return diffs.value.slice(start, end)
})
const allExpanded = computed(() => {
  if (displayedDiffs.value.length === 0) return false
  return displayedDiffs.value.every((_, i) => expandedFiles.value.has(i))
})

function formatCommitDate(timestamp?: number) {
  if (!timestamp) return '-'
  return dayjs.unix(timestamp).format('YYYY-MM-DD HH:mm:ss')
}

function statusText(status: string) {
  const map: Record<string, string> = {
    added: '新增',
    modified: '修改',
    deleted: '删除',
    renamed: '重命名'
  }
  return map[status] || status
}

function getLanguageFromPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() || ''
  const langMap: Record<string, string> = {
    js: 'javascript',
    ts: 'typescript',
    jsx: 'javascript',
    tsx: 'typescript',
    vue: 'vue',
    py: 'python',
    rs: 'rust',
    go: 'go',
    java: 'java',
    c: 'c',
    cpp: 'cpp',
    cc: 'cpp',
    cxx: 'cpp',
    h: 'c',
    hpp: 'cpp',
    cs: 'csharp',
    rb: 'ruby',
    php: 'php',
    sh: 'shell',
    bash: 'shell',
    yml: 'yaml',
    yaml: 'yaml',
    json: 'json',
    xml: 'xml',
    html: 'html',
    css: 'css',
    scss: 'scss',
    sass: 'sass',
    less: 'less',
    md: 'markdown',
    sql: 'sql',
    txt: 'plaintext'
  }
  return langMap[ext] || 'plaintext'
}

function toggleFile(index: number) {
  if (expandedFiles.value.has(index)) {
    expandedFiles.value.delete(index)
  } else {
    expandedFiles.value.add(index)
  }
}

function toggleAllFiles() {
  if (allExpanded.value) {
    expandedFiles.value.clear()
  } else {
    displayedDiffs.value.forEach((_, i) => expandedFiles.value.add(i))
  }
}

function isBinaryFile(diff: DiffFile): boolean {
  // 检查是否是二进制文件（没有文本内容）
  return !diff.original_content && !diff.modified_content && diff.diff === ''
}

async function loadCommit() {
  const sha = route.params.sha as string
  if (!props.project?.owner_name || !props.project?.name || !sha) return
  
  loading.value = true
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    const detail = await api.commits.get(path, sha)
    commit.value = detail
    diffs.value = (detail.diffs || []).map(d => ({
      file_path: d.new_path || d.old_path,
      status: d.status.toLowerCase(),
      additions: d.additions || 0,
      deletions: d.deletions || 0,
      diff: d.diff,
      original_content: d.original_content,
      modified_content: d.modified_content,
      is_truncated: d.is_truncated,
      total_lines: d.total_lines
    }))
    
    // 重置状态
    expandedFiles.value.clear()
    currentPage.value = 1
    
    // 文件较少时自动展开
    if (diffs.value.length <= MAX_FILES_AUTO_EXPAND) {
      diffs.value.forEach((_, i) => expandedFiles.value.add(i))
    }
  } catch (error) {
    console.error('Failed to load commit:', error)
    commit.value = null
  } finally {
    loading.value = false
  }
}

watch([() => props.project?.owner_name, () => props.project?.name, () => route.params.sha], () => {
  loadCommit()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.commit-detail {
  padding: $spacing-lg;
}

.commit-header {
  margin-bottom: $spacing-xl;
  
  h2 {
    margin-bottom: $spacing-md;
  }
}

.commit-body {
  background: $bg-secondary;
  padding: $spacing-md;
  border-radius: $border-radius;
  font-family: inherit;
  white-space: pre-wrap;
  margin-bottom: $spacing-lg;
  color: $text-secondary;
}

.commit-info {
  display: flex;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: $spacing-lg;
  padding: $spacing-lg;
  background: $bg-secondary;
  border-radius: $border-radius;
}

.author {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  
  .avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: $primary-color;
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
  }
  
  .author-info {
    display: flex;
    flex-direction: column;
    
    span {
      color: $text-muted;
      font-size: $font-size-sm;
    }
  }
}

.commit-meta {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
}

.meta-item {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  font-size: $font-size-sm;
  
  .label {
    color: $text-muted;
  }
  
  code {
    font-size: $font-size-xs;
  }
}

.diff-section {
  .diff-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: $spacing-md;
    
    h3 {
      margin: 0;
    }
    
    .diff-actions {
      display: flex;
      gap: $spacing-sm;
    }
  }
}

.diff-file {
  margin-bottom: $spacing-lg;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  overflow: hidden;
}

.diff-file-header {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  padding: $spacing-sm $spacing-md;
  background: $bg-secondary;
  border-bottom: 1px solid $border-color;
  cursor: pointer;
  user-select: none;
  transition: background 0.2s;
  
  &:hover {
    background: darken($bg-secondary, 3%);
  }
  
  .expand-icon {
    width: 16px;
    text-align: center;
    color: $text-muted;
    font-size: 12px;
  }
}

.file-status {
  font-size: $font-size-xs;
  padding: 2px 6px;
  border-radius: 3px;
  
  &.added { background: rgba($success-color, 0.2); color: $success-color; }
  &.modified { background: rgba($warning-color, 0.2); color: $warning-color; }
  &.deleted { background: rgba($danger-color, 0.2); color: $danger-color; }
}

.file-path {
  flex: 1;
  font-family: 'JetBrains Mono', monospace;
  font-size: $font-size-sm;
}

.diff-stats {
  font-size: $font-size-sm;
  
  .additions { color: $success-color; margin-right: $spacing-sm; }
  .deletions { color: $danger-color; }
}

.no-diff {
  padding: $spacing-lg;
  text-align: center;
  color: $text-muted;
  font-style: italic;
}

.warning-banner {
  background: #fff3cd;
  border: 1px solid #ffc107;
  border-radius: $border-radius;
  padding: $spacing-lg;
  margin-bottom: $spacing-lg;
  
  .warning-content {
    display: flex;
    gap: $spacing-md;
    
    .warning-icon {
      width: 24px;
      height: 24px;
      color: #856404;
      flex-shrink: 0;
    }
    
    strong {
      display: block;
      margin-bottom: $spacing-sm;
      color: #856404;
    }
    
    p {
      margin: $spacing-sm 0;
      color: #856404;
    }
  }
  
  .warning-actions {
    display: flex;
    gap: $spacing-sm;
    margin-top: $spacing-md;
  }
}

.diff-content-wrapper {
  position: relative;
}

.diff-actions-bar {
  background: #f8f9fa;
  border-bottom: 1px solid $border-color;
  padding: $spacing-sm $spacing-md;
  display: flex;
  align-items: center;
  gap: $spacing-md;
  font-size: $font-size-sm;
  
  .btn-link {
    color: #0056b3;
    text-decoration: none;
    cursor: pointer;
    background: none;
    border: none;
    padding: $spacing-xs $spacing-sm;
    font-size: $font-size-sm;
    transition: all 0.2s;
    border-radius: $border-radius-sm;
    
    &:hover {
      background: #e7f3ff;
      color: #003d82;
    }
    
    &:disabled {
      color: $text-muted;
      cursor: not-allowed;
      opacity: 0.6;
    }
  }
  
  .truncate-hint {
    color: $text-muted;
    font-size: $font-size-xs;
    margin-left: auto;
  }
}

.large-file-warning {
  background: #e7f3ff;
  border-bottom: 1px solid #b3d9ff;
  padding: $spacing-sm $spacing-md;
  font-size: $font-size-sm;
  color: #004085;
  display: flex;
  align-items: center;
  justify-content: space-between;
  
  .btn-link {
    color: #0056b3;
    text-decoration: underline;
    cursor: pointer;
    background: none;
    border: none;
    padding: 0;
    font-size: $font-size-sm;
    
    &:hover {
      color: #003d82;
    }
  }
}

.pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: $spacing-md;
  margin-top: $spacing-xl;
  padding: $spacing-lg;
  
  .page-info {
    color: $text-muted;
    font-size: $font-size-sm;
  }
}
</style>
