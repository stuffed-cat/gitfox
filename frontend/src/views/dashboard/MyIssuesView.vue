<template>
  <div class="my-issues-page">
    <div class="page-header">
      <h1>议题</h1>
    </div>
    
    <div class="tabs">
      <button 
        :class="['tab', { active: activeTab === 'assigned' }]" 
        @click="switchTab('assigned')"
      >
        分配给我 <span v-if="!loading && activeTab === 'assigned'" class="count">({{ issues.length }})</span>
      </button>
      <button 
        :class="['tab', { active: activeTab === 'created' }]" 
        @click="switchTab('created')"
      >
        我创建的 <span v-if="!loading && activeTab === 'created'" class="count">({{ issues.length }})</span>
      </button>
    </div>

    <div class="state-filter">
      <button 
        v-for="state in states"
        :key="state.key"
        :class="['state-btn', { active: activeState === state.key }]"
        @click="switchState(state.key)"
      >
        {{ state.label }}
      </button>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="spinner"></div>
      <p>加载中...</p>
    </div>
    
    <div v-else-if="issues.length === 0" class="empty-state">
      <h3>暂无议题</h3>
      <p v-if="activeTab === 'assigned'">当有议题分配给您时，它们会显示在这里</p>
      <p v-else>您还没有创建任何议题</p>
    </div>

    <div v-else class="issues-list">
      <div 
        v-for="issue in issues" 
        :key="issue.id"
        class="issue-item"
      >
        <router-link 
          :to="`/${issue.namespace_path}/${issue.project_name}/-/issues/${issue.iid}`"
          class="issue-link"
        >
          <div class="issue-header">
            <h3 class="issue-title">{{ issue.title }}</h3>
            <span :class="['issue-state', issue.state]">{{ issueStateLabel(issue.state) }}</span>
          </div>
          
          <div class="issue-meta">
            <span class="project-name">{{ issue.namespace_path }}/{{ issue.project_name }}</span>
            <span class="separator">•</span>
            <span class="issue-number">#{{ issue.iid }}</span>
            <span class="separator">•</span>
            <span class="author">由 {{ issue.author.username }} 创建</span>
            <span class="separator">•</span>
            <span class="time">{{ formatTime(issue.created_at) }}</span>
            <span v-if="issue.comment_count > 0" class="comments">
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                <path d="M2 2h10v8H7l-3 2v-2H2V2z" stroke="currentColor" stroke-width="1.5" fill="none"/>
              </svg>
              {{ issue.comment_count }}
            </span>
          </div>

          <div v-if="issue.labels && issue.labels.length > 0" class="labels">
            <span v-for="label in issue.labels" :key="label" class="label">{{ label }}</span>
          </div>
        </router-link>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '@/api'

const activeTab = ref<'assigned' | 'created'>('assigned')
const activeState = ref<'open' | 'closed' | 'all'>('open')
const loading = ref(false)
const issues = ref<any[]>([])

const states = [
  { key: 'open' as const, label: '进行中' },
  { key: 'closed' as const, label: '已关闭' },
  { key: 'all' as const, label: '全部' }
]

async function loadIssues() {
  loading.value = true
  try {
    issues.value = await api.issues.my({
      scope: activeTab.value,
      state: activeState.value
    })
  } catch (error) {
    console.error('Failed to load issues:', error)
    issues.value = []
  } finally {
    loading.value = false
  }
}

function switchTab(tab: 'assigned' | 'created') {
  activeTab.value = tab
  loadIssues()
}

function switchState(state: 'open' | 'closed' | 'all') {
  activeState.value = state
  loadIssues()
}

function issueStateLabel(state: string): string {
  return state === 'open' ? '进行中' : '已关闭'
}

