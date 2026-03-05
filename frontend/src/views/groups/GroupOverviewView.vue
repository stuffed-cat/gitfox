<template>
  <div class="group-overview">
    <!-- 群组头部 -->
    <div class="group-header">
      <div class="group-header-content">
        <div class="group-avatar-large">
          <span>{{ (group?.name || 'G')[0].toUpperCase() }}</span>
        </div>
        <div class="group-title-section">
          <h1 class="group-name">{{ group?.name }}</h1>
          <p class="group-path">
            <span class="visibility-badge" :class="group?.visibility">
              <svg v-if="group?.visibility === 'private'" viewBox="0 0 16 16" fill="currentColor"><path d="M4 7V5a4 4 0 018 0v2h1a1 1 0 011 1v6a2 2 0 01-2 2H4a2 2 0 01-2-2V8a1 1 0 011-1h1zm2 0h4V5a2 2 0 10-4 0v2z"/></svg>
              <svg v-else-if="group?.visibility === 'internal'" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1L1 5v6l7 4 7-4V5L8 1z"/></svg>
              <svg v-else viewBox="0 0 16 16" fill="currentColor"><circle cx="8" cy="8" r="6"/></svg>
            </span>
            @{{ group?.path }}
          </p>
          <p v-if="group?.description" class="group-description">{{ group.description }}</p>
        </div>
      </div>
      <div class="group-actions">
        <router-link v-if="canManage" :to="`/${group?.path}/-/settings`" class="btn btn-default btn-sm">
          <svg class="btn-icon" viewBox="0 0 16 16" fill="none">
            <path d="M8 8m-2 0a2 2 0 104 0a2 2 0 10-4 0M8 1v2M8 13v2M1 8h2M13 8h2M3 3l1.5 1.5M11.5 11.5l1.5 1.5M3 13l1.5-1.5M11.5 4.5l1.5-1.5" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          设置
        </router-link>
      </div>
    </div>

    <!-- 统计栏 -->
    <div class="group-stats-bar">
      <div class="stat-item">
        <svg viewBox="0 0 16 16" fill="none"><path d="M1 3h6v6H1zM9 3h6v6H9zM5 9h6v6H5z" stroke="currentColor" stroke-width="1.5"/></svg>
        <span>{{ subgroups.length }} 个子群组</span>
      </div>
      <div class="stat-item">
        <svg viewBox="0 0 16 16" fill="none"><path d="M2 3a1 1 0 011-1h10a1 1 0 011 1v10a1 1 0 01-1 1H3a1 1 0 01-1-1V3zM5 6h6M5 9h4" stroke="currentColor" stroke-width="1.5"/></svg>
        <span>{{ projects.length }} 个项目</span>
      </div>
      <div class="stat-item">
        <svg viewBox="0 0 16 16" fill="none"><path d="M5 5m-2 0a2 2 0 104 0a2 2 0 10-4 0M11 5m-2 0a2 2 0 104 0a2 2 0 10-4 0M1 12a4 4 0 018 0M7 12a4 4 0 018 0" stroke="currentColor" stroke-width="1.5"/></svg>
        <span>{{ members.length }} 个成员</span>
      </div>
    </div>

    <!-- 标签页 -->
    <div class="group-tabs">
      <button :class="['tab', { active: activeTab === 'subgroups-projects' }]" @click="activeTab = 'subgroups-projects'">
        子群组和项目
      </button>
      <button :class="['tab', { active: activeTab === 'shared' }]" @click="activeTab = 'shared'">
        共享的项目
      </button>
      <button :class="['tab', { active: activeTab === 'archived' }]" @click="activeTab = 'archived'">
        已归档的项目
      </button>
    </div>

    <!-- 筛选栏 -->
    <div class="filter-bar">
      <div class="filter-left">
        <input v-model="searchQuery" type="text" placeholder="按名称筛选..." class="search-input" />
      </div>
      <div class="filter-right">
        <router-link :to="`/groups/new?parent=${group?.path}`" class="btn btn-confirm btn-sm">
          新建子群组
        </router-link>
        <router-link to="/projects/new" class="btn btn-confirm btn-sm">
          新建项目
        </router-link>
      </div>
    </div>

    <!-- 子群组和项目列表 -->
    <div v-if="activeTab === 'subgroups-projects'" class="items-list">
      <!-- 子群组 -->
      <router-link
        v-for="sg in filteredSubgroups"
        :key="'g-' + sg.id"
        :to="`/${sg.path}`"
        class="list-item"
      >
        <div class="item-avatar subgroup">
          <svg viewBox="0 0 16 16" fill="none"><path d="M1 3h6v6H1zM9 3h6v6H9zM5 9h6v6H5z" stroke="currentColor" stroke-width="1.5"/></svg>
        </div>
        <div class="item-content">
          <div class="item-title-row">
            <h4 class="item-name">{{ sg.name }}</h4>
            <span class="visibility-label" :class="sg.visibility">{{ visibilityLabel(sg.visibility) }}</span>
          </div>
          <p class="item-path">{{ sg.path }}</p>
          <p v-if="sg.description" class="item-description">{{ sg.description }}</p>
        </div>
      </router-link>

      <!-- 项目 -->
      <router-link
        v-for="proj in filteredProjects"
        :key="'p-' + proj.id"
        :to="`/${proj.owner_name}/${proj.name}`"
        class="list-item"
      >
        <div class="item-avatar project">
          <span>{{ proj.name[0].toUpperCase() }}</span>
        </div>
        <div class="item-content">
          <div class="item-title-row">
            <h4 class="item-name">{{ proj.name }}</h4>
            <span class="visibility-label" :class="proj.visibility">{{ visibilityLabel(proj.visibility) }}</span>
          </div>
          <p v-if="proj.description" class="item-description">{{ proj.description }}</p>
          <div class="item-meta">
            <span v-if="proj.updated_at" class="meta-item">
              更新于 {{ formatTime(proj.updated_at) }}
            </span>
          </div>
        </div>
        <div class="item-stats">
          <span class="stat" title="星标">
            <svg viewBox="0 0 16 16" fill="none"><path d="M8 2l1.8 3.6 4 .6-2.9 2.8.7 4L8 11.3 4.4 13l.7-4L2.2 6.2l4-.6L8 2z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/></svg>
            0
          </span>
          <span class="stat" title="派生">
            <svg viewBox="0 0 16 16" fill="none"><circle cx="5" cy="3" r="2" stroke="currentColor" stroke-width="1.5"/><circle cx="11" cy="3" r="2" stroke="currentColor" stroke-width="1.5"/><circle cx="8" cy="13" r="2" stroke="currentColor" stroke-width="1.5"/><path d="M5 5v2a3 3 0 003 3m3-5v2a3 3 0 01-3 3" stroke="currentColor" stroke-width="1.5"/></svg>
            0
          </span>
        </div>
      </router-link>

      <!-- 空状态 -->
      <div v-if="filteredSubgroups.length === 0 && filteredProjects.length === 0" class="empty-state">
        <svg class="empty-icon" viewBox="0 0 64 64" fill="none">
          <rect x="8" y="12" width="48" height="40" rx="4" stroke="currentColor" stroke-width="2"/>
          <path d="M8 20h48" stroke="currentColor" stroke-width="2"/>
          <path d="M24 36h16M32 28v16" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <h3>此群组中暂无子群组或项目</h3>
        <p>通过创建子群组或项目来开始组织您的工作</p>
        <div class="empty-actions">
          <router-link :to="`/groups/new?parent=${group?.path}`" class="btn btn-confirm">新建子群组</router-link>
          <router-link to="/projects/new" class="btn btn-default">新建项目</router-link>
        </div>
      </div>
    </div>

    <!-- 其他标签页 -->
    <div v-else class="tab-placeholder">
      <div class="empty-state">
        <p>暂无数据</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import { useAuthStore } from '@/stores/auth'
