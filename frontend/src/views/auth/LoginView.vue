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
        <div class="auth-header">
          <h2>登录到 DevOps</h2>
        </div>
        
        <form @submit.prevent="handleSubmit" class="auth-form">
          <div v-if="error" class="alert alert-danger">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
              <path d="M8 5v4M8 11v.01" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <span>{{ error }}</span>
          </div>
          
          <div class="form-group">
            <label class="form-label" for="username">用户名或邮箱</label>
            <div class="input-wrapper">
              <svg class="input-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="5" r="3" stroke="currentColor" stroke-width="1.5"/>
                <path d="M2 14a6 6 0 0112 0" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              <input
                id="username"
                v-model="form.username"
                type="text"
                class="form-control with-icon"
                placeholder="请输入用户名或邮箱"
                required
                autocomplete="username"
              />
            </div>
          </div>
          
          <div class="form-group">
            <div class="label-row">
              <label class="form-label" for="password">密码</label>
              <router-link to="/forgot-password" class="forgot-link">忘记密码？</router-link>
            </div>
            <div class="input-wrapper">
              <svg class="input-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
                <rect x="3" y="7" width="10" height="7" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <path d="M5 7V5a3 3 0 016 0v2" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <input
                id="password"
                v-model="form.password"
                :type="showPassword ? 'text' : 'password'"
                class="form-control with-icon"
                placeholder="请输入密码"
                required
                autocomplete="current-password"
              />
              <button type="button" class="toggle-password" @click="showPassword = !showPassword">
                <svg v-if="!showPassword" width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <path d="M1 8s2.5-5 7-5 7 5 7 5-2.5 5-7 5-7-5-7-5z" stroke="currentColor" stroke-width="1.5"/>
                  <circle cx="8" cy="8" r="2" stroke="currentColor" stroke-width="1.5"/>
                </svg>
                <svg v-else width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <path d="M2 2l12 12M6.5 6.5A2 2 0 109.5 9.5M1 8s2.5-5 7-5c1.5 0 2.7.5 3.7 1.2M15 8s-1.2 2.5-3.3 3.8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                </svg>
              </button>
            </div>
          </div>
          
          <div class="form-group">
            <label class="checkbox-wrapper">
              <input type="checkbox" v-model="rememberMe" />
              <span class="checkbox-label">记住我</span>
            </label>
          </div>
          
          <button type="submit" class="btn btn-primary btn-lg w-full" :disabled="loading">
            <span v-if="loading" class="spinner-sm"></span>
            {{ loading ? '登录中...' : '登录' }}
          </button>
          
          <!-- Passkey 直接登录按钮 - 与登录按钮并列 -->
          <button 
            v-if="supportsPasskey"
            type="button" 
            class="btn btn-passkey btn-lg w-full"
            @click="passkeyDirectLogin"
            :disabled="passkeyLoading"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M14 2l-1 1m-4 4a3 3 0 11-4.243 4.243A3 3 0 019 7zm0 0L11 5m0 0l2 2 2-2-2-2m-2 2l1-1" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <span v-if="passkeyLoading">验证中...</span>
            <span v-else>Passkey</span>
          </button>
        </form>
        
        <div class="auth-divider" v-if="oauthProviders.length > 0">
          <span>或</span>
        </div>
        
        <div class="social-login" v-if="oauthProviders.length > 0">
          <button 
            v-for="provider in oauthProviders" 
            :key="provider.name"
            type="button" 
            class="btn btn-social"
            @click="startOAuthLogin(provider)"
          >
            <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
              <path :d="getProviderIcon(provider)" :fill="getProviderColor(provider)"/>
            </svg>
            使用 {{ provider.display_name }} 登录
          </button>
        </div>
      </div>
      
      <!-- Two-Factor Authentication Modal -->
      <div v-if="showTwoFactorModal" class="modal-overlay" @click.self="closeTwoFactorModal">
        <div class="modal-content twofa-modal">
          <div class="modal-header">
            <h3>双因素认证</h3>
            <button class="modal-close" @click="closeTwoFactorModal">
              <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
                <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
            </button>
          </div>

          <div class="modal-body">
            <div v-if="twoFactorError" class="alert alert-danger">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
                <path d="M8 5v4M8 11v.01" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              <span>{{ twoFactorError }}</span>
            </div>

            <!-- Method Selection -->
            <div class="method-tabs">
              <button
                v-if="availableMethods.includes('totp')"
                :class="['method-tab', { active: twoFactorMethod === 'totp' }]"
                @click="twoFactorMethod = 'totp'; twoFactorCode = ''; twoFactorError = ''"
              >
                <svg width="20" height="20" viewBox="0 0 16 16" fill="none">
                  <rect x="4" y="2" width="8" height="12" rx="1.5" stroke="currentColor" stroke-width="1.2"/>
                  <path d="M6 5h4M6 7h4M6 9h4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                </svg>
                身份验证器
              </button>
              <button
                v-if="availableMethods.includes('webauthn')"
                :class="['method-tab', { active: twoFactorMethod === 'webauthn' }]"
                @click="selectWebAuthnMethod"
              >
                <svg width="20" height="20" viewBox="0 0 16 16" fill="none">
                  <path d="M8 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM4 14c0-2.21 1.79-4 4-4s4 1.79 4 4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                </svg>
                Passkey
              </button>
              <button
                v-if="availableMethods.includes('recovery')"
                :class="['method-tab', { active: twoFactorMethod === 'recovery' }]"
                @click="twoFactorMethod = 'recovery'; twoFactorCode = ''; twoFactorError = ''"
              >
                <svg width="20" height="20" viewBox="0 0 16 16" fill="none">
                  <rect x="3" y="5" width="10" height="9" rx="1.5" stroke="currentColor" stroke-width="1.2"/>
                  <path d="M5 5V4a3 3 0 0 1 6 0v1" stroke="currentColor" stroke-width="1.2"/>
                  <circle cx="8" cy="9.5" r="1" fill="currentColor"/>
                </svg>
                恢复代码
              </button>
            </div>

            <!-- TOTP Input -->
            <div v-if="twoFactorMethod === 'totp'" class="twofa-input-group">
              <p class="input-description">请输入身份验证器应用中的 6 位验证码</p>
              <input
                v-model="twoFactorCode"
                type="text"
                class="twofa-code-input"
                placeholder="000000"
                maxlength="6"
                pattern="[0-9]{6}"
                autofocus
                @keyup.enter="verifyTwoFactor"
              />
            </div>

            <!-- Recovery Code Input -->
            <div v-if="twoFactorMethod === 'recovery'" class="twofa-input-group">
              <p class="input-description">请输入您的恢复代码</p>
              <input
                v-model="twoFactorCode"
                type="text"
                class="twofa-code-input recovery"
                placeholder="XXXXXXXXXX"
                maxlength="12"
                autofocus
                @keyup.enter="verifyTwoFactor"
              />
            </div>

            <!-- WebAuthn -->
            <div v-if="twoFactorMethod === 'webauthn'" class="twofa-input-group">
              <p class="input-description">点击"验证"按钮，然后使用您的安全密钥、生物识别或设备密码进行认证</p>
              <div class="webauthn-indicator">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none">
                  <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 3c1.66 0 3 1.34 3 3s-1.34 3-3 3-3-1.34-3-3 1.34-3 3-3zm0 14.2a7.2 7.2 0 01-6-3.22c.03-1.99 4-3.08 6-3.08 1.99 0 5.97 1.09 6 3.08a7.2 7.2 0 01-6 3.22z" fill="currentColor" opacity="0.3"/>
                </svg>
                <p class="webauthn-ready">准备就绪</p>
              </div>
            </div>
          </div>

          <div class="modal-footer">
            <button class="btn btn-secondary" @click="closeTwoFactorModal">取消</button>
            <button
              class="btn btn-primary"
              @click="verifyTwoFactor"
              :disabled="(twoFactorMethod !== 'webauthn' && !twoFactorCode) || twoFactorLoading"
            >
              <span v-if="twoFactorLoading" class="spinner-sm"></span>
              {{ twoFactorLoading ? '验证中...' : '验证' }}
            </button>
          </div>
        </div>
      </div>
      
      <div class="auth-footer">
        还没有账户？<router-link to="/register">立即注册</router-link>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import api from '@/api'
