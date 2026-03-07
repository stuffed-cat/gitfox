<template>
  <div class="pat-page">
    <!-- 面包屑导航 -->
    <div class="breadcrumb">
      <router-link to="/-/profile">用户设置</router-link>
      <span class="separator">/</span>
      <span>访问令牌</span>
    </div>

    <!-- 搜索框 -->
    <div class="search-box">
      <svg class="search-icon" viewBox="0 0 16 16" width="16" height="16" fill="none">
        <path d="M11.5 7a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0ZM10.5 11a5.5 5.5 0 1 1 1-1l3 3a.75.75 0 1 1-1 1l-3-3Z" stroke="currentColor" stroke-width="1.2"/>
      </svg>
      <input type="text" placeholder="搜索设置" v-model="searchQuery" />
    </div>

    <!-- 标题和描述 -->
    <section class="page-section">
      <h1>个人访问令牌</h1>
      <p class="section-description">
        个人访问令牌用于通过 API 或 Git 访问 GitFox。令牌可以设置有效期和权限范围。
        <br>
        <strong>注意：</strong>令牌只会在创建时显示一次，请妥善保存。
      </p>
    </section>

    <!-- 令牌列表 -->
    <div class="tokens-card">
      <div class="tokens-header">
        <div class="tokens-title">
          <span>您的访问令牌</span>
          <svg class="token-icon" viewBox="0 0 16 16" width="16" height="16" fill="none">
            <path d="M8 2a2 2 0 00-2 2v2H4a2 2 0 00-2 2v4a2 2 0 002 2h8a2 2 0 002-2V8a2 2 0 00-2-2h-2V4a2 2 0 00-2-2z" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <span class="tokens-count">{{ tokens.length }}</span>
        </div>
        <button class="btn btn-add" @click="showAddModal = true">
          创建新令牌
        </button>
      </div>

      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <span>加载中...</span>
      </div>

      <div v-else-if="tokens.length === 0" class="empty-state">
        <div class="empty-icon">
          <svg viewBox="0 0 64 64" width="64" height="64" fill="none">
            <circle cx="32" cy="32" r="28" fill="#e9d8fd" stroke="#c4b5fd" stroke-width="2"/>
            <rect x="24" y="18" width="16" height="10" rx="2" fill="#8b5cf6"/>
            <rect x="24" y="32" width="16" height="14" rx="2" fill="#8b5cf6"/>
            <rect x="28" y="38" width="8" height="4" rx="1" fill="#c4b5fd"/>
          </svg>
        </div>
        <p class="empty-title">您还没有创建任何访问令牌</p>
        <button class="btn btn-primary" @click="showAddModal = true">创建第一个令牌</button>
      </div>

      <div v-else class="tokens-list">
        <div v-for="token in tokens" :key="token.id" class="token-item" :class="{ revoked: token.revoked }">
          <div class="token-icon-wrapper">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M8 2a2 2 0 00-2 2v2H4a2 2 0 00-2 2v4a2 2 0 002 2h8a2 2 0 002-2V8a2 2 0 00-2-2h-2V4a2 2 0 00-2-2z" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <div class="token-info">
            <div class="token-title">
              {{ token.name }}
              <span v-if="token.revoked" class="badge badge-revoked">已撤销</span>
              <span v-else-if="isExpired(token)" class="badge badge-expired">已过期</span>
            </div>
            <div class="token-scopes">
              <span v-for="scope in token.scopes" :key="scope" class="scope-badge">{{ getScopeLabel(scope) }}</span>
            </div>
            <div class="token-meta">
              <span class="token-prefix">前缀: {{ token.token_prefix }}</span>
              <span v-if="token.last_used_at">· 最后使用: {{ formatDate(token.last_used_at) }}</span>
              <span v-else class="never-used">· 从未使用</span>
              <span v-if="token.expires_at">· 过期: {{ formatDate(token.expires_at) }}</span>
              <span v-else>· 永不过期</span>
              <span>· 创建于: {{ formatDate(token.created_at) }}</span>
            </div>
          </div>
          <button 
            v-if="!token.revoked" 
            class="btn btn-danger-outline" 
            @click="confirmRevoke(token)"
          >
            撤销
          </button>
        </div>
      </div>
    </div>

    <!-- 创建令牌弹窗 -->
    <div v-if="showAddModal" class="modal-overlay" @click.self="closeAddModal">
      <div class="modal-content modal-large">
        <div class="modal-header">
          <h3>创建个人访问令牌</h3>
          <button class="modal-close" @click="closeAddModal">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <!-- 创建成功显示令牌 -->
        <div v-if="createdToken" class="token-created">
          <div class="success-icon">
            <svg viewBox="0 0 24 24" width="48" height="48" fill="none">
              <circle cx="12" cy="12" r="10" stroke="#10b981" stroke-width="2"/>
              <path d="M8 12l3 3 5-6" stroke="#10b981" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <p class="success-title">令牌创建成功！</p>
          <p class="success-hint">请复制下面的令牌，它只会显示这一次：</p>
          <div class="token-display">
            <code>{{ createdToken.token }}</code>
            <button class="btn btn-icon" @click="copyToken" :title="copied ? '已复制' : '复制'">
              <svg v-if="!copied" viewBox="0 0 16 16" width="16" height="16" fill="none">
                <rect x="4" y="4" width="10" height="10" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <path d="M2 10V3a1 1 0 011-1h7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              <svg v-else viewBox="0 0 16 16" width="16" height="16" fill="none">
                <path d="M3 8l4 4 6-7" stroke="#10b981" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>
          </div>
          <div class="modal-actions">
            <button class="btn btn-primary" @click="closeAddModal">完成</button>
          </div>
        </div>

        <!-- 创建表单 -->
        <form v-else @submit.prevent="createToken">
          <div class="form-group">
            <label for="token-name">令牌名称 <span class="required">*</span></label>
            <input
              id="token-name"
              v-model="newToken.name"
              type="text"
              class="form-input"
              placeholder="例如：CI/CD Pipeline"
              required
            />
            <p class="form-hint">用于标识令牌用途的名称</p>
          </div>

          <div class="form-group">
            <label>有效期</label>
            <div class="expiry-options">
              <label class="radio-option">
                <input type="radio" v-model="expiryOption" value="30" />
                <span>30 天</span>
              </label>
              <label class="radio-option">
                <input type="radio" v-model="expiryOption" value="60" />
                <span>60 天</span>
              </label>
              <label class="radio-option">
                <input type="radio" v-model="expiryOption" value="90" />
                <span>90 天</span>
              </label>
              <label class="radio-option">
                <input type="radio" v-model="expiryOption" value="365" />
                <span>1 年</span>
              </label>
              <label class="radio-option">
                <input type="radio" v-model="expiryOption" value="never" />
                <span>永不过期</span>
              </label>
            </div>
          </div>

          <div class="form-group">
            <label>权限范围 <span class="required">*</span></label>
            <p class="form-hint scope-hint">选择此令牌可以访问的功能</p>
            <div class="scopes-grid">
              <label v-for="scope in availableScopes" :key="scope.name" class="scope-option">
                <input 
                  type="checkbox" 
                  :value="scope.name" 
                  v-model="newToken.scopes"
                />
                <span class="scope-label">
                  <strong>{{ scope.name }}</strong>
                  <small>{{ scope.description }}</small>
                </span>
              </label>
            </div>
          </div>

          <div v-if="addError" class="alert alert-error">
            {{ addError }}
          </div>

          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" @click="closeAddModal">
              取消
            </button>
            <button 
              type="submit" 
              class="btn btn-primary" 
              :disabled="creating || newToken.scopes.length === 0"
            >
              {{ creating ? '创建中...' : '创建令牌' }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 撤销确认弹窗 -->
    <div v-if="tokenToRevoke" class="modal-overlay" @click.self="tokenToRevoke = null">
      <div class="modal-content modal-danger">
        <h3>撤销访问令牌</h3>
        <p>确定要撤销令牌 <strong>"{{ tokenToRevoke.name }}"</strong>？</p>
        <p class="warning">撤销后，使用此令牌的所有应用将无法访问 GitFox。此操作不可逆。</p>
        <div class="modal-actions">
          <button class="btn btn-secondary" @click="tokenToRevoke = null">取消</button>
          <button class="btn btn-danger" @click="revokeToken(tokenToRevoke)">撤销令牌</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import api from '@/api'
import type { PersonalAccessToken, PatScope, PatScopeInfo, CreatePatResponse } from '@/types'

const searchQuery = ref('')
const tokens = ref<PersonalAccessToken[]>([])
const availableScopes = ref<PatScopeInfo[]>([])
const loading = ref(true)
const creating = ref(false)
const addError = ref('')
const tokenToRevoke = ref<PersonalAccessToken | null>(null)
const showAddModal = ref(false)
const createdToken = ref<CreatePatResponse | null>(null)
const copied = ref(false)
const expiryOption = ref('90')

const newToken = ref({
  name: '',
  scopes: [] as PatScope[]
})

const scopeLabels: Record<string, string> = {
  'read_api': '读取 API',
  'write_api': '写入 API',
  'read_repository': '读取仓库',
  'write_repository': '写入仓库',
  'read_user': '读取用户',
  'write_user': '写入用户',
  'read_registry': '读取注册表',
  'write_registry': '写入注册表',
  'admin': '管理员'
}

const getScopeLabel = (scope: string) => scopeLabels[scope] || scope

const isExpired = (token: PersonalAccessToken) => {
  if (!token.expires_at) return false
  return new Date(token.expires_at) < new Date()
}

const loadTokens = async () => {
  loading.value = true
  try {
    const [tokenList, scopes] = await Promise.all([
      api.accessTokens.list(),
      api.accessTokens.getScopes()
    ])
    tokens.value = tokenList
    availableScopes.value = scopes
  } catch (error: any) {
    console.error('Failed to load tokens:', error)
  } finally {
    loading.value = false
  }
}

const createToken = async () => {
  creating.value = true
  addError.value = ''

  try {
    const expiresInDays = expiryOption.value === 'never' ? undefined : parseInt(expiryOption.value)
    const response = await api.accessTokens.create({
      name: newToken.value.name,
      scopes: newToken.value.scopes,
      expires_in_days: expiresInDays
    })
    createdToken.value = response
    // Reload list to show new token
    await loadTokens()
  } catch (error: any) {
    addError.value = error.response?.data?.message || '创建令牌失败'
  } finally {
    creating.value = false
  }
}

const closeAddModal = () => {
  showAddModal.value = false
  createdToken.value = null
  newToken.value = { name: '', scopes: [] }
  expiryOption.value = '90'
  addError.value = ''
  copied.value = false
}

const copyToken = async () => {
  if (!createdToken.value) return
  try {
    await navigator.clipboard.writeText(createdToken.value.token)
    copied.value = true
    setTimeout(() => { copied.value = false }, 2000)
  } catch (e) {
    console.error('Failed to copy:', e)
  }
}

const confirmRevoke = (token: PersonalAccessToken) => {
  tokenToRevoke.value = token
}

const revokeToken = async (token: PersonalAccessToken) => {
  try {
    await api.accessTokens.revoke(token.id)
    await loadTokens()
  } catch (error: any) {
    console.error('Failed to revoke token:', error)
  } finally {
    tokenToRevoke.value = null
  }
}

const formatDate = (dateStr: string) => {
  const date = new Date(dateStr)
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'short',
    day: 'numeric'
  })
}

