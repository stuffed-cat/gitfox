<template>
  <aside class="sidebar" :class="{ collapsed, hidden }">
    <nav class="sidebar-nav">
      <!-- 项目上下文头部 -->
      <div v-if="projectContext" class="nav-context">
        <router-link :to="`/${projectContext.owner}/${projectContext.repo}`" class="context-header">
          <div class="context-avatar">{{ projectContext.repo.charAt(0).toUpperCase() }}</div>
          <div class="context-info" v-if="!collapsed">
            <div class="context-name">{{ projectContext.repo }}</div>
            <div class="context-path">{{ projectContext.owner }}</div>
          </div>
        </router-link>
      </div>

      <!-- 动态菜单 -->
      <div v-for="section in visibleSections" :key="section.title" class="nav-section">
        <div class="nav-section-title">{{ section.title }}</div>
        <router-link
          v-for="item in section.items"
          :key="item.id"
          :to="item.to"
          class="nav-item"
          :class="{ active: isActiveRoute(item) }"
        >
          <component :is="item.icon" class="nav-icon" />
          <span class="nav-label">{{ item.label }}</span>
          <span v-if="item.badge" class="nav-badge" :class="item.badgeClass">{{ item.badge }}</span>
        </router-link>
      </div>
    </nav>
    
    <!-- Sidebar Footer -->
    <div class="sidebar-footer">
      <button class="collapse-btn" @click="$emit('toggle-collapse')" :title="collapsed ? '展开侧边栏' : '收起侧边栏'">
        <svg :class="{ rotated: !collapsed }" viewBox="0 0 16 16" fill="none">
          <path d="M10 4L6 8l4 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { computed, h, type FunctionalComponent } from 'vue'
import { useRoute } from 'vue-router'

defineProps<{
  collapsed: boolean
  hidden: boolean
}>()

defineEmits(['toggle-collapse'])

const route = useRoute()

// 项目上下文
const projectContext = computed(() => {
  const { owner, repo } = route.params
  if (owner && repo && typeof owner === 'string' && typeof repo === 'string') {
    return { owner, repo }
  }
  return null
})

// 项目基础路径
const projectBasePath = computed(() => {
  if (projectContext.value) {
    return `/${projectContext.value.owner}/${projectContext.value.repo}`
  }
  return ''
})

// 图标组件
const icons = {
  home: createIcon('M8 1L1 6v8a1 1 0 001 1h4v-5h4v5h4a1 1 0 001-1V6L8 1z'),
  project: createIcon('M2 2h12v12H2zM5 6h6M5 9h4', true),
  group: createIcon('M1 3h6v6H1zM9 3h6v6H9zM5 9h6v6H5z', true),
  issue: createIcon('M8 8m-6 0a6 6 0 1012 0a6 6 0 10-12 0M8 8m-2 0a2 2 0 104 0a2 2 0 10-4 0', false, true),
  mergeRequest: createIcon('M4 4m-2 0a2 2 0 104 0a2 2 0 10-4 0M12 4m-2 0a2 2 0 104 0a2 2 0 10-4 0M8 12m-2 0a2 2 0 104 0a2 2 0 10-4 0M4 6v2a4 4 0 004 4m4-6v2a4 4 0 01-4 4', false, true),
  todo: createIcon('M3 4h10M3 8h10M3 12h6'),
  activity: createIcon('M1 8h3l2-5 2 10 2-5h5'),
  explore: createIcon('M8 8m-6 0a6 6 0 1012 0a6 6 0 10-12 0M8 5v3l2 2'),
  users: createIcon('M5 6m-2 0a2 2 0 104 0a2 6 0 10-4 0M11 6m-2 0a2 2 0 104 0a2 2 0 10-4 0M1 14a4 4 0 018 0M7 14a4 4 0 018 0'),
  code: createIcon('M5 4L1 8l4 4M11 4l4 4-4 4M9 2l-2 12'),
  commit: createIcon('M8 8m-3 0a3 3 0 106 0a3 3 0 10-6 0M1 8h4M11 8h4'),
  branch: createIcon('M4 2v8a2 2 0 002 2h2M12 2v12M4 6h4a2 2 0 012 2v4'),
  tag: createIcon('M1 3l6-1 8 8-5 5-8-8zM5 5m-1 0a1 1 0 102 0a1 1 0 10-2 0', false, true),
  pipeline: createIcon('M2 4h4v4H2zM10 4h4v4h-4zM6 6h4M2 10h4v4H2zM10 10h4v4h-4zM6 12h4', true),
  settings: createIcon('M8 8m-2 0a2 2 0 104 0a2 2 0 10-4 0M8 1v2M8 13v2M1 8h2M13 8h2M3 3l1.5 1.5M11.5 11.5l1.5 1.5M3 13l1.5-1.5M11.5 4.5l1.5-1.5'),
}