import type { OAuthProviderInfo } from '@/types'

const router = useRouter()
const authStore = useAuthStore()

// 安全的 redirect 路径验证（防止 Open Redirect 攻击）
function isSafeRedirect(path: string): boolean {
  if (!path || typeof path !== 'string') return false
  
  // 只允许内部路径（以 / 开头，不包含协议）
  if (!path.startsWith('/')) return false
  if (path.includes('://')) return false
  if (path.startsWith('//')) return false // 防止 protocol-relative URL
  
  return true
}

// 登录后安全跳转
function redirectAfterLogin() {
  const savedRedirect = sessionStorage.getItem('login_redirect')
  
  if (savedRedirect && isSafeRedirect(savedRedirect)) {
    sessionStorage.removeItem('login_redirect')
    router.push(savedRedirect)
  } else {
    router.push('/')
  }
}

// Base64URL 解码辅助函数
function base64UrlDecode(base64url: string): Uint8Array {
  if (!base64url) {
    throw new Error('base64url is undefined or empty')
  }
  const base64 = base64url.replace(/-/g, '+').replace(/_/g, '/')
  const padded = base64.padEnd(base64.length + (4 - base64.length % 4) % 4, '=')
  const binary = atob(padded)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes
}

// 转换 WebAuthn 认证 options（Base64 → ArrayBuffer）
function convertWebAuthnAuthOptions(options: any): PublicKeyCredentialRequestOptions {
  console.log('Converting WebAuthn auth options:', options)
  
  // webauthn-rs 返回的格式可能直接是 publicKey 内容，或者包含 publicKey 字段
  const publicKey = options.publicKey || options
  
  return {
    ...publicKey,
    challenge: typeof publicKey.challenge === 'string'
      ? base64UrlDecode(publicKey.challenge)
      : publicKey.challenge,
    allowCredentials: publicKey.allowCredentials?.map((cred: any) => ({
      ...cred,
      id: typeof cred.id === 'string' ? base64UrlDecode(cred.id) : cred.id
    })) || []
  }
}

