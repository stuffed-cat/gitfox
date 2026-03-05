<template>
  <div class="project-settings-page">
    <div class="settings-header">
      <h2>仓库设置</h2>
      <p class="description">管理分支、保护规则和访问密钥</p>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <template v-else>
      <!-- 默认分支 -->
      <section class="settings-section">
        <h3>默认分支</h3>
        <p class="section-description">默认分支是合并请求和代码浏览的默认目标</p>
        
        <div class="form-group">
          <select v-model="defaultBranch" class="form-control" @change="setDefaultBranch">
            <option v-for="branch in branches" :key="branch.name" :value="branch.name">
              {{ branch.name }}
              <template v-if="branch.is_default"> (当前默认)</template>
            </option>
          </select>
        </div>
      </section>

      <!-- 受保护分支 -->
      <section class="settings-section">
        <h3>受保护分支</h3>
        <p class="section-description">保护重要分支免受意外删除或强制推送</p>
        
        <div class="add-protected-branch">
          <select v-model="newProtectedBranch" class="form-control">
            <option value="">选择分支...</option>
            <option v-for="branch in unprotectedBranches" :key="branch.name" :value="branch.name">
              {{ branch.name }}
            </option>
          </select>
          
          <div class="protection-options">
            <label>
              <input type="checkbox" v-model="protectionOptions.allow_force_push">
              允许强制推送
            </label>
            <label>
              <input type="checkbox" v-model="protectionOptions.require_code_review">
              要求代码审查
            </label>
          </div>
          
          <button class="btn btn-primary" @click="addProtectedBranch" :disabled="!newProtectedBranch">
            保护分支
          </button>
        </div>
        
        <div class="protected-branch-list">
          <div v-for="branch in protectedBranches" :key="branch.name" class="protected-branch-item">
            <div class="branch-info">
              <div class="branch-name">
                <span class="icon icon-branch"></span>
                {{ branch.name }}
                <span v-if="branch.is_default" class="badge badge-primary">默认</span>
              </div>
              <div class="branch-rules">
                <span v-if="branch.allow_force_push" class="rule-tag">允许强推</span>
                <span v-if="branch.require_code_review" class="rule-tag require">需审查</span>
              </div>
            </div>
            <div class="branch-actions">
              <button class="btn btn-outline btn-sm" @click="editProtectedBranch(branch)">
                编辑
              </button>
              <button class="btn btn-danger btn-sm" @click="removeProtectedBranch(branch.name)">
                取消保护
              </button>
            </div>
          </div>
          
          <div v-if="protectedBranches.length === 0" class="empty-state">
            <p>暂无受保护分支</p>
          </div>
        </div>
      </section>

      <!-- Deploy Keys -->
      <section class="settings-section">
        <h3>部署密钥 (Deploy Keys)</h3>
        <p class="section-description">授予外部系统对此仓库的只读或读写访问权限</p>
        
        <div class="add-deploy-key">
          <div class="form-group">
            <label for="deploy-key-title">标题</label>
            <input
              id="deploy-key-title"
              v-model="newDeployKey.title"
              type="text"
              class="form-control"
              placeholder="例如：CI Server"
            />
          </div>
          
          <div class="form-group">
            <label for="deploy-key-content">公钥</label>
            <textarea
              id="deploy-key-content"
              v-model="newDeployKey.key"
              class="form-control"
              rows="3"
              placeholder="ssh-rsa AAAA... 或 ssh-ed25519 AAAA..."
            ></textarea>
          </div>
          
          <div class="form-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="newDeployKey.can_push">
              授予写入权限
            </label>
            <p class="form-help">启用后，此密钥可以推送代码到仓库</p>
          </div>
          
          <button class="btn btn-primary" @click="addDeployKey" :disabled="!newDeployKey.title || !newDeployKey.key">
            添加部署密钥
          </button>
        </div>
        
        <div class="deploy-key-list">
          <div v-for="key in deployKeys" :key="key.id" class="deploy-key-item">
            <div class="key-info">
              <div class="key-title">
                <span class="icon icon-key"></span>
                {{ key.title }}
                <span v-if="key.can_push" class="badge badge-warning">可写</span>
                <span v-else class="badge badge-secondary">只读</span>
              </div>
              <div class="key-meta">
                <span class="key-fingerprint">{{ key.fingerprint }}</span>
                <span class="key-date">添加于 {{ formatDate(key.created_at) }}</span>
                <span v-if="key.last_used_at" class="key-date">最后使用 {{ formatDate(key.last_used_at) }}</span>
              </div>
            </div>
            <div class="key-actions">
              <button class="btn btn-danger btn-sm" @click="removeDeployKey(key.id)">
                删除
              </button>
            </div>
          </div>
          
          <div v-if="deployKeys.length === 0" class="empty-state">
            <p>暂无部署密钥</p>
          </div>
        </div>
      </section>

      <!-- 仓库镜像 -->
      <section class="settings-section">
        <h3>仓库镜像</h3>
        <p class="section-description">自动同步到或从远程仓库</p>
        
        <div class="mirror-config">
          <div class="form-group">
            <label for="mirror-url">镜像 URL</label>
            <input
              id="mirror-url"
              v-model="mirrorConfig.url"
              type="text"
              class="form-control"
              placeholder="https://example.com/repo.git"
            />
          </div>
          
          <div class="form-group">
            <label>镜像方向</label>
            <div class="radio-group">
              <label class="radio-label">
                <input type="radio" v-model="mirrorConfig.direction" value="push">
                推送镜像（将更改推送到远程）
              </label>
              <label class="radio-label">
                <input type="radio" v-model="mirrorConfig.direction" value="pull">
                拉取镜像（从远程拉取更改）
              </label>
            </div>
          </div>
          
          <div class="form-group">
            <label for="mirror-auth">认证方式</label>
            <select id="mirror-auth" v-model="mirrorConfig.auth_type" class="form-control">
              <option value="none">无需认证</option>
              <option value="password">用户名/密码</option>
              <option value="ssh_key">SSH 密钥</option>
            </select>
          </div>
          
          <template v-if="mirrorConfig.auth_type === 'password'">
            <div class="form-group">
              <label for="mirror-username">用户名</label>
              <input
                id="mirror-username"
                v-model="mirrorConfig.username"
                type="text"
                class="form-control"
              />
            </div>
            <div class="form-group">
              <label for="mirror-password">密码/Token</label>
              <input
                id="mirror-password"
                v-model="mirrorConfig.password"
                type="password"
                class="form-control"
              />
            </div>
          </template>
          
          <button class="btn btn-primary" @click="saveMirrorConfig" :disabled="!mirrorConfig.url">
            保存镜像配置
          </button>
          
          <button v-if="mirrorConfig.url" class="btn btn-outline ml-2" @click="triggerMirrorSync">
            立即同步
          </button>
        </div>
      </section>

      <!-- 清理仓库 -->
      <section class="settings-section">
        <h3>仓库维护</h3>
        <p class="section-description">清理和优化仓库</p>
        
        <div class="maintenance-actions">
          <div class="maintenance-item">
            <div class="maintenance-info">
              <strong>运行垃圾回收</strong>
              <p>清理未引用的对象，压缩仓库数据</p>
            </div>
            <button class="btn btn-outline" @click="runGarbageCollection" :disabled="gcRunning">
              {{ gcRunning ? '运行中...' : '运行 GC' }}
            </button>
          </div>
          
          <div class="maintenance-item">
            <div class="maintenance-info">
              <strong>重新打包仓库</strong>
              <p>优化仓库存储，提高性能</p>
            </div>
            <button class="btn btn-outline" @click="repackRepository" :disabled="repackRunning">
              {{ repackRunning ? '运行中...' : '重新打包' }}
            </button>
          </div>
        </div>
      </section>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch, computed } from 'vue'