function createIcon(path: string, _rect = false, fill = false): FunctionalComponent {
  return () => h('svg', { class: 'nav-icon', viewBox: '0 0 16 16', fill: 'none' }, [
    h('path', { 
      d: path, 
      stroke: 'currentColor', 
      'stroke-width': '1.5',
      'stroke-linecap': 'round',
      'stroke-linejoin': 'round',
      fill: fill ? 'currentColor' : 'none'
    })
  ])
}

// 菜单项类型
interface MenuItem {
  id: string
  label: string
  to: string
  icon: FunctionalComponent
  badge?: string | number
  badgeClass?: string
  activeMatch?: RegExp
}

interface MenuSection {
  title: string
  items: MenuItem[]
  context: 'global' | 'project'
}

// 全局菜单配置
const globalMenuSections: MenuSection[] = [
  {
    title: '你的工作',
    context: 'global',
    items: [
      { id: 'home', label: 'Home', to: '/', icon: icons.home },
      { id: 'projects', label: '项目', to: '/dashboard/projects', icon: icons.project },
      { id: 'groups', label: '群组', to: '/dashboard/groups', icon: icons.group },
      { id: 'issues', label: '议题', to: '/dashboard/issues', icon: icons.issue },
      { id: 'merge-requests', label: '合并请求', to: '/dashboard/merge-requests', icon: icons.mergeRequest },
      { id: 'todos', label: '待办事项列表', to: '/dashboard/todos', icon: icons.todo, badge: 2, badgeClass: 'warning' },
      { id: 'activity', label: '动态', to: '/dashboard/activity', icon: icons.activity },
    ]
  },
  {
    title: '探索',
    context: 'global',
    items: [
      { id: 'explore-projects', label: '项目', to: '/explore/projects', icon: icons.explore },
      { id: 'explore-groups', label: '群组', to: '/explore/groups', icon: icons.users },
    ]
  }
]

// 项目菜单配置（动态生成）
const projectMenuSections = computed<MenuSection[]>(() => {
  const base = projectBasePath.value
  if (!base) return []
  
  return [
    {
      title: '项目',
      context: 'project',
      items: [
        { id: 'project-overview', label: '项目概览', to: base, icon: icons.project, activeMatch: new RegExp(`^${base}$`) },
      ]
    },
    {
      title: '代码',
      context: 'project',
      items: [
        { id: 'files', label: '文件', to: `${base}/-/tree`, icon: icons.code, activeMatch: /\/-\/(tree|blob)/ },
        { id: 'commits', label: '提交', to: `${base}/-/commits`, icon: icons.commit, activeMatch: /\/-\/commit/ },
        { id: 'branches', label: '分支', to: `${base}/-/branches`, icon: icons.branch },
        { id: 'tags', label: '标签', to: `${base}/-/tags`, icon: icons.tag },
      ]
    },
    {
      title: '计划',
      context: 'project',
      items: [
        { id: 'project-issues', label: '议题', to: `${base}/-/issues`, icon: icons.issue },
        { id: 'project-mr', label: '合并请求', to: `${base}/-/merge_requests`, icon: icons.mergeRequest, activeMatch: /\/-\/merge_requests/ },
      ]
    },
    {
      title: '构建',
      context: 'project',
      items: [
        { id: 'pipelines', label: '流水线', to: `${base}/-/pipelines`, icon: icons.pipeline, activeMatch: /\/-\/pipelines/ },
      ]
    },
    {
      title: '设置',
      context: 'project',
      items: [
        { id: 'settings', label: '设置', to: `${base}/-/settings`, icon: icons.settings },
      ]
    }
  ]
})

