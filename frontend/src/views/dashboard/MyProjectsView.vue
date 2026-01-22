<template>
  <div class="my-projects-page">
    <!-- Breadcrumb -->
    <div class="breadcrumb">
      <router-link to="/">你的工作</router-link>
      <span class="separator">/</span>
      <span>项目</span>
    </div>
    
    <!-- Page Header -->
    <div class="page-header">
      <h1>项目</h1>
      <div class="header-actions">
        <router-link to="/explore/projects" class="btn btn-outline">浏览项目</router-link>
        <router-link to="/projects/new" class="btn btn-primary">新建项目</router-link>
      </div>
    </div>
    
    <!-- Tabs -->
    <div class="tabs">
      <button :class="['tab', { active: activeTab === 'contributed' }]" @click="activeTab = 'contributed'">
        贡献者 <span class="count">{{ contributedCount }}</span>
      </button>
      <button :class="['tab', { active: activeTab === 'starred' }]" @click="activeTab = 'starred'">
        已加星标 <span class="count">{{ starredCount }}</span>
      </button>
      <button :class="['tab', { active: activeTab === 'personal' }]" @click="activeTab = 'personal'">
        个人 <span class="count">{{ personalCount }}</span>
      </button>
      <button :class="['tab', { active: activeTab === 'member' }]" @click="activeTab = 'member'">
        成员 <span class="count">{{ memberCount }}</span>
      </button>
    </div>
    
    <!-- Filter Bar -->
    <div class="filter-bar">
      <div class="search-box">
        <svg class="search-icon" viewBox="0 0 16 16" fill="none">
          <circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.5"/>
          <path d="M11 11l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <input v-model="searchQuery" type="text" placeholder="过滤或搜索（至少 3 个字符）" />
      </div>
      <div class="sort-options">
        <select v-model="sortBy" class="sort-select">
          <option value="name">名称</option>
          <option value="updated">最近更新</option>
          <option value="created">创建时间</option>
          <option value="stars">星标数</option>
        </select>
        <button class="sort-order" @click="toggleSortOrder">
          <svg :class="{ desc: sortOrder === 'desc' }" viewBox="0 0 16 16" fill="none">
            <path d="M4 6l4-4 4 4M4 10l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
    </div>
    
    <!-- Loading State -->
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>
    
    <!-- Empty State -->
    <div v-else-if="filteredProjects.length === 0" class="empty-state">
      <svg viewBox="0 0 64 64" fill="none" class="empty-icon">
        <rect x="8" y="12" width="48" height="40" rx="4" stroke="currentColor" stroke-width="2"/>
        <path d="M8 24h48" stroke="currentColor" stroke-width="2"/>
      </svg>
      <h3>暂无项目</h3>
      <p>创建您的第一个项目来开始使用</p>
      <router-link to="/projects/new" class="btn btn-primary">新建项目</router-link>
    </div>
    
    <!-- Project List -->
    <div v-else class="project-list">
      <router-link
        v-for="project in filteredProjects"
        :key="project.id"
        :to="`/${project.owner_name}/${project.slug}`"
        class="project-item"
      >
        <div class="project-avatar">
          <span>{{ project.name.charAt(0).toUpperCase() }}</span>
        </div>
        <div class="project-info">
          <div class="project-name">
            <span class="namespace">{{ project.owner_name }}</span>
            <span class="separator">/</span>
            <span class="name">{{ project.slug }}</span>
            <span v-if="project.visibility === 'private'" class="visibility-badge">
              <svg viewBox="0 0 16 16" fill="none">
                <rect x="4" y="7" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <path d="M6 7V5a2 2 0 014 0v2" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              所有者
            </span>
          </div>
          <p class="project-description">{{ project.description || '暂无描述' }}</p>
        </div>
        <div class="project-stats">
          <span class="stat">
            <svg viewBox="0 0 16 16" fill="none">
              <path d="M8 2l2 4 4.5.5-3.25 3 .75 4.5L8 12l-4 2 .75-4.5L1.5 6.5 6 6l2-4z" stroke="currentColor" stroke-width="1.5"/>
            </svg>
            0
          </span>
          <span class="stat">
            <svg viewBox="0 0 16 16" fill="none">
              <circle cx="4" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
              <circle cx="8" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/>
              <path d="M4 6v6" stroke="currentColor" stroke-width="1.5"/>
            </svg>
            0
          </span>
          <span class="stat">
            <svg viewBox="0 0 16 16" fill="none">
              <circle cx="4" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
              <circle cx="12" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
              <circle cx="8" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/>
              <path d="M4 6v2a4 4 0 004 4m4-6v2a4 4 0 01-4 4" stroke="currentColor" stroke-width="1.5"/>
            </svg>
            0
          </span>
          <span class="stat">
            <svg viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
              <circle cx="8" cy="8" r="2" fill="currentColor"/>
            </svg>
            0
          </span>
          <span class="updated-at">创建于 {{ formatDate(project.created_at) }}</span>
        </div>
      </router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useProjectStore } from '@/stores/project'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

const projectStore = useProjectStore()

const loading = computed(() => projectStore.loading)
const projects = computed(() => projectStore.projects)

const activeTab = ref('contributed')
const searchQuery = ref('')
const sortBy = ref('updated')
const sortOrder = ref<'asc' | 'desc'>('desc')

const contributedCount = computed(() => projects.value.length)
const starredCount = ref(0)
const personalCount = computed(() => projects.value.length)
const memberCount = computed(() => projects.value.length)

