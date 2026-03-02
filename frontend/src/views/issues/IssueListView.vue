<template>
  <div class="issue-list-view">
    <div class="page-header">
      <div class="header-content">
        <h1>议题</h1>
        <router-link :to="`/${$route.meta.namespace}/${$route.meta.projectName}/-/issues/new`" class="btn btn-primary">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          新建议题
        </router-link>
      </div>
    </div>

    <!-- Filters -->
    <div class="filters-bar">
      <div class="filter-tabs">
        <button 
          class="filter-tab" 
          :class="{ active: filter === 'open' }"
          @click="filter = 'open'"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          开放 <span class="count">{{ openCount }}</span>
        </button>
        <button 
          class="filter-tab"
          :class="{ active: filter === 'closed' }"
          @click="filter = 'closed'"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
            <path d="M5 8l2 2 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          已关闭 <span class="count">{{ closedCount }}</span>
        </button>
      </div>
      <div class="filter-actions">
        <input 
          type="text" 
          v-model="searchQuery"
          placeholder="搜索议题..."
          class="search-input"
        />
      </div>
    </div>

    <!-- Issues List -->
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>加载中...</span>
    </div>

    <div v-else-if="filteredIssues.length === 0" class="empty-state">
      <div class="empty-icon">
        <svg width="48" height="48" viewBox="0 0 16 16" fill="none">
          <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1"/>
          <path d="M8 5v3M8 10v.5" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
        </svg>
      </div>
      <h3>没有议题</h3>
      <p v-if="filter === 'open'">当前没有开放的议题</p>
      <p v-else>当前没有已关闭的议题</p>
      <router-link :to="`/${$route.meta.namespace}/${$route.meta.projectName}/-/issues/new`" class="btn btn-primary">
        创建第一个议题
      </router-link>
    </div>

    <div v-else class="issues-list">
      <div 
        v-for="issue in filteredIssues" 
        :key="issue.id" 
        class="issue-item"
      >
        <div class="issue-status">
          <svg v-if="issue.state === 'open'" class="status-icon open" width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          <svg v-else class="status-icon closed" width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
            <path d="M5 8l2 2 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <div class="issue-content">
          <div class="issue-title-row">
            <router-link 
              :to="`/${$route.meta.namespace}/${$route.meta.projectName}/-/issues/${issue.iid}`"
              class="issue-title"
            >
              {{ issue.title }}
            </router-link>
            <div class="issue-labels" v-if="issue.labels?.length">
              <span 
                v-for="label in issue.labels" 
                :key="label.id"
                class="label"
                :style="{ backgroundColor: label.color, color: getContrastColor(label.color) }"
              >
                {{ label.name }}
              </span>
            </div>
          </div>
          <div class="issue-meta">
            <span class="issue-number">#{{ issue.iid }}</span>
            <span class="separator">·</span>
            <span>{{ formatDate(issue.created_at) }} 由 {{ issue.author?.username }} 创建</span>
            <template v-if="issue.assignees?.length">
              <span class="separator">·</span>
              <span>指派给 {{ issue.assignees.map(a => a.username).join(', ') }}</span>
            </template>
          </div>
        </div>
        <div class="issue-stats">
          <span v-if="issue.comments_count" class="stat" title="评论">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <path d="M2 3h12v8H6l-4 3v-3H2V3z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            {{ issue.comments_count }}
          </span>
        </div>
      </div>
    </div>

    <!-- Pagination -->
    <div v-if="totalPages > 1" class="pagination">
      <button 
        class="btn btn-secondary"
        :disabled="page === 1"
        @click="page--"
      >
        上一页
      </button>
      <span class="page-info">第 {{ page }} / {{ totalPages }} 页</span>
      <button 
        class="btn btn-secondary"
        :disabled="page === totalPages"
        @click="page++"
      >
        下一页
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import apiClient from '@/api'

interface Label {
  id: number
  name: string
  color: string
}

interface User {
  id: number
  username: string
  avatar_url?: string
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
  comments_count: number
  created_at: string
  updated_at: string
  closed_at?: string
}

const route = useRoute()

const issues = ref<Issue[]>([])
const loading = ref(true)
const filter = ref<'open' | 'closed'>('open')
const searchQuery = ref('')
const page = ref(1)
const perPage = 20
const totalCount = ref(0)

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

const openCount = computed(() => issues.value.filter(i => i.state === 'open').length)
const closedCount = computed(() => issues.value.filter(i => i.state === 'closed').length)

const filteredIssues = computed(() => {
  let result = issues.value.filter(i => i.state === filter.value)
  
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(i => 
      i.title.toLowerCase().includes(query) ||
      i.description?.toLowerCase().includes(query)
    )
  }
  
  return result
})

const totalPages = computed(() => Math.ceil(totalCount.value / perPage))