// 可见的菜单分区
const visibleSections = computed(() => {
  if (projectContext.value) {
    return projectMenuSections.value
  }
  return globalMenuSections
})

// 判断路由是否激活
function isActiveRoute(item: MenuItem): boolean {
  if (item.activeMatch) {
    return item.activeMatch.test(route.path)
  }
  // 精确匹配
  if (item.to === route.path) return true
  // 前缀匹配（非首页）
  if (item.to !== '/' && route.path.startsWith(item.to)) return true
  return false
}
</script>

<style lang="scss" scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: $bg-sidebar;
  overflow: hidden;
  transition: width $transition-normal;
}

.sidebar-nav {
  flex: 1;
  padding: $spacing-3;
  overflow-y: auto;
  overflow-x: hidden;
  
  &::-webkit-scrollbar {
    width: 6px;
  }
  
  &::-webkit-scrollbar-track {
    background: transparent;
  }
  
  &::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 3px;
    
    &:hover {
      background: rgba(255, 255, 255, 0.2);
    }
  }
}

.nav-section {
  margin-bottom: $spacing-4;
  
  &:last-child {
    margin-bottom: 0;
  }
}

.nav-section-title {
  padding: $spacing-2 $spacing-3;
  font-size: $font-size-xs;
  font-weight: $font-weight-semibold;
  color: $gray-400;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  white-space: nowrap;
  
  .collapsed & {
    opacity: 0;
    visibility: hidden;
  }
}

.nav-item {
  display: flex;
  align-items: center;
  gap: $spacing-3;
  padding: $spacing-2 $spacing-3;
  font-size: $font-size-sm;
  color: $gray-300;
  text-decoration: none;
  border-radius: $border-radius;
  transition: all $transition-fast;
  white-space: nowrap;
  
  &:hover {
    background: $bg-sidebar-hover;
    color: $text-light;
  }
  
  &.active {
    background: $bg-sidebar-active;
    color: $text-light;
    
    .nav-icon {
      color: $brand-primary;
    }
  }
}

// 项目上下文头部
.nav-context {
  margin-bottom: $spacing-4;
  padding-bottom: $spacing-3;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.context-header {
  display: flex;
  align-items: center;
  gap: $spacing-3;
  padding: $spacing-2 $spacing-3;
  text-decoration: none;
  border-radius: $border-radius;
  transition: background $transition-fast;
  
  &:hover {
    background: $bg-sidebar-hover;
  }
}

.context-avatar {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, $brand-primary, $brand-secondary);
  color: white;
  font-weight: $font-weight-semibold;
  font-size: $font-size-sm;
  border-radius: $border-radius;
  flex-shrink: 0;
}

.context-info {
  min-width: 0;
  overflow: hidden;
}

.context-name {
  font-size: $font-size-sm;
  font-weight: $font-weight-semibold;
  color: $text-light;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.context-path {
  font-size: $font-size-xs;
  color: $gray-400;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.nav-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
  opacity: 0.8;
}

.nav-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  
  .collapsed & {
    opacity: 0;
    width: 0;
  }
}

.nav-badge {
  padding: 2px 6px;
  font-size: $font-size-xs;
  font-weight: $font-weight-medium;
  color: $text-light;
  background: $gray-600;
  border-radius: $border-radius-full;
  
  &.warning {
    background: $color-warning;
    color: $gray-900;
  }
  
  &.danger {
    background: $color-danger;
  }
  
  .collapsed & {
    display: none;
  }
}

.sidebar-footer {
  padding: $spacing-3;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
}

.collapse-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 32px;
  background: transparent;
  border: none;
  color: $gray-400;
  cursor: pointer;
  border-radius: $border-radius;
  transition: all $transition-fast;
  
  &:hover {
    background: $bg-sidebar-hover;
    color: $text-light;
  }
  
  svg {
    width: 16px;
    height: 16px;
    transition: transform $transition-normal;
    
    &.rotated {
      transform: rotate(180deg);
    }
  }
}

// Collapsed state
.collapsed {
  .nav-section-title,
  .nav-label,
  .nav-badge {
    opacity: 0;
    visibility: hidden;
  }
  
  .nav-item {
    justify-content: center;
    padding: $spacing-2;
  }
}

// Hidden state (mobile)
.hidden {
  transform: translateX(-100%);
}
</style>
