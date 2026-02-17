<template>
  <div class="auth-page">
    <div class="auth-container">
      <!-- Logo Section -->
      <div class="auth-logo">
        <div class="logo-icon">
          <svg width="48" height="48" viewBox="0 0 28 28" fill="none">
            <path d="M14 0L17.5 10.5H28L19.5 17L23 28L14 21.5L5 28L8.5 17L0 10.5H10.5L14 0Z" fill="currentColor"/>
          </svg>
        </div>
        <h1 class="logo-text">DevOps</h1>
      </div>
      
      <!-- Auth Card -->
      <div class="auth-card">
  、、
        <!-- Loading State -->
        <template v-if="loading">
          <div class="loading-state">
            <div class="spinner-lg"></div>
            <p>正在加载授权信息...</p>
          </div>
        </template>
        
        <!-- Error State -->
        <template v-else-if="error">
          <div class="auth-header">
            <h2>授权失败</h2>
          </div>
          
          <div class="alert alert-danger">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
              <path d="M8 5v4M8 11v.01" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <span>{{ error }}</span>
          </div>
          
          <div v-if="isAppConfigError" class="help-box">
            <p class="help-title">这通常是由于应用配置不正确导致的</p>
            <p class="help-text">请联系应用开发者检查以下配置：</p>
            <ul class="help-list">
              <li>Client ID 是否正确</li>
              <li>Redirect URI 是否已在 DevOps 中注册</li>
              <li>请求的 Scopes 是否被授权</li>
            </ul>
          </div>
          
          <button @click="goBack" class="btn btn-secondary w-full">返回</button>
        </template>
        
        <!-- Consent Screen -->
        <template v-else-if="authInfo">
          <div class="auth-header oauth-header">
            <div class="app-icon" v-if="authInfo.application.logo_url">
              <img :src="authInfo.application.logo_url" :alt="authInfo.application.name" />
            </div>
            <div class="app-icon app-icon-placeholder" v-else>
              {{ authInfo.application.name.charAt(0).toUpperCase() }}
            </div>
            <h2>{{ authInfo.application.name }}</h2>
            <p v-if="authInfo.application.description">{{ authInfo.application.description }}</p>
          </div>
          
          <div class="consent-section">
            <div class="consent-message">
              <strong>{{ authInfo.application.name }}</strong> 请求访问您的账户
              <span class="user-badge">@{{ authInfo.user.username }}</span>
            </div>
            
            <div v-if="authInfo.requested_scopes.length > 0" class="scopes-box">
              <h4>此应用将能够：</h4>
              <ul class="scopes-list">
                <li v-for="scope in authInfo.requested_scopes" :key="scope">
                  <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                    <path d="M13 4L6 11L3 8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                  <span>{{ getScopeDescription(scope) }}</span>
                </li>
              </ul>
            </div>
            
            <div class="redirect-notice">
              <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
                <path d="M8 5v4M8 11v.01" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              <span>授权后将跳转到：<code>{{ authInfo.redirect_uri }}</code></span>
            </div>
          </div>
          
          <div class="auth-actions">
            <button @click="deny" class="btn btn-secondary" :disabled="submitting">
              拒绝
            </button>
            <button @click="authorize" class="btn btn-primary" :disabled="submitting">
              <span v-if="submitting" class="spinner-sm"></span>
              {{ submitting ? '授权中...' : '授权' }}
            </button>
          </div>
          
          <div class="security-hint">
            <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
              <rect x="3" y="7" width="10" height="7" rx="1" stroke="currentColor" stroke-width="1.5"/>
              <path d="M5 7V5a3 3 0 016 0v2" stroke="currentColor" stroke-width="1.5"/>
            </svg>
            <span>授权后，您可以随时在设置中撤销此应用的访问权限</span>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { api } from '@/api'

interface OAuthAuthInfo {
  application: {
    name: string
    description: string | null
    homepage_url: string | null
    logo_url: string | null
  }
  requested_scopes: string[]
  redirect_uri: string
  state: string | null
  trusted: boolean
  user: {
    id: number
    username: string
  }
}

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()