onMounted(() => {
  loadTokens()
})
</script>

<style lang="scss" scoped>
.pat-page {
  padding: 24px 40px;
  max-width: 1000px;
  background: #fff;
  min-height: 100vh;
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  margin-bottom: 24px;
  color: #737278;
  
  a {
    color: #1f75cb;
    text-decoration: none;
    
    &:hover {
      text-decoration: underline;
    }
  }
  
  .separator {
    color: #737278;
  }
  
  span:last-child {
    color: #303030;
  }
}

.search-box {
  position: relative;
  margin-bottom: 32px;
  
  .search-icon {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: #737278;
  }
  
  input {
    width: 100%;
    padding: 10px 12px 10px 40px;
    font-size: 14px;
    color: #303030;
    background: #fff;
    border: 1px solid #dcdcde;
    border-radius: 4px;
    
    &:focus {
      outline: none;
      border-color: #1f75cb;
      box-shadow: 0 0 0 3px rgba(31, 117, 203, 0.15);
    }
    
    &::placeholder {
      color: #737278;
    }
  }
}

.page-section {
  margin-bottom: 32px;
  
  h1 {
    font-size: 24px;
    font-weight: 600;
    color: #303030;
    margin: 0 0 8px 0;
  }
  
  .section-description {
    color: #737278;
    font-size: 14px;
    line-height: 1.6;
  }
}

