<template>
  <div class="project-settings-page">
    <div class="settings-header">
      <h2>通用设置</h2>
      <p class="description">管理项目的基本信息和配置</p>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <template v-else>
      <!-- 基本信息 -->
      <section class="settings-section">
        <h3>基本信息</h3>
        <form @submit.prevent="saveBasicInfo" class="settings-form">
          <div class="form-group">
            <label for="name">项目名称</label>
            <input
              id="name"
              v-model="basicForm.name"
              type="text"
              class="form-control"
              required
            />
            <p class="form-help">项目名称将作为 URL 的一部分，建议使用简洁的名称</p>
          </div>
          
          <div class="form-group">
            <label for="description">项目描述</label>
            <textarea
              id="description"
              v-model="basicForm.description"
              class="form-control"
              rows="3"
              placeholder="描述这个项目的用途..."
            ></textarea>
          </div>
          
          <div class="form-group">
            <label for="visibility">可见性级别</label>
            <select id="visibility" v-model="basicForm.visibility" class="form-control">
              <option value="private">私有 - 仅项目成员可见</option>
              <option value="internal">内部 - 所有登录用户可见</option>
              <option value="public">公开 - 所有人可见</option>
            </select>
            <p class="form-help">
              <template v-if="basicForm.visibility === 'private'">只有项目成员才能查看和克隆此项目</template>
              <template v-else-if="basicForm.visibility === 'internal'">所有登录用户都可以查看此项目</template>
              <template v-else>任何人都可以查看此项目</template>
            </p>
          </div>

          <div class="form-group">
            <label for="topics">项目标签</label>
            <input
              id="topics"
              v-model="topicsInput"
              type="text"
              class="form-control"
              placeholder="输入标签，用逗号分隔"
            />
            <p class="form-help">添加标签帮助他人发现你的项目</p>
          </div>
          
          <button type="submit" class="btn btn-primary" :disabled="saving">
            {{ saving ? '保存中...' : '保存更改' }}
          </button>
        </form>
      </section>

      <!-- 项目功能开关 -->
      <section class="settings-section">
        <h3>功能开关</h3>
        <p class="section-description">启用或禁用项目功能</p>
        
        <div class="feature-toggles">
          <div class="feature-toggle">
            <div class="feature-info">
              <strong>议题 (Issues)</strong>
              <p>用于跟踪问题、bug 和功能请求</p>
            </div>
            <label class="toggle-switch">
              <input type="checkbox" v-model="features.issues_enabled" @change="saveFeatures">
              <span class="toggle-slider"></span>
            </label>
          </div>
          
          <div class="feature-toggle">
            <div class="feature-info">
              <strong>合并请求 (Merge Requests)</strong>
              <p>用于代码审查和合并代码变更</p>
            </div>
            <label class="toggle-switch">
              <input type="checkbox" v-model="features.merge_requests_enabled" @change="saveFeatures">
              <span class="toggle-slider"></span>
            </label>
          </div>
          
          <div class="feature-toggle">
            <div class="feature-info">
              <strong>流水线 (Pipelines)</strong>
              <p>启用 CI/CD 自动化构建和部署</p>
            </div>
            <label class="toggle-switch">
              <input type="checkbox" v-model="features.pipelines_enabled" @change="saveFeatures">
              <span class="toggle-slider"></span>
            </label>
          </div>
          
          <div class="feature-toggle">
            <div class="feature-info">
              <strong>软件包仓库 (Packages)</strong>
              <p>存储和分发软件包</p>
            </div>
            <label class="toggle-switch">
              <input type="checkbox" v-model="features.packages_enabled" @change="saveFeatures">
              <span class="toggle-slider"></span>
            </label>
          </div>

          <div class="feature-toggle">
            <div class="feature-info">
              <strong>Wiki</strong>
              <p>项目文档和知识库</p>
            </div>
            <label class="toggle-switch">
              <input type="checkbox" v-model="features.wiki_enabled" @change="saveFeatures">
              <span class="toggle-slider"></span>
            </label>
          </div>
        </div>
      </section>

      <!-- 高级设置 -->
      <section class="settings-section">
        <h3>高级设置</h3>
        
        <div class="form-group">
          <label for="default_branch">默认分支</label>
          <select id="default_branch" v-model="advancedForm.default_branch" class="form-control" @change="saveAdvanced">
            <option v-for="branch in branches" :key="branch.name" :value="branch.name">
              {{ branch.name }}
            </option>
          </select>
          <p class="form-help">默认分支是合并请求和代码浏览的默认目标</p>
        </div>
      </section>
      
      <!-- 危险区域 -->
      <section class="settings-section danger-zone">
        <h3>危险操作</h3>
        <p class="section-description">这些操作可能造成不可逆的影响，请谨慎操作</p>
        
        <div class="danger-item">
          <div class="danger-info">
            <strong>归档项目</strong>
            <p>归档后项目将变为只读状态，无法推送代码或创建议题。可以随时取消归档。</p>
          </div>
          <button class="btn btn-outline-warning" @click="archiveProject" :disabled="isArchived">
            {{ isArchived ? '已归档' : '归档项目' }}
          </button>
        </div>

        <div class="danger-item">
          <div class="danger-info">
            <strong>转移项目</strong>
            <p>将项目转移到另一个命名空间（用户或群组）</p>
          </div>
          <button class="btn btn-outline-warning" @click="showTransferModal = true">
            转移项目
          </button>
        </div>
        
        <div class="danger-item">
          <div class="danger-info">
            <strong>删除项目</strong>
            <p>删除项目后，所有相关数据将被永久删除，此操作不可撤销。</p>
          </div>
          <button class="btn btn-danger" @click="deleteProject">
            删除项目
          </button>
        </div>
      </section>
    </template>

    <!-- 转移项目模态框 -->
    <div v-if="showTransferModal" class="modal-overlay" @click.self="showTransferModal = false">
      <div class="modal-content">
        <h3>转移项目</h3>
        <p>选择要将项目转移到的目标命名空间：</p>
        
        <select v-model="transferTarget" class="form-control">
          <option value="">选择命名空间...</option>
          <option v-for="ns in availableNamespaces" :key="ns.id" :value="ns.id">
            {{ ns.path }} ({{ ns.namespace_type === 'user' ? '用户' : '群组' }})
          </option>
        </select>
        
        <div class="modal-actions">
          <button class="btn btn-outline" @click="showTransferModal = false">取消</button>
          <button class="btn btn-primary" @click="transferProject" :disabled="!transferTarget">
            确认转移
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch, computed } from 'vue'
import { useRouter } from 'vue-router'
import api from '@/api'
import type { Project, BranchInfo, NamespaceOption } from '@/types'

