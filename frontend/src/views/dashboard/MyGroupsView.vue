<template>
  <div class="my-groups-page">
    <div class="page-header">
      <div class="header-left">
        <h1>你的群组</h1>
        <p class="page-desc">你创建或加入的所有群组</p>
      </div>
      <router-link to="/groups/new" class="btn btn-primary">
        <svg viewBox="0 0 16 16" fill="none" class="btn-icon">
          <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        新建群组
      </router-link>
    </div>
    
    <div class="filter-bar">
      <div class="search-wrap">
        <svg viewBox="0 0 16 16" fill="none" class="search-icon">
          <circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.5"/>
          <path d="M11 11l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <input v-model="searchQuery" type="text" placeholder="按名称筛选群组..." class="search-input" />
      </div>
      <div class="view-toggle">
        <button :class="['toggle-btn', { active: viewMode === 'list' }]" @click="viewMode = 'list'" title="列表视图">
          <svg viewBox="0 0 16 16" fill="none"><path d="M2 4h12M2 8h12M2 12h12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        </button>
        <button :class="['toggle-btn', { active: viewMode === 'grid' }]" @click="viewMode = 'grid'" title="网格视图">
          <svg viewBox="0 0 16 16" fill="none"><rect x="2" y="2" width="5" height="5" rx="1" stroke="currentColor" stroke-width="1.5"/><rect x="9" y="2" width="5" height="5" rx="1" stroke="currentColor" stroke-width="1.5"/><rect x="2" y="9" width="5" height="5" rx="1" stroke="currentColor" stroke-width="1.5"/><rect x="9" y="9" width="5" height="5" rx="1" stroke="currentColor" stroke-width="1.5"/></svg>
        </button>
      </div>
    </div>
    
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>
    
    <div v-else-if="groups.length === 0" class="empty-state">
      <svg viewBox="0 0 64 64" fill="none" class="empty-icon">
        <rect x="8" y="12" width="20" height="20" rx="4" stroke="currentColor" stroke-width="2"/>
        <rect x="36" y="12" width="20" height="20" rx="4" stroke="currentColor" stroke-width="2"/>
        <rect x="22" y="32" width="20" height="20" rx="4" stroke="currentColor" stroke-width="2"/>
      </svg>
      <h3>你还没有任何群组</h3>
      <p>群组是组织项目和管理团队的好方法。创建或加入一个群组来开始协作。</p>
      <router-link to="/groups/new" class="btn btn-primary">新建群组</router-link>
    </div>
    
    <!-- 列表视图 -->
    <div v-else-if="viewMode === 'list'" class="group-list">
      <router-link
        v-for="group in filteredGroups"
        :key="group.id"
        :to="`/${group.path}`"
        class="group-item"
      >
        <div class="group-avatar">
          {{ group.name.charAt(0).toUpperCase() }}
        </div>
        <div class="group-info">
          <div class="group-title-row">
            <h3>{{ group.name }}</h3>
            <span class="visibility-badge" :class="group.visibility">
              {{ group.visibility === 'public' ? '公开' : group.visibility === 'internal' ? '内部' : '私有' }}
            </span>
          </div>
          <p class="group-path">{{ group.path }}</p>
          <p v-if="group.description" class="group-desc">{{ group.description }}</p>
        </div>
        <div class="group-actions">
          <span class="meta-date">{{ formatDate(group.updated_at) }}</span>
        </div>
      </router-link>
    </div>

    <!-- 网格视图 -->
    <div v-else class="group-grid">
      <router-link
        v-for="group in filteredGroups"
        :key="group.id"
        :to="`/${group.path}`"
        class="group-card"
      >
        <div class="card-header">
          <div class="group-avatar-lg">
            {{ group.name.charAt(0).toUpperCase() }}
          </div>
          <span class="visibility-badge" :class="group.visibility">
            {{ group.visibility === 'public' ? '公开' : group.visibility === 'internal' ? '内部' : '私有' }}
          </span>
        </div>
        <h3>{{ group.name }}</h3>
        <p class="group-path">{{ group.path }}</p>
        <p v-if="group.description" class="group-desc">{{ group.description }}</p>
      </router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api } from '@/api'
