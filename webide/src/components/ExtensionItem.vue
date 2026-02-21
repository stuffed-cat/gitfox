<template>
  <div class="extension-item" @click="handleClick">
    <div class="extension-icon">
      <svg viewBox="0 0 24 24" fill="none">
        <rect x="3" y="3" width="18" height="18" rx="2" fill="currentColor" opacity="0.2"/>
        <rect x="3" y="3" width="18" height="18" rx="2" stroke="currentColor" stroke-width="1.5"/>
      </svg>
    </div>
    <div class="extension-info">
      <div class="extension-name">{{ extension.name }}</div>
      <div class="extension-meta">
        <span class="publisher">{{ extension.publisher }}</span>
        <span class="version">v{{ extension.version }}</span>
      </div>
      <div class="extension-description">{{ extension.description }}</div>
    </div>
    <div class="extension-actions">
      <button 
        v-if="extension.enabled !== undefined" 
        class="btn btn-sm"
        :class="extension.enabled ? 'btn-secondary' : 'btn-primary'"
        @click.stop="$emit('toggle', extension)"
      >
        {{ extension.enabled ? '禁用' : '启用' }}
      </button>
      <button 
        v-else 
        class="btn btn-sm btn-primary"
        @click.stop="$emit('install', extension)"
      >
        安装
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Extension } from '@/types'

defineProps<{
  extension: Extension
}>()

defineEmits<{
  (e: 'toggle', ext: Extension): void
  (e: 'install', ext: Extension): void
  (e: 'uninstall', ext: Extension): void
}>()

function handleClick() {
  // Show extension details
}
</script>

<style lang="scss" scoped>
.extension-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px;
  cursor: pointer;
  transition: background var(--ide-transition-fast);
  
  &:hover {
    background: var(--ide-surface-hover);
  }
}

.extension-icon {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  
  svg {
    width: 32px;
    height: 32px;
    color: var(--ide-primary);
  }
}

.extension-info {
  flex: 1;
  min-width: 0;
}

.extension-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--ide-text);
  margin-bottom: 2px;
}

.extension-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  color: var(--ide-text-muted);
  margin-bottom: 4px;
}

.extension-description {
  font-size: 12px;
  color: var(--ide-text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.extension-actions {
  flex-shrink: 0;
}
</style>