function formatTime(dateString: string): string {
  const date = new Date(dateString)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))
  
  if (days === 0) {
    const hours = Math.floor(diff / (1000 * 60 * 60))
    if (hours === 0) {
      const minutes = Math.floor(diff / (1000 * 60))
      return minutes === 0 ? '刚刚' : `${minutes} 分钟前`
    }
    return `${hours} 小时前`
  } else if (days === 1) {
    return '昨天'
  } else if (days < 7) {
    return `${days} 天前`
  } else {
    return date.toLocaleDateString('zh-CN')
  }
}

onMounted(() => {
  loadIssues()
})
</script>

<style lang="scss" scoped>
@import '@/styles/variables.scss';

.my-issues-page {
  padding: $spacing-6;
  max-width: 1200px;
  margin: 0 auto;
}

.page-header {
  margin-bottom: $spacing-6;
  h1 { font-size: $text-2xl; font-weight: 600; }
}

.tabs {
  display: flex;
  gap: $spacing-1;
  border-bottom: 1px solid $border-color;
  margin-bottom: $spacing-4;
  
  .tab {
    padding: $spacing-3 $spacing-4;
    background: none;
    border: none;
    color: $text-secondary;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    font-size: $text-base;
    
    &:hover { color: $text-primary; }
    &.active { 
      color: $color-primary; 
      border-bottom-color: $color-primary;
      font-weight: 500;
    }

    .count {
      color: $text-muted;
      font-size: $text-sm;
    }
  }
}

.state-filter {
  display: flex;
  gap: $spacing-2;
  margin-bottom: $spacing-6;

  .state-btn {
    padding: $spacing-2 $spacing-3;
    background: $bg-secondary;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    color: $text-secondary;
    cursor: pointer;
    font-size: $text-sm;
    transition: all 0.2s;

    &:hover {
      background: $bg-tertiary;
      border-color: $color-primary;
    }

    &.active {
      background: $color-primary;
      color: white;
      border-color: $color-primary;
    }
  }
}

.loading {
  text-align: center;
  padding: $spacing-12;

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid $bg-secondary;
    border-top-color: $color-primary;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto $spacing-4;
  }

  p {
    color: $text-secondary;
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.empty-state {
  text-align: center;
  padding: $spacing-12;
  
  h3 { 
    color: $text-primary; 
    margin-bottom: $spacing-2;
    font-size: $text-lg;
  }
  
  p { 
    color: $text-secondary;
    font-size: $text-base;
  }
}

.issues-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-3;
}

.issue-item {
  background: white;
  border: 1px solid $border-color;
  border-radius: $radius-md;
  transition: all 0.2s;

  &:hover {
    border-color: $color-primary;
    box-shadow: $shadow-sm;
  }
}

.issue-link {
  display: block;
  padding: $spacing-4;
  text-decoration: none;
  color: inherit;
}

.issue-header {
  display: flex;
  align-items: flex-start;
  gap: $spacing-3;
  margin-bottom: $spacing-2;
}

.issue-title {
  flex: 1;
  font-size: $text-base;
  font-weight: 500;
  color: $text-primary;
  margin: 0;
  
  &:hover {
    color: $color-primary;
  }
}

.issue-state {
  padding: 2px 8px;
  border-radius: $radius-sm;
  font-size: $text-xs;
  font-weight: 500;
  white-space: nowrap;

  &.open {
    background: $success-light;
    color: $success-dark;
  }

  &.closed {
    background: $bg-secondary;
    color: $text-secondary;
  }
}

.issue-meta {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  font-size: $text-sm;
  color: $text-secondary;
  margin-bottom: $spacing-2;

  .project-name {
    font-weight: 500;
    color: $text-primary;
  }

  .separator {
    color: $border-color;
  }

  .comments {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: $spacing-2;
    
    svg {
      opacity: 0.5;
    }
  }
}

.labels {
  display: flex;
  flex-wrap: wrap;
  gap: $spacing-2;
}

.label {
  display: inline-block;
  padding: 2px 8px;
  background: $bg-secondary;
  border-radius: $radius-sm;
  font-size: $text-xs;
  color: $text-secondary;
}
</style>
