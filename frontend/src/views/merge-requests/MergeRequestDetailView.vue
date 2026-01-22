<template>
  <div class="merge-request-detail">
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <template v-else-if="mergeRequest">
      <div class="mr-header">
        <div class="mr-title-row">
          <h2>{{ mergeRequest.title }}</h2>
          <span class="badge" :class="statusClass">{{ statusText }}</span>
        </div>
        <div class="mr-meta">
          <span class="mr-id">#{{ mergeRequest.iid }}</span>
          <span class="separator">·</span>
          <span>{{ mergeRequest.author_name }} 请求将</span>
          <code>{{ mergeRequest.source_branch }}</code>
          <span>合并到</span>
          <code>{{ mergeRequest.target_branch }}</code>
          <span class="separator">·</span>
          <span>{{ formatDate(mergeRequest.created_at) }}</span>
        </div>
      </div>
      
      <div class="mr-actions" v-if="mergeRequest.status === 'open'">
        <button class="btn btn-success" @click="handleMerge" :disabled="merging">
          {{ merging ? '合并中...' : '合并' }}
        </button>
        <button class="btn btn-danger" @click="handleClose">关闭</button>
      </div>
      
      <div class="mr-content">
        <div class="mr-description" v-if="mergeRequest.description">
          <h3>描述</h3>
          <div class="description-content">{{ mergeRequest.description }}</div>
        </div>
        
        <div class="mr-changes">
          <h3>更改 ({{ diffs.length }} 个文件)</h3>
          <div v-for="diff in diffs" :key="diff.file_path" class="diff-file">
            <div class="diff-file-header">
              <span class="file-status" :class="diff.status">{{ statusIcon(diff.status) }}</span>
              <span class="file-path">{{ diff.file_path }}</span>
              <span class="diff-stats">
                <span class="additions">+{{ diff.additions }}</span>
                <span class="deletions">-{{ diff.deletions }}</span>
              </span>
            </div>
            <pre class="diff-content"><code v-html="formatDiff(diff.diff)"></code></pre>
          </div>
        </div>
        
        <div class="mr-comments">
          <h3>评论 ({{ comments.length }})</h3>
          
          <div v-if="comments.length === 0" class="no-comments">
            暂无评论
          </div>
          
          <div v-else class="comment-list">
            <div v-for="comment in comments" :key="comment.id" class="comment-item">
              <div class="comment-avatar">{{ comment.author_name?.charAt(0).toUpperCase() }}</div>
              <div class="comment-body">
                <div class="comment-header">
                  <strong>{{ comment.author_name }}</strong>
                  <span class="time">{{ formatDate(comment.created_at) }}</span>
                </div>
                <div class="comment-content">{{ comment.content }}</div>
              </div>
            </div>
          </div>
          
          <form @submit.prevent="submitComment" class="comment-form">
            <textarea
              v-model="newComment"
              class="form-control"
              placeholder="添加评论..."
              rows="3"
            ></textarea>
            <button type="submit" class="btn btn-primary" :disabled="!newComment.trim() || commenting">
              {{ commenting ? '提交中...' : '提交评论' }}
            </button>
          </form>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import api from '@/api'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import type { Project, MergeRequest } from '@/types'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

interface DiffFile {
  file_path: string
  status: string
  additions: number
  deletions: number
  diff: string
}

interface Comment {
  id: string
  author_name: string
  content: string
  created_at: string
}

const props = defineProps<{
  project?: Project
}>()

const route = useRoute()
const router = useRouter()

const loading = ref(false)
const merging = ref(false)
const commenting = ref(false)
const mergeRequest = ref<MergeRequest | null>(null)
const diffs = ref<DiffFile[]>([])
const comments = ref<Comment[]>([])
const newComment = ref('')

const statusClass = computed(() => ({
  'badge-success': mergeRequest.value?.status === 'open',
  'badge-primary': mergeRequest.value?.status === 'merged',
  'badge-secondary': mergeRequest.value?.status === 'closed'
}))

const statusText = computed(() => {
  const map = { open: '开放', merged: '已合并', closed: '已关闭' }
  return map[mergeRequest.value?.status as keyof typeof map] || ''
})

function formatDate(date: string) {
  return dayjs(date).fromNow()
}

