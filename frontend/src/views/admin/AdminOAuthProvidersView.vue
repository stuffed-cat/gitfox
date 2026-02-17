<template>
  <div class="admin-oauth-providers">
    <div class="page-header">
      <div class="header-content">
        <h1>OAuth 提供商</h1>
        <p class="page-description">管理外部身份提供商配置，允许用户通过第三方账号登录</p>
      </div>
      <button class="btn btn-primary" @click="showAddModal = true">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        添加提供商
      </button>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>加载中...</span>
    </div>

    <template v-else>
      <!-- Provider list -->
      <div v-if="providers.length === 0" class="empty-state">
        <div class="empty-icon">
          <svg viewBox="0 0 64 64" width="64" height="64" fill="none">
            <circle cx="32" cy="32" r="28" fill="#dbeafe" stroke="#93c5fd" stroke-width="2"/>
            <path d="M24 28a4 4 0 108 0 4 4 0 00-8 0zM32 28a4 4 0 108 0 4 4 0 00-8 0z" stroke="#3b82f6" stroke-width="2"/>
            <path d="M22 40c2-4 6-6 10-6s8 2 10 6" stroke="#3b82f6" stroke-width="2" stroke-linecap="round"/>
          </svg>
        </div>
        <p class="empty-title">尚未配置任何 OAuth 提供商</p>
        <p class="empty-description">添加 OAuth 提供商允许用户通过 GitHub、Google 等账号登录</p>
        <button class="btn btn-primary" @click="showAddModal = true">添加第一个提供商</button>
      </div>

      <div v-else class="providers-list">
        <div 
          v-for="provider in providers" 
          :key="provider.id" 
          class="provider-card"
          :class="{ disabled: !provider.enabled }"
        >
          <div class="provider-icon">
            <component :is="getProviderIcon(provider.provider_type)" />
          </div>
          <div class="provider-info">
            <div class="provider-header">
              <h3>{{ provider.display_name }}</h3>
              <span class="provider-type">{{ provider.provider_type }}</span>
              <span v-if="isBuiltinProvider(provider.name)" class="badge badge-builtin" title="凭证来自 .env 配置">内置</span>
              <span v-if="!provider.enabled" class="badge badge-disabled">已禁用</span>
            </div>
            <div class="provider-meta">
              <span><strong>名称:</strong> {{ provider.name }}</span>
              <span v-if="!isBuiltinProvider(provider.name)"><strong>Client ID:</strong> {{ truncate(provider.client_id, 20) }}</span>
              <span v-else><strong>凭证:</strong> <code>.env</code></span>
            </div>
            <div class="provider-scopes">
              <span v-for="scope in provider.scopes" :key="scope" class="scope-tag">{{ scope }}</span>
            </div>
          </div>
          <div class="provider-actions">
            <button class="btn btn-icon" @click="editProvider(provider)" title="编辑">
              <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
                <path d="M11.5 2.5l2 2-8 8H3.5v-2l8-8z" stroke="currentColor" stroke-width="1.2"/>
              </svg>
            </button>
            <button 
              class="btn btn-icon" 
              @click="toggleProvider(provider)" 
              :title="provider.enabled ? '禁用' : '启用'"
            >
              <svg v-if="provider.enabled" viewBox="0 0 16 16" width="16" height="16" fill="none">
                <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.2"/>
                <path d="M6 8l2 2 3-4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <svg v-else viewBox="0 0 16 16" width="16" height="16" fill="none">
                <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.2"/>
                <path d="M6 6l4 4M10 6l-4 4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
              </svg>
            </button>
            <button class="btn btn-icon btn-danger" @click="confirmDelete(provider)" title="删除">
              <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
                <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
            </button>
          </div>
        </div>
      </div>
    </template>

    <!-- Add/Edit Modal -->
    <div v-if="showAddModal || editingProvider" class="modal-overlay" @click.self="closeModal">
      <div class="modal-content modal-large">
        <div class="modal-header">
          <h3>{{ editingProvider ? '编辑' : '添加' }} OAuth 提供商</h3>
          <button class="modal-close" @click="closeModal">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <form @submit.prevent="saveProvider">
          <div class="form-row">
            <div class="form-group">
              <label for="provider-name">提供商标识 <span class="required">*</span></label>
              <input
                id="provider-name"
                v-model="form.name"
                type="text"
                class="form-input"
                placeholder="例如: github, google"
                :disabled="!!editingProvider"
                required
              />
              <p class="form-hint">唯一标识符，仅小写字母、数字和下划线</p>
            </div>
            <div class="form-group">
              <label for="provider-display-name">显示名称 <span class="required">*</span></label>
              <input
                id="provider-display-name"
                v-model="form.display_name"
                type="text"
                class="form-input"
                placeholder="例如: GitHub"
                required
              />
            </div>
          </div>

          <div class="form-group">
            <label for="provider-type">提供商类型 <span class="required">*</span></label>
            <select id="provider-type" v-model="form.provider_type" class="form-input" required>
              <option value="">选择类型...</option>
              <option value="github">GitHub</option>
              <option value="gitlab">GitLab</option>
              <option value="google">Google</option>
              <option value="azure_ad">Azure AD / Microsoft</option>
              <option value="bitbucket">Bitbucket</option>
              <option value="oidc">OpenID Connect (通用)</option>
              <option value="oauth2">OAuth2 (通用)</option>
            </select>
            <p class="form-hint">选择类型后会自动填充默认 URL</p>
          </div>

          <!-- 内置提供商提示 -->
          <div v-if="isBuiltinProvider(form.name)" class="alert alert-info">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
              <path d="M8 5v4M8 11v.01" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <span>
              <strong>{{ form.display_name || '此提供商' }}</strong> 是内置提供商，
              Client ID 和 Secret 应在 <code>.env</code> 文件中配置
              （如 <code>OAUTH_{{ form.name?.toUpperCase() }}_CLIENT_ID</code>），
              这里的凭证配置仅作为备用。
            </span>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="client-id">
                Client ID 
                <span class="required" v-if="!isBuiltinProvider(form.name)">*</span>
              </label>
              <input
                id="client-id"
                v-model="form.client_id"
                type="text"
                class="form-input"
                :placeholder="isBuiltinProvider(form.name) ? '内置提供商使用 .env 配置' : '从 OAuth 应用获取'"
                :required="!isBuiltinProvider(form.name)"
              />
            </div>
            <div class="form-group">
              <label for="client-secret">
                Client Secret 
                <span class="required" v-if="!editingProvider && !isBuiltinProvider(form.name)">*</span>
              </label>
              <input
                id="client-secret"
                v-model="form.client_secret"
                type="password"
                class="form-input"
                :placeholder="isBuiltinProvider(form.name) ? '内置提供商使用 .env 配置' : (editingProvider ? '留空保持不变' : '从 OAuth 应用获取')"
                :required="!editingProvider && !isBuiltinProvider(form.name)"
              />
            </div>
          </div>

          <div class="form-group">
            <label for="authorization-url">授权 URL</label>
            <input
              id="authorization-url"
              v-model="form.authorization_url"
              type="url"
              class="form-input"
              placeholder="https://example.com/oauth/authorize"
            />
          </div>

          <div class="form-group">
            <label for="token-url">Token URL</label>
            <input
              id="token-url"
              v-model="form.token_url"
              type="url"
              class="form-input"
              placeholder="https://example.com/oauth/token"
            />
          </div>

          <div class="form-group">
            <label for="userinfo-url">用户信息 URL</label>
            <input
              id="userinfo-url"
              v-model="form.userinfo_url"
              type="url"
              class="form-input"
              placeholder="https://example.com/api/user"
            />
          </div>

          <div class="form-group">
            <label for="scopes">Scopes</label>
            <input
              id="scopes"
              v-model="scopesInput"
              type="text"
              class="form-input"
              placeholder="openid email profile (空格分隔)"
            />
            <p class="form-hint">请求的权限范围，多个用空格分隔</p>
          </div>

          <div class="form-group">
            <label for="icon">图标</label>
            <select id="icon" v-model="form.icon" class="form-input">
              <option value="">默认</option>
              <option value="github">GitHub</option>
              <option value="gitlab">GitLab</option>
              <option value="google">Google</option>
              <option value="microsoft">Microsoft</option>
              <option value="bitbucket">Bitbucket</option>
            </select>
          </div>

          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="form.enabled" />
              <span>启用此提供商</span>
            </label>
          </div>

          <div v-if="formError" class="alert alert-error">
            {{ formError }}
          </div>

          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" @click="closeModal">
              取消
            </button>
            <button type="submit" class="btn btn-primary" :disabled="saving">
              {{ saving ? '保存中...' : '保存' }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- Delete Confirmation Modal -->
    <div v-if="providerToDelete" class="modal-overlay" @click.self="providerToDelete = null">
      <div class="modal-content modal-danger">
        <h3>删除 OAuth 提供商</h3>
        <p>确定要删除 <strong>{{ providerToDelete.display_name }}</strong>？</p>
        <p class="warning">如果有用户通过此提供商关联了账号，将无法删除。</p>
        <div class="modal-actions">
          <button class="btn btn-secondary" @click="providerToDelete = null">取消</button>
          <button class="btn btn-danger" @click="deleteProvider(providerToDelete)">删除</button>
        </div>
      </div>
    </div>

    <!-- Success toast -->
    <Transition name="fade">
      <div v-if="successMsg" class="success-toast">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M3 8l4 4 6-8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        {{ successMsg }}
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, h, watch } from 'vue'
import api from '@/api'