import type { Group, GroupMember, Project } from '@/types'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

const authStore = useAuthStore()

const props = defineProps<{
  group: Group
  subgroups: Group[]
  projects: Project[]
  members: GroupMember[]
}>()

const activeTab = ref('subgroups-projects')
const searchQuery = ref('')

const canManage = computed(() => {
  const currentUser = authStore.user
  if (!currentUser) return false
  
  // 检查是否为组的 owner 或 maintainer
  // access_level: 50 = Owner, 40 = Maintainer
  const member = props.members.find(m => String(m.user_id) === currentUser.id)
  if (member && member.access_level >= 40) {
    return true
  }
  
  return false
})

const filteredSubgroups = computed(() => {
  if (!searchQuery.value) return props.subgroups
  const q = searchQuery.value.toLowerCase()
  return props.subgroups.filter(sg => sg.name.toLowerCase().includes(q))
})

const filteredProjects = computed(() => {
  if (!searchQuery.value) return props.projects
  const q = searchQuery.value.toLowerCase()
  return props.projects.filter(p => p.name.toLowerCase().includes(q))
})

function visibilityLabel(v: string) {
  return v === 'private' ? '私有' : v === 'internal' ? '内部' : '公开'
}

function formatTime(date: string) {
  return dayjs(date).fromNow()
}
</script>

<style lang="scss" scoped>
.group-overview {
  padding: $spacing-6;
  max-width: 1200px;
  margin: 0 auto;
}

// Group Header
.group-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: $spacing-6;
  padding-bottom: $spacing-6;
  border-bottom: 1px solid $border-color;
}

.group-header-content {
  display: flex;
  gap: $spacing-5;
  align-items: flex-start;
}

.group-avatar-large {
  width: 72px;
  height: 72px;
  border-radius: $radius-lg;
  background: linear-gradient(135deg, $brand-primary, $brand-secondary);
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  font-weight: 600;
  flex-shrink: 0;
}