function statusIcon(status: string) {
  const map: Record<string, string> = {
    added: '+',
    modified: '~',
    deleted: '-'
  }
  return map[status] || '?'
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

async function loadMergeRequest() {
  const iid = route.params.iid
  if (!props.project?.id || !iid) return
  
  loading.value = true
  
  try {
    const [mrRes, commentsRes] = await Promise.all([
      api.getMergeRequest(props.project.id, Number(iid)),
      api.getMergeRequestComments(props.project.id, Number(iid))
    ])
    mergeRequest.value = mrRes.data
    comments.value = commentsRes.data
    
    // Load diffs
    if (mergeRequest.value) {
      const diffRes = await api.getMergeRequestDiff(props.project.id, Number(iid))
      diffs.value = diffRes.data
    }
  } catch (error) {
    console.error('Failed to load merge request:', error)
  } finally {
    loading.value = false
  }
}

async function handleMerge() {
  if (!props.project?.id || !mergeRequest.value) return
  if (!confirm('确定要合并此合并请求吗？')) return
  
  merging.value = true
  
  try {
    await api.mergeMergeRequest(props.project.id, mergeRequest.value.iid)
    loadMergeRequest()
  } catch (error) {
    console.error('Failed to merge:', error)
  } finally {
    merging.value = false
  }
}

async function handleClose() {
  if (!props.project?.id || !mergeRequest.value) return
  if (!confirm('确定要关闭此合并请求吗？')) return
  
  try {
    await api.closeMergeRequest(props.project.id, mergeRequest.value.iid)
    loadMergeRequest()
  } catch (error) {
    console.error('Failed to close:', error)
  }
}

async function submitComment() {
  if (!props.project?.id || !mergeRequest.value || !newComment.value.trim()) return
  
  commenting.value = true
  
  try {
    await api.createMergeRequestComment(props.project.id, mergeRequest.value.iid, {
      content: newComment.value
    })
    newComment.value = ''
    loadMergeRequest()
  } catch (error) {
    console.error('Failed to submit comment:', error)
  } finally {
    commenting.value = false
  }
}

watch([() => props.project?.id, () => route.params.iid], () => {
  loadMergeRequest()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.merge-request-detail {
  padding: $spacing-lg;
}

.mr-header {
  margin-bottom: $spacing-lg;
}

.mr-title-row {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  margin-bottom: $spacing-sm;
  
  h2 {
    margin: 0;
  }
}

.mr-meta {
  font-size: $font-size-sm;
  color: $text-muted;
  
  .separator {
    margin: 0 $spacing-xs;
  }
  
  .mr-id {
    font-weight: 500;
  }
  
  code {
    font-size: $font-size-xs;
    padding: 2px 6px;
  }
}

.mr-actions {
  display: flex;
  gap: $spacing-md;
  margin-bottom: $spacing-lg;
  padding-bottom: $spacing-lg;
  border-bottom: 1px solid $border-color;
}

.mr-description {
  margin-bottom: $spacing-xl;
  
  h3 {
    margin-bottom: $spacing-md;
  }
  
  .description-content {
    padding: $spacing-md;
    background: $bg-secondary;
    border-radius: $border-radius;
    white-space: pre-wrap;
  }
}

.mr-changes {
  margin-bottom: $spacing-xl;
  
  h3 {
    margin-bottom: $spacing-md;
  }
}

.diff-file {
  margin-bottom: $spacing-md;
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
  font-size: $font-size-sm;
}

.file-status {
  width: 20px;
  text-align: center;
  font-weight: bold;
  
  &.added { color: $success-color; }
  &.modified { color: $warning-color; }
  &.deleted { color: $danger-color; }
}

.file-path {
  flex: 1;
  font-family: 'JetBrains Mono', monospace;
}

.diff-stats {
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
  max-height: 400px;
  
  :deep(.line-add) { color: $success-color; display: block; }
  :deep(.line-del) { color: $danger-color; display: block; }
  :deep(.line-info) { color: $info-color; display: block; }
}

.mr-comments {
  h3 {
    margin-bottom: $spacing-md;
  }
}

.no-comments {
  padding: $spacing-lg;
  text-align: center;
  color: $text-muted;
  background: $bg-secondary;
  border-radius: $border-radius;
  margin-bottom: $spacing-lg;
}

.comment-list {
  margin-bottom: $spacing-lg;
}

.comment-item {
  display: flex;
  gap: $spacing-md;
  padding: $spacing-md;
  border-bottom: 1px solid $border-color;
  
  &:last-child {
    border-bottom: none;
  }
}

.comment-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: $primary-color;
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  flex-shrink: 0;
}

.comment-body {
  flex: 1;
}

.comment-header {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  margin-bottom: $spacing-xs;
  
  .time {
    color: $text-muted;
    font-size: $font-size-sm;
  }
}

.comment-content {
  color: $text-primary;
}

.comment-form {
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
  
  button {
    align-self: flex-end;
  }
}
</style>
