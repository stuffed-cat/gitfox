<template>
  <div class="new-group-page">
    <!-- 面包屑 -->
    <div class="breadcrumb">
      <router-link to="/">你的工作</router-link>
      <span class="separator">/</span>
      <router-link to="/dashboard/groups">群组</router-link>
      <span class="separator">/</span>
      <span>新建群组</span>
    </div>

    <!-- 步骤一：选择类型 -->
    <div v-if="step === 'choose'" class="choose-type">
      <h1 class="page-title">创建新群组</h1>
      <p class="page-subtitle">群组允许你管理和协作多个项目。群组中的成员可以访问该群组下的所有项目。</p>
      
      <div class="group-types">
        <div class="type-card" @click="selectType('group')">
          <div class="type-icon">
            <svg viewBox="0 0 64 64" fill="none">
              <rect x="4" y="8" width="24" height="24" rx="4" stroke="currentColor" stroke-width="2"/>
              <rect x="36" y="8" width="24" height="24" rx="4" stroke="currentColor" stroke-width="2"/>
              <rect x="20" y="32" width="24" height="24" rx="4" stroke="currentColor" stroke-width="2"/>
              <path d="M28 24v12M36 24v12" stroke="currentColor" stroke-width="2" stroke-dasharray="4 4"/>
            </svg>
          </div>
          <div class="type-info">
            <h3>创建群组</h3>
            <p>将相关项目组织在一起，分配群组级别的成员和权限，以便高效协作。</p>
          </div>
        </div>
        
        <div class="type-card" @click="selectType('import')" :class="{ disabled: true }">
          <div class="type-icon">
            <svg viewBox="0 0 64 64" fill="none">
              <circle cx="32" cy="32" r="20" stroke="currentColor" stroke-width="2"/>
              <path d="M32 22v20M22 32h20" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              <path d="M26 28l6-6 6 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <div class="type-info">
            <h3>导入群组</h3>
            <p>从其他平台导入已有的群组和项目（即将推出）。</p>
          </div>
        </div>
      </div>
    </div>

    <!-- 步骤二：创建群组表单 -->
    <div v-else-if="step === 'group'" class="create-form">
      <div class="form-header">
        <button class="back-btn" @click="step = 'choose'">
          <svg viewBox="0 0 16 16" fill="none">
            <path d="M10 12L6 8l4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          返回
        </button>
        <h1>创建群组</h1>
        <p>将相关项目组织在一起，分配群组级别的成员和权限。</p>
      </div>

      <!-- 信息提示 -->
      <div v-if="showBanner" class="info-banner">
        <svg viewBox="0 0 16 16" fill="none">
          <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
          <path d="M8 5v.5M8 7v4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <div>
          <strong>群组可以包含子群组和项目</strong>
          <p>成员可以访问群组下的所有项目。你可以在创建群组后，在群组内创建子群组和项目。</p>
        </div>
        <button class="close-btn" @click="showBanner = false">×</button>
      </div>
      
      <form @submit.prevent="createGroup" class="group-form">
        <div v-if="error" class="alert alert-error">{{ error }}</div>

        <div class="form-group">
          <label for="name">群组名称 <span class="required">*</span></label>
          <input 
            id="name" 
            v-model="form.name" 
            type="text" 
            class="form-input"
            required 
            placeholder="我的群组"
          />
          <span class="hint">必须以小写或大写字母、数字、表情符号或下划线开头。也可以包含点、加号、破折号或空格。</span>
        </div>
        
        <div class="form-group">
          <label>父群组（可选）</label>
          <select v-model="selectedParentId" class="form-input" :disabled="loadingGroups">
            <option :value="null">无（创建顶级群组）</option>
            <option v-for="g in availableParentGroups" :key="g.id" :value="Number(g.id)">
              {{ g.path }}
            </option>
          </select>
          <span class="hint">选择一个父群组来创建子群组，或保留为空创建顶级群组。</span>
        </div>
        
        <div class="form-row">
          <div class="form-group flex-grow">
            <label>群组 URL</label>
            <div class="url-input">
              <span class="prefix">{{ baseUrl }}/</span>
              <span v-if="parentPath" class="parent-path">{{ parentPath }}/</span>
            </div>
          </div>
          <div class="form-group flex-grow">
            <label for="path">群组标识 <span class="required">*</span></label>
            <input 
              id="path" 
              v-model="form.path" 
              type="text" 
              class="form-input"
              required 
              placeholder="my-group"
            />
          </div>
        </div>
        
        <div class="form-group">
          <label for="description">群组描述（可选）</label>
          <textarea 
            id="description" 
            v-model="form.description" 
            rows="4"
            class="form-input"
            placeholder="描述这个群组是做什么的..."
          ></textarea>
          <span class="hint">此描述将对所有有权访问该群组的用户可见。</span>
        </div>
        
        <div class="form-group">
          <label>可见性级别</label>
          <div class="visibility-options">
            <label class="visibility-option" :class="{ active: form.visibility === 'private' }">
              <input type="radio" v-model="form.visibility" value="private" />
              <svg viewBox="0 0 16 16" fill="none">
                <rect x="4" y="7" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <path d="M6 7V5a2 2 0 014 0v2" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <div>
                <strong>私有</strong>
                <span>群组及其项目仅对群组成员可见。</span>
              </div>
            </label>
            <label class="visibility-option" :class="{ active: form.visibility === 'internal' }">
              <input type="radio" v-model="form.visibility" value="internal" />
              <svg viewBox="0 0 16 16" fill="none">
                <path d="M8 1L1 5v6l7 4 7-4V5L8 1z" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <div>
                <strong>内部</strong>
                <span>除外部用户外，任何登录用户均可访问该群组。</span>
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
                <span>群组及任何公开项目无需身份验证即可访问。</span>
              </div>
            </label>
          </div>
        </div>

        <div class="form-section">
          <h3>角色和权限</h3>
          <p class="section-desc">创建群组后，你将成为该群组的所有者。你可以在群组设置中邀请其他成员。</p>
        </div>
        
        <div class="form-actions">
          <button type="button" class="btn btn-secondary" @click="step = 'choose'">取消</button>
          <button type="submit" class="btn btn-primary" :disabled="submitting || !form.name || !form.path">
            {{ submitting ? '创建中...' : '创建群组' }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { api } from '@/api'
import type { Group } from '@/types'

const router = useRouter()
const route = useRoute()

const baseUrl = window.location.origin

// 可用的父群组列表（用户有权限创建子群组的群组）
const availableParentGroups = ref<Group[]>([])
const loadingGroups = ref(false)

// 选中的父群组
const selectedParentId = ref<number | null>(null)
const parentGroup = computed(() => {
  if (!selectedParentId.value) return null
  return availableParentGroups.value.find(g => Number(g.id) === selectedParentId.value) || null
})
const parentPath = computed(() => parentGroup.value?.path || '')

const step = ref<'choose' | 'group' | 'import'>('choose')
const showBanner = ref(true)

const form = ref({
  name: '',
  path: '',
  description: '',
  visibility: 'private' as 'public' | 'private' | 'internal'
})

const submitting = ref(false)
const error = ref('')

// 加载用户有权限的群组
onMounted(async () => {
  loadingGroups.value = true
  try {
    // 获取用户有权限的群组（可以创建子群组的）
    availableParentGroups.value = await api.groups.list()
    
    // 如果 URL 有 parent 参数，预选该群组
    const parentFromQuery = route.query.parent as string
    if (parentFromQuery) {
      const found = availableParentGroups.value.find(g => g.path === parentFromQuery)
      if (found) {
        selectedParentId.value = Number(found.id)
      }
    }
  } catch (e) {
    console.warn('Failed to load groups:', e)
  } finally {
    loadingGroups.value = false
  }
})

function selectType(type: 'group' | 'import') {
  if (type === 'import') return // disabled
  step.value = type
}

watch(() => form.value.name, (name) => {
  form.value.path = name.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '')
})

