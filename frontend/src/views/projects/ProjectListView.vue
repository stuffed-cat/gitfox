<template>
  <div class="project-list-page">
    <div class="page-container">
      <!-- Page Header -->
      <div class="page-header">
        <div class="header-content">
          <h1>项目</h1>
          <p class="page-description">管理您的代码仓库和项目</p>
        </div>
        <div class="header-actions">
          <router-link to="/projects/new" class="btn btn-primary">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </svg>
            新建项目
          </router-link>
        </div>
      </div>
      
      <!-- Filters & Search -->
      <div class="toolbar">
        <div class="toolbar-left">
          <div class="search-box">
            <svg class="search-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.5"/>
              <path d="M11 11l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <input 
              v-model="searchQuery"
              type="text" 
              placeholder="按名称搜索项目..."
              class="search-input"
            />
          </div>
          
          <div class="filter-group">
            <select v-model="visibilityFilter" class="filter-select">
              <option value="">所有可见性</option>
              <option value="public">公开</option>
              <option value="private">私有</option>
              <option value="internal">内部</option>
            </select>
            
            <select v-model="sortBy" class="filter-select">
              <option value="updated">最近更新</option>
              <option value="name">名称</option>
              <option value="created">创建时间</option>
            </select>
          </div>
        </div>
        
        <div class="toolbar-right">
          <div class="view-toggle">
            <button 
              class="toggle-btn" 
              :class="{ active: viewMode === 'grid' }"
              @click="viewMode = 'grid'"
              title="网格视图"
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <rect x="2" y="2" width="5" height="5" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <rect x="9" y="2" width="5" height="5" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <rect x="2" y="9" width="5" height="5" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <rect x="9" y="9" width="5" height="5" rx="1" stroke="currentColor" stroke-width="1.5"/>
              </svg>
            </button>
            <button 
              class="toggle-btn" 
              :class="{ active: viewMode === 'list' }"
              @click="viewMode = 'list'"
              title="列表视图"
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M2 4h12M2 8h12M2 12h12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
            </button>
          </div>
        </div>
      </div>
      
      <!-- Loading State -->
      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <p>加载项目中...</p>
      </div>
      
      <!-- Empty State -->
      <div v-else-if="filteredProjects.length === 0" class="empty-state">
        <svg class="empty-icon" viewBox="0 0 64 64" fill="none">
          <rect x="8" y="12" width="48" height="40" rx="4" stroke="currentColor" stroke-width="2"/>
          <path d="M8 24h48M20 12v12M44 12v12" stroke="currentColor" stroke-width="2"/>
          <circle cx="32" cy="38" r="8" stroke="currentColor" stroke-width="2"/>
          <path d="M32 34v4M32 42v.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <h3>暂无项目</h3>
        <p>创建您的第一个项目开始使用 DevOps 进行版本管理和协作</p>
        <router-link to="/projects/new" class="btn btn-primary">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
          创建第一个项目
        </router-link>
      </div>
      
      <!-- Grid View -->
      <div v-else-if="viewMode === 'grid'" class="projects-grid">
        <router-link
          v-for="project in filteredProjects"
          :key="project.id"
          :to="`/projects/${project.slug}`"
          class="project-card"
        >
          <div class="project-card-header">
            <div class="project-avatar">
              {{ project.name.charAt(0).toUpperCase() }}
            </div>
            <div class="project-title">
              <h3>{{ project.name }}</h3>
              <span class="project-path">{{ project.slug }}</span>
            </div>
          </div>
          
          <p class="project-description">{{ project.description || '暂无描述' }}</p>
          
          <div class="project-stats">
            <div class="stat-item">
              <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
                <circle cx="4" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
                <circle cx="8" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/>
                <path d="M4 6v2a4 4 0 004 4" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>{{ project.default_branch }}</span>
            </div>
            <div class="stat-item">
              <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
                <path d="M8 5v3l2 2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              <span>{{ formatDate(project.updated_at) }}</span>
            </div>
          </div>
          
          <div class="project-card-footer">
            <span class="badge" :class="'badge-' + visibilityClass(project.visibility)">
              <svg v-if="project.visibility === 'private'" width="12" height="12" viewBox="0 0 16 16" fill="none">
                <rect x="4" y="7" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <path d="M6 7V5a2 2 0 014 0v2" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <svg v-else-if="project.visibility === 'public'" width="12" height="12" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
                <path d="M2 8h12M8 2a10 10 0 010 12M8 2a10 10 0 000 12" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <svg v-else width="12" height="12" viewBox="0 0 16 16" fill="none">
                <path d="M8 2v12M2 8h12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              {{ visibilityText(project.visibility) }}
            </span>
          </div>
        </router-link>
      </div>
      
      <!-- List View -->
      <div v-else class="projects-list">
        <div class="list-header">
          <span class="col-name">项目</span>
          <span class="col-visibility">可见性</span>
          <span class="col-branch">分支</span>
          <span class="col-updated">更新时间</span>
        </div>
        <router-link
          v-for="project in filteredProjects"
          :key="project.id"
          :to="`/projects/${project.slug}`"
          class="list-item"
        >
          <div class="col-name">
            <div class="project-avatar-sm">
              {{ project.name.charAt(0).toUpperCase() }}
            </div>
            <div class="project-info">
              <h3>{{ project.name }}</h3>
              <p>{{ project.description || '暂无描述' }}</p>
            </div>
          </div>
          <div class="col-visibility">
            <span class="badge" :class="'badge-' + visibilityClass(project.visibility)">
              {{ visibilityText(project.visibility) }}
            </span>
          </div>
          <div class="col-branch">
            <code>{{ project.default_branch }}</code>
          </div>
          <div class="col-updated">
            {{ formatDate(project.updated_at) }}
          </div>
        </router-link>
      </div>
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

