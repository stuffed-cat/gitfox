<template>
  <div class="fork-project-page">
    <!-- Breadcrumb -->
    <div class="breadcrumb">
      <router-link :to="`/${owner}/${repo}`">{{ owner }}</router-link>
      <span class="separator">/</span>
      <router-link :to="`/${owner}/${repo}`">{{ repo }}</router-link>
      <span class="separator">/</span>
      <span>派生（Fork）项目</span>
    </div>

    <div class="fork-header">
      <div class="fork-icon">
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="5" cy="3" r="2"/>
          <circle cx="11" cy="3" r="2"/>
          <circle cx="8" cy="13" r="2"/>
          <path d="M5 5v2a3 3 0 003 3m3-5v2a3 3 0 01-3 3m0 0v0"/>
        </svg>
      </div>
      <div class="fork-info">
        <h1>派生（Fork）项目</h1>
        <p>派生（fork）是一个项目的副本。<br/>派生（Fork）一个仓库，允许您在不影响原始项目的情况下进行更改。</p>
      </div>
    </div>

    <form @submit.prevent="handleFork" class="fork-form">
      <div v-if="error" class="alert alert-error">{{ error }}</div>
      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <p>正在派生项目...</p>
      </div>

      <div v-else>
        <!-- Project URL and Name -->
        <div class="form-row">
          <div class="form-group flex-1">
            <label>项目 URL</label>
            <div class="url-input">
              <span class="prefix">{{ baseUrl }}/</span>
              <select v-model="form.namespace_id" class="namespace-select" :disabled="loadingNamespaces">
                <option :value="null" disabled>选择命名空间</option>
                <option v-for="ns in availableNamespaces" :key="ns.id" :value="ns.id">
                  {{ ns.path }}
                </option>
              </select>
            </div>
            <span class="hint">想在同一个命名空间下阻止多个依赖项目？<a href="#">创建群组</a></span>
          </div>
          <div class="form-group flex-1">
            <label for="name">项目名称</label>
            <input
              id="name"
              v-model="form.name"
              type="text"
              class="form-input"
              required
            />
          </div>
        </div>

        <!-- Description -->
        <div class="form-group">
          <label for="description">项目描述（可选）</label>
          <textarea
            id="description"
            v-model="form.description"
            class="form-input"
            rows="3"
            placeholder="输入项目描述..."
          ></textarea>
        </div>

        <!-- Branches -->
        <div class="form-group">
          <label>要包含的分支</label>
          <div class="branch-options">
            <label class="radio-option" :class="{ active: form.branches === 'all' }">
              <input type="radio" v-model="form.branches" value="all" />
              <span>所有分支</span>
            </label>
            <label class="radio-option" :class="{ active: form.branches === 'default' }">
              <input type="radio" v-model="form.branches" value="default" />
              <span>仅默认分支 <code>{{ defaultBranch }}</code></span>
            </label>
          </div>
        </div>

        <!-- Visibility -->
        <div class="form-group">
          <label>可见性级别 <span class="help-icon" title="帮助">?</span></label>
          <div class="visibility-options">
            <label class="visibility-option" :class="{ active: form.visibility === 'private' }">
              <input type="radio" v-model="form.visibility" value="private" />
              <svg viewBox="0 0 16 16" fill="none">
                <rect x="4" y="7" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <path d="M6 7V5a2 2 0 014 0v2" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <div>
                <strong>私有</strong>
                <span>项目访问权限必须明确授予每个用户。如果此项目属于某个群组，则将授予该组成员访问权限。</span>
              </div>
            </label>
            
            <label class="visibility-option" :class="{ active: form.visibility === 'internal', disabled: true }">
              <input type="radio" v-model="form.visibility" value="internal" disabled />
              <svg viewBox="0 0 16 16" fill="none">
                <path d="M8 1L1 5v6l7 4 7-4V5L8 1z" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <div>
                <strong>内部</strong>
                <span>任何登录用户都可以访问该项目。</span>
              </div>
            </label>
            
            <label class="visibility-option" :class="{ active: form.visibility === 'public' }">
              <input type="radio" v-model="form.visibility" value="public" />
              <svg viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
                <path d="M1 8h14M8 2c-2 2-3 4-3 6s1 4 3 6c2-2 3-4 3-6s-1-4-3-6" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <div>
                <strong>公开</strong>
                <span>该项目无需任何身份验证即可访问。</span>
              </div>
            </label>
          </div>
        </div>

        <div class="form-actions">
          <button type="submit" class="btn btn-primary" :disabled="!canFork">
            派生（Fork）项目
          </button>
          <button type="button" class="btn btn-secondary" @click="handleCancel">
            取消
          </button>
        </div>
      </div>
    </form>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import api from '@/api'
