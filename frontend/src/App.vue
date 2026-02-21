<template>
  <div id="app" :class="{ 'sidebar-collapsed': sidebarCollapsed, 'sidebar-hidden': !showSidebar }">
    <!-- Auth pages (login/register) - no header/sidebar -->
    <template v-if="isAuthPage">
      <router-view />
    </template>
    <!-- All other pages - show header and sidebar -->
    <template v-else>
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
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import AppHeader from '@/components/layout/AppHeader.vue'
import AppSidebar from '@/components/layout/AppSidebar.vue'

const route = useRoute()

// 判断是否是认证页面（不显示侧边栏的页面）
const isAuthPage = computed(() => {
  // 所有标记为 guest 的页面都不显示头部和侧边栏
  return route.meta.guest === true
})

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

// Mobile layout
@media (max-width: $breakpoint-md) {
  .app-main {
    margin-left: 0;
  }
}
</style>

