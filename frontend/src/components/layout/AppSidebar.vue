<template>
  <aside class="app-sidebar" :class="{ collapsed, hidden }">
    <nav class="sidebar-nav">
      <!-- 上下文头部 -->
      <div v-if="contextHeader" class="nav-context">
        <router-link :to="contextHeader.to" class="context-header">
          <div class="context-avatar">{{ contextHeader.avatar }}</div>
          <div class="context-info" v-if="!collapsed">
            <div class="context-name">{{ contextHeader.title }}</div>
            <div class="context-path">{{ contextHeader.subtitle }}</div>
          </div>
        </router-link>
      </div>

      <!-- 动态菜单 -->
      <div v-for="section in sections" :key="section.id" class="nav-section">
        <div class="nav-section-title" v-if="!collapsed">{{ section.title }}</div>
        <router-link
          v-for="item in section.items"
          :key="item.id"
          :to="getItemPath(item)"
          class="nav-item"
          :class="{ active: isActive(item) }"
        >
          <NavIcon :name="item.icon" class="nav-icon" />
          <span class="nav-label" v-if="!collapsed">{{ item.label }}</span>
          <span 
            v-if="item.badge && !collapsed" 
            class="nav-badge" 
            :class="item.badgeType ? `badge-${item.badgeType}` : ''"
          >
            {{ item.badge }}
          </span>
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
import { useNavigation, type NavItem } from '@/navigation'
import NavIcon from './NavIcon.vue'

defineProps<{
  collapsed: boolean
  hidden: boolean
}>()

defineEmits(['toggle-collapse'])

const { sections, contextHeader, isActive, context } = useNavigation()

function getItemPath(item: NavItem): string {
  if (typeof item.to === 'function') {
    return item.to(context.value)
  }
  return item.to
}
</script>

<style lang="scss" scoped>
.app-sidebar {
  display: flex;
  flex-direction: column;
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
  
  .collapsed & {
    justify-content: center;
    padding: $spacing-2;
  }
}

// 上下文头部
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
  
  .collapsed & {
    justify-content: center;
    padding: $spacing-2;
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
}

.nav-badge {
  padding: 2px 6px;
  font-size: $font-size-xs;
  font-weight: $font-weight-medium;
  color: $text-light;
  background: $gray-600;
  border-radius: $border-radius-full;
  
  &.badge-warning {
    background: $color-warning;
    color: $gray-900;
  }
  
  &.badge-danger {
    background: $color-danger;
  }
  
  &.badge-success {
    background: $color-success;
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
  .nav-section-title {
    display: none;
  }
}

// Hidden state (mobile)
.hidden {
  transform: translateX(-100%);
}
</style>