.tokens-card {
  background: #fff;
  border: 1px solid #dcdcde;
  border-radius: 8px;
  overflow: hidden;
}

.tokens-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  background: #fafafa;
  border-bottom: 1px solid #dcdcde;
  
  .tokens-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 500;
    color: #303030;
    
    .token-icon {
      color: #737278;
    }
    
    .tokens-count {
      background: #e0e0e0;
      color: #303030;
      padding: 2px 8px;
      border-radius: 10px;
      font-size: 12px;
    }
  }
}

.btn-add {
  background: #1f75cb;
  color: #fff;
  border: none;
  padding: 8px 16px;
  border-radius: 4px;
  font-size: 14px;
  cursor: pointer;
  
  &:hover {
    background: #1a65b3;
  }
}

.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 60px 20px;
  color: #737278;
  
  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid #e0e0e0;
    border-top-color: #1f75cb;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 60px 20px;
  text-align: center;
  
  .empty-title {
    color: #303030;
    font-size: 16px;
    margin: 16px 0;
  }
}

.tokens-list {
  .token-item {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    padding: 16px 20px;
    border-bottom: 1px solid #ececec;
    
    &:last-child {
      border-bottom: none;
    }
    
    &.revoked {
      opacity: 0.6;
      background: #fafafa;
    }
    
    .token-icon-wrapper {
      width: 36px;
      height: 36px;
      background: #f0e6fa;
      border-radius: 6px;
      display: flex;
      align-items: center;
      justify-content: center;
      color: #8b5cf6;
      flex-shrink: 0;
    }
    
    .token-info {
      flex: 1;
      min-width: 0;
      
      .token-title {
        font-weight: 500;
        color: #303030;
        display: flex;
        align-items: center;
        gap: 8px;
        margin-bottom: 6px;
      }
      
      .token-scopes {
        display: flex;
        flex-wrap: wrap;
        gap: 4px;
        margin-bottom: 8px;
        
        .scope-badge {
          background: #e8f4fd;
          color: #1f75cb;
          padding: 2px 8px;
          border-radius: 4px;
          font-size: 11px;
        }
      }
      
      .token-meta {
        font-size: 12px;
        color: #737278;
        
        .token-prefix {
          font-family: monospace;
          background: #f5f5f5;
          padding: 1px 4px;
          border-radius: 2px;
        }
        
        .never-used {
          color: #999;
        }
      }
    }
  }
}

