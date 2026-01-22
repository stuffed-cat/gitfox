<template>
  <header class="app-header">
    <div class="header-left">
      <router-link to="/" class="logo">
        <svg width="32" height="32" viewBox="0 0 32 32" fill="none">
          <circle cx="16" cy="16" r="14" stroke="currentColor" stroke-width="2"/>
          <path d="M10 16l4 4 8-8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span>DevOps</span>
      </router-link>
      <nav class="main-nav">
        <router-link to="/projects">项目</router-link>
      </nav>
    </div>
    <div class="header-right">
      <div class="user-menu">
        <button class="user-btn" @click="toggleMenu">
          <span class="avatar">{{ userInitial }}</span>
          <span class="username">{{ user?.display_name || user?.username }}</span>
        </button>
        <div v-if="menuOpen" class="dropdown-menu">
          <router-link to="/profile" class="dropdown-item">个人设置</router-link>
          <hr />
          <button class="dropdown-item" @click="handleLogout">退出登录</button>
        </div>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = useRouter()
const authStore = useAuthStore()

const menuOpen = ref(false)
const user = computed(() => authStore.user)
const userInitial = computed(() => {
  const name = user.value?.display_name || user.value?.username || 'U'
  return name.charAt(0).toUpperCase()
})

function toggleMenu() {
  menuOpen.value = !menuOpen.value
}

function handleLogout() {
  authStore.logout()
  router.push('/login')
}
</script>

<style lang="scss" scoped>
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: $header-height;
  padding: 0 $spacing-lg;
  background: $bg-primary;
  border-bottom: 1px solid $border-color;
  position: sticky;
  top: 0;
  z-index: 100;
}

.header-left {
  display: flex;
  align-items: center;
  gap: $spacing-xl;
}

.logo {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  color: $text-primary;
  font-weight: 600;
  font-size: $font-size-lg;
  text-decoration: none;
  
  svg {
    color: $primary-color;
  }
}

.main-nav {
  display: flex;
  gap: $spacing-md;
  
  a {
    color: $text-secondary;
    text-decoration: none;
    padding: $spacing-sm $spacing-md;
    border-radius: $border-radius;
    transition: all $transition-fast;
    
    &:hover {
      color: $text-primary;
      background: $bg-secondary;
    }
    
    &.router-link-active {
      color: $primary-color;
      background: rgba($primary-color, 0.1);
    }
  }
}

.header-right {
  display: flex;
  align-items: center;
}

.user-menu {
  position: relative;
}

.user-btn {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  padding: $spacing-xs $spacing-sm;
  background: transparent;
  border: none;
  cursor: pointer;
  border-radius: $border-radius;
  
  &:hover {
    background: $bg-secondary;
  }
}

.avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: $primary-color;
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
}

.username {
  color: $text-primary;
  font-weight: 500;
}

.dropdown-menu {
  position: absolute;
  top: 100%;
  right: 0;
  margin-top: $spacing-xs;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  box-shadow: $shadow-md;
  min-width: 160px;
  overflow: hidden;
  
  hr {
    border: none;
    border-top: 1px solid $border-color;
    margin: 0;
  }
}

.dropdown-item {
  display: block;
  width: 100%;
  padding: $spacing-sm $spacing-md;
  text-align: left;
  background: transparent;
  border: none;
  color: $text-primary;
  text-decoration: none;
  cursor: pointer;
  
  &:hover {
    background: $bg-secondary;
  }
}
</style>
