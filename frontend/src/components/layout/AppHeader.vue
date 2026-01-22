<template>
  <header class="app-header">
    <div class="header-left">
      <button class="sidebar-toggle" @click="$emit('toggle-sidebar')" title="切换侧边栏">
        <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
          <path d="M3 5h14M3 10h14M3 15h14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
      
      <router-link to="/" class="logo">
        <div class="logo-icon">
          <svg width="28" height="28" viewBox="0 0 28 28" fill="none">
            <path d="M14 0L17.5 10.5H28L19.5 17L23 28L14 21.5L5 28L8.5 17L0 10.5H10.5L14 0Z" fill="currentColor"/>
          </svg>
        </div>
        <span class="logo-text">DevOps</span>
      </router-link>
      
      <nav class="nav-links">
        <router-link to="/" class="nav-link">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M8 1L1 6v8a1 1 0 001 1h4v-5h4v5h4a1 1 0 001-1V6L8 1z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          首页
        </router-link>
        <router-link to="/projects" class="nav-link">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M2 3a1 1 0 011-1h10a1 1 0 011 1v10a1 1 0 01-1 1H3a1 1 0 01-1-1V3z" stroke="currentColor" stroke-width="1.5"/>
            <path d="M5 6h6M5 9h4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          项目
        </router-link>
      </nav>
    </div>
    
    <div class="header-center">
      <div class="search-box">
        <svg class="search-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
          <circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.5"/>
          <path d="M11 11l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <input 
          type="text" 
          placeholder="搜索或转到..."
          class="search-input"
          @focus="searchFocused = true"
          @blur="searchFocused = false"
        />
        <span class="search-shortcut">/</span>
      </div>
    </div>
    
    <div class="header-right">
      <button class="header-btn" title="新建">
        <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
          <path d="M9 4v10M4 9h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
      
      <button class="header-btn" title="待办事项">
        <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
          <path d="M3 5h12M3 9h12M3 13h8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <span class="badge-count">2</span>
      </button>
      
      <button class="header-btn" title="帮助">
        <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
          <circle cx="9" cy="9" r="7" stroke="currentColor" stroke-width="1.5"/>
          <path d="M7 7a2 2 0 113 1.73V10M9 13v.01" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
      
      <div class="user-dropdown" ref="userDropdownRef">
        <button class="user-btn" @click="toggleUserMenu">
          <span class="avatar avatar-md">{{ userInitial }}</span>
          <svg class="chevron" :class="{ rotated: userMenuOpen }" width="12" height="12" viewBox="0 0 12 12">
            <path d="M3 4.5L6 7.5L9 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
        
        <Transition name="dropdown">
          <div v-if="userMenuOpen" class="dropdown-menu">
            <div class="dropdown-header">
              <div class="user-info">
                <span class="avatar avatar-lg">{{ userInitial }}</span>
                <div class="user-details">
                  <div class="user-name">{{ user?.display_name || user?.username }}</div>
                  <div class="user-email">{{ user?.email }}</div>
                </div>
              </div>
            </div>
            <div class="dropdown-divider"></div>
            <router-link to="/profile" class="dropdown-item" @click="userMenuOpen = false">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="5" r="3" stroke="currentColor" stroke-width="1.5"/>
                <path d="M2 14a6 6 0 0112 0" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              个人设置
            </router-link>
            <router-link to="/settings" class="dropdown-item" @click="userMenuOpen = false">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="8" r="2" stroke="currentColor" stroke-width="1.5"/>
                <path d="M8 1v2M8 13v2M1 8h2M13 8h2M3 3l1.5 1.5M11.5 11.5l1.5 1.5M3 13l1.5-1.5M11.5 4.5l1.5-1.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              偏好设置
            </router-link>
            <div class="dropdown-divider"></div>
            <button class="dropdown-item danger" @click="handleLogout">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M6 14H3a1 1 0 01-1-1V3a1 1 0 011-1h3M11 11l3-3-3-3M14 8H6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              退出登录
            </button>
          </div>
        </Transition>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

defineEmits(['toggle-sidebar'])

const router = useRouter()
const authStore = useAuthStore()

const userMenuOpen = ref(false)
const searchFocused = ref(false)
const userDropdownRef = ref<HTMLElement | null>(null)

const user = computed(() => authStore.user)
const userInitial = computed(() => {
  const name = user.value?.display_name || user.value?.username || 'U'
  return name.charAt(0).toUpperCase()
})

function toggleUserMenu() {
  userMenuOpen.value = !userMenuOpen.value
}

function handleLogout() {
  userMenuOpen.value = false
  authStore.logout()
  router.push('/login')
}