interface OAuthProviderAdmin {
  id: number
  name: string
  display_name: string
  provider_type: string
  client_id: string
  authorization_url: string
  token_url: string
  userinfo_url?: string
  scopes: string[]
  icon?: string
  enabled: boolean
  sort_order: number
  created_at: string
  updated_at: string
}

const providers = ref<OAuthProviderAdmin[]>([])
const loading = ref(true)
const saving = ref(false)
const showAddModal = ref(false)
const editingProvider = ref<OAuthProviderAdmin | null>(null)
const providerToDelete = ref<OAuthProviderAdmin | null>(null)
const formError = ref('')
const successMsg = ref('')

// 内置提供商列表 - 这些提供商的 Client ID/Secret 应在 .env 中配置
const BUILTIN_PROVIDERS = ['github', 'gitlab', 'google', 'azure_ad', 'bitbucket']

const isBuiltinProvider = (name: string | undefined): boolean => {
  return name ? BUILTIN_PROVIDERS.includes(name) : false
}

const form = reactive({
  name: '',
  display_name: '',
  provider_type: '',
  client_id: '',
  client_secret: '',
  authorization_url: '',
  token_url: '',
  userinfo_url: '',
  icon: '',
  enabled: true
})

const scopesInput = ref('')

// Watch provider_type to auto-fill URLs
watch(() => form.provider_type, (type) => {
  if (editingProvider.value) return // Don't auto-fill when editing
  
  const defaults: Record<string, { auth: string; token: string; user: string; scopes: string }> = {
    github: {
      auth: 'https://github.com/login/oauth/authorize',
      token: 'https://github.com/login/oauth/access_token',
      user: 'https://api.github.com/user',
      scopes: 'user:email'
    },
    gitlab: {
      auth: 'https://gitlab.com/oauth/authorize',
      token: 'https://gitlab.com/oauth/token',
      user: 'https://gitlab.com/api/v4/user',
      scopes: 'read_user'
    },
    google: {
      auth: 'https://accounts.google.com/o/oauth2/v2/auth',
      token: 'https://oauth2.googleapis.com/token',
      user: 'https://www.googleapis.com/oauth2/v2/userinfo',
      scopes: 'openid email profile'
    },
    azure_ad: {
      auth: 'https://login.microsoftonline.com/common/oauth2/v2.0/authorize',
      token: 'https://login.microsoftonline.com/common/oauth2/v2.0/token',
      user: 'https://graph.microsoft.com/v1.0/me',
      scopes: 'openid email profile'
    },
    bitbucket: {
      auth: 'https://bitbucket.org/site/oauth2/authorize',
      token: 'https://bitbucket.org/site/oauth2/access_token',
      user: 'https://api.bitbucket.org/2.0/user',
      scopes: 'account'
    }
  }

  if (defaults[type]) {
    form.authorization_url = defaults[type].auth
    form.token_url = defaults[type].token
    form.userinfo_url = defaults[type].user
    scopesInput.value = defaults[type].scopes
    form.icon = type === 'azure_ad' ? 'microsoft' : type
  }
})

