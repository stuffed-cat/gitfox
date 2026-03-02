<template>
  <div class="new-issue-view">
    <div class="page-header">
      <h1>新建议题</h1>
    </div>

    <form @submit.prevent="createIssue" class="issue-form">
      <div class="form-main">
        <div class="form-group">
          <label for="title">标题</label>
          <input
            id="title"
            v-model="form.title"
            type="text"
            class="form-control"
            placeholder="输入议题标题"
            required
          />
        </div>

        <div class="form-group">
          <label for="description">描述</label>
          <div class="editor-toolbar">
            <button type="button" class="toolbar-btn" title="粗体" @click="insertMarkdown('**', '**')">
              <strong>B</strong>
            </button>
            <button type="button" class="toolbar-btn" title="斜体" @click="insertMarkdown('_', '_')">
              <em>I</em>
            </button>
            <button type="button" class="toolbar-btn" title="代码" @click="insertMarkdown('`', '`')">
              <code>&lt;&gt;</code>
            </button>
            <button type="button" class="toolbar-btn" title="链接" @click="insertMarkdown('[', '](url)')">
              🔗
            </button>
            <button type="button" class="toolbar-btn" title="列表" @click="insertMarkdown('\n- ', '')">
              ☰
            </button>
          </div>
          <textarea
            id="description"
            ref="descriptionRef"
            v-model="form.description"
            class="form-control"
            rows="12"
            placeholder="使用 Markdown 描述议题..."
          ></textarea>
          <small class="form-text">支持 Markdown 格式</small>
        </div>
      </div>

      <div class="form-sidebar">
        <div class="sidebar-section">
          <label>指派给</label>
          <select v-model="form.assignee_id" class="form-control">
            <option :value="null">未指派</option>
            <option v-for="member in members" :key="member.id" :value="member.user_id">
              {{ member.username }}
            </option>
          </select>
        </div>

        <div class="sidebar-section">
          <label>标签</label>
          <div class="labels-list">
            <label v-for="label in labels" :key="label.id" class="label-checkbox">
              <input type="checkbox" v-model="form.label_ids" :value="label.id" />
              <span class="label-color" :style="{ backgroundColor: label.color }"></span>
              {{ label.name }}
            </label>
          </div>
          <div v-if="labels.length === 0" class="no-labels">
            暂无标签
          </div>
        </div>

        <div class="sidebar-section">
          <label>里程碑</label>
          <select v-model="form.milestone_id" class="form-control">
            <option :value="null">无里程碑</option>
            <option v-for="milestone in milestones" :key="milestone.id" :value="milestone.id">
              {{ milestone.title }}
            </option>
          </select>
        </div>
      </div>

      <div class="form-actions">
        <router-link 
          :to="`/${$route.meta.namespace}/${$route.meta.projectName}/-/issues`" 
          class="btn btn-secondary"
        >
          取消
        </router-link>
        <button type="submit" class="btn btn-primary" :disabled="submitting || !form.title">
          {{ submitting ? '创建中...' : '创建议题' }}
        </button>
      </div>

      <div v-if="error" class="alert alert-danger">
        {{ error }}
      </div>
    </form>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import apiClient from '@/api'

interface Label {
  id: number
  name: string
  color: string
}

interface Milestone {
  id: number
  title: string
}

interface Member {
  id: number
  user_id: number
  username: string
}

const route = useRoute()
const router = useRouter()

const form = ref({
  title: '',
  description: '',
  assignee_id: null as number | null,
  label_ids: [] as number[],
  milestone_id: null as number | null
})

const labels = ref<Label[]>([])
const milestones = ref<Milestone[]>([])
const members = ref<Member[]>([])
const submitting = ref(false)
const error = ref('')
const descriptionRef = ref<HTMLTextAreaElement | null>(null)

const owner = computed(() => {
  const segments = route.params.pathSegments as string[]
  if (!segments || segments.length < 2) return ''
  return segments.slice(0, -1).join('/')
})
const repo = computed(() => {
  const segments = route.params.pathSegments as string[]
  if (!segments || segments.length < 2) return ''
  return segments[segments.length - 1]
})

async function loadMetadata() {
  try {
    // Load labels, milestones, members in parallel
    const [labelsRes, milestonesRes, membersRes] = await Promise.all([
      apiClient.client.get(`/projects/${owner.value}/${repo.value}/labels`).catch(() => ({ data: [] })),
      apiClient.client.get(`/projects/${owner.value}/${repo.value}/milestones`).catch(() => ({ data: [] })),
      apiClient.client.get(`/projects/${owner.value}/${repo.value}/members`).catch(() => ({ data: [] }))
    ])
    
    labels.value = labelsRes.data || []
    milestones.value = milestonesRes.data || []
    members.value = membersRes.data || []
  } catch (e) {
    console.error('Failed to load metadata:', e)
  }
}

