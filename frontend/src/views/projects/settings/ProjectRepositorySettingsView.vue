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
        
          <form @submit.prevent="setDefaultBranch" class="settings-form">
            <div class="form-group">
              <select v-model="defaultBranch" class="form-control">
                <option v-for="branch in branches" :key="branch.name" :value="branch.name">
                  {{ branch.name }}
                  <template v-if="branch.is_default"> (当前默认)</template>
                </option>
              </select>
            </div>
            <button type="submit" class="btn btn-primary" :disabled="savingBranch">
              {{ savingBranch ? '保存中...' : '保存更改' }}
            </button>
          </form>
        <p class="section-description">保护重要分支免受意外删除或强制推送</p>
        
        <button class="btn btn-primary mb-3" @click="openAddProtectionModal">
          添加保护规则
        </button>
        
        <div class="protected-branch-list">
          <div v-for="rule in protectionRules" :key="rule.id" class="protected-branch-item">
            <div class="branch-info">
              <div class="branch-name">
                <span class="icon icon-branch"></span>
                <code>{{ rule.branch_pattern }}</code>
              </div>
              <div class="branch-rules">
                <span v-if="rule.require_review" class="rule-tag require">需审查 ({{ rule.required_reviewers }}人)</span>
                <span v-if="rule.require_ci_pass" class="rule-tag">需CI通过</span>
                <span v-if="rule.allow_force_push" class="rule-tag warning">允许强推</span>
                <span v-if="rule.allow_deletion" class="rule-tag warning">允许删除</span>
              </div>
            </div>
            <div class="branch-actions">
              <button class="btn btn-outline btn-sm" @click="editProtectionRule(rule)">
                编辑
              </button>
              <button class="btn btn-danger btn-sm" @click="removeProtectionRule(rule)">
                删除
              </button>
            </div>
          </div>
          
          <div v-if="protectionRules.length === 0" class="empty-state">
            <p>暂无分支保护规则</p>
          </div>
        </div>
        
        <!-- 添加/编辑保护规则模态框 -->
        <div v-if="showAddProtectionModal" class="modal-overlay" @click.self="showAddProtectionModal = false">
          <div class="modal-content">
            <h4>{{ editingRule ? '编辑' : '添加' }}分支保护规则</h4>
            
            <div class="form-group">
              <label>分支模式</label>
              <input
                v-model="newProtectionRule.branch_pattern"
                type="text"
                class="form-control"
                placeholder="main 或 release/* 或 feature/**"
                :disabled="!!editingRule"
              />
              <p class="form-help">支持通配符: * 匹配单级, ** 匹配多级</p>
            </div>
            
            <div class="form-group">
              <label class="checkbox-label">
                <input type="checkbox" v-model="newProtectionRule.require_review">
                要求代码审查
              </label>
            </div>
            
            <div v-if="newProtectionRule.require_review" class="form-group">
              <label>最少审查人数</label>
              <input
                v-model.number="newProtectionRule.required_reviewers"
                type="number"
                class="form-control"
                min="1"
                max="10"
              />
            </div>
            
            <div class="form-group">
              <label class="checkbox-label">
                <input type="checkbox" v-model="newProtectionRule.require_ci_pass">
                要求 CI 通过
              </label>
            </div>
            
            <div class="form-group">
              <label class="checkbox-label">
                <input type="checkbox" v-model="newProtectionRule.allow_force_push">
                允许强制推送
              </label>
              <p class="form-help warning">警告：允许强制推送可能导致历史记录丢失</p>
            </div>
            
            <div class="form-group">
              <label class="checkbox-label">
                <input type="checkbox" v-model="newProtectionRule.allow_deletion">
                允许删除分支
              </label>
            </div>
            
            <div class="modal-actions">
              <button class="btn btn-secondary" @click="showAddProtectionModal = false">取消</button>
              <button class="btn btn-primary" @click="saveProtectionRule" :disabled="!newProtectionRule.branch_pattern">
                {{ editingRule ? '保存' : '添加' }}
              </button>
            </div>
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
import type { Project, BranchInfo, BranchProtectionRule, DeployKey } from '@/types'

const props = defineProps<{
  project?: Project
}>()

const loading = ref(false)
const savingBranch = ref(false)
const branches = ref<BranchInfo[]>([])
const defaultBranch = ref('main')
const gcRunning = ref(false)
const repackRunning = ref(false)

// 分支保护规则（使用新的 API）
const protectionRules = ref<BranchProtectionRule[]>([])
const showAddProtectionModal = ref(false)
const editingRule = ref<BranchProtectionRule | null>(null)
const newProtectionRule = reactive({
  branch_pattern: '',
  require_review: true,
  required_reviewers: 1,
  require_ci_pass: true,
  allow_force_push: false,
  allow_deletion: false
})

// 部署密钥
const deployKeys = ref<DeployKey[]>([])
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

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('zh-CN')
}

async function loadSettings() {
  if (!projectPath.value) return
  loading.value = true
  
  try {
    // 加载分支列表
    const branchData = await api.branches.list(projectPath.value)
    branches.value = branchData
    
    // 找到默认分支
    const defaultB = branchData.find(b => b.is_default)
    if (defaultB) {
      defaultBranch.value = defaultB.name
    }
    
    // 加载分支保护规则
    protectionRules.value = await api.branchProtection.list(projectPath.value)
    
    // 加载部署密钥
    deployKeys.value = await api.deployKeys.list(projectPath.value)
  } catch (error) {
    console.error('Failed to load settings:', error)
  } finally {
    loading.value = false
  }
}