const props = defineProps<{
  project?: Project
}>()

const router = useRouter()

const loading = ref(false)
const saving = ref(false)
const branches = ref<BranchInfo[]>([])
const availableNamespaces = ref<NamespaceOption[]>([])

const basicForm = reactive({
  name: '',
  description: '',
  visibility: 'private' as 'public' | 'private' | 'internal'
})

const features = reactive({
  issues_enabled: true,
  merge_requests_enabled: true,
  pipelines_enabled: true,
  packages_enabled: true,
  wiki_enabled: false
})

const advancedForm = reactive({
  default_branch: 'main'
})

const topicsInput = ref('')
const isArchived = ref(false)
const showTransferModal = ref(false)
const transferTarget = ref<number | ''>('')

const projectPath = computed(() => {
  if (!props.project?.owner_name || !props.project?.name) return null
  return { namespace: props.project.owner_name, project: props.project.name }
})

async function loadSettings() {
  if (!projectPath.value) return
  loading.value = true
  
  try {
    const [branchData, namespacesData] = await Promise.all([
      api.branches.list(projectPath.value),
      api.namespaces.listForProjectCreation()
    ])
    
    branches.value = branchData
    availableNamespaces.value = namespacesData
    
    // 初始化表单
    if (props.project) {
      basicForm.name = props.project.name
      basicForm.description = props.project.description || ''
      basicForm.visibility = props.project.visibility
    }
    
    // 找到默认分支
    const defaultBranch = branchData.find(b => b.is_default)
    if (defaultBranch) {
      advancedForm.default_branch = defaultBranch.name
    }
  } catch (error) {
    console.error('Failed to load settings:', error)
  } finally {
    loading.value = false
  }
}

async function saveBasicInfo() {
  if (!projectPath.value) return
  saving.value = true
  
  try {
    await api.projects.update(projectPath.value, basicForm)
    alert('保存成功')
  } catch (error) {
    console.error('Failed to save:', error)
    alert('保存失败')
  } finally {
    saving.value = false
  }
}

async function saveFeatures() {
  // TODO: 后端 API 尚未支持功能开关设置
  // 等待后端实现后启用此功能
  console.log('Features to save:', features)
  alert('功能开关设置即将支持')
}