import type { Project, NamespaceOption } from '@/types'

const route = useRoute()
const router = useRouter()

const owner = computed(() => route.params.owner as string)
const repo = computed(() => route.params.repo as string)
const baseUrl = window.location.origin

const loading = ref(false)
const error = ref('')
const sourceProject = ref<Project | null>(null)
const defaultBranch = ref('main')

// Namespaces
const availableNamespaces = ref<NamespaceOption[]>([])
const loadingNamespaces = ref(false)

const form = reactive({
  name: '',
  namespace_id: null as number | null,
  description: '',
  branches: 'all' as 'all' | 'default',
  visibility: 'private' as 'public' | 'private' | 'internal'
})

const canFork = computed(() => {
  return form.name && form.namespace_id !== null
})

onMounted(async () => {
  await Promise.all([loadSourceProject(), loadNamespaces()])
})

async function loadSourceProject() {
  try {
    sourceProject.value = await api.projects.get({ namespace: owner.value, project: repo.value })
    form.name = sourceProject.value.name
    form.description = sourceProject.value.description || ''
    form.visibility = sourceProject.value.visibility || 'private'
    // Load default branch
    try {
      const repoInfo = await api.repository.getInfo({ namespace: owner.value, project: repo.value })
      defaultBranch.value = repoInfo.default_branch || 'main'
    } catch {
      defaultBranch.value = 'main'
    }
  } catch (e: any) {
    error.value = '加载项目信息失败'
    console.error(e)
  }
}

async function loadNamespaces() {
  loadingNamespaces.value = true
  try {
    availableNamespaces.value = await api.namespaces.listForProjectCreation()
    // Set default to user's namespace
    const userNs = availableNamespaces.value.find(n => n.namespace_type === 'user')
    if (userNs) {
      form.namespace_id = userNs.id
    }
  } catch (e) {
    console.error('Failed to load namespaces:', e)
  } finally {
    loadingNamespaces.value = false
  }
}

async function handleFork() {
  if (!canFork.value) return
  
  loading.value = true
  error.value = ''
  
  try {
    const forkedProject = await api.projects.fork(
      { namespace: owner.value, project: repo.value },
      { 
        namespace_id: form.namespace_id!, 
        name: form.name,
        description: form.description,
        visibility: form.visibility,
        branches: form.branches
      }
    )
    // Navigate to the forked project
    router.push(`/${forkedProject.owner_name}/${forkedProject.name}`)
  } catch (e: any) {
    console.error('Fork failed:', e)
    error.value = e.response?.data?.message || '派生失败，请重试'
    loading.value = false
  }
}

function handleCancel() {
  router.push(`/${owner.value}/${repo.value}`)
}
</script>

<style lang="scss" scoped>
.fork-project-page {
  padding: $spacing-6;
  max-width: 900px;
  margin: 0 auto;
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  margin-bottom: $spacing-6;
  font-size: $text-sm;
  color: $text-secondary;
  
  a {
    color: $color-primary;
    text-decoration: none;
    &:hover { text-decoration: underline; }
  }
  .separator { color: $text-muted; }
}

.fork-header {
  display: flex;
  gap: $spacing-4;
  margin-bottom: $spacing-8;
  padding-bottom: $spacing-6;
  border-bottom: 1px solid $border-color;
}

.fork-icon {
  width: 64px;
  height: 64px;
  background: $bg-secondary;
  border-radius: $radius-lg;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  
  svg {
    width: 32px;
    height: 32px;
    color: $color-primary;
  }
}

.fork-info {
  h1 {
    font-size: $text-xl;
    font-weight: 600;
    margin-bottom: $spacing-2;
  }
  
  p {
    color: $text-secondary;
    font-size: $text-sm;
    line-height: 1.5;
  }
}