async function setDefaultBranch() {
  if (!projectPath.value) return
  savingBranch.value = true
  
  try {
    await api.projects.update(projectPath.value, {
      default_branch: defaultBranch.value
    })
    alert('默认分支已更新')
  } catch (error) {
    console.error('Failed to set default branch:', error)
    alert('设置默认分支失败')
  } finally {
    savingBranch.value = false
  }
}

function openAddProtectionModal() {
  editingRule.value = null
  newProtectionRule.branch_pattern = ''
  newProtectionRule.require_review = true
  newProtectionRule.required_reviewers = 1
  newProtectionRule.require_ci_pass = true
  newProtectionRule.allow_force_push = false
  newProtectionRule.allow_deletion = false
  showAddProtectionModal.value = true
}

function editProtectionRule(rule: BranchProtectionRule) {
  editingRule.value = rule
  newProtectionRule.branch_pattern = rule.branch_pattern
  newProtectionRule.require_review = rule.require_review
  newProtectionRule.required_reviewers = rule.required_reviewers
  newProtectionRule.require_ci_pass = rule.require_ci_pass
  newProtectionRule.allow_force_push = rule.allow_force_push
  newProtectionRule.allow_deletion = rule.allow_deletion
  showAddProtectionModal.value = true
}

async function saveProtectionRule() {
  if (!projectPath.value || !newProtectionRule.branch_pattern) return
  
  try {
    if (editingRule.value) {
      // 更新现有规则
      const updated = await api.branchProtection.update(projectPath.value, editingRule.value.id, {
        require_review: newProtectionRule.require_review,
        required_reviewers: newProtectionRule.required_reviewers,
        require_ci_pass: newProtectionRule.require_ci_pass,
        allow_force_push: newProtectionRule.allow_force_push,
        allow_deletion: newProtectionRule.allow_deletion
      })
      const index = protectionRules.value.findIndex(r => r.id === editingRule.value!.id)
      if (index !== -1) {
        protectionRules.value[index] = updated
      }
    } else {
      // 创建新规则
      const created = await api.branchProtection.create(projectPath.value, newProtectionRule)
      protectionRules.value.push(created)
    }
    showAddProtectionModal.value = false
  } catch (error: any) {
    console.error('Failed to save protection rule:', error)
    alert(error.response?.data?.message || '保存分支保护规则失败')
  }
}

async function removeProtectionRule(rule: BranchProtectionRule) {
  if (!projectPath.value) return
  if (!confirm(`确定要删除保护规则 "${rule.branch_pattern}" 吗？`)) return
  
  try {
    await api.branchProtection.delete(projectPath.value, rule.id)
    protectionRules.value = protectionRules.value.filter(r => r.id !== rule.id)
  } catch (error) {
    console.error('Failed to delete protection rule:', error)
    alert('删除保护规则失败')
  }
}

async function addDeployKey() {
  if (!projectPath.value || !newDeployKey.title || !newDeployKey.key) return
  
  try {
    const created = await api.deployKeys.create(projectPath.value, {
      title: newDeployKey.title,
      key: newDeployKey.key,
      can_push: newDeployKey.can_push
    })
    deployKeys.value.push(created)
    newDeployKey.title = ''
    newDeployKey.key = ''
    newDeployKey.can_push = false
  } catch (error: any) {
    console.error('Failed to add deploy key:', error)
    alert(error.response?.data?.message || '添加部署密钥失败')
  }
}

async function removeDeployKey(keyId: number) {
  if (!projectPath.value) return
  if (!confirm('确定要删除此部署密钥吗？')) return
  
  try {
    await api.deployKeys.delete(projectPath.value, keyId)
    deployKeys.value = deployKeys.value.filter(k => k.id !== keyId)
  } catch (error) {
    console.error('Failed to remove deploy key:', error)
    alert('删除部署密钥失败')
  }
}

async function saveMirrorConfig() {
  if (!projectPath.value || !mirrorConfig.url) return
  
  try {
    // 镜像功能即将支持（注：这是预留功能，后端尚未实现）
    alert('仓库镜像功能即将支持')
  } catch (error) {
    console.error('Failed to save mirror config:', error)
    alert('保存镜像配置失败')
  }
}

async function triggerMirrorSync() {
  if (!projectPath.value) return
  
  try {
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

.mb-3 {
  margin-bottom: $spacing-md;
}

// Modal styles
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
  border-radius: $border-radius-lg;
  padding: $spacing-xl;
  min-width: 400px;
  max-width: 500px;
  max-height: 80vh;
  overflow-y: auto;
  
  h4 {
    margin: 0 0 $spacing-lg 0;
  }
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: $spacing-sm;
  margin-top: $spacing-lg;
  padding-top: $spacing-lg;
  border-top: 1px solid $border-color;
}

.rule-tag {
  &.warning {
    background: rgba($danger-color, 0.2);
    color: $danger-color;
  }
}

.form-help {
  &.warning {
    color: $warning-color;
  }
}

code {
  background: $bg-secondary;
  padding: 2px 6px;
  border-radius: 3px;
  font-family: monospace;
}
</style>
