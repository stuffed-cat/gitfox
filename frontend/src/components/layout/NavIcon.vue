<template>
  <svg class="nav-icon" viewBox="0 0 16 16" fill="none">
    <path 
      :d="iconPath" 
      stroke="currentColor" 
      stroke-width="1.5" 
      stroke-linecap="round" 
      stroke-linejoin="round"
    />
  </svg>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { navIcons } from '@/navigation/icons'

const props = defineProps<{
  name: string
}>()

const iconPath = computed(() => {
  return navIcons[props.name] || navIcons.file
})
</script>

<style scoped>
.nav-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}
</style>
