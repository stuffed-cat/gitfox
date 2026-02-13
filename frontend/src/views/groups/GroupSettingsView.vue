<template>
  <div class="group-settings-page">
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>

    <template v-else-if="group">
      <div class="page-header">
        <div class="breadcrumb">
          <router-link :to="`/${group.path}`">{{ group.name }}</router-link>
          <span class="sep">/</span>
          <span>设置</span>
        </div>
        <h1>通用设置</h1>
        <p class="subtitle">管理群组 <strong>{{ group?.name }}</strong> 的基本设置</p>
      </div>

      <!-- 命名、可见性和描述 -->
      <section class="settings-section">
        <div class="section-header" @click="toggleSection('naming')">
          <h3>命名、可见性和描述</h3>
          <svg :class="{ rotated: expandedSections.naming }" viewBox="0 0 16 16" fill="none">
            <path d="M4 6l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </div>
        
        <div v-if="expandedSections.naming" class="section-body">
          <div v-if="updateError" class="alert alert-error">{{ updateError }}</div>
          <div v-if="updateSuccess" class="alert alert-success">设置已更新</div>
          
          <div class="form-group">
            <label for="group-name">群组名称</label>
            <input id="group-name" v-model="form.name" type="text" class="form-input" />
            <span class="hint">群组名称将在 URL 和导航中显示</span>
          </div>
          
          <div class="form-group">
            <label for="group-id">群组 ID</label>
            <input id="group-id" :value="group?.id" type="text" class="form-input" readonly disabled />
            <span class="hint">群组 ID 用于 API 访问</span>
          </div>
          
          <div class="form-group">
            <label for="group-desc">群组描述</label>
            <textarea id="group-desc" v-model="form.description" rows="4" class="form-input"></textarea>
            <span class="hint">简要描述群组的用途</span>
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

          <div class="form-actions">
            <button class="btn btn-confirm" @click="updateGroup" :disabled="saving">
              {{ saving ? '保存中...' : '保存更改' }}
            </button>
          </div>
        </div>
      </section>

      <!-- 高级设置 -->
      <section class="settings-section">
        <div class="section-header" @click="toggleSection('advanced')">
          <h3>高级设置</h3>
          <svg :class="{ rotated: expandedSections.advanced }" viewBox="0 0 16 16" fill="none">
            <path d="M4 6l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </div>
        
        <div v-if="expandedSections.advanced" class="section-body">
          <div class="danger-zone">
            <div class="danger-item">
              <div class="danger-info">
                <h4>转让群组</h4>
                <p>将群组转让给其他命名空间。请谨慎操作。</p>
              </div>
              <button class="btn btn-warning-outline" disabled>转让群组</button>
            </div>
          </div>
        </div>
      </section>

      <!-- 删除群组 -->
      <section class="settings-section danger">
        <div class="section-header" @click="toggleSection('delete')">
          <h3>删除群组</h3>
          <svg :class="{ rotated: expandedSections.delete }" viewBox="0 0 16 16" fill="none">
            <path d="M4 6l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </div>
        
        <div v-if="expandedSections.delete" class="section-body">
          <div class="danger-zone">
            <div class="danger-item">
              <div class="danger-info">
                <h4>删除此群组</h4>
                <p>
                  删除群组后，所有项目、议题、合并请求及其他相关资源将被永久移除。
                  <strong>此操作无法撤消。</strong>
                </p>
              </div>
              <button class="btn btn-danger" @click="handleDelete" :disabled="deleting">
                {{ deleting ? '删除中...' : '删除群组' }}
              </button>
            </div>
            
            <div v-if="showDeleteConfirm" class="delete-confirm">
              <p>请输入群组路径 <code>{{ group?.path }}</code> 以确认删除：</p>
              <input v-model="deleteConfirmInput" type="text" class="form-input" :placeholder="group?.path" />
              <div class="confirm-actions">
                <button class="btn btn-default" @click="showDeleteConfirm = false">取消</button>
                <button 
                  class="btn btn-danger" 
                  @click="confirmDelete"
                  :disabled="deleteConfirmInput !== group?.path || deleting"
                >
                  确认删除
                </button>
              </div>
            </div>
          </div>
        </div>
      </section>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, watch, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { api } from '@/api'
import { useNamespaceStore } from '@/stores/namespace'
import type { Group } from '@/types'

const router = useRouter()
const route = useRoute()
const namespaceStore = useNamespaceStore()

const loading = ref(true)
const group = ref<Group | null>(null)

const groupPath = computed(() => route.params.namespace as string)

const expandedSections = reactive({
  naming: true,
  advanced: false,
  delete: false
})

const form = ref({
  name: '',
  description: '',
  visibility: 'private' as 'public' | 'private' | 'internal'
})

const saving = ref(false)
const updateError = ref('')
const updateSuccess = ref(false)
const deleting = ref(false)
const showDeleteConfirm = ref(false)
const deleteConfirmInput = ref('')

async function loadData() {
  loading.value = true
  try {
    const g = await api.groups.get(groupPath.value)
    group.value = g
    form.value.name = g.name
    form.value.description = g.description || ''
    form.value.visibility = g.visibility
    // 设置群组上下文到 store
    namespaceStore.setNamespaceContext('group', groupPath.value, g)
  } catch (e) {
    console.error('Failed to load group:', e)
  } finally {
    loading.value = false
  }
}

function toggleSection(section: keyof typeof expandedSections) {
  expandedSections[section] = !expandedSections[section]
}