const form = reactive({
  username: '',
  password: ''
})
const loading = ref(false)
const error = ref('')
const showPassword = ref(false)
const rememberMe = ref(false)
const oauthProviders = ref<OAuthProviderInfo[]>([])
const oauthLoading = ref(false)

onMounted(async () => {
  // Load available OAuth providers
  try {
    oauthLoading.value = true
    oauthProviders.value = await api.oauth.getProviders()
  } catch {
    // OAuth providers not available, silent fail
    console.log('OAuth providers not configured')
  } finally {
    oauthLoading.value = false
  }
})

function startOAuthLogin(provider: OAuthProviderInfo) {
  // OAuth redirect 已由 router beforeEach 处理，存储在 sessionStorage 中
  // Redirect to OAuth authorization
  window.location.href = `/api/v1/oauth/${provider.name}/authorize`
}

function getProviderIcon(provider: OAuthProviderInfo) {
  const icons: Record<string, string> = {
    github: 'M9 0C4.03 0 0 4.03 0 9c0 3.98 2.58 7.35 6.16 8.54.45.08.62-.2.62-.43v-1.52c-2.5.54-3.03-1.2-3.03-1.2-.41-1.04-1-1.32-1-1.32-.82-.56.06-.55.06-.55.9.06 1.38.93 1.38.93.8 1.37 2.1.98 2.62.75.08-.58.31-.98.57-1.2-2-.23-4.1-1-4.1-4.45 0-.98.35-1.78.93-2.41-.09-.23-.4-1.14.09-2.38 0 0 .76-.24 2.48.92a8.64 8.64 0 014.52 0c1.72-1.16 2.48-.92 2.48-.92.49 1.24.18 2.15.09 2.38.58.63.92 1.43.92 2.41 0 3.46-2.1 4.22-4.11 4.44.32.28.61.83.61 1.67v2.47c0 .24.16.52.62.43A9 9 0 0018 9c0-4.97-4.03-9-9-9z',
    gitlab: 'M9 17.93L11.43 10.5H6.57L9 17.93zM1.26 10.5l-.95 2.92c-.09.27 0 .57.23.74L9 17.93 1.26 10.5zM1.26 10.5h5.31L4.3 3.8a.27.27 0 00-.52 0L1.26 10.5zM16.74 10.5l.95 2.92c.09.27 0 .57-.23.74L9 17.93l7.74-7.43zM16.74 10.5h-5.31l2.27-6.7a.27.27 0 01.52 0l2.52 6.7z',
    google: 'M17.64 9.2c0-.64-.06-1.25-.16-1.84H9v3.48h4.84a4.14 4.14 0 01-1.8 2.72v2.26h2.91c1.7-1.57 2.69-3.88 2.69-6.62z',
    azure_ad: 'M0 4.5v9l7.5 4.5v-9L0 4.5zm9 0v9l7.5 4.5v-9L9 4.5zm0-4.5l-9 4.5 9 4.5 9-4.5L9 0z',
    bitbucket: 'M.78 1.14c-.42 0-.78.37-.72.8l2.17 13.17c.07.39.4.69.8.69h12.02c.3 0 .56-.21.62-.5L17.95 1.93a.72.72 0 00-.72-.8H.78zM10.89 11h-3.8L6.23 7h5.54l-.88 4z'
  }
  return icons[provider.provider_type] || icons[provider.name] || ''
}

