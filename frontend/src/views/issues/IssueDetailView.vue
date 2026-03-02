<template>
  <div class="issue-detail-view" v-if="issue">
    <div class="issue-header">
      <div class="issue-title-section">
        <h1>
          {{ issue.title }}
          <span class="issue-number">#{{ issue.iid }}</span>
        </h1>
        <div class="issue-status-badge" :class="issue.state">
          <svg v-if="issue.state === 'open'" width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          <svg v-else width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
            <path d="M5 8l2 2 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          {{ issue.state === 'open' ? '开放' : '已关闭' }}
        </div>
      </div>
      <div class="issue-meta">
        由 <strong>{{ issue.author?.username }}</strong> 创建于 {{ formatDate(issue.created_at) }}
        <template v-if="issue.closed_at">
          · 关闭于 {{ formatDate(issue.closed_at) }}
        </template>
      </div>
    </div>

    <div class="issue-content-grid">
      <div class="issue-main">
        <!-- Issue Description -->
        <div class="issue-description card">
          <div class="card-header">
            <div class="author-info">
              <span class="avatar">{{ issue.author?.username?.charAt(0).toUpperCase() }}</span>
              <span class="author-name">{{ issue.author?.username }}</span>
              <span class="timestamp">{{ formatDate(issue.created_at) }}</span>
            </div>
          </div>
          <div class="card-body">
            <div v-if="issue.description" class="markdown-body" v-html="renderMarkdown(issue.description)"></div>
            <div v-else class="no-description">没有描述</div>
          </div>
        </div>

        <!-- Comments -->
        <div class="comments-section">
          <div v-for="comment in comments" :key="comment.id" class="comment card">
            <div class="card-header">
              <div class="author-info">
                <span class="avatar">{{ comment.author?.username?.charAt(0).toUpperCase() }}</span>
                <span class="author-name">{{ comment.author?.username }}</span>
                <span class="timestamp">{{ formatDate(comment.created_at) }}</span>
              </div>
            </div>
            <div class="card-body">
              <div class="markdown-body" v-html="renderMarkdown(comment.body)"></div>
            </div>
          </div>
        </div>

        <!-- Add Comment -->
        <div class="add-comment card">
          <div class="card-header">
            <h3>添加评论</h3>
          </div>
          <div class="card-body">
            <textarea
              v-model="newComment"
              class="form-control"
              rows="4"
              placeholder="写下你的评论..."
            ></textarea>
            <div class="comment-actions">
              <button 
                v-if="issue.state === 'open'"
                class="btn btn-secondary"
                @click="closeIssue"
                :disabled="submitting"
              >
                关闭议题
              </button>
              <button 
                v-else
                class="btn btn-secondary"
                @click="reopenIssue"
                :disabled="submitting"
              >
                重新打开
              </button>
              <button 
                class="btn btn-primary"
                @click="addComment"
                :disabled="!newComment.trim() || submitting"
              >
                {{ submitting ? '提交中...' : '评论' }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <div class="issue-sidebar">
        <div class="sidebar-section">
          <label>指派给</label>
          <div v-if="issue.assignees?.length" class="assignees-list">
            <div v-for="assignee in issue.assignees" :key="assignee.id" class="assignee">
              <span class="avatar avatar-sm">{{ assignee.username?.charAt(0).toUpperCase() }}</span>
              {{ assignee.username }}
            </div>
          </div>
          <div v-else class="empty-value">无</div>
        </div>

        <div class="sidebar-section">
          <label>标签</label>
          <div v-if="issue.labels?.length" class="labels-list">
            <span 
              v-for="label in issue.labels" 
              :key="label.id"
              class="label"
              :style="{ backgroundColor: label.color, color: getContrastColor(label.color) }"
            >
              {{ label.name }}
            </span>
          </div>
          <div v-else class="empty-value">无</div>
        </div>

        <div class="sidebar-section">
          <label>里程碑</label>
          <div v-if="issue.milestone" class="milestone">
            {{ issue.milestone.title }}
          </div>
          <div v-else class="empty-value">无</div>
        </div>

        <div class="sidebar-section">
          <label>创建时间</label>
          <div>{{ formatFullDate(issue.created_at) }}</div>
        </div>

        <div class="sidebar-section">
          <label>更新时间</label>
          <div>{{ formatFullDate(issue.updated_at) }}</div>
        </div>
      </div>
    </div>
  </div>
  
  <div v-else-if="loading" class="loading-state">
    <div class="spinner"></div>
    <span>加载中...</span>
  </div>
  
  <div v-else class="error-state">
    <h2>议题未找到</h2>
    <router-link :to="`/${$route.meta.namespace}/${$route.meta.projectName}/-/issues`" class="btn btn-primary">
      返回议题列表
    </router-link>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import apiClient from '@/api'

interface User {
  id: number
  username: string
}

interface Label {
  id: number
  name: string
  color: string
}

interface Milestone {
  id: number
  title: string
}

interface Issue {
  id: number
  iid: number
  title: string
  description: string
  state: 'open' | 'closed'
  author: User
  assignees: User[]
  labels: Label[]
  milestone: Milestone | null
  created_at: string
  updated_at: string
  closed_at?: string
}

interface Comment {
  id: number
  body: string
  author: User
  created_at: string
}

const route = useRoute()

const issue = ref<Issue | null>(null)
const comments = ref<Comment[]>([])
const loading = ref(true)
const submitting = ref(false)
const newComment = ref('')

const owner = computed(() => {
  const segments = route.params.pathSegments as string[]
  if (!segments || segments.length < 2) return ''
  return segments.slice(0, -1).join('/')
})
const repo = computed(() => {
  const segments = route.params.pathSegments as string[]
  if (!segments || segments.length < 2) return ''
  return segments[segments.length - 1]
})
const iid = computed(() => route.params.iid as string)

async function loadIssue() {
  loading.value = true
  try {
    const [issueRes, commentsRes] = await Promise.all([
      apiClient.client.get(`/projects/${owner.value}/${repo.value}/issues/${iid.value}`),
      apiClient.client.get(`/projects/${owner.value}/${repo.value}/issues/${iid.value}/notes`).catch(() => ({ data: [] }))
    ])
    
    issue.value = issueRes.data
    comments.value = commentsRes.data || []
  } catch (error) {
    console.error('Failed to load issue:', error)
    issue.value = null
  } finally {
    loading.value = false
  }
}

async function addComment() {
  if (!newComment.value.trim()) return
  
  submitting.value = true
  try {
    const response = await apiClient.client.post(
      `/projects/${owner.value}/${repo.value}/issues/${iid.value}/notes`,
      { body: newComment.value }
    )
    comments.value.push(response.data)
    newComment.value = ''
  } catch (error) {
    console.error('Failed to add comment:', error)
  } finally {
    submitting.value = false
  }
}

async function closeIssue() {
  submitting.value = true
  try {
    const response = await apiClient.client.put(
      `/projects/${owner.value}/${repo.value}/issues/${iid.value}`,
      { state: 'closed' }
    )
    issue.value = response.data
  } catch (error) {
    console.error('Failed to close issue:', error)
  } finally {
    submitting.value = false
  }
}

async function reopenIssue() {
  submitting.value = true
  try {
    const response = await apiClient.client.put(
      `/projects/${owner.value}/${repo.value}/issues/${iid.value}`,
      { state: 'open' }
    )
    issue.value = response.data
  } catch (error) {
    console.error('Failed to reopen issue:', error)
  } finally {
    submitting.value = false
  }
}

function renderMarkdown(text: string): string {
  // Simple markdown rendering - in production use a proper library
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.*?)\*/g, '<em>$1</em>')
    .replace(/`(.*?)`/g, '<code>$1</code>')
    .replace(/\n/g, '<br>')
}

