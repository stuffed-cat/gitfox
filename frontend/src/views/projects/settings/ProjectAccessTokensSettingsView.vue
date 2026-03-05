<template>
  <div class="project-settings-page">
    <div class="settings-header">
      <h2>项目访问令牌</h2>
      <p class="description">创建用于 API 访问和自动化的项目级别令牌</p>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <template v-else>
      <!-- 创建访问令牌 -->
      <section class="settings-section">
        <h3>创建项目访问令牌</h3>
        <p class="section-description">
          项目访问令牌允许以项目身份访问 API，适用于 CI/CD 和自动化脚本。
        </p>
        
        <form @submit.prevent="createToken" class="token-form">
          <div class="form-group">
            <label for="token-name">令牌名称 <span class="required">*</span></label>
            <input
              id="token-name"
              v-model="tokenForm.name"
              type="text"
              class="form-control"
              placeholder="例如：CI/CD 部署"
              required
            />
          </div>
          
          <div class="form-group">
            <label for="token-expires">到期日期 (可选)</label>
            <input
              id="token-expires"
              v-model="tokenForm.expires_at"
              type="date"
              class="form-control"
              :min="minDate"
              :max="maxDate"
            />
            <p class="form-help">留空表示永不过期（不推荐）</p>
          </div>
          
          <div class="form-group">
            <label>角色</label>
            <select v-model="tokenForm.role" class="form-control">
              <option value="reporter">报告者 - 只读访问</option>
              <option value="developer">开发者 - 读写访问</option>
              <option value="maintainer">维护者 - 完全访问</option>
            </select>
            <p class="form-help">令牌将具有所选角色的权限</p>
          </div>
          
          <div class="form-group">
            <label>权限范围</label>
            <div class="scope-checkboxes">
              <label class="checkbox-label">
                <input type="checkbox" v-model="tokenForm.scopes" value="api">
                <div class="scope-info">
                  <strong>api</strong>
                  <span>读写 API 访问</span>
                </div>
              </label>
              
              <label class="checkbox-label">
                <input type="checkbox" v-model="tokenForm.scopes" value="read_api">
                <div class="scope-info">
                  <strong>read_api</strong>
                  <span>只读 API 访问</span>
                </div>
              </label>
              
              <label class="checkbox-label">
                <input type="checkbox" v-model="tokenForm.scopes" value="read_repository">
                <div class="scope-info">
                  <strong>read_repository</strong>
                  <span>读取仓库（git pull）</span>
                </div>
              </label>
              
              <label class="checkbox-label">
                <input type="checkbox" v-model="tokenForm.scopes" value="write_repository">
                <div class="scope-info">
                  <strong>write_repository</strong>
                  <span>写入仓库（git push）</span>
                </div>
              </label>
              
              <label class="checkbox-label">
                <input type="checkbox" v-model="tokenForm.scopes" value="read_registry">
                <div class="scope-info">
                  <strong>read_registry</strong>
                  <span>读取容器/包仓库</span>
                </div>
              </label>
              
              <label class="checkbox-label">
                <input type="checkbox" v-model="tokenForm.scopes" value="write_registry">
                <div class="scope-info">
                  <strong>write_registry</strong>
                  <span>写入容器/包仓库</span>
                </div>
              </label>
            </div>
          </div>
          
          <button type="submit" class="btn btn-primary" :disabled="!tokenForm.name || tokenForm.scopes.length === 0">
            创建令牌
          </button>
        </form>
      </section>

      <!-- 新创建的令牌显示 -->
      <section v-if="newToken" class="settings-section token-created">
        <h3>新令牌已创建</h3>
        <div class="token-display">
          <div class="token-warning">
            请立即复制此令牌，它只会显示一次！
          </div>
          <div class="token-value">
            <code>{{ newToken }}</code>
            <button class="btn btn-outline btn-sm" @click="copyToken">
              复制
            </button>
          </div>
        </div>
        
        <div class="usage-example">
          <h4>使用示例</h4>
          <pre><code># 使用此令牌克隆仓库
git clone https://oauth2:{{ newToken }}@{{ host }}/{{ project?.owner_name }}/{{ project?.name }}.git