const loading = ref(true)
const error = ref<string | null>(null)
const authInfo = ref<OAuthAuthInfo | null>(null)
const submitting = ref(false)

// OAuth request parameters
const clientId = ref('')
const redirectUri = ref('')
const responseType = ref('')
const scope = ref('')
const state = ref('')
const codeChallenge = ref('')
const codeChallengeMethod = ref('')

// Check if error is due to app configuration issues
const isAppConfigError = computed(() => {
  if (!error.value) return false
  return error.value.includes('应用存在配置问题') || 
         error.value.includes('应用管理员') ||
         error.value.includes('配置不完整')
})

const scopeDescriptions: Record<string, string> = {
  'read_user': '读取您的用户信息',
  'write_user': '修改您的用户信息',
  'api': '完整的 API 访问权限',
  'read_api': '只读 API 访问权限',
  'write_api': '读写 API 访问权限',
  'read_repository': '读取您的仓库',
  'write_repository': '写入您的仓库',
  'email': '访问您的邮箱地址',
  'openid': 'OpenID Connect 身份认证',
  'profile': '访问您的个人资料',
}

function getScopeDescription(scope: string): string {
  return scopeDescriptions[scope] || scope
}

// Convert backend technical errors to user-friendly messages
function getUserFriendlyError(err: any): string {
  const message = err.response?.data?.message || err.message || ''
  
  // Application configuration errors (should report to app admin)
  if (message.includes('Invalid client_id') || 
      message.includes('Invalid redirect_uri') || 
      message.includes('client_id') ||
      message.includes('redirect_uri') ||
      message.includes('Application not found')) {
    return '此应用存在配置问题，请联系应用管理员反馈'
  }
  
  // Scope errors
  if (message.includes('Scope') && message.includes('not allowed')) {
    return '此应用请求的权限未被授权，请联系应用管理员'
  }
  
  // Missing parameters
  if (message.includes('缺少必要') || message.includes('required')) {
    return '授权请求参数不完整，请联系应用管理员'
  }
  
  // Unsupported features
  if (message.includes('Unsupported')) {
    return '此应用使用的授权方式不受支持，请联系应用管理员'
  }
  
  // Default fallback
  return message || '授权过程中出现错误'
}

function saveOAuthParams() {
  // Save OAuth parameters to sessionStorage for after login
  const params = {
    client_id: clientId.value,
    redirect_uri: redirectUri.value,
    response_type: responseType.value,
    scope: scope.value,
    state: state.value,
    code_challenge: codeChallenge.value,
    code_challenge_method: codeChallengeMethod.value,
  }
  sessionStorage.setItem('oauth_authorize_params', JSON.stringify(params))
}

function loadOAuthParams() {
  // Load OAuth parameters from query string
  clientId.value = route.query.client_id as string || ''
  redirectUri.value = route.query.redirect_uri as string || ''
  responseType.value = route.query.response_type as string || ''
  scope.value = route.query.scope as string || ''
  state.value = route.query.state as string || ''
  codeChallenge.value = route.query.code_challenge as string || ''
  codeChallengeMethod.value = route.query.code_challenge_method as string || ''
  
  // If no params in URL, try to load from sessionStorage (after login redirect)
  if (!clientId.value) {
    const saved = sessionStorage.getItem('oauth_authorize_params')
    if (saved) {
      const params = JSON.parse(saved)
      clientId.value = params.client_id || ''
      redirectUri.value = params.redirect_uri || ''
      responseType.value = params.response_type || ''
      scope.value = params.scope || ''
      state.value = params.state || ''
      codeChallenge.value = params.code_challenge || ''
      codeChallengeMethod.value = params.code_challenge_method || ''
      sessionStorage.removeItem('oauth_authorize_params')
    }
  }
}