function getProviderColor(provider: OAuthProviderInfo) {
  const colors: Record<string, string> = {
    github: '#24292e',
    gitlab: '#fc6d26',
    google: '#4285F4',
    azure_ad: '#0078d4',
    bitbucket: '#0052cc'
  }
  return colors[provider.provider_type] || colors[provider.name] || '#666'
}

// Passkey 直接登录（无密码）
const supportsPasskey = ref(false)
const passkeyLoading = ref(false)

onMounted(() => {
  // 检查浏览器是否支持 WebAuthn
  supportsPasskey.value = !!window.PublicKeyCredential
})

async function passkeyDirectLogin() {
  if (!window.PublicKeyCredential) {
    error.value = '您的浏览器不支持 Passkey'
    return
  }

  passkeyLoading.value = true
  error.value = ''

  try {
    // 开始 WebAuthn 认证（无需密码）
    const startResponse = await api.auth.passkeyLoginStart()
    
    // 转换 options（Base64 → ArrayBuffer）
    const publicKeyOptions = convertWebAuthnAuthOptions(startResponse.challenge)
    
    // 调用浏览器 WebAuthn API
    const credential = await navigator.credentials.get({
      publicKey: publicKeyOptions
    }) as PublicKeyCredential

    if (!credential) {
      throw new Error('未能获取凭证')
    }

    // 完成验证并登录
    const loginResponse = await api.auth.passkeyLoginFinish({
      state_key: startResponse.state_key,
      credential: credential
    })

    // 保存登录状态
    authStore.token = loginResponse.token
    authStore.user = loginResponse.user
    localStorage.setItem('token', loginResponse.token)
    localStorage.setItem('user', JSON.stringify(loginResponse.user))

    redirectAfterLogin()
  } catch (e: any) {
    console.error('Passkey login failed:', e)
    if (e.name === 'NotAllowedError') {
      error.value = '操作已取消'
    } else {
      error.value = e.response?.data?.error || e.message || 'Passkey 登录失败'
    }
  } finally {
    passkeyLoading.value = false
  }
}

// Two-Factor Authentication state
const showTwoFactorModal = ref(false)
const twoFactorMethod = ref('totp')
const twoFactorCode = ref('')
const twoFactorLoading = ref(false)
const twoFactorError = ref('')
let temporaryToken = ''
let availableMethods: string[] = []

async function handleSubmit() {
  loading.value = true
  error.value = ''
  
  try {
    const response = await authStore.login(form)
    
    // Check if 2FA is required
    if ('requires_two_factor' in response && response.requires_two_factor) {
      // Store temporary token and available methods
      temporaryToken = response.temporary_token
      availableMethods = response.available_methods
      
      // Set default method
      if (availableMethods.includes('totp')) {
        twoFactorMethod.value = 'totp'
      } else if (availableMethods.includes('webauthn')) {
        twoFactorMethod.value = 'webauthn'
      } else {
        twoFactorMethod.value = 'recovery'
      }
      
      // Show 2FA modal
      showTwoFactorModal.value = true
    } else {
      // Normal login, redirect safely
      redirectAfterLogin()
    }
  } catch (e: any) {
    error.value = e.response?.data?.error || e.response?.data?.message || '登录失败，请检查用户名和密码'
  } finally {
    loading.value = false
  }
}

function selectWebAuthnMethod() {
  twoFactorMethod.value = 'webauthn'
  twoFactorCode.value = ''
  twoFactorError.value = ''
}

