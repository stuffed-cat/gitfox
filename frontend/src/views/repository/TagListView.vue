<template>
  <div class="tag-list">
    <div class="list-header">
      <h3>标签 ({{ tags.length }})</h3>
      <button class="btn btn-primary" @click="showCreateModal = true">
        + 创建标签
      </button>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <div v-else-if="tags.length === 0" class="empty-state">
      <h3>暂无标签</h3>
      <p>创建标签来标记重要版本</p>
    </div>
    
    <div v-else class="tags">
      <div v-for="tag in tags" :key="tag.name" class="tag-item">
        <div class="tag-info">
          <div class="tag-name">
            <span class="icon">🏷️</span>
            {{ tag.name }}
          </div>
          <div class="tag-meta">
            <span v-if="tag.message">{{ tag.message }}</span>
            <span class="separator" v-if="tag.message">·</span>
            <span>{{ formatDate(tag.created_at) }}</span>
            <span class="separator">·</span>
            <code>{{ tag.commit_sha?.substring(0, 8) }}</code>
          </div>
        </div>
        <div class="tag-actions">
          <router-link
            :to="`/projects/${project?.slug}/files?ref=${tag.name}`"
            class="btn btn-outline btn-sm"
          >
            浏览
          </router-link>
          <button class="btn btn-danger btn-sm" @click="handleDelete(tag.name)">
            删除
          </button>
        </div>
      </div>
    </div>
    
    <!-- 创建标签弹窗 -->
    <div v-if="showCreateModal" class="modal-overlay" @click.self="showCreateModal = false">
      <div class="modal">
        <div class="modal-header">
          <h3>创建标签</h3>
          <button class="close-btn" @click="showCreateModal = false">&times;</button>
        </div>
        <form @submit.prevent="handleCreate" class="modal-body">
          <div class="form-group">
            <label for="tagName">标签名称</label>
            <input
              id="tagName"
              v-model="newTag.name"
              type="text"
              class="form-control"
              placeholder="v1.0.0"
              required
            />
          </div>
          <div class="form-group">
            <label for="tagRef">基于</label>
            <select id="tagRef" v-model="newTag.ref" class="form-control">
              <option v-for="branch in branches" :key="branch.name" :value="branch.name">
                {{ branch.name }}
              </option>
            </select>
          </div>
          <div class="form-group">
            <label for="tagMessage">标签信息（可选）</label>
            <textarea
              id="tagMessage"
              v-model="newTag.message"
              class="form-control"
              placeholder="版本发布说明..."
              rows="3"
            ></textarea>
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
import type { Project, Branch, Tag } from '@/types'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

const props = defineProps<{
  project?: Project
}>()

const loading = ref(false)
const tags = ref<Tag[]>([])
const branches = ref<Branch[]>([])
const showCreateModal = ref(false)
const creating = ref(false)
const newTag = reactive({
  name: '',
  ref: '',
  message: ''
})

function formatDate(date: string) {
  return dayjs(date).fromNow()
}

async function loadTags() {
  if (!props.project?.id) return
  loading.value = true
  
  try {
    const response = await api.getTags(props.project.id)
    tags.value = response.data
  } catch (error) {
    console.error('Failed to load tags:', error)
  } finally {
    loading.value = false
  }
}

async function loadBranches() {
  if (!props.project?.id) return
  
  try {
    const response = await api.getBranches(props.project.id)
    branches.value = response.data
    if (branches.value.length > 0 && !newTag.ref) {
      newTag.ref = props.project.default_branch || branches.value[0].name
    }
  } catch (error) {
    console.error('Failed to load branches:', error)
  }
}

async function handleCreate() {
  if (!props.project?.id) return
  creating.value = true
  
  try {
    await api.createTag(props.project.id, {
      name: newTag.name,
      ref: newTag.ref,
      message: newTag.message || undefined
    })
    showCreateModal.value = false
    newTag.name = ''
    newTag.message = ''
    loadTags()
  } catch (error) {
    console.error('Failed to create tag:', error)
  } finally {
    creating.value = false
  }
}

async function handleDelete(name: string) {
  if (!props.project?.id) return
  if (!confirm(`确定要删除标签 "${name}" 吗？此操作不可撤销。`)) return
  
  try {
    await api.deleteTag(props.project.id, name)
    loadTags()
  } catch (error) {
    console.error('Failed to delete tag:', error)
  }
}

watch(() => props.project?.id, () => {
  loadTags()
  loadBranches()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.tag-list {
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

.tags {
  display: flex;
  flex-direction: column;
}

.tag-item {
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
}

.tag-info {
  flex: 1;
  min-width: 0;
}

.tag-name {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  font-weight: 500;
  margin-bottom: $spacing-xs;
  
  .icon {
    font-size: 16px;
  }
}

.tag-meta {
  font-size: $font-size-sm;
  color: $text-muted;
  
  .separator {
    margin: 0 $spacing-xs;
  }
  
  code {
    font-size: $font-size-xs;
  }
}

.tag-actions {
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
