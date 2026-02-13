<template>
  <div class="explore-groups-page">
    <div class="page-header">
      <h1>探索群组</h1>
      <p class="page-desc">发现公开的群组和组织</p>
    </div>
    
    <div class="filter-bar">
      <div class="search-wrap">
        <svg viewBox="0 0 16 16" fill="none" class="search-icon">
          <circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.5"/>
          <path d="M11 11l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <input v-model="searchQuery" type="text" placeholder="按名称搜索群组..." class="search-input" />
      </div>
      <div class="sort-wrap">
        <select v-model="sortBy" class="sort-select">
          <option value="name">按名称排序</option>
          <option value="created">按创建时间排序</option>
          <option value="updated">最近更新</option>
        </select>
      </div>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <p>加载中...</p>
    </div>
    
    <div v-else-if="filteredGroups.length === 0 && !searchQuery" class="empty-state">
      <svg viewBox="0 0 64 64" fill="none" class="empty-icon">
        <rect x="8" y="12" width="20" height="20" rx="4" stroke="currentColor" stroke-width="2"/>
        <rect x="36" y="12" width="20" height="20" rx="4" stroke="currentColor" stroke-width="2"/>
        <rect x="22" y="32" width="20" height="20" rx="4" stroke="currentColor" stroke-width="2"/>
      </svg>
      <h3>暂无公开群组</h3>
      <p>创建第一个公开群组来开始协作</p>
      <router-link to="/groups/new" class="btn btn-primary">新建群组</router-link>
    </div>

    <div v-else-if="filteredGroups.length === 0 && searchQuery" class="empty-state">
      <h3>没有找到匹配的群组</h3>
      <p>尝试使用不同的搜索词</p>
    </div>
    
    <div v-else class="group-list">
      <router-link 
        v-for="group in filteredGroups" 
        :key="group.id" 
        :to="`/${group.path}`"
        class="group-card"
      >
        <div class="group-avatar">
          {{ group.name.charAt(0).toUpperCase() }}
        </div>
        <div class="group-body">
          <div class="group-title-row">
            <h3>{{ group.name }}</h3>
            <span class="visibility-badge" :class="group.visibility">
              {{ group.visibility === 'public' ? '公开' : group.visibility === 'internal' ? '内部' : '私有' }}
            </span>
          </div>
          <p class="group-path">{{ group.path }}</p>
          <p v-if="group.description" class="group-desc">{{ group.description }}</p>
        </div>
        <div class="group-meta">
          <span class="meta-item" title="创建时间">
            {{ formatDate(group.created_at) }}
          </span>
        </div>
      </router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api } from '@/api'
import type { Group } from '@/types'

const searchQuery = ref('')
const sortBy = ref('name')
const loading = ref(false)
const groups = ref<Group[]>([])

const filteredGroups = computed(() => {
  let result = groups.value
  
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(g => 
      g.name.toLowerCase().includes(query) || 
      g.path.toLowerCase().includes(query) ||
      (g.description && g.description.toLowerCase().includes(query))
    )
  }
  
  result = [...result].sort((a, b) => {
    switch (sortBy.value) {
      case 'created': return new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      case 'updated': return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
      default: return a.name.localeCompare(b.name)
    }
  })
  
  return result
})

function formatDate(date: string) {
  return new Date(date).toLocaleDateString('zh-CN', { year: 'numeric', month: 'short', day: 'numeric' })
}

onMounted(async () => {
  loading.value = true
  try {
    groups.value = await api.groups.list()
  } catch (e) {
    console.error('Failed to load groups:', e)
  } finally {
    loading.value = false
  }
})
</script>

<style lang="scss" scoped>
.explore-groups-page {
  padding: $spacing-6;
  max-width: 1200px;
  margin: 0 auto;
}

.page-header {
  margin-bottom: $spacing-6;
  h1 { font-size: $text-2xl; font-weight: 600; margin-bottom: $spacing-1; }
  .page-desc { color: $text-secondary; font-size: $text-sm; }
}

.filter-bar {
  display: flex;
  gap: $spacing-4;
  margin-bottom: $spacing-6;
  
  .search-wrap {
    position: relative;
    flex: 1;
    max-width: 400px;
    
    .search-icon {
      position: absolute;
      left: 12px;
      top: 50%;
      transform: translateY(-50%);
      width: 16px;
      height: 16px;
      color: $text-muted;
    }
    
    .search-input {
      width: 100%;
      padding: $spacing-2 $spacing-3 $spacing-2 36px;
      border: 1px solid $border-color;
      border-radius: $radius-md;
      background: $bg-primary;
      color: $text-primary;
      font-size: $text-sm;
      
      &:focus {
        outline: none;
        border-color: $color-primary;
        box-shadow: $shadow-focus;
      }
    }
  }
  
  .sort-select {
    padding: $spacing-2 $spacing-3;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    background: $bg-primary;
    color: $text-primary;
    font-size: $text-sm;
    cursor: pointer;
  }
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: $spacing-3;
  padding: $spacing-12;
  
  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid $border-color;
    border-top-color: $color-primary;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }
  p { color: $text-muted; font-size: $text-sm; }
}

.empty-state {
  text-align: center;
  padding: $spacing-12;
  
  .empty-icon {
    width: 64px;
    height: 64px;
    color: $text-muted;
    margin: 0 auto $spacing-4;
  }
  h3 { color: $text-primary; margin-bottom: $spacing-2; }
  p { color: $text-secondary; margin-bottom: $spacing-4; }
}

.group-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-3;
}

.group-card {
  display: flex;
  gap: $spacing-4;
  padding: $spacing-4;
  border: 1px solid $border-color;
  border-radius: $radius-md;
  text-decoration: none;
  transition: all 0.15s ease;
  
  &:hover {
    border-color: $color-primary;
    background: $bg-secondary;
  }
  
  .group-avatar {
    width: 48px;
    height: 48px;
    border-radius: $radius-md;
    background: linear-gradient(135deg, $color-primary, $color-primary-dark);
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
    font-size: $text-lg;
    flex-shrink: 0;
  }
  
  .group-body {
    flex: 1;
    min-width: 0;
    
    .group-title-row {
      display: flex;
      align-items: center;
      gap: $spacing-2;
      margin-bottom: $spacing-1;
      
      h3 {
        font-size: $text-base;
        font-weight: 600;
        color: $text-primary;
      }
    }
    
    .group-path {
      font-size: $text-sm;
      color: $text-muted;
      margin-bottom: $spacing-1;
    }
    
    .group-desc {
      font-size: $text-sm;
      color: $text-secondary;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
  }
  
  .group-meta {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: $spacing-2;
    flex-shrink: 0;
    
    .meta-item {
      font-size: $text-xs;
      color: $text-muted;
    }
  }
}

.visibility-badge {
  display: inline-block;
  padding: 1px $spacing-2;
  border-radius: $radius-full;
  font-size: $text-xs;
  font-weight: 500;
  
  &.public { background: #ddf4ff; color: #0969da; }
  &.internal { background: #fff8c5; color: #9a6700; }
  &.private { background: #ffebe9; color: #cf222e; }
}

.btn {
  display: inline-block;
  padding: $spacing-2 $spacing-4;
  border-radius: $radius-md;
  font-size: $text-sm;
  font-weight: 500;
  text-decoration: none;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-primary {
  background: $color-primary;
  color: white;
  border: none;
  
  &:hover { background: $color-primary-dark; }
}

@keyframes spin { to { transform: rotate(360deg); } }
</style>