const searchQuery = ref('')
const visibilityFilter = ref('')
const sortBy = ref('updated')
const viewMode = ref<'grid' | 'list'>('grid')

const projects = computed(() => projectStore.projects)
const loading = computed(() => projectStore.loading)

const filteredProjects = computed(() => {
  let result = [...projects.value]
  
  // Filter by search
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(p => 
      p.name.toLowerCase().includes(query) ||
      (p.description && p.description.toLowerCase().includes(query))
    )
  }
  
  // Filter by visibility
  if (visibilityFilter.value) {
    result = result.filter(p => p.visibility === visibilityFilter.value)
  }
  
  // Sort
  result.sort((a, b) => {
    switch (sortBy.value) {
      case 'name':
        return a.name.localeCompare(b.name)
      case 'created':
        return new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      case 'updated':
      default:
        return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
    }
  })
  
  return result
})

function visibilityClass(visibility: string) {
  const map: Record<string, string> = {
    public: 'success',
    private: 'secondary',
    internal: 'info'
  }
  return map[visibility] || 'secondary'
}

function visibilityText(visibility: string) {
  const map: Record<string, string> = {
    public: '公开',
    private: '私有',
    internal: '内部'
  }
  return map[visibility] || visibility
}

function formatDate(date: string) {
  return dayjs(date).fromNow()
}

onMounted(() => {
  projectStore.fetchProjects()
})
</script>

<style lang="scss" scoped>
.project-list-page {
  min-height: 100%;
}

.page-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: $spacing-6;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: $spacing-6;
  
  h1 {
    font-size: $font-size-3xl;
    font-weight: $font-weight-semibold;
    margin: 0 0 $spacing-1;
  }
  
  .page-description {
    color: $text-secondary;
    margin: 0;
  }
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: $spacing-4;
  margin-bottom: $spacing-6;
  flex-wrap: wrap;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: $spacing-4;
  flex-wrap: wrap;
}

.search-box {
  position: relative;
  width: 280px;
  
  .search-icon {
    position: absolute;
    left: $spacing-3;
    top: 50%;
    transform: translateY(-50%);
    color: $text-muted;
    pointer-events: none;
  }
  
  .search-input {
    width: 100%;
    padding: $spacing-2 $spacing-3 $spacing-2 36px;
    font-size: $font-size-sm;
    border: 1px solid $border-color;
    border-radius: $border-radius;
    background: $bg-primary;
    transition: all $transition-fast;
    
    &:focus {
      outline: none;
      border-color: $brand-primary;
      box-shadow: $shadow-focus;
    }
    
    &::placeholder {
      color: $text-muted;
    }
  }
}

.filter-group {
  display: flex;
  gap: $spacing-3;
}

.filter-select {
  padding: $spacing-2 $spacing-8 $spacing-2 $spacing-3;
  font-size: $font-size-sm;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  background: $bg-primary url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'%3E%3Cpath fill='%236b7280' d='M3 4.5L6 7.5L9 4.5'/%3E%3C/svg%3E") no-repeat right $spacing-3 center;
  cursor: pointer;
  appearance: none;
  
  &:focus {
    outline: none;
    border-color: $brand-primary;
  }
}

.view-toggle {
  display: flex;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  overflow: hidden;
}