.badge {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 500;
  
  &.badge-revoked {
    background: #fee2e2;
    color: #dc2626;
  }
  
  &.badge-expired {
    background: #fef3c7;
    color: #d97706;
  }
}

.btn-danger-outline {
  background: transparent;
  color: #dc2626;
  border: 1px solid #dc2626;
  padding: 6px 12px;
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
  
  &:hover {
    background: #fee2e2;
  }
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

.modal-content {
  background: #fff;
  border-radius: 8px;
  width: 90%;
  max-width: 480px;
  max-height: 90vh;
  overflow-y: auto;
  padding: 24px;
  
  &.modal-large {
    max-width: 600px;
  }
  
  &.modal-danger {
    h3 {
      color: #dc2626;
    }
    
    .warning {
      color: #dc2626;
      font-size: 13px;
      margin-top: 8px;
    }
  }
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
  
  h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: #303030;
  }
  
  .modal-close {
    background: none;
    border: none;
    color: #737278;
    cursor: pointer;
    padding: 4px;
    
    &:hover {
      color: #303030;
    }
  }
}

.form-group {
  margin-bottom: 20px;
  
  label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    color: #303030;
    margin-bottom: 6px;
    
    .required {
      color: #dc2626;
    }
  }
  
  .form-input {
    width: 100%;
    padding: 10px 12px;
    font-size: 14px;
    border: 1px solid #dcdcde;
    border-radius: 4px;
    
    &:focus {
      outline: none;
      border-color: #1f75cb;
      box-shadow: 0 0 0 3px rgba(31, 117, 203, 0.15);
    }
  }
  
  .form-hint {
    font-size: 12px;
    color: #737278;
    margin-top: 4px;
    
    &.scope-hint {
      margin-bottom: 12px;
    }
  }
}

