import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Group } from '@/types'

export const useNamespaceStore = defineStore('namespace', () => {
  const currentNamespaceType = ref<'user' | 'group' | null>(null)
  const currentNamespace = ref<string>('')
  const currentGroup = ref<Group | null>(null)

  function setNamespaceContext(type: 'user' | 'group', path: string, group?: Group) {
    currentNamespaceType.value = type
    currentNamespace.value = path
    currentGroup.value = group || null
  }

  function clearNamespaceContext() {
    currentNamespaceType.value = null
    currentNamespace.value = ''
    currentGroup.value = null
  }

  return {
    currentNamespaceType,
    currentNamespace,
    currentGroup,
    setNamespaceContext,
    clearNamespaceContext
  }
})