async function createIssue() {
  submitting.value = true
  error.value = ''
  
  try {
    const response = await apiClient.client.post(
      `/projects/${owner.value}/${repo.value}/issues`,
      {
        title: form.value.title,
        description: form.value.description,
        assignee_id: form.value.assignee_id,
        label_ids: form.value.label_ids,
        milestone_id: form.value.milestone_id
      }
    )
    
    const issue = response.data
    router.push(`/${owner.value}/${repo.value}/-/issues/${issue.iid}`)
  } catch (e: any) {
    error.value = e.response?.data?.message || '创建议题失败'
  } finally {
    submitting.value = false
  }
}

function insertMarkdown(before: string, after: string) {
  const textarea = descriptionRef.value
  if (!textarea) return
  
  const start = textarea.selectionStart
  const end = textarea.selectionEnd
  const text = form.value.description
  const selectedText = text.substring(start, end)
  
  form.value.description = text.substring(0, start) + before + selectedText + after + text.substring(end)
  
  // Set cursor position
  setTimeout(() => {
    textarea.focus()
    const newPos = start + before.length + selectedText.length
    textarea.setSelectionRange(newPos, newPos)
  }, 0)
}

onMounted(() => {
  loadMetadata()
})
</script>

<style lang="scss" scoped>
@import '@/styles/variables';

.new-issue-view {
  padding: $spacing-6;
  max-width: 1200px;
}

.page-header {
  margin-bottom: $spacing-6;
  
  h1 {
    font-size: $font-size-2xl;
    font-weight: $font-weight-semibold;
    margin: 0;
  }
}

.issue-form {
  display: grid;
  grid-template-columns: 1fr 280px;
  gap: $spacing-6;
}

.form-main {
  grid-column: 1;
}

.form-sidebar {
  grid-column: 2;
}

.form-actions {
  grid-column: 1 / -1;
  display: flex;
  gap: $spacing-3;
  justify-content: flex-end;
  padding-top: $spacing-4;
  border-top: 1px solid $border-color;
}

.form-group {
  margin-bottom: $spacing-4;
  
  label {
    display: block;
    margin-bottom: $spacing-2;
    font-weight: $font-weight-medium;
    color: $text-primary;
  }
}

.form-control {
  width: 100%;
  padding: $spacing-3;
  background: $bg-tertiary;
  border: 1px solid $border-color;
  border-radius: $radius-md;
  color: $text-primary;
  font-size: $font-size-base;
  
  &:focus {
    outline: none;
    border-color: $brand-primary;
  }
  
  &::placeholder {
    color: $text-tertiary;
  }
}

textarea.form-control {
  resize: vertical;
  min-height: 200px;
  font-family: $font-mono;
}

.editor-toolbar {
  display: flex;
  gap: $spacing-1;
  padding: $spacing-2;
  background: $bg-tertiary;
  border: 1px solid $border-color;
  border-bottom: none;
  border-radius: $radius-md $radius-md 0 0;
}

.toolbar-btn {
  padding: $spacing-1 $spacing-2;
  background: transparent;
  border: none;
  border-radius: $radius-sm;
  color: $text-secondary;
  cursor: pointer;
  font-size: $font-size-sm;
  
  &:hover {
    background: rgba(255, 255, 255, 0.1);
    color: $text-primary;
  }
}

textarea.form-control {
  border-radius: 0 0 $radius-md $radius-md;
}

.form-text {
  display: block;
  margin-top: $spacing-1;
  font-size: $font-size-sm;
  color: $text-secondary;
}

.sidebar-section {
  margin-bottom: $spacing-5;
  
  label {
    display: block;
    margin-bottom: $spacing-2;
    font-weight: $font-weight-medium;
    color: $text-secondary;
    font-size: $font-size-sm;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
}

.labels-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-2;
}

.label-checkbox {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  cursor: pointer;
  font-weight: normal;
  text-transform: none;
  
  input {
    margin: 0;
  }
}

.label-color {
  width: 12px;
  height: 12px;
  border-radius: $radius-sm;
}

.no-labels {
  color: $text-tertiary;
  font-size: $font-size-sm;
}

.alert {
  grid-column: 1 / -1;
  padding: $spacing-3;
  border-radius: $radius-md;
  
  &.alert-danger {
    background: rgba($danger, 0.1);
    border: 1px solid rgba($danger, 0.3);
    color: $danger;
  }
}

@media (max-width: 768px) {
  .issue-form {
    grid-template-columns: 1fr;
  }
  
  .form-sidebar {
    grid-column: 1;
    order: -1;
  }
}
</style>