function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))
  
  if (days === 0) return '今天'
  if (days === 1) return '昨天'
  if (days < 7) return `${days} 天前`
  if (days < 30) return `${Math.floor(days / 7)} 周前`
  if (days < 365) return `${Math.floor(days / 30)} 个月前`
  return `${Math.floor(days / 365)} 年前`
}

function formatFullDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString('zh-CN')
}

function getContrastColor(hexColor: string): string {
  const hex = hexColor.replace('#', '')
  const r = parseInt(hex.substring(0, 2), 16)
  const g = parseInt(hex.substring(2, 4), 16)
  const b = parseInt(hex.substring(4, 6), 16)
  const brightness = (r * 299 + g * 587 + b * 114) / 1000
  return brightness > 128 ? '#000000' : '#ffffff'
}

onMounted(() => {
  loadIssue()
})
</script>

<style lang="scss" scoped>
.issue-detail-view {
  padding: $spacing-6;
}

.issue-header {
  margin-bottom: $spacing-6;
  padding-bottom: $spacing-4;
  border-bottom: 1px solid $border-color;
}

.issue-title-section {
  display: flex;
  align-items: flex-start;
  gap: $spacing-3;
  margin-bottom: $spacing-2;
  
  h1 {
    flex: 1;
    font-size: $font-size-2xl;
    font-weight: $font-weight-semibold;
    margin: 0;
    line-height: 1.3;
  }
  
  .issue-number {
    color: $text-secondary;
    font-weight: normal;
  }
}