# 或作为 CI 变量使用
export PROJECT_ACCESS_TOKEN="{{ newToken }}"</code></pre>
        </div>
      </section>

      <!-- 现有令牌列表 -->
      <section class="settings-section">
        <h3>活跃的访问令牌</h3>
        
        <div class="token-list">
          <div v-for="token in tokens" :key="token.id" class="token-item">
            <div class="token-info">
              <div class="token-name">
                {{ token.name }}
                <span v-if="isExpired(token)" class="badge badge-danger">已过期</span>
                <span v-else-if="isExpiringSoon(token)" class="badge badge-warning">即将过期</span>
                <span v-else class="badge badge-success">活跃</span>
              </div>
              <div class="token-scopes">
                <span v-for="scope in token.scopes" :key="scope" class="scope-tag">
                  {{ scope }}
                </span>
              </div>
              <div class="token-meta">
                <span>角色: {{ roleLabels[token.role] || token.role }}</span>
                <span>创建于 {{ formatDate(token.created_at) }}</span>
                <span v-if="token.expires_at">
                  到期: {{ formatDate(token.expires_at) }}
                </span>
                <span v-else>永不过期</span>
                <span v-if="token.last_used_at">
                  最后使用: {{ formatDate(token.last_used_at) }}
                </span>
              </div>
            </div>
            
            <div class="token-actions">
              <button class="btn btn-danger btn-sm" @click="revokeToken(token)">
                撤销
              </button>
            </div>
          </div>
          
          <div v-if="tokens.length === 0" class="empty-state">
            <p>暂无项目访问令牌</p>
            <p class="text-muted">创建一个令牌以允许 API 和自动化访问</p>
          </div>
        </div>
      </section>

      <!-- 注意事项 -->
      <section class="settings-section">
        <h3>安全注意事项</h3>
        
        <div class="security-notes">
          <div class="note">
            <span class="note-icon">⚠️</span>
            <div class="note-content">
              <strong>保护好你的令牌</strong>
              <p>访问令牌相当于用户名和密码，请勿分享或提交到代码仓库中。</p>
            </div>
          </div>
          
          <div class="note">
            <span class="note-icon">🔒</span>
            <div class="note-content">
              <strong>使用最小权限原则</strong>
              <p>只授予令牌必需的权限范围，避免使用过于宽泛的权限。</p>
            </div>
          </div>
          
          <div class="note">
            <span class="note-icon">⏰</span>
            <div class="note-content">
              <strong>设置过期时间</strong>
              <p>建议为令牌设置过期时间，定期轮换以降低安全风险。</p>
            </div>
          </div>
          
          <div class="note">
            <span class="note-icon">📋</span>
            <div class="note-content">
              <strong>监控令牌使用</strong>
              <p>定期检查令牌的最后使用时间，撤销不再需要的令牌。</p>
            </div>
          </div>
        </div>
      </section>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch, computed } from 'vue'
import type { Project } from '@/types'

interface ProjectAccessToken {
  id: string
  name: string
  role: 'reporter' | 'developer' | 'maintainer'
  scopes: string[]
  expires_at?: string
  created_at: string
  last_used_at?: string
}

const props = defineProps<{
  project?: Project
}>()

const loading = ref(false)
const tokens = ref<ProjectAccessToken[]>([])
const newToken = ref<string | null>(null)

const host = computed(() => window.location.host)

const minDate = computed(() => {
  const tomorrow = new Date()
  tomorrow.setDate(tomorrow.getDate() + 1)
  return tomorrow.toISOString().split('T')[0]
})

const maxDate = computed(() => {
  const oneYear = new Date()
  oneYear.setFullYear(oneYear.getFullYear() + 1)
  return oneYear.toISOString().split('T')[0]
})

const tokenForm = reactive({
  name: '',
  expires_at: '',
  role: 'developer' as 'reporter' | 'developer' | 'maintainer',
  scopes: [] as string[]
})

const roleLabels: Record<string, string> = {
  reporter: '报告者',
  developer: '开发者',
  maintainer: '维护者'
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('zh-CN')
}

function isExpired(token: ProjectAccessToken): boolean {
  if (!token.expires_at) return false
  return new Date(token.expires_at) < new Date()
}

function isExpiringSoon(token: ProjectAccessToken): boolean {
  if (!token.expires_at) return false
  const expiresAt = new Date(token.expires_at)
  const sevenDaysFromNow = new Date()
  sevenDaysFromNow.setDate(sevenDaysFromNow.getDate() + 7)
  return expiresAt > new Date() && expiresAt <= sevenDaysFromNow
}

async function loadTokens() {
  if (!props.project?.owner_name || !props.project?.name) return
  loading.value = true
  
  try {
    // 需要 API 支持
    // const path = { namespace: props.project.owner_name, project: props.project.name }
    // tokens.value = await api.projectTokens.list(path)
    tokens.value = []
  } catch (error) {
    console.error('Failed to load tokens:', error)
  } finally {
    loading.value = false
  }
}

async function createToken() {
  if (!props.project?.owner_name || !props.project?.name) return
  if (!tokenForm.name || tokenForm.scopes.length === 0) return
  
  try {
    // 需要 API 支持
    // const path = { namespace: props.project.owner_name, project: props.project.name }
    // const result = await api.projectTokens.create(path, tokenForm)
    // newToken.value = result.token
    // tokens.value.unshift(result.tokenInfo)
    
    alert('项目访问令牌功能即将实现')
    
    // 重置表单
    tokenForm.name = ''
    tokenForm.expires_at = ''
    tokenForm.role = 'developer'
    tokenForm.scopes = []
  } catch (error) {
    console.error('Failed to create token:', error)
    alert('创建令牌失败')
  }
}