async function updateGroup() {
  saving.value = true
  updateError.value = ''
  updateSuccess.value = false
  
  try {
    await api.groups.update(groupPath.value, {
      name: form.value.name,
      description: form.value.description || undefined,
      visibility: form.value.visibility
    })
    updateSuccess.value = true
    setTimeout(() => { updateSuccess.value = false }, 3000)
  } catch (e: any) {
    updateError.value = e.response?.data?.message || e.message || '更新失败'
  } finally {
    saving.value = false
  }
}

function handleDelete() {
  showDeleteConfirm.value = true
  deleteConfirmInput.value = ''
}

async function confirmDelete() {
  deleting.value = true
  try {
    await api.groups.delete(groupPath.value)
    router.push('/dashboard/groups')
  } catch (e: any) {
    alert(e.response?.data?.message || '删除失败')
  } finally {
    deleting.value = false
  }
}

onMounted(loadData)
watch(groupPath, loadData)
</script>

<style lang="scss" scoped>
.group-settings-page {
  padding: $spacing-6;
  max-width: 900px;
  margin: 0 auto;
}

.page-header {
  margin-bottom: $spacing-6;
  
  h1 {
    font-size: $text-2xl;
    font-weight: 600;
    margin-bottom: $spacing-1;
  }
  
  .subtitle {
    color: $text-secondary;
    font-size: $text-sm;
  }
}

// Settings Sections
.settings-section {
  border: 1px solid $border-color;
  border-radius: $radius-lg;
  margin-bottom: $spacing-4;
  overflow: hidden;
  
  &.danger {
    border-color: rgba($color-danger, 0.3);
  }
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-4 $spacing-5;
  background: $bg-secondary;
  cursor: pointer;
  user-select: none;
  transition: background $transition-fast;
  
  &:hover { background: $bg-tertiary; }
  
  h3 {
    font-size: $text-base;
    font-weight: 600;
  }
  
  svg {
    width: 16px;
    height: 16px;
    color: $text-secondary;
    transition: transform $transition-normal;
    
    &.rotated { transform: rotate(180deg); }
  }
  
  .danger & {
    h3 { color: $color-danger; }
  }
}

.section-body {
  padding: $spacing-5;
  border-top: 1px solid $border-color;
}

// Forms
.form-group {
  margin-bottom: $spacing-5;
  
  label {
    display: block;
    font-size: $text-sm;
    font-weight: 500;
    margin-bottom: $spacing-2;
    color: $text-primary;
  }
  
  .hint {
    display: block;
    font-size: $text-xs;
    color: $text-muted;
    margin-top: $spacing-1;
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
  
  &:disabled {
    background: $bg-tertiary;
    color: $text-muted;
    cursor: not-allowed;
  }
}

textarea.form-input {
  resize: vertical;
  min-height: 80px;
}

// Visibility Options
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
  transition: all $transition-fast;
  
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
      font-size: $text-sm;
    }
    span {
      font-size: $text-sm;
      color: $text-secondary;
    }
  }
}

.form-actions {
  display: flex;
  justify-content: flex-start;
  gap: $spacing-3;
  margin-top: $spacing-6;
  padding-top: $spacing-4;
  border-top: 1px solid $border-color;
}

// Danger Zone
.danger-zone {
  .danger-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: $spacing-4;
    
    .danger-info {
      h4 {
        font-size: $text-sm;
        font-weight: 600;
        margin-bottom: $spacing-1;
      }
      p {
        font-size: $text-sm;
        color: $text-secondary;
        line-height: 1.5;
      }
    }
  }
  
  .delete-confirm {
    margin-top: $spacing-4;
    padding-top: $spacing-4;
    border-top: 1px solid $border-color;
    
    p {
      font-size: $text-sm;
      margin-bottom: $spacing-3;
      
      code {
        background: $bg-tertiary;
        padding: 2px 6px;
        border-radius: $radius-sm;
        font-family: $font-mono;
        color: $color-danger;
      }
    }
    
    .confirm-actions {
      display: flex;
      gap: $spacing-3;
      margin-top: $spacing-3;
    }
  }
}

// Alerts
.alert {
  padding: $spacing-3 $spacing-4;
  border-radius: $radius-md;
  font-size: $text-sm;
  margin-bottom: $spacing-4;
}

.alert-error {
  background: $color-danger-light;
  color: $color-danger;
  border: 1px solid rgba($color-danger, 0.2);
}

.alert-success {
  background: $color-success-light;
  color: darken($color-success, 10%);
  border: 1px solid rgba($color-success, 0.2);
}

// Buttons
.btn {
  display: inline-flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-2 $spacing-4;
  border-radius: $radius-md;
  font-size: $text-sm;
  font-weight: 500;
  cursor: pointer;
  transition: all $transition-fast;
  text-decoration: none;
  border: 1px solid transparent;
  white-space: nowrap;
  
  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.btn-default {
  background: $bg-primary;
  color: $text-primary;
  border-color: $border-color;
  
  &:hover:not(:disabled) { background: $bg-secondary; }
}

.btn-confirm {
  background: $color-primary;
  color: white;
  
  &:hover:not(:disabled) { background: $color-primary-dark; }
}

.btn-danger {
  background: $color-danger;
  color: white;
  
  &:hover:not(:disabled) { background: darken($color-danger, 10%); }
}

.btn-warning-outline {
  background: transparent;
  color: $color-warning;
  border-color: $color-warning;
  
  &:hover:not(:disabled) {
    background: $color-warning;
    color: white;
  }
}
</style>