async function verifyTwoFactor() {
  // WebAuthn verification
  if (twoFactorMethod.value === 'webauthn') {
    await verifyWebAuthn()
    return
  }
  
  // TOTP/Recovery code verification
  if (!twoFactorCode.value || twoFactorCode.value.length < 6) {
    twoFactorError.value = '请输入验证码'
    return
  }
  
  twoFactorLoading.value = true
  twoFactorError.value = ''
  
  try {
    await authStore.verifyTwoFactor({
      temporary_token: temporaryToken,
      method: twoFactorMethod.value,
      code: twoFactorCode.value
    })
    
    showTwoFactorModal.value = false
    redirectAfterLogin()
  } catch (e: any) {
    twoFactorError.value = e.response?.data?.error || e.response?.data?.message || '验证失败，请检查验证码'
  } finally {
    twoFactorLoading.value = false
  }
}

async function verifyWebAuthn() {
  if (!window.PublicKeyCredential) {
    twoFactorError.value = '您的浏览器不支持 WebAuthn'
    return
  }

  twoFactorLoading.value = true
  twoFactorError.value = ''

  try {
    // 开始 WebAuthn 验证
    const startResponse = await api.twoFactor.webauthnAuthStart(temporaryToken)
    
    // 转换 options（Base64 → ArrayBuffer）
    const publicKeyOptions = convertWebAuthnAuthOptions(startResponse.challenge)
    
    // 调用浏览器 WebAuthn API
    const credential = await navigator.credentials.get({
      publicKey: publicKeyOptions
    }) as PublicKeyCredential

    if (!credential) {
      throw new Error('未能获取凭证')
    }

    // 完成验证
    const loginResponse = await api.twoFactor.webauthnAuthFinish({
      temporary_token: temporaryToken,
      state_key: startResponse.state_key,
      credential: credential
    })

    // 保存登录状态
    authStore.token = loginResponse.token
    authStore.user = loginResponse.user
    localStorage.setItem('token', loginResponse.token)
    localStorage.setItem('user', JSON.stringify(loginResponse.user))

    showTwoFactorModal.value = false
    redirectAfterLogin()
  } catch (e: any) {
    console.error('WebAuthn verification failed:', e)
    if (e.name === 'NotAllowedError') {
      twoFactorError.value = '操作已取消'
    } else {
      twoFactorError.value = e.response?.data?.error || e.message || 'WebAuthn 验证失败'
    }
  } finally {
    twoFactorLoading.value = false
  }
}

function closeTwoFactorModal() {
  showTwoFactorModal.value = false
  twoFactorCode.value = ''
  twoFactorError.value = ''
  temporaryToken = ''
}
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
  max-width: 400px;
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
    margin: 0;
  }
}

.auth-form {
  .form-group {
    margin-bottom: $spacing-5;
  }
  
  .form-label {
    display: block;
    font-size: $font-size-sm;
    font-weight: $font-weight-medium;
    color: $text-primary;
    margin-bottom: $spacing-2;
  }
  
  .label-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: $spacing-2;
  }
  
  .forgot-link {
    font-size: $font-size-sm;
    color: $text-link;
    
    &:hover {
      text-decoration: underline;
    }
  }
}

.input-wrapper {
  position: relative;
}

.input-icon {
  position: absolute;
  left: $spacing-3;
  top: 50%;
  transform: translateY(-50%);
  color: $text-muted;
  pointer-events: none;
}

.form-control.with-icon {
  padding-left: 40px;
  padding-right: 40px;
}

.toggle-password {
  position: absolute;
  right: $spacing-3;
  top: 50%;
  transform: translateY(-50%);
  background: transparent;
  border: none;
  color: $text-muted;
  cursor: pointer;
  padding: $spacing-1;
  
  &:hover {
    color: $text-primary;
  }
}

.checkbox-wrapper {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  cursor: pointer;
  
  input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: $brand-primary;
  }
  
  .checkbox-label {
    font-size: $font-size-sm;
    color: $text-primary;
  }
}

.w-full {
  width: 100%;
}