import type { Group } from '@/types'

const loading = ref(false)
const searchQuery = ref('')
const viewMode = ref<'list' | 'grid'>('list')
const groups = ref<Group[]>([])

const filteredGroups = computed(() => {
  if (!searchQuery.value) return groups.value
  const query = searchQuery.value.toLowerCase()
  return groups.value.filter(g =>
    g.name.toLowerCase().includes(query) ||
    g.path.toLowerCase().includes(query)
  )
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
.my-groups-page {
  padding: $spacing-6;
  max-width: 1200px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: $spacing-6;
  
  .header-left {
    h1 { font-size: $text-2xl; font-weight: 600; margin-bottom: $spacing-1; }
    .page-desc { color: $text-secondary; font-size: $text-sm; }
  }
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-2 $spacing-4;
  border-radius: $radius-md;
  font-size: $text-sm;
  font-weight: 500;
  text-decoration: none;
  cursor: pointer;
  transition: all 0.2s;
  border: none;
  
  .btn-icon { width: 16px; height: 16px; }
}

.btn-primary {
  background: $color-primary;
  color: white;
  &:hover { background: $color-primary-dark; }
}

.filter-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: $spacing-4;
  margin-bottom: $spacing-5;
  
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
  
  .view-toggle {
    display: flex;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    overflow: hidden;
    
    .toggle-btn {
      padding: $spacing-2;
      background: none;
      border: none;
      cursor: pointer;
      color: $text-muted;
      display: flex;
      align-items: center;
      justify-content: center;
      
      svg { width: 16px; height: 16px; }
      
      &:not(:last-child) { border-right: 1px solid $border-color; }
      &:hover { background: $bg-secondary; }
      &.active { background: $bg-tertiary; color: $text-primary; }
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
    color: $text-muted;
    margin: 0 auto $spacing-4;
  }
  h3 { color: $text-primary; margin-bottom: $spacing-2; }
  p { color: $text-secondary; margin-bottom: $spacing-4; max-width: 400px; margin-left: auto; margin-right: auto; line-height: 1.5; }
}

.group-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-3;
  
  .group-item {
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
    
    .group-info {
      flex: 1;
      min-width: 0;
      
      .group-title-row {
        display: flex;
        align-items: center;
        gap: $spacing-2;
        margin-bottom: $spacing-1;
        
        h3 { font-size: $text-base; font-weight: 600; color: $text-primary; }
      }
      
      .group-path { font-size: $text-sm; color: $text-muted; margin-bottom: $spacing-1; }
      .group-desc {
        font-size: $text-sm;
        color: $text-secondary;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
    }
    
    .group-actions {
      display: flex;
      align-items: center;
      flex-shrink: 0;
      
      .meta-date { font-size: $text-xs; color: $text-muted; }
    }
  }
}

.group-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: $spacing-4;
  
  .group-card {
    display: flex;
    flex-direction: column;
    padding: $spacing-5;
    border: 1px solid $border-color;
    border-radius: $radius-lg;
    text-decoration: none;
    transition: all 0.15s ease;
    
    &:hover {
      border-color: $color-primary;
      background: $bg-secondary;
      transform: translateY(-1px);
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
    }
    
    .card-header {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      margin-bottom: $spacing-3;
    }
    
    .group-avatar-lg {
      width: 40px;
      height: 40px;
      border-radius: $radius-md;
      background: linear-gradient(135deg, $color-primary, $color-primary-dark);
      color: white;
      display: flex;
      align-items: center;
      justify-content: center;
      font-weight: 600;
    }
    
    h3 {
      font-size: $text-base;
      font-weight: 600;
      color: $text-primary;
      margin-bottom: $spacing-1;
    }
    
    .group-path { font-size: $text-sm; color: $text-muted; margin-bottom: $spacing-2; }
    .group-desc {
      font-size: $text-sm;
      color: $text-secondary;
      line-height: 1.5;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
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

@keyframes spin { to { transform: rotate(360deg); } }
</style>