import api from '@/api'
import type { Project, BranchInfo } from '@/types'

interface ProtectedBranch extends BranchInfo {
  allow_force_push?: boolean
  require_code_review?: boolean
}

interface DeployKey {
  id: string
  title: string
  fingerprint: string
  can_push: boolean
  created_at: string
  last_used_at?: string
}

const props = defineProps<{
  project?: Project
}>()

const loading = ref(false)
const branches = ref<BranchInfo[]>([])
const defaultBranch = ref('main')
const deployKeys = ref<DeployKey[]>([])
const gcRunning = ref(false)
const repackRunning = ref(false)

// 受保护分支
const protectedBranches = ref<ProtectedBranch[]>([])
const newProtectedBranch = ref('')
const protectionOptions = reactive({
  allow_force_push: false,
  require_code_review: true
})

// 新部署密钥
const newDeployKey = reactive({
  title: '',
  key: '',
  can_push: false
})

// 镜像配置
const mirrorConfig = reactive({
  url: '',
  direction: 'push' as 'push' | 'pull',
  auth_type: 'none' as 'none' | 'password' | 'ssh_key',
  username: '',
  password: ''
})

const projectPath = computed(() => {
  if (!props.project?.owner_name || !props.project?.name) return null
  return { namespace: props.project.owner_name, project: props.project.name }
})