// 点击外部关闭下拉菜单
function handleClickOutside(event: MouseEvent) {
  if (userDropdownRef.value && !userDropdownRef.value.contains(event.target as Node)) {
    userMenuOpen.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<style lang="scss" scoped>
.app-header {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: $header-height;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 $spacing-4;
  background: $bg-sidebar;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  z-index: $z-fixed;
}

.header-left {
  display: flex;
  align-items: center;
  gap: $spacing-4;
}

.sidebar-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: $border-radius;
  background: transparent;
  border: none;
  color: $gray-400;
  cursor: pointer;
  transition: all $transition-fast;
  
  &:hover {
    background: $bg-sidebar-hover;
    color: $text-light;
  }
}

.logo {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  text-decoration: none;
  
  .logo-icon {
    width: 28px;
    height: 28px;
    color: #e24329;
  }
  
  .logo-text {
    font-size: $font-size-lg;
    font-weight: $font-weight-semibold;
    color: $text-light;
  }
}

.nav-links {
  display: flex;
  align-items: center;
  gap: $spacing-1;
  margin-left: $spacing-4;
}

.nav-link {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-2 $spacing-3;
  font-size: $font-size-sm;
  color: $gray-300;
  text-decoration: none;
  border-radius: $border-radius;
  transition: all $transition-fast;
  
  svg {
    opacity: 0.7;
  }
  
  &:hover {
    background: $bg-sidebar-hover;
    color: $text-light;
    
    svg {
      opacity: 1;
    }
  }
  
  &.router-link-active {
    background: $bg-sidebar-active;
    color: $text-light;
    
    svg {
      opacity: 1;
    }
  }
}

.header-center {
  flex: 1;
  max-width: 480px;
  margin: 0 $spacing-6;
}

.search-box {
  position: relative;
  display: flex;
  align-items: center;
}

.search-icon {
  position: absolute;
  left: $spacing-3;
  color: $gray-400;
  pointer-events: none;
}

.search-input {
  width: 100%;
  padding: $spacing-2 $spacing-3;
  padding-left: 36px;
  padding-right: 36px;
  font-size: $font-size-sm;
  color: $text-light;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid transparent;
  border-radius: $border-radius;
  transition: all $transition-fast;
  
  &::placeholder {
    color: $gray-400;
  }
  
  &:hover {
    background: rgba(255, 255, 255, 0.12);
  }
  
  &:focus {
    outline: none;
    background: rgba(255, 255, 255, 0.15);
    border-color: $brand-primary;
  }
}

.search-shortcut {
  position: absolute;
  right: $spacing-3;
  padding: 2px 6px;
  font-size: $font-size-xs;
  color: $gray-400;
  background: rgba(255, 255, 255, 0.1);
  border-radius: $border-radius-sm;
}

.header-right {
  display: flex;
  align-items: center;
  gap: $spacing-2;
}

.header-btn {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: $border-radius;
  background: transparent;
  border: none;
  color: $gray-400;
  cursor: pointer;
  transition: all $transition-fast;
  
  &:hover {
    background: $bg-sidebar-hover;
    color: $text-light;
  }
}

.badge-count {
  position: absolute;
  top: 2px;
  right: 2px;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  font-size: 10px;
  font-weight: $font-weight-semibold;
  color: white;
  background: $color-danger;
  border-radius: $border-radius-full;
  display: flex;
  align-items: center;
  justify-content: center;
}

.user-dropdown {
  position: relative;
  margin-left: $spacing-2;
}

.user-btn {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-1;
  background: transparent;
  border: none;
  cursor: pointer;
  border-radius: $border-radius;
  transition: all $transition-fast;
  
  &:hover {
    background: $bg-sidebar-hover;
  }
  
  .chevron {
    color: $gray-400;
    transition: transform $transition-fast;
    
    &.rotated {
      transform: rotate(180deg);
    }
  }
}

.avatar {
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: $border-radius-full;
  background: $brand-gradient;
  color: white;
  font-weight: $font-weight-semibold;
  
  &.avatar-md {
    width: 28px;
    height: 28px;
    font-size: $font-size-sm;
  }
  
  &.avatar-lg {
    width: 40px;
    height: 40px;
    font-size: $font-size-lg;
  }
}

.dropdown-menu {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  min-width: 240px;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
  box-shadow: $shadow-lg;
  overflow: hidden;
  z-index: $z-dropdown;
}

.dropdown-header {
  padding: $spacing-4;
  background: $bg-secondary;
}

.user-info {
  display: flex;
  align-items: center;
  gap: $spacing-3;
}

.user-details {
  flex: 1;
  min-width: 0;
}

.user-name {
  font-weight: $font-weight-semibold;
  color: $text-primary;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.user-email {
  font-size: $font-size-sm;
  color: $text-secondary;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.dropdown-divider {
  height: 1px;
  background: $border-color;
}

.dropdown-item {
  display: flex;
  align-items: center;
  gap: $spacing-3;
  width: 100%;
  padding: $spacing-3 $spacing-4;
  font-size: $font-size-base;
  color: $text-primary;
  text-decoration: none;
  background: transparent;
  border: none;
  cursor: pointer;
  transition: background $transition-fast;
  
  svg {
    color: $text-secondary;
  }
  
  &:hover {
    background: $bg-secondary;
  }
  
  &.danger {
    color: $color-danger;
    
    svg {
      color: $color-danger;
    }
    
    &:hover {
      background: $color-danger-light;
    }
  }
}

// Dropdown transition
.dropdown-enter-active,
.dropdown-leave-active {
  transition: all 0.15s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