.expiry-options {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  
  .radio-option {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font-size: 14px;
    
    input[type="radio"] {
      accent-color: #1f75cb;
    }
  }
}

.scopes-grid {
  display: grid;
  gap: 8px;
  
  .scope-option {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px;
    border: 1px solid #dcdcde;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s;
    
    &:hover {
      border-color: #1f75cb;
      background: #f8fafc;
    }
    
    &:has(input:checked) {
      border-color: #1f75cb;
      background: #e8f4fd;
    }
    
    input[type="checkbox"] {
      margin-top: 2px;
      accent-color: #1f75cb;
    }
    
    .scope-label {
      display: flex;
      flex-direction: column;
      gap: 2px;
      
      strong {
        font-size: 14px;
        color: #303030;
      }
      
      small {
        font-size: 12px;
        color: #737278;
      }
    }
  }
}

.alert-error {
  background: #fee2e2;
  color: #dc2626;
  padding: 12px;
  border-radius: 4px;
  font-size: 14px;
  margin-bottom: 16px;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
}

.btn {
  padding: 10px 20px;
  border-radius: 4px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.15s;
  
  &.btn-primary {
    background: #1f75cb;
    color: #fff;
    border: none;
    
    &:hover:not(:disabled) {
      background: #1a65b3;
    }
    
    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }
  
  &.btn-secondary {
    background: #fff;
    color: #303030;
    border: 1px solid #dcdcde;
    
    &:hover {
      background: #f5f5f5;
    }
  }
  
  &.btn-danger {
    background: #dc2626;
    color: #fff;
    border: none;
    
    &:hover {
      background: #b91c1c;
    }
  }
  
  &.btn-icon {
    background: transparent;
    border: 1px solid #dcdcde;
    padding: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    
    &:hover {
      background: #f5f5f5;
    }
  }
}

// Token created success state
.token-created {
  text-align: center;
  
  .success-icon {
    margin-bottom: 16px;
  }
  
  .success-title {
    font-size: 18px;
    font-weight: 600;
    color: #10b981;
    margin: 0 0 8px 0;
  }
  
  .success-hint {
    font-size: 14px;
    color: #737278;
    margin: 0 0 16px 0;
  }
  
  .token-display {
    display: flex;
    align-items: center;
    gap: 8px;
    background: #f5f5f5;
    border: 1px solid #dcdcde;
    border-radius: 6px;
    padding: 12px 16px;
    
    code {
      flex: 1;
      font-family: monospace;
      font-size: 14px;
      word-break: break-all;
      color: #303030;
    }
  }
}
</style>
