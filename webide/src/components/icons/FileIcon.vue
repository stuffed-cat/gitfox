<template>
  <span class="file-icon" :style="{ color: iconColor }">
    <component :is="iconComponent" v-if="iconComponent" :size="size" />
    <DefaultFileIcon v-else :size="size" />
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import DefaultFileIcon from './DefaultFileIcon.vue'

const props = withDefaults(defineProps<{
  name: string
  size?: number
}>(), {
  size: 16
})

const iconComponent = computed(() => {
  // For now, just use default icon
  // Can be extended to load specific icons based on file type
  return null
})

const iconColor = computed(() => {
  const ext = props.name.split('.').pop()?.toLowerCase() || ''
  
  const colors: Record<string, string> = {
    ts: '#3178c6',
    tsx: '#3178c6',
    js: '#f7df1e',
    jsx: '#f7df1e',
    vue: '#42b883',
    html: '#e34f26',
    css: '#1572b6',
    scss: '#cc6699',
    json: '#f5a623',
    md: '#083fa1',
    py: '#3776ab',
    rs: '#dea584',
    go: '#00add8',
    java: '#007396',
    toml: '#9c4221',
    yaml: '#cb171e',
    yml: '#cb171e',
    sql: '#e38c00',
    sh: '#4eaa25',
    dockerfile: '#2496ed',
    gitignore: '#f05032',
    lock: '#888888'
  }
  
  return colors[ext] || 'var(--ide-text-muted)'
})
</script>

<style lang="scss" scoped>
.file-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
</style>
