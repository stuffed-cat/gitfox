<template>
  <aside class="sidebar" :class="{ collapsed, hidden }">
    <nav class="sidebar-nav">
      <!-- 你的工作 Section -->
      <div class="nav-section">
        <div class="nav-section-title">你的工作</div>
        <router-link to="/" class="nav-item">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="none">
            <path d="M8 1L1 6v8a1 1 0 001 1h4v-5h4v5h4a1 1 0 001-1V6L8 1z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <span class="nav-label">首页</span>
        </router-link>
        <router-link to="/projects" class="nav-item">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="none">
            <rect x="2" y="2" width="12" height="12" rx="1" stroke="currentColor" stroke-width="1.5"/>
            <path d="M5 6h6M5 9h4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          <span class="nav-label">项目</span>
        </router-link>
        <router-link to="/groups" class="nav-item">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="none">
            <rect x="1" y="3" width="6" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
            <rect x="9" y="3" width="6" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
            <rect x="5" y="9" width="6" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          <span class="nav-label">群组</span>
        </router-link>
      </div>
      
      <!-- 议题与合并请求 Section -->
      <div class="nav-section">
        <div class="nav-section-title">议题与合并请求</div>
        <router-link to="/issues" class="nav-item">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="8" cy="8" r="2" fill="currentColor"/>
          </svg>
          <span class="nav-label">议题</span>
          <span class="nav-badge">0</span>
        </router-link>
        <router-link to="/merge-requests" class="nav-item">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="none">
            <circle cx="4" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="12" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="8" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/>
            <path d="M4 6v2a4 4 0 004 4m4-6v2a4 4 0 01-4 4" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          <span class="nav-label">合并请求</span>
          <span class="nav-badge">2</span>
        </router-link>
      </div>
      
      <!-- 待办事项 Section -->
      <div class="nav-section">
        <div class="nav-section-title">其他</div>
        <router-link to="/todos" class="nav-item">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="none">
            <path d="M3 4h10M3 8h10M3 12h6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          <span class="nav-label">待办事项列表</span>
          <span class="nav-badge warning">2</span>
        </router-link>
        <router-link to="/milestones" class="nav-item">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="none">
            <path d="M3 8h10M8 3v10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          <span class="nav-label">里程碑</span>
        </router-link>
        <router-link to="/snippets" class="nav-item">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="none">
            <path d="M5 4l-3 4 3 4M11 4l3 4-3 4M9 2l-2 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <span class="nav-label">代码片段</span>
        </router-link>
        <router-link to="/activity" class="nav-item">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="none">
            <path d="M1 8h3l2-5 2 10 2-5h5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <span class="nav-label">动态</span>
        </router-link>
      </div>
      
      <!-- Explore Section -->
      <div class="nav-section">
        <div class="nav-section-title">探索</div>
        <router-link to="/explore/projects" class="nav-item">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
            <path d="M8 5v3l2 2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          <span class="nav-label">项目</span>
        </router-link>
        <router-link to="/explore/groups" class="nav-item">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="none">
            <circle cx="5" cy="6" r="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="11" cy="6" r="2" stroke="currentColor" stroke-width="1.5"/>
            <path d="M1 14a4 4 0 018 0M7 14a4 4 0 018 0" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          <span class="nav-label">群组</span>
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
defineProps<{
  collapsed: boolean
  hidden: boolean
}>()

defineEmits(['toggle-collapse'])
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
  
  &.router-link-active,
  &.router-link-exact-active {
    background: $bg-sidebar-active;
    color: $text-light;
    
    .nav-icon {
      color: $brand-primary;
    }
  }
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
