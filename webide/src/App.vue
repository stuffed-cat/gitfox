<template>
  <div class="gitfox-ide" :class="{ 'theme-dark': isDarkTheme }">
    <router-view />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useThemeStore } from './stores/theme'

const themeStore = useThemeStore()
const isDarkTheme = computed(() => themeStore.theme === 'dark')
</script>

<style lang="scss">
.gitfox-ide {
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
</style>