const loadProviders = async () => {
  loading.value = true
  try {
    const response = await api.client.get('/admin/oauth/providers')
    providers.value = response.data
  } catch (error: any) {
    console.error('Failed to load providers:', error)
  } finally {
    loading.value = false
  }
}

const editProvider = (provider: OAuthProviderAdmin) => {
  editingProvider.value = provider
  form.name = provider.name
  form.display_name = provider.display_name
  form.provider_type = provider.provider_type
  form.client_id = provider.client_id
  form.client_secret = ''
  form.authorization_url = provider.authorization_url
  form.token_url = provider.token_url
  form.userinfo_url = provider.userinfo_url || ''
  form.icon = provider.icon || ''
  form.enabled = provider.enabled
  scopesInput.value = provider.scopes.join(' ')
}

const saveProvider = async () => {
  saving.value = true
  formError.value = ''

  try {
    const data: any = {
      display_name: form.display_name,
      client_id: form.client_id,
      authorization_url: form.authorization_url || undefined,
      token_url: form.token_url || undefined,
      userinfo_url: form.userinfo_url || undefined,
      scopes: scopesInput.value.split(/\s+/).filter(Boolean),
      icon: form.icon || undefined,
      enabled: form.enabled
    }

    if (editingProvider.value) {
      // Update
      if (form.client_secret) {
        data.client_secret = form.client_secret
      }
      await api.client.put(`/admin/oauth/providers/${editingProvider.value.id}`, data)
      successMsg.value = '提供商已更新'
    } else {
      // Create
      data.name = form.name
      data.provider_type = form.provider_type
      data.client_secret = form.client_secret
      await api.client.post('/admin/oauth/providers', data)
      successMsg.value = '提供商已创建'
    }

    closeModal()
    await loadProviders()
    setTimeout(() => { successMsg.value = '' }, 3000)
  } catch (error: any) {
    formError.value = error.response?.data?.message || '保存失败'
  } finally {
    saving.value = false
  }
}