.spinner-sm {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
  margin-right: $spacing-2;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.auth-divider {
  display: flex;
  align-items: center;
  margin: $spacing-6 0;
  
  &::before,
  &::after {
    content: '';
    flex: 1;
    height: 1px;
    background: $border-color;
  }
  
  span {
    padding: 0 $spacing-4;
    font-size: $font-size-sm;
    color: $text-muted;
  }
}

.social-login {
  display: flex;
  flex-direction: column;
  gap: $spacing-3;
}

.btn-social {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: $spacing-3;
  width: 100%;
  padding: $spacing-3 $spacing-4;
  font-size: $font-size-base;
  font-weight: $font-weight-medium;
  color: $text-primary;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  cursor: pointer;
  transition: all $transition-fast;
  
  &:hover {
    background: $bg-secondary;
    border-color: $border-color-dark;
  }
}

// Passkey Login Section
.passkey-login-section {
  margin-bottom: $spacing-5;
}

.btn-passkey {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: $spacing-3;
  width: 100%;
  padding: $spacing-4;
  font-size: $font-size-base;
  font-weight: $font-weight-medium;
  color: white;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border: none;
  border-radius: $border-radius;
  cursor: pointer;
  transition: all $transition-fast;
  
  &:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
  }
  
  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  
  svg {
    color: white;
  }
}

.auth-footer {
  text-align: center;
  margin-top: $spacing-6;
  color: $gray-400;
  font-size: $font-size-sm;
  
  a {
    color: $text-light;
    font-weight: $font-weight-medium;
    
    &:hover {
      text-decoration: underline;
    }
  }
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
}

// Two-Factor Authentication Modal
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 1rem;
}

.modal-content {
  background: white;
  border-radius: $border-radius-xl;
  max-width: 450px;
  width: 100%;
  max-height: 90vh;
  overflow-y: auto;
  
  &.twofa-modal {
    max-width: 500px;
  }
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1.5rem;
  border-bottom: 1px solid $border-color;

  h3 {
    font-size: $font-size-xl;
    font-weight: $font-weight-semibold;
    color: $text-primary;
    margin: 0;
  }

  .modal-close {
    width: 32px;
    height: 32px;
    border-radius: $border-radius;
    background: transparent;
    border: none;
    cursor: pointer;
    color: $text-muted;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all $transition-fast;

    &:hover {
      background: $bg-secondary;
    }
  }
}

.modal-body {
  padding: 1.5rem;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: $spacing-3;
  padding: 1.5rem;
  border-top: 1px solid $border-color;
  
  .btn {
    padding: $spacing-2 $spacing-4;
    
    &-secondary {
      background: $bg-secondary;
      color: $text-primary;
      border: 1px solid $border-color;
      
      &:hover {
        background: darken($bg-secondary, 5%);
      }
    }
  }
}

.method-tabs {
  display: flex;
  gap: $spacing-2;
  margin-bottom: $spacing-6;
  border-bottom: 1px solid $border-color;
}

.method-tab {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: $spacing-2;
  padding: $spacing-3;
  background: transparent;
  border: none;
  border-bottom: 2px solid transparent;
  color: $text-muted;
  font-size: $font-size-sm;
  font-weight: $font-weight-medium;
  cursor: pointer;
  transition: all $transition-fast;

  &:hover:not(:disabled) {
    color: $text-primary;
  }

  &.active {
    color: $brand-primary;
    border-bottom-color: $brand-primary;
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  svg {
    flex-shrink: 0;
  }
}

.twofa-input-group {
  .input-description {
    color: $text-secondary;
    font-size: $font-size-sm;
    margin-bottom: $spacing-4;
    text-align: center;
  }
}

.twofa-code-input {
  width: 100%;
  max-width: 250px;
  margin: 0 auto;
  padding: $spacing-3;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  font-size: $font-size-xl;
  font-family: 'Monaco', 'Courier New', monospace;
  letter-spacing: 0.5rem;
  text-align: center;
  display: block;

  &:focus {
    outline: none;
    border-color: $brand-primary;
    box-shadow: 0 0 0 3px rgba($brand-primary, 0.1);
  }

  &.recovery {
    letter-spacing: 0.2rem;
    text-transform: uppercase;
  }
}

.webauthn-indicator {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: $spacing-3;
  padding: $spacing-5 0;
  color: #6b7280;

  svg {
    color: $brand-primary;
  }

  .webauthn-ready {
    margin: 0;
    font-size: $font-size-sm;
    color: #6b7280;
  }
}
</style>