import { onMounted } from 'vue'

async function createGroup() {
  submitting.value = true
  error.value = ''
  
  try {
    // 子群组的路径需要包含父路径
    const fullPath = parentPath.value 
      ? `${parentPath.value}/${form.value.path}` 
      : form.value.path
    
    const group = await api.groups.create({
      name: form.value.name,
      path: fullPath,
      description: form.value.description || undefined,
      visibility: form.value.visibility,
      parent_id: selectedParentId.value || undefined
    })
    router.push(`/${group.path}`)
  } catch (e: any) {
    error.value = e.response?.data?.message || e.message || '创建群组失败'
  } finally {
    submitting.value = false
  }
}
</script>

<style lang="scss" scoped>
.new-group-page {
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

.page-title {
  font-size: $text-2xl;
  font-weight: 600;
  text-align: center;
  margin-bottom: $spacing-2;
}

.page-subtitle {
  text-align: center;
  color: $text-secondary;
  font-size: $text-sm;
  margin-bottom: $spacing-8;
  max-width: 600px;
  margin-left: auto;
  margin-right: auto;
  line-height: 1.5;
}

.group-types {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: $spacing-4;
  margin-bottom: $spacing-6;
}

.type-card {
  display: flex;
  gap: $spacing-4;
  padding: $spacing-6;
  border: 1px solid $border-color;
  border-radius: $radius-lg;
  cursor: pointer;
  transition: all 0.2s;
  
  &:hover:not(.disabled) {
    border-color: $color-primary;
    background: rgba($color-primary, 0.02);
  }
  
  &.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .type-icon {
    width: 64px;
    height: 64px;
    flex-shrink: 0;
    color: $color-primary;
    
    svg { width: 100%; height: 100%; }
  }
  
  .type-info {
    h3 {
      font-size: $text-lg;
      font-weight: 600;
      margin-bottom: $spacing-2;
    }
    p {
      font-size: $text-sm;
      color: $text-secondary;
      line-height: 1.5;
    }
  }
}

.form-header {
  margin-bottom: $spacing-6;
  
  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: $spacing-2;
    background: none;
    border: none;
    color: $color-primary;
    cursor: pointer;
    margin-bottom: $spacing-4;
    font-size: $text-sm;
    
    svg { width: 16px; height: 16px; }
  }
  
  h1 {
    font-size: $text-2xl;
    font-weight: 600;
    margin-bottom: $spacing-2;
  }
  
  p { color: $text-secondary; }
}