const toggleProvider = async (provider: OAuthProviderAdmin) => {
  try {
    await api.client.put(`/admin/oauth/providers/${provider.id}`, {
      enabled: !provider.enabled
    })
    provider.enabled = !provider.enabled
    successMsg.value = provider.enabled ? '提供商已启用' : '提供商已禁用'
    setTimeout(() => { successMsg.value = '' }, 3000)
  } catch (error: any) {
    console.error('Failed to toggle provider:', error)
  }
}

const confirmDelete = (provider: OAuthProviderAdmin) => {
  providerToDelete.value = provider
}

const deleteProvider = async (provider: OAuthProviderAdmin) => {
  try {
    await api.client.delete(`/admin/oauth/providers/${provider.id}`)
    providers.value = providers.value.filter(p => p.id !== provider.id)
    successMsg.value = '提供商已删除'
    setTimeout(() => { successMsg.value = '' }, 3000)
  } catch (error: any) {
    formError.value = error.response?.data?.message || '删除失败'
  } finally {
    providerToDelete.value = null
  }
}

const closeModal = () => {
  showAddModal.value = false
  editingProvider.value = null
  formError.value = ''
  // Reset form
  form.name = ''
  form.display_name = ''
  form.provider_type = ''
  form.client_id = ''
  form.client_secret = ''
  form.authorization_url = ''
  form.token_url = ''
  form.userinfo_url = ''
  form.icon = ''
  form.enabled = true
  scopesInput.value = ''
}

const truncate = (str: string, len: number) => {
  return str.length > len ? str.slice(0, len) + '...' : str
}

