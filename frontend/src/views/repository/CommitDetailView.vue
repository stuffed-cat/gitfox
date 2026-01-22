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
              <span>{{ formatDate(commit.committed_at) }}</span>
            </div>
            <div v-if="commit.parent_sha" class="meta-item">
              <span class="label">父提交:</span>
              <router-link :to="`/${project?.owner_name}/${project?.slug}/-/commit/${commit.parent_sha}`">
                {{ commit.parent_sha.substring(0, 8) }}
              </router-link>
            </div>
          </div>
        </div>
      </div>
      
      <div class="diff-section">
        <h3>更改的文件 ({{ diffs.length }})</h3>
        
        <div v-for="diff in diffs" :key="diff.file_path" class="diff-file">
          <div class="diff-file-header">
            <span class="file-status" :class="diff.status">{{ statusText(diff.status) }}</span>
            <span class="file-path">{{ diff.file_path }}</span>
            <span class="diff-stats">
              <span class="additions">+{{ diff.additions }}</span>
              <span class="deletions">-{{ diff.deletions }}</span>
            </span>
          </div>
          <pre class="diff-content"><code v-html="formatDiff(diff.diff)"></code></pre>
        </div>
      </div>
    </template>
    
    <div v-else class="empty-state">
      <h3>提交不存在</h3>
      <router-link :to="`/${project?.owner_name}/${project?.slug}/-/commits`" class="btn btn-primary">
        返回提交列表
      </router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import api from '@/api'
import dayjs from 'dayjs'
import type { Project, Commit } from '@/types'

interface DiffFile {
  file_path: string
  status: string
  additions: number
  deletions: number
  diff: string
}

const props = defineProps<{
  project?: Project
}>()

const route = useRoute()

const loading = ref(false)
const commit = ref<Commit | null>(null)
const diffs = ref<DiffFile[]>([])

function formatDate(date: string) {
  return dayjs(date).format('YYYY-MM-DD HH:mm:ss')
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

function formatDiff(diff: string) {
  if (!diff) return ''
  return diff
    .split('\n')
    .map(line => {
      if (line.startsWith('+') && !line.startsWith('+++')) {
        return `<span class="line-add">${escapeHtml(line)}</span>`
      }
      if (line.startsWith('-') && !line.startsWith('---')) {
        return `<span class="line-del">${escapeHtml(line)}</span>`
      }
      if (line.startsWith('@@')) {
        return `<span class="line-info">${escapeHtml(line)}</span>`
      }
      return escapeHtml(line)
    })
    .join('\n')
}

function escapeHtml(text: string) {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
}

async function loadCommit() {
  const sha = route.params.sha as string
  if (!props.project?.id || !sha) return
  
  loading.value = true
  
  try {
    const [commitRes, diffRes] = await Promise.all([
      api.getCommit(props.project.id, sha),
      api.getDiff(props.project.id, sha)
    ])
    commit.value = commitRes.data
    diffs.value = diffRes.data
  } catch (error) {
    console.error('Failed to load commit:', error)
    commit.value = null
  } finally {
    loading.value = false
  }
}

watch([() => props.project?.id, () => route.params.sha], () => {
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
  h3 {
    margin-bottom: $spacing-md;
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

.diff-content {
  margin: 0;
  padding: $spacing-md;
  overflow-x: auto;
  font-family: 'JetBrains Mono', monospace;
  font-size: $font-size-xs;
  line-height: 1.6;
  
  :deep(.line-add) { color: $success-color; display: block; }
  :deep(.line-del) { color: $danger-color; display: block; }
  :deep(.line-info) { color: $info-color; display: block; }
}
</style>
