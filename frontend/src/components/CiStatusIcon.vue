<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 16 16"
    :style="{ color: statusColor }"
    fill="currentColor"
    aria-hidden="true"
  >
    <path fill-rule="evenodd" :d="iconPath" />
  </svg>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { navIcons } from '@/navigation/icons'

const props = withDefaults(defineProps<{
  status: string
  size?: number
}>(), {
  size: 16
})

const iconPath = computed(() => {
  const map: Record<string, string> = {
    pending:  navIcons.statusPending,
    running:  navIcons.statusRunning,
    success:  navIcons.statusSuccess,
    failed:   navIcons.statusFailed,
    error:    navIcons.statusError,
    canceled: navIcons.statusCanceled,
    skipped:  navIcons.statusSkipped,
    blocked:  navIcons.statusBlocked,
    warning:  navIcons.statusWarning,
    manual:   navIcons.statusManual,
    created:  navIcons.statusCreated,
  }
  return map[props.status] ?? navIcons.statusCreated
})

const statusColor = computed(() => {
  const colors: Record<string, string> = {
    pending:  '#8b8fa9',
    running:  '#1f75cb',
    success:  '#108548',
    failed:   '#dd2b0e',
    error:    '#dd2b0e',
    canceled: '#737278',
    skipped:  '#737278',
    warning:  '#c17d10',
    blocked:  '#8b8fa9',
    manual:   '#1f75cb',
    created:  '#8b8fa9',
  }
  return colors[props.status] ?? '#8b8fa9'
})
</script>
