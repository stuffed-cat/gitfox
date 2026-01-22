<template>
  <div class="new-project-page">
    <div class="page-container">
      <!-- Breadcrumb -->
      <div class="breadcrumb">
        <router-link to="/projects">项目</router-link>
        <span class="separator">/</span>
        <span class="current">新建项目</span>
      </div>
      
      <div class="page-header">
        <h1>创建空白项目</h1>
        <p class="page-description">创建一个空白项目，用于存放和管理您的代码</p>
      </div>
      
      <div class="form-card">
        <form @submit.prevent="handleSubmit" class="project-form">
          <div v-if="error" class="alert alert-error">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
              <path d="M8 5v3M8 10v.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <span>{{ error }}</span>
          </div>
          
          <div class="form-section">
            <div class="form-group">
              <label for="name">
                项目名称
                <span class="required">*</span>
              </label>
              <input
                id="name"
                v-model="form.name"
                type="text"
                class="form-control"
                placeholder="my-awesome-project"
                required
                @input="generateSlug"
              />
              <span class="form-hint">项目名称应该简短且易于记忆</span>
            </div>
            
            <div class="form-group">
              <label for="slug">项目标识</label>
              <div class="input-with-prefix">
                <span class="prefix">{{ baseUrl }}/</span>
                <input
                  id="slug"
                  v-model="form.slug"
                  type="text"
                  class="form-control"
                  placeholder="my-awesome-project"
                />
              </div>
              <span class="form-hint">项目的URL标识，只能包含字母、数字和连字符</span>
            </div>
            
            <div class="form-group">
              <label for="description">项目描述 <span class="optional">(可选)</span></label>
              <textarea
                id="description"
                v-model="form.description"
                class="form-control"
                placeholder="描述您的项目用途..."
                rows="3"
              ></textarea>
            </div>
          </div>
          
          <div class="form-section">
            <h3 class="section-title">可见性级别</h3>
            
            <div class="visibility-options">
              <label class="visibility-option" :class="{ active: form.visibility === 'private' }">
                <input type="radio" v-model="form.visibility" value="private" />
                <div class="option-content">
                  <div class="option-icon">
                    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                      <rect x="5" y="9" width="10" height="7" rx="1" stroke="currentColor" stroke-width="1.5"/>
                      <path d="M7 9V7a3 3 0 016 0v2" stroke="currentColor" stroke-width="1.5"/>
                    </svg>
                  </div>
                  <div class="option-text">
                    <span class="option-label">私有</span>
                    <span class="option-desc">只有您和明确授权的成员可以访问</span>
                  </div>
                </div>
              </label>
              
              <label class="visibility-option" :class="{ active: form.visibility === 'internal' }">
                <input type="radio" v-model="form.visibility" value="internal" />
                <div class="option-content">
                  <div class="option-icon">
                    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                      <path d="M10 2L2 6v4c0 5 3.5 9 8 10 4.5-1 8-5 8-10V6l-8-4z" stroke="currentColor" stroke-width="1.5"/>
                    </svg>
                  </div>
                  <div class="option-text">
                    <span class="option-label">内部</span>
                    <span class="option-desc">任何登录用户都可以访问（不包括外部用户）</span>
                  </div>
                </div>
              </label>
              
              <label class="visibility-option" :class="{ active: form.visibility === 'public' }">
                <input type="radio" v-model="form.visibility" value="public" />
                <div class="option-content">
                  <div class="option-icon">
                    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                      <circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="1.5"/>
                      <path d="M3 10h14M10 3c-2 2-3 4.5-3 7s1 5 3 7c2-2 3-4.5 3-7s-1-5-3-7" stroke="currentColor" stroke-width="1.5"/>
                    </svg>
                  </div>
                  <div class="option-text">
                    <span class="option-label">公开</span>
                    <span class="option-desc">任何人都可以访问，无需身份验证</span>
                  </div>
                </div>
              </label>
            </div>
          </div>
          
          <div class="form-section">
            <h3 class="section-title">项目配置</h3>
            
            <div class="form-group">
              <label for="default_branch">默认分支名称</label>
              <input
                id="default_branch"
                v-model="form.default_branch"
                type="text"
                class="form-control"
                placeholder="main"
              />
              <span class="form-hint">默认分支将作为项目的主要开发分支</span>
            </div>
            
            <div class="form-checkbox">
              <input type="checkbox" id="readme" v-model="form.initializeWithReadme" />
              <label for="readme">
                使用 README 初始化仓库
                <span class="checkbox-hint">创建一个包含基本信息的 README.md 文件</span>
              </label>
            </div>
          </div>
          
          <div class="form-actions">
            <router-link to="/projects" class="btn btn-secondary">取消</router-link>
            <button type="submit" class="btn btn-primary" :disabled="loading || !form.name">
              <span v-if="loading" class="btn-spinner"></span>
              {{ loading ? '创建中...' : '创建项目' }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { useProjectStore } from '@/stores/project'

const router = useRouter()
const projectStore = useProjectStore()

const baseUrl = window.location.hostname

const form = reactive({
  name: '',
  slug: '',
  description: '',
  visibility: 'private' as 'public' | 'private' | 'internal',
  default_branch: 'main',
  initializeWithReadme: true
})

const loading = ref(false)
const error = ref('')

function generateSlug() {
  form.slug = form.name
    .toLowerCase()
    .replace(/\s+/g, '-')
    .replace(/[^a-z0-9-]/g, '')
}

async function handleSubmit() {
  loading.value = true
  error.value = ''
  
  try {
    const project = await projectStore.createProject({
      name: form.name,
      description: form.description,
      visibility: form.visibility,
      default_branch: form.default_branch
    })
    // 使用 owner/repo 格式跳转
    router.push(`/${project.owner_name || 'unknown'}/${project.slug}`)
  } catch (e: any) {
    error.value = e.response?.data?.message || '创建项目失败，请重试'
  } finally {
    loading.value = false
  }
}
</script>

<style lang="scss" scoped>
.new-project-page {
  min-height: 100%;
  background: $bg-secondary;
}

.page-container {
  max-width: 720px;
  margin: 0 auto;
  padding: $spacing-6;
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  margin-bottom: $spacing-6;
  font-size: $font-size-sm;
  
  a {
    color: $text-secondary;
    text-decoration: none;
    
    &:hover {
      color: $brand-primary;
    }
  }
  
  .separator {
    color: $text-muted;
  }
  
  .current {
    color: $text-primary;
  }
}

.page-header {
  margin-bottom: $spacing-6;
  
  h1 {
    font-size: $font-size-2xl;
    font-weight: $font-weight-semibold;
    margin: 0 0 $spacing-2;
  }
  
  .page-description {
    color: $text-secondary;
    margin: 0;
  }
}

.form-card {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
}

.project-form {
  padding: $spacing-6;
}

.form-section {
  margin-bottom: $spacing-8;
  
  &:last-of-type {
    margin-bottom: 0;
  }
}

.section-title {
  font-size: $font-size-base;
  font-weight: $font-weight-semibold;
  color: $text-primary;
  margin: 0 0 $spacing-4;
  padding-bottom: $spacing-3;
  border-bottom: 1px solid $border-color;
}

.form-group {
  margin-bottom: $spacing-5;
  
  label {
    display: block;
    font-size: $font-size-sm;
    font-weight: $font-weight-medium;
    color: $text-primary;
    margin-bottom: $spacing-2;
    
    .required {
      color: $color-danger;
      margin-left: 2px;
    }
    
    .optional {
      color: $text-muted;
      font-weight: normal;
    }
  }
}

.form-control {
  width: 100%;
  padding: $spacing-3;
  font-size: $font-size-sm;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  background: $bg-primary;
  transition: all $transition-fast;
  
  &:focus {
    outline: none;
    border-color: $brand-primary;
    box-shadow: $shadow-focus;
  }
  
  &::placeholder {
    color: $text-muted;
  }
}

textarea.form-control {
  resize: vertical;
  min-height: 80px;
}

.form-hint {
  display: block;
  font-size: $font-size-xs;
  color: $text-muted;
  margin-top: $spacing-2;
}

.input-with-prefix {
  display: flex;
  align-items: center;
  
  .prefix {
    padding: $spacing-3;
    font-size: $font-size-sm;
    color: $text-secondary;
    background: $bg-tertiary;
    border: 1px solid $border-color;
    border-right: none;
    border-radius: $border-radius 0 0 $border-radius;
    white-space: nowrap;
  }
  
  .form-control {
    border-radius: 0 $border-radius $border-radius 0;
  }
}

.visibility-options {
  display: flex;
  flex-direction: column;
  gap: $spacing-3;
}

.visibility-option {
  display: block;
  cursor: pointer;
  
  input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  
  .option-content {
    display: flex;
    align-items: flex-start;
    gap: $spacing-4;
    padding: $spacing-4;
    border: 2px solid $border-color;
    border-radius: $border-radius-lg;
    transition: all $transition-fast;
  }
  
  &:hover .option-content {
    border-color: $gray-300;
  }
  
  &.active .option-content {
    border-color: $brand-primary;
    background: rgba($brand-primary, 0.02);
  }
}

.option-icon {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: $bg-tertiary;
  border-radius: $border-radius;
  color: $text-secondary;
  flex-shrink: 0;
  
  .active & {
    background: $brand-primary;
    color: white;
  }
}

.option-text {
  flex: 1;
}

.option-label {
  display: block;
  font-size: $font-size-sm;
  font-weight: $font-weight-semibold;
  color: $text-primary;
  margin-bottom: $spacing-1;
}

.option-desc {
  display: block;
  font-size: $font-size-sm;
  color: $text-secondary;
  line-height: 1.4;
}

.form-checkbox {
  display: flex;
  align-items: flex-start;
  gap: $spacing-3;
  
  input[type="checkbox"] {
    width: 18px;
    height: 18px;
    margin-top: 2px;
    cursor: pointer;
    accent-color: $brand-primary;
  }
  
  label {
    cursor: pointer;
    font-size: $font-size-sm;
    color: $text-primary;
    line-height: 1.4;
  }
  
  .checkbox-hint {
    display: block;
    color: $text-secondary;
    font-weight: normal;
    margin-top: 2px;
  }
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: $spacing-3;
  margin-top: $spacing-8;
  padding-top: $spacing-6;
  border-top: 1px solid $border-color;
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: $spacing-2;
  padding: $spacing-3 $spacing-5;
  font-size: $font-size-sm;
  font-weight: $font-weight-medium;
  border-radius: $border-radius;
  border: none;
  cursor: pointer;
  transition: all $transition-fast;
  text-decoration: none;
  
  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
}

.btn-primary {
  background: $brand-primary;
  color: white;
  
  &:hover:not(:disabled) {
    background: darken($brand-primary, 8%);
  }
}

.btn-secondary {
  background: $bg-primary;
  color: $text-primary;
  border: 1px solid $border-color;
  
  &:hover:not(:disabled) {
    background: $bg-secondary;
    border-color: $gray-300;
  }
}

.btn-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.alert {
  display: flex;
  align-items: center;
  gap: $spacing-3;
  padding: $spacing-4;
  border-radius: $border-radius;
  margin-bottom: $spacing-6;
  font-size: $font-size-sm;
}

.alert-error {
  background: $color-danger-light;
  color: $color-danger;
  
  svg {
    flex-shrink: 0;
  }
}
</style>