async function loadIssues() {
  loading.value = true
  try {
    const response = await apiClient.client.get(
      `/projects/${owner.value}/${repo.value}/issues`,
      { params: { state: filter.value, page: page.value, per_page: perPage } }
    )
    issues.value = response.data.items || response.data || []
    totalCount.value = response.data.total || issues.value.length
  } catch (error) {
    console.error('Failed to load issues:', error)
    issues.value = []
  } finally {
    loading.value = false
  }
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

function getContrastColor(hexColor: string): string {
  const hex = hexColor.replace('#', '')
  const r = parseInt(hex.substring(0, 2), 16)
  const g = parseInt(hex.substring(2, 4), 16)
  const b = parseInt(hex.substring(4, 6), 16)
  const brightness = (r * 299 + g * 587 + b * 114) / 1000
  return brightness > 128 ? '#000000' : '#ffffff'
}

watch([filter, page], () => {
  loadIssues()
})

onMounted(() => {
  loadIssues()
})
</script>

<style lang="scss" scoped>
@import '@/styles/variables';

.issue-list-view {
  padding: $spacing-6;
}

.page-header {
  margin-bottom: $spacing-6;
  
  .header-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  
  h1 {
    font-size: $font-size-2xl;
    font-weight: $font-weight-semibold;
    margin: 0;
  }
  
  .btn svg {
    margin-right: $spacing-2;
  }
}

.filters-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: $spacing-4;
  padding: $spacing-3;
  background: $bg-secondary;
  border-radius: $radius-lg;
}

.filter-tabs {
  display: flex;
  gap: $spacing-2;
}

.filter-tab {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-2 $spacing-3;
  background: transparent;
  border: none;
  border-radius: $radius-md;
  color: $text-secondary;
  cursor: pointer;
  transition: all $transition-fast;
  
  &:hover {
    background: rgba(255, 255, 255, 0.05);
    color: $text-primary;
  }
  
  &.active {
    background: $bg-tertiary;
    color: $text-primary;
  }
  
  .count {
    padding: 0 $spacing-2;
    background: rgba(255, 255, 255, 0.1);
    border-radius: $radius-full;
    font-size: $font-size-xs;
  }
}

.search-input {
  padding: $spacing-2 $spacing-3;
  background: $bg-tertiary;
  border: 1px solid $border-color;
  border-radius: $radius-md;
  color: $text-primary;
  width: 200px;
  
  &:focus {
    outline: none;
    border-color: $brand-primary;
  }
}

.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: $spacing-12;
  text-align: center;
  color: $text-secondary;
}

.empty-icon {
  margin-bottom: $spacing-4;
  opacity: 0.5;
}

.empty-state h3 {
  margin: 0 0 $spacing-2;
  color: $text-primary;
}

.empty-state p {
  margin: 0 0 $spacing-4;
}

.issues-list {
  background: $bg-secondary;
  border-radius: $radius-lg;
  border: 1px solid $border-color;
  overflow: hidden;
}

.issue-item {
  display: flex;
  align-items: flex-start;
  gap: $spacing-3;
  padding: $spacing-4;
  border-bottom: 1px solid $border-color;
  transition: background $transition-fast;
  
  &:last-child {
    border-bottom: none;
  }
  
  &:hover {
    background: rgba(255, 255, 255, 0.02);
  }
}

.issue-status {
  flex-shrink: 0;
  padding-top: 2px;
  
  .status-icon {
    &.open {
      color: $color-success;
    }
    &.closed {
      color: $purple-500;
    }
  }
}

.issue-content {
  flex: 1;
  min-width: 0;
}

.issue-title-row {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  flex-wrap: wrap;
  margin-bottom: $spacing-1;
}

.issue-title {
  font-weight: $font-weight-semibold;
  color: $text-primary;
  text-decoration: none;
  
  &:hover {
    color: $brand-primary;
  }
}

.issue-labels {
  display: flex;
  gap: $spacing-1;
  flex-wrap: wrap;
}

.label {
  display: inline-block;
  padding: 2px $spacing-2;
  border-radius: $radius-full;
  font-size: $font-size-xs;
  font-weight: $font-weight-medium;
}

.issue-meta {
  font-size: $font-size-sm;
  color: $text-secondary;
  
  .separator {
    margin: 0 $spacing-1;
  }
  
  .issue-number {
    font-family: $font-mono;
  }
}

.issue-stats {
  display: flex;
  align-items: center;
  gap: $spacing-3;
  flex-shrink: 0;
  
  .stat {
    display: flex;
    align-items: center;
    gap: $spacing-1;
    color: $text-secondary;
    font-size: $font-size-sm;
  }
}

.pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: $spacing-4;
  margin-top: $spacing-6;
  
  .page-info {
    color: $text-secondary;
    font-size: $font-size-sm;
  }
}

.spinner {
  width: 24px;
  height: 24px;
  border: 2px solid rgba(255, 255, 255, 0.1);
  border-top-color: $brand-primary;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-bottom: $spacing-2;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
