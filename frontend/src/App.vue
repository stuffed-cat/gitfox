<template>
  <div id="app" :class="{ 'sidebar-collapsed': sidebarCollapsed, 'sidebar-hidden': !showSidebar }">
    <template v-if="isAuthenticated">
      <AppHeader @toggle-sidebar="toggleSidebar" />
      <div class="app-layout">
        <AppSidebar 
          :collapsed="sidebarCollapsed" 
          :hidden="!showSidebar"
          @toggle-collapse="toggleSidebarCollapse"
        />
        <main class="app-main" :class="{ 'sidebar-collapsed': sidebarCollapsed, 'sidebar-hidden': !showSidebar }">
          <router-view />
        </main>
      </div>
    </template>
    <template v-else>
      <router-view />
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import AppHeader from '@/components/layout/AppHeader.vue'
import AppSidebar from '@/components/layout/AppSidebar.vue'

const route = useRoute()
const authStore = useAuthStore()
const isAuthenticated = computed(() => authStore.isAuthenticated)

// Sidebar state
const sidebarCollapsed = ref(false)
const showSidebar = ref(true)
const windowWidth = ref(window.innerWidth)

// 响应式侧边栏
function handleResize() {
  windowWidth.value = window.innerWidth
  if (windowWidth.value < 768) {
    showSidebar.value = false
  } else {
    showSidebar.value = true
  }
}

function toggleSidebar() {
  if (windowWidth.value < 768) {
    showSidebar.value = !showSidebar.value
  } else {
    sidebarCollapsed.value = !sidebarCollapsed.value
  }
}

function toggleSidebarCollapse() {
  sidebarCollapsed.value = !sidebarCollapsed.value
}

// 路由变化时在移动端关闭侧边栏
watch(() => route.path, () => {
  if (windowWidth.value < 768) {
    showSidebar.value = false
  }
})

onMounted(() => {
  handleResize()
  window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
})
</script>

<style lang="scss">
@import '@/styles/main.scss';

#app {
  min-height: 100vh;
}

.app-layout {
  display: flex;
  min-height: calc(100vh - $header-height);
  padding-top: $header-height;
}

.app-sidebar {
  position: fixed;
  top: $header-height;
  left: 0;
  bottom: 0;
  width: $sidebar-width;
  z-index: $z-sticky;
  transition: width $transition-normal, transform $transition-normal;
}

.sidebar-collapsed .app-sidebar {
  width: $sidebar-collapsed-width;
}

.sidebar-hidden .app-sidebar {
  transform: translateX(-100%);
}

.app-main {
  flex: 1;
  margin-left: $sidebar-width;
  transition: margin-left $transition-normal;
  background: $bg-secondary;
  min-height: calc(100vh - $header-height);
}

.sidebar-collapsed .app-main {
  margin-left: $sidebar-collapsed-width;
}

.sidebar-hidden .app-main {
  margin-left: 0;
}

// Mobile overlay for sidebar
@media (max-width: $breakpoint-md) {
  .app-sidebar {
    z-index: $z-modal;
  }
  
  .app-main {
    margin-left: 0;
  }
}
</style>