async function saveAdvanced() {
  // TODO: 后端 API 尚未支持默认分支设置
  // 等待后端实现后启用此功能
  console.log('Advanced settings to save:', advancedForm)
  alert('高级设置即将支持')
}

async function archiveProject() {
  if (!projectPath.value) return
  if (!confirm('确定要归档此项目吗？归档后项目将变为只读状态。')) return
  
  // TODO: 后端 API 尚未支持归档功能
  // 等待后端实现后启用此功能
  alert('项目归档功能即将支持')
}

async function transferProject() {
  if (!projectPath.value || !transferTarget.value) return
  
  // TODO: 后端 API 尚未支持项目转移功能
  // 等待后端实现后启用此功能
  console.log('Transfer to namespace_id:', transferTarget.value)
  alert('项目转移功能即将支持')
  showTransferModal.value = false
}

async function deleteProject() {
  if (!projectPath.value || !props.project) return
  
  const confirmed = prompt(`请输入项目名称 "${props.project.name}" 以确认删除：`)
  if (confirmed !== props.project.name) {
    alert('项目名称不匹配')
    return
  }
  
  try {
    await api.projects.delete(projectPath.value)
    router.push('/projects')
  } catch (error) {
    console.error('Failed to delete project:', error)
    alert('删除项目失败')
  }
}

watch([() => props.project?.owner_name, () => props.project?.name], () => {
  loadSettings()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.project-settings-page {
  padding: $spacing-lg;
  max-width: 800px;
}

.settings-header {
  margin-bottom: $spacing-xl;
  
  h2 {
    margin: 0 0 $spacing-xs 0;
  }
  
  .description {
    color: $text-muted;
    margin: 0;
  }
}

.settings-section {
  margin-bottom: $spacing-xl;
  padding-bottom: $spacing-xl;
  border-bottom: 1px solid $border-color;
  
  &:last-child {
    border-bottom: none;
  }
  
  h3 {
    margin-bottom: $spacing-md;
  }
  
  .section-description {
    color: $text-muted;
    margin-bottom: $spacing-lg;
  }
}

.settings-form {
  .form-group {
    margin-bottom: $spacing-lg;
  }
  
  .form-help {
    font-size: $font-size-sm;
    color: $text-muted;
    margin-top: $spacing-xs;
  }
  
  button {
    margin-top: $spacing-md;
  }
}

.feature-toggles {
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
}

.feature-toggle {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-md;
  background: $bg-secondary;
  border-radius: $border-radius;
  
  .feature-info {
    strong {
      display: block;
      margin-bottom: $spacing-xs;
    }
    
    p {
      margin: 0;
      font-size: $font-size-sm;
      color: $text-muted;
    }
  }
}

.toggle-switch {
  position: relative;
  width: 48px;
  height: 24px;
  
  input {
    opacity: 0;
    width: 0;
    height: 0;
    
    &:checked + .toggle-slider {
      background-color: $primary-color;
      
      &::before {
        transform: translateX(24px);
      }
    }
  }
  
  .toggle-slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: $bg-tertiary;
    border-radius: 24px;
    transition: 0.3s;
    
    &::before {
      position: absolute;
      content: "";
      height: 18px;
      width: 18px;
      left: 3px;
      bottom: 3px;
      background-color: white;
      border-radius: 50%;
      transition: 0.3s;
    }
  }
}

.danger-zone {
  h3 {
    color: $danger-color;
  }
}

.danger-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-lg;
  background: rgba($danger-color, 0.05);
  border: 1px solid rgba($danger-color, 0.2);
  border-radius: $border-radius;
  margin-bottom: $spacing-md;
  
  &:last-child {
    margin-bottom: 0;
  }
  
  .danger-info {
    flex: 1;
    margin-right: $spacing-lg;
    
    strong {
      display: block;
      margin-bottom: $spacing-xs;
    }
    
    p {
      margin: 0;
      font-size: $font-size-sm;
      color: $text-muted;
    }
  }
}

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: $bg-primary;
  padding: $spacing-xl;
  border-radius: $border-radius-lg;
  width: 100%;
  max-width: 480px;
  
  h3 {
    margin-top: 0;
  }
  
  .form-control {
    margin: $spacing-md 0;
  }
  
  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: $spacing-md;
    margin-top: $spacing-lg;
  }
}

.btn-outline-warning {
  border: 1px solid $warning-color;
  color: $warning-color;
  background: transparent;
  
  &:hover:not(:disabled) {
    background: rgba($warning-color, 0.1);
  }
  
  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}
</style>