async function fetchAuthInfo() {
  try {
    const params = new URLSearchParams()
    params.set('client_id', clientId.value)
    params.set('redirect_uri', redirectUri.value)
    params.set('response_type', responseType.value)
    if (scope.value) params.set('scope', scope.value)
    if (state.value) params.set('state', state.value)
    if (codeChallenge.value) params.set('code_challenge', codeChallenge.value)
    if (codeChallengeMethod.value) params.set('code_challenge_method', codeChallengeMethod.value)
    
    const response = await api.client.get(`/oauth/authorize/info?${params.toString()}`)
    authInfo.value = response.data
    
    // If it's a trusted app, auto-authorize
    if (authInfo.value?.trusted) {
      await authorize()
    }
  } catch (err: any) {
    error.value = getUserFriendlyError(err)
  }
}

async function authorize() {
  submitting.value = true
  try {
    const response = await api.client.post('/oauth/authorize/confirm', {
      client_id: clientId.value,
      redirect_uri: redirectUri.value,
      response_type: responseType.value,
      scope: scope.value || undefined,
      state: state.value || undefined,
      code_challenge: codeChallenge.value || undefined,
      code_challenge_method: codeChallengeMethod.value || undefined,
    })
    
    // Redirect to the callback URL with the authorization code
    const { redirect_uri, code } = response.data
    if (redirect_uri) {
      window.location.href = redirect_uri
    } else {
      // Build redirect URL manually
      let url = redirectUri.value
      url += url.includes('?') ? '&' : '?'
      url += `code=${encodeURIComponent(code)}`
      if (state.value) {
        url += `&state=${encodeURIComponent(state.value)}`
      }
      window.location.href = url
    }
  } catch (err: any) {
    error.value = getUserFriendlyError(err)
    submitting.value = false
  }
}

function deny() {
  // Redirect back with error
  let url = redirectUri.value
  url += url.includes('?') ? '&' : '?'
  url += 'error=access_denied&error_description=The+user+denied+the+request'
  if (state.value) {
    url += `&state=${encodeURIComponent(state.value)}`
  }
  window.location.href = url
}

function goBack() {
  router.back()
}

onMounted(async () => {
  loadOAuthParams()
  
  // Validate required parameters
  if (!clientId.value || !redirectUri.value || !responseType.value) {
    error.value = '缺少必要的 OAuth 参数'
    loading.value = false
    return
  }
  
  // Check if user is logged in
  if (!authStore.isAuthenticated) {
    // Save params and redirect to login
    saveOAuthParams()
    router.push({
      name: 'Login',
      query: { redirect: route.fullPath }
    })
    return
  }
  
  // Ensure API client has the token set (important after page refresh)
  const storedToken = localStorage.getItem('token')
  if (storedToken) {
    api.setAuthToken(storedToken)
  }
  
  // Fetch authorization info
  await fetchAuthInfo()
  loading.value = false
})
</script>

<style lang="scss" scoped>
.auth-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #1f2937 0%, #111827 100%);
  padding: $spacing-6;
}

.auth-container {
  width: 100%;
  max-width: 480px;
}

.auth-logo {
  text-align: center;
  margin-bottom: $spacing-8;
  
  .logo-icon {
    width: 64px;
    height: 64px;
    margin: 0 auto $spacing-4;
    color: #e24329;
  }
  
  .logo-text {
    font-size: $font-size-3xl;
    font-weight: $font-weight-bold;
    color: $text-light;
    margin: 0;
  }
}

.auth-card {
  background: $bg-primary;
  border-radius: $border-radius-xl;
  padding: $spacing-8;
  box-shadow: $shadow-xl;
}

.auth-header {
  text-align: center;
  margin-bottom: $spacing-6;
  
  h2 {
    font-size: $font-size-2xl;
    font-weight: $font-weight-semibold;
    color: $text-primary;
    margin: 0 0 $spacing-2;
  }
  
  p {
    color: $text-secondary;
    margin: 0;
    font-size: $font-size-sm;
  }
  
  &.oauth-header {
    .app-icon {
      width: 72px;
      height: 72px;
      margin: 0 auto $spacing-4;
      border-radius: $border-radius-lg;
      overflow: hidden;
      display: flex;
      align-items: center;
      justify-content: center;
      
      img {
        width: 100%;
        height: 100%;
        object-fit: cover;
      }
      
      &-placeholder {
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
        font-size: $font-size-3xl;
        font-weight: $font-weight-bold;
      }
    }
  }
}