// Provider icons
const getProviderIcon = (type: string) => {
  const icons: Record<string, any> = {
    github: () => h('svg', { viewBox: '0 0 24 24', width: 24, height: 24, fill: 'currentColor' }, [
      h('path', { d: 'M12 0C5.37 0 0 5.37 0 12c0 5.3 3.44 9.8 8.2 11.38.6.11.82-.26.82-.58v-2.03c-3.34.73-4.04-1.61-4.04-1.61-.55-1.39-1.34-1.76-1.34-1.76-1.09-.75.08-.73.08-.73 1.21.08 1.85 1.24 1.85 1.24 1.07 1.84 2.81 1.31 3.5 1 .1-.78.42-1.31.76-1.61-2.67-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.13-.3-.54-1.52.11-3.18 0 0 1.01-.32 3.3 1.23a11.5 11.5 0 016 0c2.28-1.55 3.29-1.23 3.29-1.23.66 1.66.25 2.88.12 3.18.77.84 1.24 1.91 1.24 3.22 0 4.61-2.81 5.63-5.48 5.92.43.37.81 1.1.81 2.22v3.29c0 .32.21.7.82.58A12 12 0 0024 12c0-6.63-5.37-12-12-12z' })
    ]),
    gitlab: () => h('svg', { viewBox: '0 0 24 24', width: 24, height: 24, fill: 'currentColor' }, [
      h('path', { d: 'M22.65 14.39L12 22.13 1.35 14.39a.84.84 0 01-.3-.94l1.22-3.78 2.44-7.51A.42.42 0 014.82 2a.43.43 0 01.58 0 .42.42 0 01.11.18l2.44 7.49h8.1l2.44-7.51A.42.42 0 0118.6 2a.43.43 0 01.58 0 .42.42 0 01.11.18l2.44 7.51 1.22 3.78a.84.84 0 01-.3.94z' })
    ]),
    google: () => h('svg', { viewBox: '0 0 24 24', width: 24, height: 24 }, [
      h('path', { fill: '#4285F4', d: 'M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z' }),
      h('path', { fill: '#34A853', d: 'M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z' }),
      h('path', { fill: '#FBBC05', d: 'M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z' }),
      h('path', { fill: '#EA4335', d: 'M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z' })
    ]),
    microsoft: () => h('svg', { viewBox: '0 0 24 24', width: 24, height: 24 }, [
      h('rect', { x: 1, y: 1, width: 10, height: 10, fill: '#f25022' }),
      h('rect', { x: 13, y: 1, width: 10, height: 10, fill: '#7fba00' }),
      h('rect', { x: 1, y: 13, width: 10, height: 10, fill: '#00a4ef' }),
      h('rect', { x: 13, y: 13, width: 10, height: 10, fill: '#ffb900' })
    ]),
    azure_ad: () => h('svg', { viewBox: '0 0 24 24', width: 24, height: 24 }, [
      h('rect', { x: 1, y: 1, width: 10, height: 10, fill: '#f25022' }),
      h('rect', { x: 13, y: 1, width: 10, height: 10, fill: '#7fba00' }),
      h('rect', { x: 1, y: 13, width: 10, height: 10, fill: '#00a4ef' }),
      h('rect', { x: 13, y: 13, width: 10, height: 10, fill: '#ffb900' })
    ]),
    bitbucket: () => h('svg', { viewBox: '0 0 24 24', width: 24, height: 24, fill: '#2684FF' }, [
      h('path', { d: 'M.778 1.213a.77.77 0 00-.768.892l3.263 19.81c.084.5.515.868 1.022.873H19.95a.772.772 0 00.77-.646l3.27-20.03a.77.77 0 00-.768-.891zM14.52 15.53H9.522L8.17 8.466h7.561z' })
    ])
  }
  return icons[type] || icons.github
}

onMounted(() => {
  loadProviders()
})
</script>