.issue-status-badge {
  display: flex;
  align-items: center;
  gap: $spacing-1;
  padding: $spacing-1 $spacing-3;
  border-radius: $radius-full;
  font-size: $font-size-sm;
  font-weight: $font-weight-medium;
  white-space: nowrap;
  
  &.open {
    background: rgba($success, 0.15);
    color: $success;
  }
  
  &.closed {
    background: rgba($purple, 0.15);
    color: $purple;
  }
}

.issue-meta {
  color: $text-secondary;
  font-size: $font-size-sm;
}

.issue-content-grid {
  display: grid;
  grid-template-columns: 1fr 280px;
  gap: $spacing-6;
}

.issue-main {
  min-width: 0;
}

.issue-sidebar {
  .sidebar-section {
    margin-bottom: $spacing-5;
    
    label {
      display: block;
      margin-bottom: $spacing-2;
      font-weight: $font-weight-medium;
      color: $text-secondary;
      font-size: $font-size-sm;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }
  }
  
  .empty-value {
    color: $text-tertiary;
  }
}

.assignees-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-2;
}

.assignee {
  display: flex;
  align-items: center;
  gap: $spacing-2;
}

.labels-list {
  display: flex;
  flex-wrap: wrap;
  gap: $spacing-1;
}

.label {
  display: inline-block;
  padding: 2px $spacing-2;
  border-radius: $radius-full;
  font-size: $font-size-xs;
  font-weight: $font-weight-medium;
}

.card {
  background: $bg-secondary;
  border: 1px solid $border-color;
  border-radius: $radius-lg;
  margin-bottom: $spacing-4;
}

.card-header {
  padding: $spacing-3 $spacing-4;
  border-bottom: 1px solid $border-color;
  
  h3 {
    margin: 0;
    font-size: $font-size-base;
  }
}

.card-body {
  padding: $spacing-4;
}

.author-info {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  
  .author-name {
    font-weight: $font-weight-medium;
  }
  
  .timestamp {
    color: $text-secondary;
    font-size: $font-size-sm;
  }
}

.avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: linear-gradient(135deg, $primary, $purple);
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: $font-weight-semibold;
  font-size: $font-size-sm;
  
  &.avatar-sm {
    width: 24px;
    height: 24px;
    font-size: $font-size-xs;
  }
}

.markdown-body {
  line-height: 1.6;
  
  code {
    background: rgba(255, 255, 255, 0.1);
    padding: 2px 6px;
    border-radius: $radius-sm;
    font-family: $font-mono;
    font-size: 0.9em;
  }
}

.no-description {
  color: $text-tertiary;
  font-style: italic;
}

.add-comment {
  .form-control {
    width: 100%;
    padding: $spacing-3;
    background: $bg-tertiary;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    color: $text-primary;
    resize: vertical;
    min-height: 100px;
    
    &:focus {
      outline: none;
      border-color: $primary;
    }
  }
  
  .comment-actions {
    display: flex;
    justify-content: flex-end;
    gap: $spacing-2;
    margin-top: $spacing-3;
  }
}

.loading-state,
.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: $spacing-12;
  text-align: center;
}

.spinner {
  width: 24px;
  height: 24px;
  border: 2px solid rgba(255, 255, 255, 0.1);
  border-top-color: $primary;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-bottom: $spacing-2;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@media (max-width: 768px) {
  .issue-content-grid {
    grid-template-columns: 1fr;
  }
  
  .issue-sidebar {
    order: -1;
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: $spacing-4;
    padding: $spacing-4;
    background: $bg-secondary;
    border-radius: $radius-lg;
    margin-bottom: $spacing-4;
    
    .sidebar-section {
      margin-bottom: 0;
    }
  }
}
</style>
