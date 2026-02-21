<template>
  <svg viewBox="0 0 24 24" fill="none" :style="{ width: `${size}px`, height: `${size}px` }">
    <path :d="iconPath" fill="currentColor" opacity="0.2"/>
    <path :d="iconPath" stroke="currentColor" stroke-width="1.5"/>
  </svg>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  name: string
  size?: number
}>(), {
  size: 16
})

const iconPath = computed(() => {
  const ext = props.name.split('.').pop()?.toLowerCase() || ''
  
  // Return specific icon path based on extension
  const iconPaths: Record<string, string> = {
    // Default file icon
    default: 'M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6z'
  }
  
  return iconPaths[ext] || iconPaths.default
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
    dockerfile: '#2496ed'
  }
  
  return colors[ext] || 'currentColor'
})
</script>

<style lang="scss" scoped>
svg {
  color: v-bind(iconColor);
}
</style>