const filteredProjects = computed(() => {
  let result = [...projects.value]
  
  if (searchQuery.value.length >= 3) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(p => 
      p.name.toLowerCase().includes(query) || 
      p.slug.toLowerCase().includes(query) ||
      p.description?.toLowerCase().includes(query)
    )
  }
  
  result.sort((a, b) => {
    let cmp = 0
    if (sortBy.value === 'name') {
      cmp = a.name.localeCompare(b.name)
    } else if (sortBy.value === 'updated') {
      cmp = new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
    } else if (sortBy.value === 'created') {
      cmp = new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
    }
    return sortOrder.value === 'desc' ? cmp : -cmp
  })
  
  return result
})

function toggleSortOrder() {
  sortOrder.value = sortOrder.value === 'asc' ? 'desc' : 'asc'
}

function formatDate(date: string) {
  return dayjs(date).fromNow()
}

onMounted(() => {
  projectStore.fetchProjects()
})
</script>

<style lang="scss" scoped>
.my-projects-page {
  padding: $spacing-6;
  max-width: 1200px;
  margin: 0 auto;
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  margin-bottom: $spacing-4;
  font-size: $text-sm;
  color: $text-secondary;
  
  a {
    color: $color-primary;
    text-decoration: none;
    
    &:hover {
      text-decoration: underline;
    }
  }
  
  .separator {
    color: $text-muted;
  }
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: $spacing-6;
  
  h1 {
    font-size: $text-2xl;
    font-weight: 600;
    color: $text-primary;
  }
  
  .header-actions {
    display: flex;
    gap: $spacing-3;
  }
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
    display: flex;
    align-items: center;
    gap: $spacing-2;
    transition: all $transition-fast;
    
    &:hover {
      color: $text-primary;
    }
    
    &.active {
      color: $text-primary;
      border-bottom-color: $color-primary;
    }
    
    .count {
      background: $bg-tertiary;
      padding: 2px 6px;
      border-radius: $radius-full;
      font-size: $text-xs;
    }
  }
}

.filter-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: $spacing-4;
  gap: $spacing-4;
  
  .search-box {
    flex: 1;
    position: relative;
    
    .search-icon {
      position: absolute;
      left: $spacing-3;
      top: 50%;
      transform: translateY(-50%);
      width: 16px;
      height: 16px;
      color: $text-muted;
    }
    
    input {
      width: 100%;
      padding: $spacing-2 $spacing-3 $spacing-2 $spacing-10;
      border: 1px solid $border-color;
      border-radius: $radius-md;
      background: $bg-primary;
      color: $text-primary;
      
      &::placeholder {
        color: $text-muted;
      }
      
      &:focus {
        outline: none;
        border-color: $color-primary;
      }
    }
  }
  
  .sort-options {
    display: flex;
    align-items: center;
    gap: $spacing-2;
    
    .sort-select {
      padding: $spacing-2 $spacing-3;
      border: 1px solid $border-color;
      border-radius: $radius-md;
      background: $bg-primary;
      color: $text-primary;
      cursor: pointer;
    }
    
    .sort-order {
      padding: $spacing-2;
      border: 1px solid $border-color;
      border-radius: $radius-md;
      background: $bg-primary;
      cursor: pointer;
      
      svg {
        width: 16px;
        height: 16px;
        color: $text-secondary;
        transition: transform $transition-fast;
        
        &.desc {
          transform: rotate(180deg);
        }
      }
    }
  }
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: $spacing-12;
  
  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid $border-color;
    border-top-color: $color-primary;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }
}

.empty-state {
  text-align: center;
  padding: $spacing-12;
  
  .empty-icon {
    width: 64px;
    height: 64px;
    margin: 0 auto $spacing-4;
    color: $text-muted;
  }
  
  h3 {
    font-size: $text-lg;
    color: $text-primary;
    margin-bottom: $spacing-2;
  }
  
  p {
    color: $text-secondary;
    margin-bottom: $spacing-4;
  }
}

.project-list {
  .project-item {
    display: flex;
    align-items: flex-start;
    gap: $spacing-4;
    padding: $spacing-4;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    margin-bottom: $spacing-3;
    text-decoration: none;
    transition: all $transition-fast;
    
    &:hover {
      border-color: $color-primary;
      background: $bg-secondary;
    }
    
    .project-avatar {
      width: 48px;
      height: 48px;
      border-radius: $radius-md;
      background: linear-gradient(135deg, $color-primary, $color-primary-dark);
      display: flex;
      align-items: center;
      justify-content: center;
      color: white;
      font-weight: 600;
      font-size: $text-lg;
      flex-shrink: 0;
    }
    
    .project-info {
      flex: 1;
      min-width: 0;
      
      .project-name {
        display: flex;
        align-items: center;
        gap: $spacing-1;
        margin-bottom: $spacing-1;
        
        .namespace {
          color: $text-secondary;
        }
        
        .separator {
          color: $text-muted;
        }
        
        .name {
          color: $text-primary;
          font-weight: 600;
        }
        
        .visibility-badge {
          display: inline-flex;
          align-items: center;
          gap: 4px;
          margin-left: $spacing-2;
          padding: 2px 6px;
          background: $bg-tertiary;
          border-radius: $radius-sm;
          font-size: $text-xs;
          color: $text-secondary;
          
          svg {
            width: 12px;
            height: 12px;
          }
        }
      }
      
      .project-description {
        color: $text-secondary;
        font-size: $text-sm;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
    }
    
    .project-stats {
      display: flex;
      align-items: center;
      gap: $spacing-4;
      color: $text-muted;
      font-size: $text-sm;
      
      .stat {
        display: flex;
        align-items: center;
        gap: 4px;
        
        svg {
          width: 14px;
          height: 14px;
        }
      }
      
      .updated-at {
        color: $text-muted;
      }
    }
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