.toggle-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  background: $bg-primary;
  border: none;
  color: $text-secondary;
  cursor: pointer;
  transition: all $transition-fast;
  
  &:hover {
    background: $bg-secondary;
    color: $text-primary;
  }
  
  &.active {
    background: $brand-primary;
    color: white;
  }
  
  &:not(:last-child) {
    border-right: 1px solid $border-color;
  }
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: $spacing-12;
  
  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid $border-color;
    border-top-color: $brand-primary;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: $spacing-4;
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
  
  .empty-icon {
    width: 80px;
    height: 80px;
    margin: 0 auto $spacing-6;
    color: $text-muted;
  }
  
  h3 {
    font-size: $font-size-xl;
    font-weight: $font-weight-semibold;
    margin: 0 0 $spacing-2;
  }
  
  p {
    color: $text-secondary;
    margin: 0 0 $spacing-6;
    max-width: 400px;
    margin-left: auto;
    margin-right: auto;
  }
}

.projects-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: $spacing-5;
}

.project-card {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
  padding: $spacing-5;
  text-decoration: none;
  color: inherit;
  transition: all $transition-fast;
  
  &:hover {
    border-color: $brand-primary;
    box-shadow: $shadow-md;
  }
}

.project-card-header {
  display: flex;
  align-items: center;
  gap: $spacing-3;
  margin-bottom: $spacing-4;
}

.project-avatar {
  width: 40px;
  height: 40px;
  border-radius: $border-radius;
  background: $brand-gradient;
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: $font-size-lg;
  font-weight: $font-weight-semibold;
  flex-shrink: 0;
}

.project-title {
  min-width: 0;
  
  h3 {
    font-size: $font-size-lg;
    font-weight: $font-weight-semibold;
    color: $text-primary;
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  
  .project-path {
    font-size: $font-size-sm;
    color: $text-secondary;
  }
}

.project-description {
  color: $text-secondary;
  font-size: $font-size-sm;
  line-height: 1.5;
  margin-bottom: $spacing-4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  min-height: 42px;
}

.project-stats {
  display: flex;
  gap: $spacing-4;
  margin-bottom: $spacing-4;
  padding-bottom: $spacing-4;
  border-bottom: 1px solid $border-color;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  font-size: $font-size-sm;
  color: $text-secondary;
  
  svg {
    color: $text-muted;
  }
}

.project-card-footer {
  display: flex;
  align-items: center;
  gap: $spacing-2;
}

.badge {
  display: inline-flex;
  align-items: center;
  gap: $spacing-1;
  padding: $spacing-1 $spacing-2;
  font-size: $font-size-xs;
  font-weight: $font-weight-medium;
  border-radius: $border-radius-full;
}

.badge-success {
  background: $color-success-light;
  color: darken($color-success, 10%);
}

.badge-secondary {
  background: $gray-100;
  color: $gray-600;
}

.badge-info {
  background: $color-info-light;
  color: darken($color-info, 10%);
}

// List View Styles
.projects-list {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
  overflow: hidden;
}

.list-header {
  display: grid;
  grid-template-columns: 1fr 100px 120px 140px;
  gap: $spacing-4;
  padding: $spacing-3 $spacing-5;
  background: $bg-secondary;
  font-size: $font-size-sm;
  font-weight: $font-weight-semibold;
  color: $text-secondary;
  border-bottom: 1px solid $border-color;
}

.list-item {
  display: grid;
  grid-template-columns: 1fr 100px 120px 140px;
  gap: $spacing-4;
  padding: $spacing-4 $spacing-5;
  text-decoration: none;
  color: inherit;
  border-bottom: 1px solid $border-color;
  transition: background $transition-fast;
  
  &:last-child {
    border-bottom: none;
  }
  
  &:hover {
    background: $bg-secondary;
  }
}

.col-name {
  display: flex;
  align-items: center;
  gap: $spacing-3;
  min-width: 0;
}

.project-avatar-sm {
  width: 32px;
  height: 32px;
  border-radius: $border-radius;
  background: $brand-gradient;
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: $font-size-sm;
  font-weight: $font-weight-semibold;
  flex-shrink: 0;
}

.project-info {
  min-width: 0;
  
  h3 {
    font-size: $font-size-base;
    font-weight: $font-weight-medium;
    color: $text-primary;
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  
  p {
    font-size: $font-size-sm;
    color: $text-secondary;
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
}

.col-visibility,
.col-branch,
.col-updated {
  display: flex;
  align-items: center;
  font-size: $font-size-sm;
}

.col-branch code {
  font-size: $font-size-xs;
  padding: 2px 6px;
  background: $bg-tertiary;
  border-radius: $border-radius-sm;
}

.col-updated {
  color: $text-secondary;
}
</style>