async function revokeToken(token: ProjectAccessToken) {
  if (!props.project?.owner_name || !props.project?.name) return
  if (!confirm(`确定要撤销令牌 "${token.name}" 吗？此操作不可撤销。`)) return
  
  try {
    // 需要 API 支持
    // const path = { namespace: props.project.owner_name, project: props.project.name }
    // await api.projectTokens.revoke(path, token.id)
    
    tokens.value = tokens.value.filter(t => t.id !== token.id)
    alert('令牌已撤销')
  } catch (error) {
    console.error('Failed to revoke token:', error)
    alert('撤销失败')
  }
}

function copyToken() {
  if (newToken.value) {
    navigator.clipboard.writeText(newToken.value)
    alert('令牌已复制到剪贴板')
  }
}

watch([() => props.project?.owner_name, () => props.project?.name], () => {
  loadTokens()
  newToken.value = null
}, { immediate: true })
</script>

<style lang="scss" scoped>
.project-settings-page {
  padding: $spacing-lg;
  max-width: 900px;
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

.token-form {
  padding: $spacing-lg;
  background: $bg-secondary;
  border-radius: $border-radius;
}

.form-group {
  margin-bottom: $spacing-lg;
  
  label {
    display: block;
    margin-bottom: $spacing-xs;
    font-weight: 500;
    
    .required {
      color: $danger-color;
    }
  }
  
  .form-help {
    font-size: $font-size-sm;
    color: $text-muted;
    margin-top: $spacing-xs;
  }
}

.scope-checkboxes {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: $spacing-sm;
}

.checkbox-label {
  display: flex;
  align-items: flex-start;
  gap: $spacing-sm;
  padding: $spacing-sm;
  background: $bg-tertiary;
  border-radius: $border-radius;
  cursor: pointer;
  transition: background 0.2s;
  
  &:hover {
    background: darken($bg-tertiary, 5%);
  }
  
  input {
    margin-top: 3px;
  }
  
  .scope-info {
    display: flex;
    flex-direction: column;
    
    strong {
      font-family: monospace;
      font-size: $font-size-sm;
    }
    
    span {
      font-size: $font-size-xs;
      color: $text-muted;
    }
  }
}

.token-created {
  .token-display {
    padding: $spacing-lg;
    background: rgba($success-color, 0.1);
    border: 1px solid $success-color;
    border-radius: $border-radius;
    margin-bottom: $spacing-lg;
  }
  
  .token-warning {
    color: $danger-color;
    font-weight: 500;
    margin-bottom: $spacing-md;
  }
  
  .token-value {
    display: flex;
    align-items: center;
    gap: $spacing-md;
    
    code {
      flex: 1;
      font-family: monospace;
      padding: $spacing-sm $spacing-md;
      background: $bg-primary;
      border-radius: $border-radius;
      word-break: break-all;
    }
  }
}

.usage-example {
  h4 {
    margin: 0 0 $spacing-sm 0;
  }
  
  pre {
    margin: 0;
    padding: $spacing-md;
    background: $bg-secondary;
    border-radius: $border-radius;
    overflow-x: auto;
    
    code {
      font-family: monospace;
      font-size: $font-size-sm;
    }
  }
}

.token-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
}

.token-item {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: $spacing-md;
  background: $bg-secondary;
  border-radius: $border-radius;
}

.token-info {
  flex: 1;
}

.token-name {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  font-weight: 500;
  margin-bottom: $spacing-xs;
}

.token-scopes {
  display: flex;
  flex-wrap: wrap;
  gap: $spacing-xs;
  margin-bottom: $spacing-sm;
}

.scope-tag {
  font-size: $font-size-xs;
  font-family: monospace;
  padding: 2px 8px;
  background: $bg-tertiary;
  border-radius: 3px;
}

.token-meta {
  font-size: $font-size-sm;
  color: $text-muted;
  display: flex;
  flex-wrap: wrap;
  gap: $spacing-md;
}

.token-actions {
  flex-shrink: 0;
}

.badge {
  font-size: $font-size-xs;
  padding: 2px 6px;
  border-radius: 3px;
  
  &.badge-success {
    background: rgba($success-color, 0.2);
    color: $success-color;
  }
  
  &.badge-warning {
    background: rgba($warning-color, 0.2);
    color: $warning-color;
  }
  
  &.badge-danger {
    background: rgba($danger-color, 0.2);
    color: $danger-color;
  }
}

.security-notes {
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
}

.note {
  display: flex;
  gap: $spacing-md;
  padding: $spacing-md;
  background: $bg-secondary;
  border-radius: $border-radius;
  
  .note-icon {
    font-size: 24px;
    flex-shrink: 0;
  }
  
  .note-content {
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
  
  .text-muted {
    font-size: $font-size-sm;
    margin-top: $spacing-xs;
  }
}
</style>
