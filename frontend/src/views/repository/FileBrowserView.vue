<template>
  <div class="file-browser">
    <div class="browser-header">
      <div class="branch-selector">
        <select v-model="currentBranch" class="form-control form-control-sm" @change="loadTree">
          <option v-for="branch in branches" :key="branch.name" :value="branch.name">
            {{ branch.name }}
          </option>
        </select>
      </div>
      <div class="path-breadcrumb">
        <router-link :to="`${projectPath}/-/tree/${currentBranch}`" @click="navigateTo('')">
          {{ project?.name }}
        </router-link>
        <template v-for="(segment, index) in pathSegments" :key="index">
          <span class="separator">/</span>
          <router-link
            :to="`${projectPath}/-/tree/${currentBranch}/${pathSegments.slice(0, index + 1).join('/')}`"
            @click="navigateTo(pathSegments.slice(0, index + 1).join('/'))"
          >
            {{ segment }}
          </router-link>
        </template>
      </div>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <div v-else-if="viewingFile" class="file-content">
      <div class="file-header">
        <span>{{ currentFileName }}</span>
        <span class="file-size">{{ formatSize(fileContent?.length || 0) }}</span>
      </div>
      <pre class="code-block"><code>{{ fileContent }}</code></pre>
    </div>
    
    <div v-else class="tree-view">
      <table class="file-table">
        <thead>
          <tr>
            <th>名称</th>
            <th>最后提交</th>
            <th>时间</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="item in treeItems"
            :key="item.name"
            class="tree-item"
            @click="handleItemClick(item)"
          >
            <td class="item-name">
              <span class="item-icon">{{ item.type === 'tree' ? '📁' : '📄' }}</span>
              {{ item.name }}
            </td>
            <td class="item-commit">{{ item.last_commit_message || '-' }}</td>
            <td class="item-time">{{ formatDate(item.last_commit_time) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import api from '@/api'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import type { Project, Branch } from '@/types'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

interface TreeItem {
  name: string
  type: 'tree' | 'blob'
  path: string
  last_commit_message?: string
  last_commit_time?: string
}

const props = defineProps<{
  project?: Project
}>()

const route = useRoute()
const router = useRouter()

const loading = ref(false)
const branches = ref<Branch[]>([])
const currentBranch = ref('')
const currentPath = ref('')
const treeItems = ref<TreeItem[]>([])
const fileContent = ref<string | null>(null)
const viewingFile = ref(false)

// GitLab 风格的项目路径
const projectPath = computed(() => {
  if (!props.project?.owner_name) return ''
  return `/${props.project.owner_name}/${props.project.slug}`
})

const pathSegments = computed(() => {
  return currentPath.value ? currentPath.value.split('/').filter(Boolean) : []
})

const currentFileName = computed(() => {
  const segments = pathSegments.value
  return segments.length > 0 ? segments[segments.length - 1] : ''
})

function formatDate(date?: string) {
  if (!date) return '-'
  return dayjs(date).fromNow()
}

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

async function loadBranches() {
  if (!props.project?.id) return
  try {
    const response = await api.getBranches(props.project.id)
    branches.value = response.data
    if (branches.value.length > 0 && !currentBranch.value) {
      currentBranch.value = props.project.default_branch || branches.value[0].name
    }
  } catch (error) {
    console.error('Failed to load branches:', error)
  }
}

async function loadTree() {
  if (!props.project?.id || !currentBranch.value) return
  loading.value = true
  viewingFile.value = false
  
  try {
    const response = await api.getTree(props.project.id, {
      ref: currentBranch.value,
      path: currentPath.value
    })
    treeItems.value = response.data
  } catch (error) {
    console.error('Failed to load tree:', error)
    treeItems.value = []
  } finally {
    loading.value = false
  }
}

async function loadFileContent(path: string) {
  if (!props.project?.id) return
  loading.value = true
  viewingFile.value = true
  
  try {
    const response = await api.getFileContent(props.project.id, currentBranch.value, path)
    fileContent.value = response.data.content
  } catch (error) {
    console.error('Failed to load file:', error)
    fileContent.value = null
  } finally {
    loading.value = false
  }
}

function handleItemClick(item: TreeItem) {
  if (item.type === 'tree') {
    currentPath.value = item.path
    loadTree()
  } else {
    currentPath.value = item.path
    loadFileContent(item.path)
  }
}

function navigateTo(path: string) {
  currentPath.value = path
  viewingFile.value = false
  loadTree()
}

watch(() => props.project?.id, () => {
  loadBranches().then(() => loadTree())
}, { immediate: true })

watch(() => route.params.path, (path) => {
  if (path && typeof path === 'string') {
    currentPath.value = path
    loadTree()
  }
})
</script>

<style lang="scss" scoped>
.file-browser {
  padding: $spacing-lg;
}

.browser-header {
  display: flex;
  align-items: center;
  gap: $spacing-lg;
  margin-bottom: $spacing-lg;
}

.branch-selector {
  select {
    width: auto;
    min-width: 150px;
  }
}

.path-breadcrumb {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  
  a {
    color: $primary-color;
    text-decoration: none;
    
    &:hover {
      text-decoration: underline;
    }
  }
  
  .separator {
    margin: 0 $spacing-xs;
    color: $text-muted;
  }
}

.file-table {
  width: 100%;
  border-collapse: collapse;
  
  th, td {
    padding: $spacing-sm $spacing-md;
    text-align: left;
    border-bottom: 1px solid $border-color;
  }
  
  th {
    font-weight: 500;
    color: $text-muted;
    font-size: $font-size-sm;
  }
}

.tree-item {
  cursor: pointer;
  transition: background $transition-fast;
  
  &:hover {
    background: $bg-secondary;
  }
}

.item-name {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
}

.item-icon {
  font-size: 16px;
}

.item-commit {
  color: $text-muted;
  font-size: $font-size-sm;
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-time {
  color: $text-muted;
  font-size: $font-size-sm;
  white-space: nowrap;
}

.file-content {
  border: 1px solid $border-color;
  border-radius: $border-radius;
  overflow: hidden;
}

.file-header {
  display: flex;
  justify-content: space-between;
  padding: $spacing-sm $spacing-md;
  background: $bg-secondary;
  border-bottom: 1px solid $border-color;
  font-size: $font-size-sm;
  
  .file-size {
    color: $text-muted;
  }
}

.code-block {
  margin: 0;
  padding: $spacing-md;
  overflow-x: auto;
  font-family: 'JetBrains Mono', monospace;
  font-size: $font-size-sm;
  line-height: 1.6;
  background: $bg-primary;
}
</style>