.fork-form {
  background: $bg-primary;
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: $spacing-12;
  
  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid $border-color;
    border-top-color: $color-primary;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }
  
  p {
    margin-top: $spacing-4;
    color: $text-secondary;
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.alert {
  padding: $spacing-3 $spacing-4;
  border-radius: $radius-md;
  margin-bottom: $spacing-4;
  
  &.alert-error {
    background: rgba($color-danger, 0.1);
    color: $color-danger;
    border: 1px solid rgba($color-danger, 0.2);
  }
}

.form-group {
  margin-bottom: $spacing-5;
  
  label {
    display: block;
    font-weight: 500;
    margin-bottom: $spacing-2;
    color: $text-primary;
    
    .required {
      color: $color-danger;
    }
    
    .help-icon {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 16px;
      height: 16px;
      border-radius: 50%;
      background: $bg-secondary;
      font-size: 10px;
      color: $text-secondary;
      cursor: help;
      margin-left: $spacing-1;
    }
  }
  
  .hint {
    display: block;
    margin-top: $spacing-1;
    font-size: $text-xs;
    color: $text-secondary;
    
    a {
      color: $color-primary;
      text-decoration: none;
      &:hover { text-decoration: underline; }
    }
  }
}

.form-input {
  width: 100%;
  padding: $spacing-2 $spacing-3;
  border: 1px solid $border-color;
  border-radius: $radius-md;
  font-size: $text-sm;
  background: $bg-primary;
  color: $text-primary;
  
  &:focus {
    outline: none;
    border-color: $color-primary;
    box-shadow: 0 0 0 2px rgba($color-primary, 0.2);
  }
  
  &:disabled {
    background: $bg-secondary;
    cursor: not-allowed;
  }
}

textarea.form-input {
  resize: vertical;
  min-height: 80px;
}

.form-row {
  display: flex;
  gap: $spacing-4;
  
  .flex-1 {
    flex: 1;
  }
}

.url-input {
  display: flex;
  align-items: center;
  border: 1px solid $border-color;
  border-radius: $radius-md;
  overflow: hidden;
  
  .prefix {
    padding: $spacing-2 $spacing-3;
    background: $bg-secondary;
    color: $text-secondary;
    font-size: $text-sm;
    white-space: nowrap;
  }
  
  .namespace-select {
    flex: 1;
    padding: $spacing-2 $spacing-3;
    border: none;
    font-size: $text-sm;
    background: $bg-primary;
    color: $text-primary;
    cursor: pointer;
    
    &:focus {
      outline: none;
    }
  }
}

.branch-options {
  display: flex;
  flex-direction: column;
  gap: $spacing-2;
}

.radio-option {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-2 $spacing-3;
  border: 1px solid $border-color;
  border-radius: $radius-md;
  cursor: pointer;
  transition: all 0.15s;
  
  &:hover {
    background: $bg-secondary;
  }
  
  &.active {
    border-color: $color-primary;
    background: rgba($color-primary, 0.05);
  }
  
  input[type="radio"] {
    accent-color: $color-primary;
  }
  
  code {
    padding: 2px 6px;
    background: $bg-secondary;
    border-radius: $radius-sm;
    font-size: $text-xs;
  }
}

.visibility-options {
  display: flex;
  flex-direction: column;
  gap: $spacing-3;
}

.visibility-option {
  display: flex;
  align-items: flex-start;
  gap: $spacing-3;
  padding: $spacing-3 $spacing-4;
  border: 1px solid $border-color;
  border-radius: $radius-md;
  cursor: pointer;
  transition: all 0.15s;
  
  &:hover:not(.disabled) {
    background: $bg-secondary;
  }
  
  &.active {
    border-color: $color-primary;
    background: rgba($color-primary, 0.05);
  }
  
  &.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  input[type="radio"] {
    margin-top: 4px;
    accent-color: $color-primary;
  }
  
  svg {
    width: 20px;
    height: 20px;
    color: $text-secondary;
    flex-shrink: 0;
    margin-top: 2px;
  }
  
  div {
    flex: 1;
    
    strong {
      display: block;
      margin-bottom: $spacing-1;
    }
    
    span {
      display: block;
      font-size: $text-sm;
      color: $text-secondary;
      line-height: 1.4;
    }
  }
}

.form-actions {
  display: flex;
  gap: $spacing-3;
  margin-top: $spacing-8;
  padding-top: $spacing-6;
  border-top: 1px solid $border-color;
}

.btn {
  padding: $spacing-2 $spacing-4;
  border-radius: $radius-md;
  font-size: $text-sm;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
  border: none;
  
  &.btn-primary {
    background: $color-primary;
    color: white;
    
    &:hover:not(:disabled) {
      background: darken($color-primary, 8%);
    }
    
    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }
  
  &.btn-secondary {
    background: $bg-secondary;
    color: $text-primary;
    border: 1px solid $border-color;
    
    &:hover {
      background: $bg-tertiary;
    }
  }
}
</style>