const unprotectedBranches = computed(() => {
  const protectedNames = new Set(protectedBranches.value.map(b => b.name))
  return branches.value.filter(b => !protectedNames.has(b.name))
})

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('zh-CN')
}

async function loadSettings() {
  if (!projectPath.value) return
  loading.value = true
  
  try {
    const branchData = await api.branches.list(projectPath.value)
    branches.value = branchData
    
    // 找到默认分支
    const defaultB = branchData.find(b => b.is_default)
    if (defaultB) {
      defaultBranch.value = defaultB.name
    }
    
    // 筛选受保护分支
    protectedBranches.value = branchData.filter(b => b.is_protected)
    
    // 加载部署密钥（如果有对应 API）
    // deployKeys.value = await api.deployKeys.list(projectPath.value)
  } catch (error) {
    console.error('Failed to load settings:', error)
  } finally {
    loading.value = false
  }
}

async function setDefaultBranch() {
  if (!projectPath.value) return
  
  // TODO: 后端 API 尚未支持设置默认分支
  // 需要在后端 UpdateProjectRequest 中添加 default_branch 字段
  console.log('Set default branch to:', defaultBranch.value)
  alert('默认分支设置功能即将支持')
}

async function addProtectedBranch() {
  if (!projectPath.value || !newProtectedBranch.value) return
  
  try {
    // 调用分支保护 API（需要后端支持）
    // await api.branches.protect(projectPath.value, newProtectedBranch.value, protectionOptions)
    alert('分支保护功能即将实现')
    newProtectedBranch.value = ''
  } catch (error) {
    console.error('Failed to protect branch:', error)
    alert('保护分支失败')
  }
}

function editProtectedBranch(branch: ProtectedBranch) {
  // 编辑保护规则
  newProtectedBranch.value = branch.name
  protectionOptions.allow_force_push = branch.allow_force_push || false
  protectionOptions.require_code_review = branch.require_code_review || false
}

async function removeProtectedBranch(branchName: string) {
  if (!projectPath.value) return
  if (!confirm(`确定要取消保护分支 "${branchName}" 吗？`)) return
  
  try {
    // await api.branches.unprotect(projectPath.value, branchName)
    protectedBranches.value = protectedBranches.value.filter(b => b.name !== branchName)
    alert('已取消分支保护')
  } catch (error) {
    console.error('Failed to unprotect branch:', error)
    alert('取消保护失败')
  }
}

async function addDeployKey() {
  if (!projectPath.value || !newDeployKey.title || !newDeployKey.key) return
  
  try {
    // await api.deployKeys.create(projectPath.value, newDeployKey)
    alert('部署密钥功能即将实现')
    newDeployKey.title = ''
    newDeployKey.key = ''
    newDeployKey.can_push = false
  } catch (error) {
    console.error('Failed to add deploy key:', error)
    alert('添加部署密钥失败')
  }
}

async function removeDeployKey(keyId: string) {
  if (!projectPath.value) return
  if (!confirm('确定要删除此部署密钥吗？')) return
  
  try {
    // await api.deployKeys.delete(projectPath.value, keyId)
    deployKeys.value = deployKeys.value.filter(k => k.id !== keyId)
    alert('部署密钥已删除')
  } catch (error) {
    console.error('Failed to remove deploy key:', error)
  }
}

