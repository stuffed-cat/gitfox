<template>
  <div class="branch-list">
    <div class="list-header">
      <h3>分支 ({{ branches.length }})</h3>
      <button class="btn btn-primary" @click="showCreateModal = true">
        + 新建分支
      </button>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <div v-else-if="branches.length === 0" class="empty-state">
      <h3>暂无分支</h3>
    </div>
    
    <div v-else class="branches">
      <div
        v-for="branch in branches"
        :key="branch.name"
        class="branch-item"
        :class="{ default: branch.is_default }"
      >
        <div class="branch-info">
          <div class="branch-name">
            <span class="icon branch-icon">
              <svg viewBox="0 0 16 16" fill="currentColor" width="14" height="14">
                <path d="M11.75 2.5a.75.75 0 100 1.5.75.75 0 000-1.5zm-2.25.75a2.25 2.25 0 113 2.122V6A2.5 2.5 0 0110 8.5H6a1 1 0 00-1 1v1.128a2.251 2.251 0 11-1.5 0V5.372a2.25 2.25 0 111.5 0v1.836A2.492 2.492 0 016 7h4a1 1 0 001-1v-.628A2.25 2.25 0 019.5 3.25zM4.25 12a.75.75 0 100 1.5.75.75 0 000-1.5zM3.5 3.25a.75.75 0 111.5 0 .75.75 0 01-1.5 0z"/>
              </svg>
            </span>
            {{ branch.name }}
            <span v-if="branch.is_default" class="badge badge-primary">默认</span>
            <span v-if="branch.is_protected" class="badge badge-warning">保护</span>
          </div>
          <div class="branch-meta">
            <span v-if="branch.commit?.message">
              {{ branch.commit.message.substring(0, 50) }}
            </span>
            <span class="separator" v-if="branch.commit?.message">·</span>
            <span>{{ formatCommitDate(branch.commit?.committed_date) }}</span>
          </div>
        </div>
        <div class="branch-actions">
          <router-link
            :to="`/${project?.owner_name}/${project?.name}/-/tree/${branch.name}`"
            class="btn btn-outline btn-sm"
          >
            浏览
          </router-link>
          <button
            v-if="!branch.is_default && !branch.is_protected"
            class="btn btn-danger btn-sm"
            @click="handleDelete(branch.name)"
          >
            删除
          </button>
        </div>
      </div>
    </div>
    
    <!-- 创建分支弹窗 -->
    <div v-if="showCreateModal" class="modal-overlay" @click.self="showCreateModal = false">
      <div class="modal">
        <div class="modal-header">
          <h3>创建新分支</h3>
          <button class="close-btn" @click="showCreateModal = false">&times;</button>
        </div>
        <form @submit.prevent="handleCreate" class="modal-body">
          <div class="form-group">
            <label for="branchName">分支名称</label>
            <input
              id="branchName"
              v-model="newBranch.name"
              type="text"
              class="form-control"
              placeholder="feature/new-feature"
              required
            />
          </div>
          <div class="form-group">
            <label for="sourceBranch">基于分支</label>
            <select id="sourceBranch" v-model="newBranch.source" class="form-control">
              <option v-for="branch in branches" :key="branch.name" :value="branch.name">
                {{ branch.name }}
              </option>
            </select>
          </div>
          <div class="modal-actions">
            <button type="button" class="btn btn-outline" @click="showCreateModal = false">
              取消
            </button>
            <button type="submit" class="btn btn-primary" :disabled="creating">
              {{ creating ? '创建中...' : '创建' }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import api from '@/api'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import type { Project, BranchInfo } from '@/types'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

const props = defineProps<{
  project?: Project
}>()

const loading = ref(false)
const branches = ref<BranchInfo[]>([])
const showCreateModal = ref(false)
const creating = ref(false)
const newBranch = reactive({
  name: '',
  source: ''
})

function formatCommitDate(timestamp?: number) {
  if (!timestamp) return '-'
  return dayjs.unix(timestamp).fromNow()
}

async function loadBranches() {
  if (!props.project?.owner_name || !props.project?.name) return
  loading.value = true
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    branches.value = await api.branches.list(path)
    if (branches.value.length > 0 && !newBranch.source) {
      // 使用API返回的默认分支或第一个分支
      const defaultBranch = branches.value.find(b => b.is_default)
      newBranch.source = defaultBranch?.name || branches.value[0].name
    }
  } catch (error) {
    console.error('Failed to load branches:', error)
  } finally {
    loading.value = false
  }
}

async function handleCreate() {
  if (!props.project?.owner_name || !props.project?.name) return
  creating.value = true
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    await api.branches.create(path, newBranch.name, newBranch.source)
    showCreateModal.value = false
    newBranch.name = ''
    loadBranches()
  } catch (error) {
    console.error('Failed to create branch:', error)
  } finally {
    creating.value = false
  }
}

async function handleDelete(name: string) {
  if (!props.project?.owner_name || !props.project?.name) return
  if (!confirm(`确定要删除分支 "${name}" 吗？此操作不可撤销。`)) return
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    await api.branches.delete(path, name)
    loadBranches()
  } catch (error) {
    console.error('Failed to delete branch:', error)
  }
}

watch([() => props.project?.owner_name, () => props.project?.name], () => {
  loadBranches()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.branch-list {
  padding: $spacing-lg;
}

.list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: $spacing-lg;
  
  h3 {
    margin: 0;
  }
}

.branches {
  display: flex;
  flex-direction: column;
}

.branch-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-md;
  border-bottom: 1px solid $border-color;
  
  &:last-child {
    border-bottom: none;
  }
  
  &:hover {
    background: $bg-secondary;
  }
  
  &.default {
    background: rgba($primary-color, 0.05);
  }
}

.branch-info {
  flex: 1;
  min-width: 0;
}

.branch-name {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  font-weight: 500;
  margin-bottom: $spacing-xs;
  
  .icon {
    font-size: 16px;
  }
  
  .badge {
    font-size: $font-size-xs;
  }
}

.branch-meta {
  font-size: $font-size-sm;
  color: $text-muted;
  
  .separator {
    margin: 0 $spacing-xs;
  }
}

.branch-actions {
  display: flex;
  gap: $spacing-sm;
  margin-left: $spacing-md;
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: $bg-primary;
  border-radius: $border-radius-lg;
  width: 100%;
  max-width: 450px;
  box-shadow: $shadow-lg;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-lg;
  border-bottom: 1px solid $border-color;
  
  h3 {
    margin: 0;
  }
  
  .close-btn {
    background: none;
    border: none;
    font-size: 24px;
    cursor: pointer;
    color: $text-muted;
    
    &:hover {
      color: $text-primary;
    }
  }
}

.modal-body {
  padding: $spacing-lg;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: $spacing-md;
  margin-top: $spacing-lg;
}
</style>