.group-title-section {
  .group-name {
    font-size: $text-2xl;
    font-weight: 600;
    margin-bottom: $spacing-1;
  }
  
  .group-path {
    display: flex;
    align-items: center;
    gap: $spacing-2;
    color: $text-secondary;
    font-size: $text-sm;
    margin-bottom: $spacing-2;
    
    .visibility-badge {
      display: inline-flex;
      align-items: center;
      
      svg {
        width: 14px;
        height: 14px;
      }
      
      &.private { color: $color-warning; }
      &.internal { color: $color-info; }
      &.public { color: $color-success; }
    }
  }
  
  .group-description {
    color: $text-secondary;
    font-size: $text-sm;
    line-height: 1.5;
    max-width: 600px;
  }
}

.group-actions {
  display: flex;
  gap: $spacing-2;
}

// Stats Bar
.group-stats-bar {
  display: flex;
  gap: $spacing-6;
  padding: $spacing-4 0;
  margin-bottom: $spacing-4;
  
  .stat-item {
    display: flex;
    align-items: center;
    gap: $spacing-2;
    color: $text-secondary;
    font-size: $text-sm;
    
    svg {
      width: 16px;
      height: 16px;
    }
  }
}

// Tabs
.group-tabs {
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
    font-size: $text-sm;
    font-weight: 500;
    transition: all $transition-fast;
    
    &:hover { color: $text-primary; }
    &.active {
      color: $text-primary;
      border-bottom-color: $color-primary;
    }
  }
}

// Filter Bar
.filter-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: $spacing-4;
  gap: $spacing-4;
  
  .filter-left {
    flex: 1;
    max-width: 400px;
  }
  
  .search-input {
    width: 100%;
    padding: $spacing-2 $spacing-3;
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
  
  .filter-right {
    display: flex;
    gap: $spacing-2;
  }
}

// List Items
.items-list {
  display: flex;
  flex-direction: column;
}

.list-item {
  display: flex;
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
}

.item-avatar {
  width: 48px;
  height: 48px;
  border-radius: $radius-md;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  
  &.subgroup {
    background: rgba($color-primary, 0.1);
    color: $color-primary;
    
    svg {
      width: 24px;
      height: 24px;
    }
  }
  
  &.project {
    background: $color-primary;
    color: white;
    font-weight: 600;
    font-size: $text-lg;
  }
}

.item-content {
  flex: 1;
  min-width: 0;
  
  .item-title-row {
    display: flex;
    align-items: center;
    gap: $spacing-2;
    margin-bottom: $spacing-1;
  }
  
  .item-name {
    font-size: $text-base;
    font-weight: 600;
    color: $text-primary;
  }
  
  .visibility-label {
    font-size: $text-xs;
    padding: 2px 6px;
    border-radius: $radius-full;
    font-weight: 500;
    
    &.private { background: rgba($color-warning, 0.1); color: darken($color-warning, 10%); }
    &.internal { background: rgba($color-info, 0.1); color: $color-info; }
    &.public { background: rgba($color-success, 0.1); color: $color-success; }
  }
  
  .item-path {
    font-size: $text-xs;
    color: $text-muted;
    margin-bottom: $spacing-1;
  }
  
  .item-description {
    font-size: $text-sm;
    color: $text-secondary;
    line-height: 1.4;
    margin-bottom: $spacing-2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  
  .item-meta {
    display: flex;
    gap: $spacing-4;
    
    .meta-item {
      font-size: $text-xs;
      color: $text-muted;
    }
  }
}

.item-stats {
  display: flex;
  gap: $spacing-4;
  align-items: flex-start;
  
  .stat {
    display: flex;
    align-items: center;
    gap: $spacing-1;
    color: $text-muted;
    font-size: $text-sm;
    
    svg {
      width: 14px;
      height: 14px;
    }
  }
}

// Empty State
.empty-state {
  text-align: center;
  padding: $spacing-12;
  
  .empty-icon {
    width: 64px;
    height: 64px;
    color: $text-muted;
    margin-bottom: $spacing-4;
  }
  
  h3 {
    color: $text-primary;
    font-size: $text-lg;
    margin-bottom: $spacing-2;
  }
  
  p {
    color: $text-secondary;
    margin-bottom: $spacing-6;
  }
  
  .empty-actions {
    display: flex;
    justify-content: center;
    gap: $spacing-3;
  }
}

// Buttons
.btn {
  display: inline-flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-2 $spacing-4;
  border-radius: $radius-md;
  font-size: $text-sm;
  font-weight: 500;
  cursor: pointer;
  transition: all $transition-fast;
  text-decoration: none;
  border: 1px solid transparent;
  
  .btn-icon {
    width: 16px;
    height: 16px;
  }
}

.btn-sm {
  padding: $spacing-1 $spacing-3;
  font-size: $text-xs;
}

.btn-default {
  background: $bg-primary;
  color: $text-primary;
  border-color: $border-color;
  
  &:hover { background: $bg-secondary; }
}

.btn-confirm {
  background: $color-primary;
  color: white;
  
  &:hover { background: $color-primary-dark; }
}
</style>
