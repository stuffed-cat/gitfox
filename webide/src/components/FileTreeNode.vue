<template>
  <div
    class="tree-node"
    :style="{ paddingLeft: `${depth * 16 + 8}px` }"
  >
    <div
      class="node-content"
      :class="{ 
        active: node.type === 'file' && activePath === node.path,
        expanded: node.type === 'directory' && isExpanded
      }"
      @click="handleClick"
      @contextmenu.prevent="handleContextMenu"
    >
      <!-- Expand arrow for directories -->
      <span v-if="node.type === 'directory'" class="expand-icon" :class="{ expanded: isExpanded }">
        <svg viewBox="0 0 16 16" fill="currentColor">
          <path d="M6 4l4 4-4 4V4z"/>
        </svg>
      </span>
      <span v-else class="expand-icon spacer"></span>
      
      <!-- Icon -->
      <span class="node-icon">
        <svg v-if="node.type === 'directory'" viewBox="0 0 24 24" fill="none">
          <path v-if="isExpanded" d="M19 10H5a2 2 0 00-2 2v7a2 2 0 002 2h14a2 2 0 002-2v-7a2 2 0 00-2-2z" fill="currentColor" opacity="0.2"/>
          <path v-else d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" fill="currentColor" opacity="0.2"/>
          <path :d="isExpanded ? 'M19 10H5a2 2 0 00-2 2v7a2 2 0 002 2h14a2 2 0 002-2v-7a2 2 0 00-2-2zM3 6v4M21 6v4' : 'M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z'" stroke="currentColor" stroke-width="1.5"/>
        </svg>
        <FileIconSvg v-else :name="node.name" />
      </span>
      
      <!-- Name -->
      <span class="node-name">{{ node.name }}</span>
    </div>
    
    <!-- Children -->
    <div v-if="node.type === 'directory' && isExpanded && node.children" class="node-children">
      <FileTreeNode
        v-for="child in sortedChildren"
        :key="child.path"
        :node="child"
        :depth="depth + 1"
        :expanded-folders="expandedFolders"
        :active-path="activePath"
        @select="(path) => emit('select', path)"
        @toggle="(path) => emit('toggle', path)"
        @context-menu="(data) => emit('context-menu', data)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { FileTreeNode as FileTreeNodeType } from '@/types'
import FileIconSvg from './icons/FileIconSvg.vue'

const props = defineProps<{
  node: FileTreeNodeType
  depth: number
  expandedFolders: Set<string>
  activePath: string | null
}>()

const emit = defineEmits<{
  (e: 'select', path: string): void
  (e: 'toggle', path: string): void
  (e: 'context-menu', data: { event: MouseEvent; path: string }): void
}>()

const isExpanded = computed(() => props.expandedFolders.has(props.node.path))

const sortedChildren = computed(() => {
  if (!props.node.children) return []
  return [...props.node.children].sort((a, b) => {
    // Directories first
    if (a.type !== b.type) {
      return a.type === 'directory' ? -1 : 1
    }
    // Then alphabetically
    return a.name.localeCompare(b.name)
  })
})

function handleClick() {
  if (props.node.type === 'directory') {
    emit('toggle', props.node.path)
  } else {
    emit('select', props.node.path)
  }
}

function handleContextMenu(event: MouseEvent) {
  emit('context-menu', { event, path: props.node.path })
}
</script>

<style lang="scss" scoped>
.tree-node {
  user-select: none;
}

.node-content {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px 3px 0;
  cursor: pointer;
  border-radius: 4px;
  
  &:hover {
    background: var(--ide-sidebar-item-hover);
  }
  
  &.active {
    background: var(--ide-sidebar-item-selected);
  }
}

.expand-icon {
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: transform 0.1s;
  
  svg {
    width: 10px;
    height: 10px;
  }
  
  &.expanded {
    transform: rotate(90deg);
  }
  
  &.spacer {
    visibility: hidden;
  }
}

.node-icon {
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  
  svg {
    width: 16px;
    height: 16px;
  }
}

.node-name {
  font-size: 13px;
  color: var(--ide-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-children {
  // Children already have their own padding via depth prop
}
</style>