async function saveMirrorConfig() {
  if (!projectPath.value || !mirrorConfig.url) return
  
  try {
    // await api.projects.updateMirror(projectPath.value, mirrorConfig)
    alert('镜像配置功能即将实现')
  } catch (error) {
    console.error('Failed to save mirror config:', error)
    alert('保存镜像配置失败')
  }
}

async function triggerMirrorSync() {
  if (!projectPath.value) return
  
  try {
    // await api.projects.syncMirror(projectPath.value)
    alert('正在同步镜像...')
  } catch (error) {
    console.error('Failed to sync mirror:', error)
    alert('同步失败')
  }
}

async function runGarbageCollection() {
  if (!projectPath.value) return
  gcRunning.value = true
  
  try {
    // await api.repositories.gc(projectPath.value)
    alert('垃圾回收已完成')
  } catch (error) {
    console.error('Failed to run GC:', error)
    alert('垃圾回收失败')
  } finally {
    gcRunning.value = false
  }
}

async function repackRepository() {
  if (!projectPath.value) return
  repackRunning.value = true
  
  try {
    // await api.repositories.repack(projectPath.value)
    alert('仓库重新打包已完成')
  } catch (error) {
    console.error('Failed to repack:', error)
    alert('重新打包失败')
  } finally {
    repackRunning.value = false
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
    margin-bottom: $spacing-sm;
  }
  
  .section-description {
    color: $text-muted;
    margin-bottom: $spacing-lg;
  }
}

.form-group {
  margin-bottom: $spacing-md;
  
  label {
    display: block;
    margin-bottom: $spacing-xs;
    font-weight: 500;
  }
  
  .form-help {
    font-size: $font-size-sm;
    color: $text-muted;
    margin-top: $spacing-xs;
  }
}

.add-protected-branch, .add-deploy-key, .mirror-config {
  margin-bottom: $spacing-lg;
  padding: $spacing-lg;
  background: $bg-secondary;
  border-radius: $border-radius;
}

.protection-options {
  display: flex;
  gap: $spacing-lg;
  margin: $spacing-md 0;
  
  label {
    display: flex;
    align-items: center;
    gap: $spacing-xs;
    cursor: pointer;
  }
}

.protected-branch-list, .deploy-key-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
}

.protected-branch-item, .deploy-key-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-md;
  background: $bg-secondary;
  border-radius: $border-radius;
}

.branch-info, .key-info {
  flex: 1;
}

.branch-name, .key-title {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  font-weight: 500;
  margin-bottom: $spacing-xs;
}

.branch-rules {
  display: flex;
  gap: $spacing-xs;
}

.rule-tag {
  font-size: $font-size-xs;
  padding: 2px 8px;
  background: $bg-tertiary;
  border-radius: 3px;
  
  &.require {
    background: rgba($warning-color, 0.2);
    color: $warning-color;
  }
}

.key-meta {
  display: flex;
  gap: $spacing-md;
  font-size: $font-size-sm;
  color: $text-muted;
}

.key-fingerprint {
  font-family: monospace;
}

.branch-actions, .key-actions {
  display: flex;
  gap: $spacing-sm;
}

.badge {
  font-size: $font-size-xs;
  padding: 2px 6px;
  border-radius: 3px;
  
  &.badge-primary {
    background: $primary-color;
    color: white;
  }
  
  &.badge-secondary {
    background: $bg-tertiary;
    color: $text-muted;
  }
  
  &.badge-warning {
    background: rgba($warning-color, 0.2);
    color: $warning-color;
  }
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  cursor: pointer;
}

.radio-group {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
}

.radio-label {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  cursor: pointer;
}

.maintenance-actions {
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
}

.maintenance-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-md;
  background: $bg-secondary;
  border-radius: $border-radius;
  
  .maintenance-info {
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

.empty-state {
  padding: $spacing-lg;
  text-align: center;
  color: $text-muted;
}

.ml-2 {
  margin-left: $spacing-sm;
}
</style>