<style lang="scss" scoped>
.admin-oauth-providers {
  padding: 24px;
  max-width: 1200px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 32px;
  
  .header-content {
    h1 {
      font-size: 24px;
      font-weight: 600;
      color: #303030;
    }
    
    .page-description {
      color: #737278;
      font-size: 14px;
    }
  }
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.15s;
  
  &.btn-primary {
    background: #1f75cb;
    color: #fff;
    border: none;
    
    &:hover {
      background: #1a65b3;
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
    padding: 8px;
    background: transparent;
    border: 1px solid #dcdcde;
    
    &:hover {
      background: #f5f5f5;
    }
    
    &.btn-danger {
      color: #dc2626;
      border-color: transparent;
      background: transparent;
      
      &:hover {
        background: #fee2e2;
      }
    }
  }
  
  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
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
    width: 24px;
    height: 24px;
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
  padding: 80px 20px;
  text-align: center;
  background: #fafafa;
  border-radius: 8px;
  border: 1px dashed #dcdcde;
  
  .empty-title {
    font-size: 18px;
    font-weight: 600;
    color: #303030;
    margin: 16px 0 8px;
  }
  
  .empty-description {
    color: #737278;
    font-size: 14px;
    margin-bottom: 24px;
  }
}

.providers-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.provider-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 20px;
  background: #fff;
  border: 1px solid #dcdcde;
  border-radius: 8px;
  transition: all 0.15s;
  
  &:hover {
    border-color: #1f75cb;
    box-shadow: 0 2px 8px rgba(31, 117, 203, 0.1);
  }
  
  &.disabled {
    opacity: 0.6;
    background: #fafafa;
  }
  
  .provider-icon {
    width: 48px;
    height: 48px;
    background: #f5f5f5;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: #303030;
  }
  
  .provider-info {
    flex: 1;
    min-width: 0;
    
    .provider-header {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-bottom: 6px;
      
      h3 {
        margin: 0;
        font-size: 16px;
        font-weight: 600;
        color: #303030;
      }
      
      .provider-type {
        background: #e8f4fd;
        color: #1f75cb;
        padding: 2px 8px;
        border-radius: 4px;
        font-size: 11px;
        font-weight: 500;
      }
    }
    
    .provider-meta {
      font-size: 13px;
      color: #737278;
      display: flex;
      gap: 16px;
      margin-bottom: 8px;
      
      code {
        background: #f0f0f0;
        padding: 1px 6px;
        border-radius: 3px;
        font-family: monospace;
        font-size: 12px;
        color: #1e40af;
      }
    }
    
    .provider-scopes {
      display: flex;
      flex-wrap: wrap;
      gap: 4px;
      
      .scope-tag {
        background: #f5f5f5;
        color: #303030;
        padding: 2px 8px;
        border-radius: 4px;
        font-size: 11px;
      }
    }
  }
  
  .provider-actions {
    display: flex;
    gap: 8px;
  }
}

.badge-disabled {
  background: #fee2e2;
  color: #dc2626;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
}

.badge-builtin {
  background: #dbeafe;
  color: #1e40af;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  cursor: help;
}

// Modal styles
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
  margin-bottom: 24px;
  
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

.form-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.form-group {
  margin-bottom: 16px;
  
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
    
    &:disabled {
      background: #f5f5f5;
      cursor: not-allowed;
    }
  }
  
  .form-hint {
    font-size: 12px;
    color: #737278;
    margin-top: 4px;
  }
  
  &.checkbox-group {
    .checkbox-label {
      display: flex;
      align-items: center;
      gap: 8px;
      cursor: pointer;
      
      input[type="checkbox"] {
        accent-color: #1f75cb;
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

.alert-info {
  background: #dbeafe;
  color: #1e40af;
  padding: 12px 16px;
  border-radius: 6px;
  font-size: 13px;
  margin: 12px 0 16px 0;
  display: flex;
  align-items: flex-start;
  gap: 10px;
  line-height: 1.5;
  
  svg {
    flex-shrink: 0;
    margin-top: 2px;
    color: #3b82f6;
  }
  
  code {
    background: rgba(59, 130, 246, 0.15);
    padding: 1px 4px;
    border-radius: 3px;
    font-family: monospace;
    font-size: 12px;
  }
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
}

// Success toast
.success-toast {
  position: fixed;
  bottom: 24px;
  right: 24px;
  display: flex;
  align-items: center;
  gap: 8px;
  background: #10b981;
  color: #fff;
  padding: 12px 20px;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  z-index: 1001;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s, transform 0.3s;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(10px);
}
</style>