.info-banner {
  display: flex;
  gap: $spacing-4;
  padding: $spacing-4;
  background: #e8f4fd;
  border: 1px solid #b8daff;
  border-radius: $radius-md;
  margin-bottom: $spacing-6;
  
  > svg {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
    color: #0066cc;
    margin-top: 2px;
  }
  
  strong {
    display: block;
    margin-bottom: $spacing-2;
  }
  
  p {
    font-size: $text-sm;
    color: $text-secondary;
    line-height: 1.5;
  }
  
  .close-btn {
    background: none;
    border: none;
    font-size: 20px;
    cursor: pointer;
    color: $text-muted;
    margin-left: auto;
    align-self: flex-start;
  }
}

.group-form {
  .form-group {
    margin-bottom: $spacing-5;
    
    label {
      display: block;
      font-size: $text-sm;
      font-weight: 500;
      margin-bottom: $spacing-2;
      color: $text-primary;
      
      .required { color: $color-danger; }
    }
    
    .hint {
      display: block;
      font-size: $text-xs;
      color: $text-muted;
      margin-top: $spacing-2;
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
      box-shadow: $shadow-focus;
    }
  }
  
  textarea.form-input {
    resize: vertical;
    min-height: 80px;
  }
  
  .form-row {
    display: flex;
    gap: $spacing-4;
    
    .flex-grow { flex: 1; }
  }
  
  .url-input {
    display: flex;
    align-items: center;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    overflow: hidden;
    height: 38px;
    
    .prefix {
      padding: $spacing-2 $spacing-3;
      background: $bg-tertiary;
      color: $text-muted;
      font-size: $text-sm;
      white-space: nowrap;
    }
    
    .parent-path {
      padding: $spacing-2 0;
      color: $text-secondary;
      font-size: $text-sm;
    }
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
  padding: $spacing-4;
  border: 1px solid $border-color;
  border-radius: $radius-md;
  cursor: pointer;
  transition: all 0.2s;
  
  &:hover { background: $bg-secondary; }
  
  &.active {
    border-color: $color-primary;
    background: rgba($color-primary, 0.02);
  }
  
  input[type="radio"] { display: none; }
  
  svg {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
    color: $text-secondary;
    margin-top: 2px;
  }
  
  div {
    strong {
      display: block;
      margin-bottom: $spacing-1;
    }
    span {
      font-size: $text-sm;
      color: $text-secondary;
    }
  }
}

.form-section {
  margin-top: $spacing-6;
  padding-top: $spacing-6;
  border-top: 1px solid $border-color;
  
  h3 {
    font-size: $text-base;
    font-weight: 600;
    margin-bottom: $spacing-2;
  }
  
  .section-desc {
    font-size: $text-sm;
    color: $text-secondary;
  }
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: $spacing-3;
  margin-top: $spacing-6;
  padding-top: $spacing-6;
  border-top: 1px solid $border-color;
}

.alert-error {
  padding: $spacing-3 $spacing-4;
  margin-bottom: $spacing-4;
  background: $color-danger-light;
  border: 1px solid rgba($color-danger, 0.2);
  border-radius: $radius-md;
  color: $color-danger;
  font-size: $text-sm;
}

.btn {
  padding: $spacing-2 $spacing-4;
  border-radius: $radius-md;
  font-size: $text-sm;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  border: 1px solid transparent;
  
  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.btn-primary {
  background: $color-primary;
  color: white;
  
  &:hover:not(:disabled) { background: $color-primary-dark; }
}

.btn-secondary {
  background: $bg-primary;
  color: $text-primary;
  border-color: $border-color;
  
  &:hover:not(:disabled) { background: $bg-secondary; }
}
</style>