.loading-state {
  text-align: center;
  padding: $spacing-8 0;
  
  .spinner-lg {
    width: 48px;
    height: 48px;
    border: 3px solid $border-color;
    border-top-color: $brand-primary;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto $spacing-4;
  }
  
  p {
    color: $text-secondary;
    margin: 0;
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.alert {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-3 $spacing-4;
  margin-bottom: $spacing-5;
  border-radius: $border-radius;
  font-size: $font-size-sm;
  
  &-danger {
    background: $color-danger-light;
    color: darken($color-danger, 15%);
    border: 1px solid rgba($color-danger, 0.2);
  }
  
  svg {
    flex-shrink: 0;
  }
}

.help-box {
  background: $bg-secondary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  padding: $spacing-4;
  margin-bottom: $spacing-5;
  
  .help-title {
    font-weight: $font-weight-medium;
    color: $text-primary;
    margin: 0 0 $spacing-2;
    font-size: $font-size-sm;
  }
  
  .help-text {
    color: $text-secondary;
    margin: 0 0 $spacing-3;
    font-size: $font-size-sm;
  }
  
  .help-list {
    margin: 0;
    padding-left: $spacing-5;
    color: $text-secondary;
    font-size: $font-size-sm;
    
    li {
      margin-bottom: $spacing-2;
      
      &:last-child {
        margin-bottom: 0;
      }
    }
  }
}

.consent-section {
  margin-bottom: $spacing-6;
}

.consent-message {
  padding: $spacing-4;
  background: $bg-secondary;
  border-radius: $border-radius;
  margin-bottom: $spacing-5;
  font-size: $font-size-sm;
  color: $text-secondary;
  text-align: center;
  
  strong {
    color: $text-primary;
    font-weight: $font-weight-semibold;
  }
  
  .user-badge {
    display: inline-block;
    margin-left: $spacing-2;
    padding: $spacing-1 $spacing-2;
    background: $brand-primary;
    color: white;
    border-radius: $border-radius-sm;
    font-weight: $font-weight-medium;
    font-size: $font-size-xs;
  }
}

.scopes-box {
  background: $bg-secondary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  padding: $spacing-4;
  margin-bottom: $spacing-4;
  
  h4 {
    margin: 0 0 $spacing-3;
    font-size: $font-size-sm;
    font-weight: $font-weight-semibold;
    color: $text-primary;
  }
  
  .scopes-list {
    list-style: none;
    padding: 0;
    margin: 0;
    
    li {
      display: flex;
      align-items: center;
      gap: $spacing-2;
      padding: $spacing-2 0;
      font-size: $font-size-sm;
      color: $text-secondary;
      border-bottom: 1px solid $border-color;
      
      &:last-child {
        border-bottom: none;
        padding-bottom: 0;
      }
      
      &:first-child {
        padding-top: 0;
      }
      
      svg {
        color: $color-success;
        flex-shrink: 0;
      }
    }
  }
}

.redirect-notice {
  display: flex;
  align-items: flex-start;
  gap: $spacing-2;
  padding: $spacing-3;
  background: $bg-tertiary;
  border-radius: $border-radius;
  font-size: $font-size-xs;
  color: $text-tertiary;
  
  svg {
    flex-shrink: 0;
    margin-top: 2px;
  }
  
  code {
    word-break: break-all;
    background: $bg-secondary;
    padding: 2px 4px;
    border-radius: 3px;
    font-family: monospace;
  }
}

.auth-actions {
  display: flex;
  gap: $spacing-3;
  margin-bottom: $spacing-4;
  
  .btn {
    flex: 1;
  }
}

.security-hint {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: $spacing-2;
  padding: $spacing-3;
  font-size: $font-size-xs;
  color: $text-tertiary;
  text-align: center;
  
  svg {
    flex-shrink: 0;
  }
}

.spinner-sm {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
  margin-right: $spacing-2;
}

.w-full {
  width: 100%;
}
</style>
