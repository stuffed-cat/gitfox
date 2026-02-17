<template>
  <div class="two-factor-page">
    <!-- 面包屑导航 -->
    <div class="breadcrumb">
      <router-link to="/-/profile">用户设置</router-link>
      <span class="separator">/</span>
      <span>双因素认证</span>
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
      <h1>双因素认证 (2FA)</h1>
      <p class="section-description">
        双因素认证为您的账户增加了额外的安全层。除了密码外，您还需要提供第二个验证因素才能登录。
      </p>
    </section>

    <!-- 加载状态 -->
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>加载中...</span>
    </div>

    <!-- 状态卡片 -->
    <div v-if="!loading" class="status-card">
      <div class="status-header">
        <div class="status-icon" :class="{ active: status?.enabled }">
          <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
            <path d="M8 1L3 3v4c0 3.5 2.5 6.5 5 7 2.5-.5 5-3.5 5-7V3l-5-2z" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <div class="status-text">
          <h3>{{ status?.enabled ? '双因素认证已启用' : '双因素认证已禁用' }}</h3>
          <p>{{ status?.enabled ? '您的账户受到额外保护' : '建议启用以保护您的账户' }}</p>
        </div>
      </div>
    </div>

    <!-- TOTP (Authenticator App) -->
    <div v-if="!loading" class="method-card">
      <div class="method-header">
        <div class="method-icon">
          <svg viewBox="0 0 16 16" width="20" height="20" fill="none">
            <rect x="4" y="2" width="8" height="12" rx="1.5" stroke="currentColor" stroke-width="1.2"/>
            <path d="M6 5h4M6 7h4M6 9h4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          </svg>
        </div>
        <div class="method-info">
          <h3>身份验证器应用</h3>
          <p>使用手机上的应用程序（如 Google Authenticator、Authy）生成验证码</p>
        </div>
        <div class="method-status">
          <span v-if="status?.totp_enabled" class="badge badge-success">已启用</span>
          <span v-else class="badge badge-secondary">未启用</span>
        </div>
      </div>

      <div class="method-actions">
        <button
          v-if="!status?.totp_enabled"
          class="btn btn-primary"
          @click="startTotpSetup"
          :disabled="settingUpTotp"
        >
          {{ settingUpTotp ? '设置中...' : '设置身份验证器' }}
        </button>
        <button
          v-else
          class="btn btn-danger-outline"
          @click="showDisableTotpModal = true"
        >
          禁用身份验证器
        </button>
      </div>
    </div>

    <!-- WebAuthn (Passkeys) -->
    <div v-if="!loading" class="method-card">
      <div class="method-header">
        <div class="method-icon">
          <svg viewBox="0 0 16 16" width="20" height="20" fill="none">
            <path d="M8 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM4 14c0-2.21 1.79-4 4-4s4 1.79 4 4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          </svg>
        </div>
        <div class="method-info">
          <h3>Passkey (WebAuthn)</h3>
          <p>使用生物识别、安全密钥或设备密码进行快速安全的登录</p>
        </div>
        <div class="method-status">
          <span v-if="status && status.webauthn_credentials.length > 0" class="badge badge-success">
            {{ status.webauthn_credentials.length }} 个已注册
          </span>
          <span v-else class="badge badge-secondary">未注册</span>
        </div>
      </div>

      <div v-if="status && status.webauthn_credentials.length > 0" class="credentials-list">
        <div v-for="cred in status.webauthn_credentials" :key="cred.id" class="credential-item">
          <div class="credential-icon">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M14 2l-1 1m-4 4a3 3 0 11-4.243 4.243A3 3 0 019 7zm0 0L11 5m0 0l2 2 2-2-2-2m-2 2l1-1" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <div class="credential-info">
            <div class="credential-name">{{ cred.name }}</div>
            <div class="credential-meta">
              <span v-if="cred.last_used_at">最后使用: {{ formatDate(cred.last_used_at) }}</span>
              <span v-else>从未使用</span>
              <span>·</span>
              <span>添加于: {{ formatDate(cred.created_at) }}</span>
            </div>
          </div>
          <button class="btn btn-danger-outline btn-sm" @click="confirmDeleteCredential(cred)">
            删除
          </button>
        </div>
      </div>

      <div class="method-actions">
        <button class="btn btn-primary" @click="addPasskey" :disabled="addingPasskey">
          {{ addingPasskey ? '添加中...' : '添加 Passkey' }}
        </button>
      </div>
    </div>

    <!-- Recovery Codes -->
    <div v-if="!loading && status?.enabled" class="method-card">
      <div class="method-header">
        <div class="method-icon">
          <svg viewBox="0 0 16 16" width="20" height="20" fill="none">
            <rect x="3" y="5" width="10" height="9" rx="1.5" stroke="currentColor" stroke-width="1.2"/>
            <path d="M5 5V4a3 3 0 0 1 6 0v1" stroke="currentColor" stroke-width="1.2"/>
            <circle cx="8" cy="9.5" r="1" fill="currentColor"/>
          </svg>
        </div>
        <div class="method-info">
          <h3>恢复代码</h3>
          <p>用于在无法访问其他方法时恢复账户访问权限</p>
        </div>
        <div class="method-status">
          <span class="badge badge-info">{{ status.recovery_codes_count }} 个可用</span>
        </div>
      </div>

      <div class="method-actions">
        <button
          class="btn btn-secondary"
          @click="regenerateRecoveryCodes"
          :disabled="regeneratingCodes"
        >
          {{ regeneratingCodes ? '生成中...' : '重新生成恢复代码' }}
        </button>
      </div>
    </div>

    <!-- TOTP Setup Modal -->
    <div v-if="showTotpSetupModal" class="modal-overlay" @click.self="closeTotpSetup">
      <div class="modal-content totp-setup-modal">
        <div class="modal-header">
          <h3>设置身份验证器应用</h3>
          <button class="modal-close" @click="closeTotpSetup">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <div class="modal-body">
          <div class="setup-step">
            <h4>步骤 1: 扫描二维码</h4>
            <p>使用您的身份验证器应用扫描此二维码：</p>
            <div v-if="totpSetup" class="qr-code-container">
              <img :src="totpSetup.qr_code" alt="TOTP QR Code" />
            </div>
            <p class="text-muted">或手动输入密钥：</p>
            <code class="secret-code" v-if="totpSetup">{{ totpSetup.secret }}</code>
          </div>

          <div class="setup-step">
            <h4>步骤 2: 输入验证码</h4>
            <p>输入您的身份验证器应用显示的 6 位验证码：</p>
            <input
              v-model="totpCode"
              type="text"
              class="form-input totp-input"
              placeholder="000000"
              maxlength="6"
              pattern="[0-9]{6}"
            />
            <div v-if="totpError" class="error-message">{{ totpError }}</div>
          </div>

          <div class="setup-step">
            <h4>步骤 3: 保存恢复代码</h4>
            <p class="warning-text">
              <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
                <path d="M8 1L1 14h14L8 1z" stroke="currentColor" stroke-width="1.2"/>
                <path d="M8 6v3M8 11v1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              请将这些恢复代码保存在安全的地方。每个代码只能使用一次。
            </p>
            <div v-if="totpSetup" class="recovery-codes">
              <code v-for="(code, index) in totpSetup.backup_codes" :key="index">
                {{ code }}
              </code>
            </div>
            <button class="btn btn-secondary btn-sm" @click="copyRecoveryCodes">
              复制恢复代码
            </button>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-secondary" @click="closeTotpSetup">取消</button>
          <button
            class="btn btn-primary"
            @click="enableTotp"
            :disabled="!totpCode || totpCode.length !== 6 || enablingTotp"
          >
            {{ enablingTotp ? '启用中...' : '启用' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Disable TOTP Modal -->
    <div v-if="showDisableTotpModal" class="modal-overlay" @click.self="showDisableTotpModal = false">
      <div class="modal-content">
        <div class="modal-header">
          <h3>禁用身份验证器应用</h3>
          <button class="modal-close" @click="showDisableTotpModal = false">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <div class="modal-body">
          <p class="warning-text">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M8 1L1 14h14L8 1z" stroke="currentColor" stroke-width="1.2"/>
              <path d="M8 6v3M8 11v1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            禁用身份验证器将降低您账户的安全性。请输入当前验证码以确认。
          </p>
          <div class="form-group">
            <label>验证码</label>
            <input
              v-model="disableTotpCode"
              type="text"
              class="form-input"
              placeholder="000000"
              maxlength="6"
              pattern="[0-9]{6}"
            />
            <div v-if="disableTotpError" class="error-message">{{ disableTotpError }}</div>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showDisableTotpModal = false">取消</button>
          <button
            class="btn btn-danger"
            @click="disableTotp"
            :disabled="!disableTotpCode || disableTotpCode.length !== 6 || disablingTotp"
          >
            {{ disablingTotp ? '禁用中...' : '禁用' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Recovery Codes Modal -->
    <div v-if="showRecoveryCodesModal && recoveryCodes" class="modal-overlay" @click.self="showRecoveryCodesModal = false">
      <div class="modal-content">
        <div class="modal-header">
          <h3>恢复代码</h3>
          <button class="modal-close" @click="showRecoveryCodesModal = false">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <div class="modal-body">
          <p class="warning-text">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M8 1L1 14h14L8 1z" stroke="currentColor" stroke-width="1.2"/>
              <path d="M8 6v3M8 11v1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            请将这些恢复代码保存在安全的地方。每个代码只能使用一次。
          </p>
          <div class="recovery-codes">
            <code v-for="(code, index) in recoveryCodes.codes" :key="index">
              {{ code }}
            </code>
          </div>
          <button class="btn btn-secondary btn-sm" @click="copyNewRecoveryCodes">
            复制恢复代码
          </button>
        </div>

        <div class="modal-footer">
          <button class="btn btn-primary" @click="showRecoveryCodesModal = false">完成</button>
        </div>
      </div>
    </div>

    <!-- Add Passkey Modal -->
    <div v-if="showAddPasskeyModal" class="modal-overlay" @click.self="cancelAddPasskey">
      <div class="modal-content">
        <div class="modal-header">
          <h3>为 Passkey 命名</h3>
          <button class="modal-close" @click="cancelAddPasskey">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <div class="modal-body">
          <p class="success-message">
            <svg viewBox="0 0 16 16" width="20" height="20" fill="none">
              <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.2"/>
              <path d="M5 8l2 2 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            认证成功！请为您的 Passkey 设置一个名称：
          </p>
          <input
            v-model="passkeyName"
            type="text"
            class="form-input"
            placeholder="例如：我的 YubiKey"
            maxlength="100"
            autofocus
          />
          <div v-if="passkeyError" class="error-message">{{ passkeyError }}</div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-secondary" @click="cancelAddPasskey">取消</button>
          <button
            class="btn btn-primary"
            @click="savePasskeyName"
            :disabled="!passkeyName.trim() || addingPasskey"
          >
            {{ addingPasskey ? '保存中...' : '保存' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Success Toast -->
    <div v-if="showSuccessToast" class="toast toast-success">
      <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
        <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.2"/>
        <path d="M5 8l2 2 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      {{ toastMessage }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '@/api'
import type { TwoFactorStatus, TotpSetupResponse, RecoveryCodesResponse, WebAuthnCredentialInfo } from '@/types'

const searchQuery = ref('')
const loading = ref(false)
const status = ref<TwoFactorStatus | null>(null)

// TOTP Setup
const showTotpSetupModal = ref(false)
const settingUpTotp = ref(false)
const totpSetup = ref<TotpSetupResponse | null>(null)
const totpCode = ref('')
const totpError = ref('')
const enablingTotp = ref(false)

// TOTP Disable
const showDisableTotpModal = ref(false)
const disableTotpCode = ref('')
const disableTotpError = ref('')
const disablingTotp = ref(false)

// Recovery Codes
const showRecoveryCodesModal = ref(false)
const recoveryCodes = ref<RecoveryCodesResponse | null>(null)
const regeneratingCodes = ref(false)

// WebAuthn/Passkey
const showAddPasskeyModal = ref(false)
const passkeyName = ref('')
const passkeyError = ref('')
const addingPasskey = ref(false)
const pendingCredential = ref<{credential: PublicKeyCredential, state_key: string} | null>(null)

// Toast
const showSuccessToast = ref(false)
const toastMessage = ref('')

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

// 转换 WebAuthn options（Base64 → ArrayBuffer）
function convertWebAuthnOptions(options: any): PublicKeyCredentialCreationOptions {
  console.log('Converting WebAuthn options:', options)
  
  // webauthn-rs 返回的格式可能直接是 publicKey 内容，或者包含 publicKey 字段
  const publicKey = options.publicKey || options
  
  return {
    ...publicKey,
    challenge: typeof publicKey.challenge === 'string' 
      ? base64UrlDecode(publicKey.challenge) 
      : publicKey.challenge,
    user: {
      ...publicKey.user,
      id: typeof publicKey.user?.id === 'string'
        ? base64UrlDecode(publicKey.user.id)
        : publicKey.user?.id
    },
    excludeCredentials: publicKey.excludeCredentials?.map((cred: any) => ({
      ...cred,
      id: typeof cred.id === 'string' ? base64UrlDecode(cred.id) : cred.id
    })) || []
  }
}

onMounted(async () => {
  await loadStatus()
})

async function loadStatus() {
  loading.value = true
  try {
    status.value = await api.twoFactor.getStatus()
  } catch (error: any) {
    console.error('Failed to load 2FA status:', error)
  } finally {
    loading.value = false
  }
}

async function startTotpSetup() {
  settingUpTotp.value = true
  totpError.value = ''
  try {
    totpSetup.value = await api.twoFactor.setupTotp()
    showTotpSetupModal.value = true
  } catch (error: any) {
    console.error('Failed to setup TOTP:', error)
    alert('设置失败：' + (error.response?.data?.error || error.message))
  } finally {
    settingUpTotp.value = false
  }
}

async function enableTotp() {
  if (!totpCode.value || totpCode.value.length !== 6) {
    totpError.value = '请输入 6 位验证码'
    return
  }

  if (!totpSetup.value?.state_key) {
    totpError.value = '设置会话已过期，请重新开始'
    return
  }

  enablingTotp.value = true
  totpError.value = ''
  try {
    await api.twoFactor.enableTotp(totpSetup.value.state_key, totpCode.value)
    showTotpSetupModal.value = false
    await loadStatus()
    showToast('身份验证器已成功启用')
  } catch (error: any) {
    console.error('Failed to enable TOTP:', error)
    const errorMsg = error.response?.data?.error || error.message
    if (errorMsg.includes('expired')) {
      totpError.value = '设置会话已过期（5分钟），请关闭弹窗重新开始'
    } else {
      totpError.value = errorMsg || '验证码无效'
    }
  } finally {
    enablingTotp.value = false
  }
}

function closeTotpSetup() {
  showTotpSetupModal.value = false
  totpSetup.value = null
  totpCode.value = ''
  totpError.value = ''
}

async function disableTotp() {
  if (!disableTotpCode.value || disableTotpCode.value.length !== 6) {
    disableTotpError.value = '请输入 6 位验证码'
    return
  }

  disablingTotp.value = true
  disableTotpError.value = ''
  try {
    await api.twoFactor.disableTotp(disableTotpCode.value)
    showDisableTotpModal.value = false
    disableTotpCode.value = ''
    await loadStatus()
    showToast('身份验证器已禁用')
  } catch (error: any) {
    console.error('Failed to disable TOTP:', error)
    disableTotpError.value = error.response?.data?.error || '验证码无效'
  } finally {
    disablingTotp.value = false
  }
}

async function regenerateRecoveryCodes() {
  if (!confirm('确定要重新生成恢复代码吗？旧的恢复代码将失效。')) {
    return
  }

  regeneratingCodes.value = true
  try {
    recoveryCodes.value = await api.twoFactor.regenerateRecoveryCodes()
    showRecoveryCodesModal.value = true
    await loadStatus()
  } catch (error: any) {
    console.error('Failed to regenerate recovery codes:', error)
    alert('生成失败：' + (error.response?.data?.error || error.message))
  } finally {
    regeneratingCodes.value = false
  }
}

function copyRecoveryCodes() {
  if (!totpSetup.value) return
  const codes = totpSetup.value.backup_codes.join('\n')
  navigator.clipboard.writeText(codes)
  showToast('恢复代码已复制到剪贴板')
}

function copyNewRecoveryCodes() {
  if (!recoveryCodes.value) return
  const codes = recoveryCodes.value.codes.join('\n')
  navigator.clipboard.writeText(codes)
  showToast('恢复代码已复制到剪贴板')
}

function confirmDeleteCredential(cred: WebAuthnCredentialInfo) {
  if (!confirm(`确定要删除 "${cred.name}" 吗？`)) {
    return
  }
  deleteCredential(cred.id)
}

async function deleteCredential(id: number) {
  try {
    await api.twoFactor.deleteWebAuthnCredential(id)
    await loadStatus()
    showToast('Passkey 已删除')
  } catch (error: any) {
    console.error('Failed to delete credential:', error)
    alert('删除失败：' + (error.response?.data?.error || error.message))
  }
}

function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))

  if (days === 0) return '今天'
  if (days === 1) return '昨天'
  if (days < 7) return `${days} 天前`
  if (days < 30) return `${Math.floor(days / 7)} 周前`
  if (days < 365) return `${Math.floor(days / 30)} 个月前`
  return `${Math.floor(days / 365)} 年前`
}

// 第一步：触发 WebAuthn 认证
async function addPasskey() {
  // 检查浏览器是否支持 WebAuthn
  if (!window.PublicKeyCredential) {
    alert('您的浏览器不支持 Passkey')
    return
  }

  addingPasskey.value = true

  try {
    // 开始注册
    const startResponse = await api.twoFactor.webauthnRegisterStart()
    
    // 转换 options（Base64 → ArrayBuffer）
    const publicKeyOptions = convertWebAuthnOptions(startResponse.challenge)
    
    // 调用浏览器 WebAuthn API
    const credential = await navigator.credentials.create({
      publicKey: publicKeyOptions
    }) as PublicKeyCredential

    if (!credential) {
      throw new Error('未能创建凭证')
    }

    // 保存凭证，等待用户命名
    pendingCredential.value = {
      credential: credential,
      state_key: startResponse.state_key
    }
    
    // 现在弹窗让用户命名
    showAddPasskeyModal.value = true
    passkeyName.value = ''
    passkeyError.value = ''
  } catch (error: any) {
    console.error('Failed to add passkey:', error)
    if (error.name === 'NotAllowedError') {
      alert('操作已取消')
    } else if (error.name === 'InvalidStateError') {
      alert('此设备已注册')
    } else {
      alert('添加失败：' + (error.response?.data?.error || error.message))
    }
  } finally {
    addingPasskey.value = false
  }
}

// 第二步：保存命名后的 Passkey
async function savePasskeyName() {
  if (!passkeyName.value.trim()) {
    passkeyError.value = '请输入 Passkey 名称'
    return
  }

  if (!pendingCredential.value) {
    passkeyError.value = '凭证已失效，请重新添加'
    return
  }

  addingPasskey.value = true
  passkeyError.value = ''

  try {
    // 完成注册
    await api.twoFactor.webauthnRegisterFinish({
      state_key: pendingCredential.value.state_key,
      name: passkeyName.value,
      credential: pendingCredential.value.credential
    })

    showAddPasskeyModal.value = false
    passkeyName.value = ''
    pendingCredential.value = null
    await loadStatus()
    showToast('Passkey 添加成功')
  } catch (error: any) {
    console.error('Failed to save passkey:', error)
    passkeyError.value = error.response?.data?.error || error.message || '保存失败'
  } finally {
    addingPasskey.value = false
  }
}

function cancelAddPasskey() {
  showAddPasskeyModal.value = false
  passkeyName.value = ''
  passkeyError.value = ''
  pendingCredential.value = null
}

function showToast(message: string) {
  toastMessage.value = message
  showSuccessToast.value = true
  setTimeout(() => {
    showSuccessToast.value = false
  }, 3000)
}
</script>

<style scoped lang="scss">
.two-factor-page {
  max-width: 900px;
  margin: 0 auto;
  padding: 2rem 1rem;
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 1.5rem;
  font-size: 0.875rem;
  color: #6b7280;

  a {
    color: #3b82f6;
    text-decoration: none;

    &:hover {
      text-decoration: underline;
    }
  }

  .separator {
    color: #d1d5db;
  }
}

.search-box {
  position: relative;
  margin-bottom: 2rem;

  .search-icon {
    position: absolute;
    left: 1rem;
    top: 50%;
    transform: translateY(-50%);
    color: #9ca3af;
  }

  input {
    width: 100%;
    padding: 0.75rem 1rem 0.75rem 2.75rem;
    border: 1px solid #d1d5db;
    border-radius: 0.5rem;
    font-size: 0.875rem;

    &:focus {
      outline: none;
      border-color: #3b82f6;
      box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
    }
  }
}

.page-section {
  margin-bottom: 2rem;

  h1 {
    font-size: 1.875rem;
    font-weight: 600;
    margin-bottom: 0.5rem;
    color: #111827;
  }

  .section-description {
    color: #6b7280;
    line-height: 1.6;
  }
}

.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  padding: 3rem;
  color: #6b7280;

  .spinner {
    width: 24px;
    height: 24px;
    border: 3px solid #e5e7eb;
    border-top-color: #3b82f6;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.status-card,
.method-card {
  background: white;
  border: 1px solid #e5e7eb;
  border-radius: 0.75rem;
  padding: 1.5rem;
  margin-bottom: 1.5rem;
}

.status-header {
  display: flex;
  align-items: flex-start;
  gap: 1rem;

  .status-icon {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #fee2e2;
    color: #dc2626;

    &.active {
      background: #d1fae5;
      color: #059669;
    }

    svg {
      width: 24px;
      height: 24px;
    }
  }

  .status-text {
    flex: 1;

    h3 {
      font-size: 1.125rem;
      font-weight: 600;
      margin-bottom: 0.25rem;
      color: #111827;
    }

    p {
      color: #6b7280;
      font-size: 0.875rem;
    }
  }
}

.method-header {
  display: flex;
  align-items: flex-start;
  gap: 1rem;
  margin-bottom: 1rem;

  .method-icon {
    width: 40px;
    height: 40px;
    border-radius: 0.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #f3f4f6;
    color: #6b7280;
  }

  .method-info {
    flex: 1;

    h3 {
      font-size: 1rem;
      font-weight: 600;
      margin-bottom: 0.25rem;
      color: #111827;
    }

    p {
      color: #6b7280;
      font-size: 0.875rem;
      line-height: 1.5;
    }
  }

  .method-status {
    .badge {
      padding: 0.25rem 0.75rem;
      border-radius: 9999px;
      font-size: 0.75rem;
      font-weight: 500;

      &.badge-success {
        background: #d1fae5;
        color: #059669;
      }

      &.badge-secondary {
        background: #f3f4f6;
        color: #6b7280;
      }

      &.badge-info {
        background: #dbeafe;
        color: #2563eb;
      }
    }
  }
}

.method-actions {
  display: flex;
  gap: 0.75rem;
}

.credentials-list,
.recovery-codes {
  margin-top: 1rem;
  margin-bottom: 1rem;
}

.credential-item {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem;
  background: #f9fafb;
  border-radius: 0.5rem;
  margin-bottom: 0.75rem;

  .credential-icon {
    width: 32px;
    height: 32px;
    border-radius: 0.375rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #e5e7eb;
    color: #6b7280;
  }

  .credential-info {
    flex: 1;

    .credential-name {
      font-weight: 500;
      margin-bottom: 0.25rem;
      color: #111827;
    }

    .credential-meta {
      font-size: 0.75rem;
      color: #6b7280;

      span {
        margin-right: 0.5rem;
      }
    }
  }
}

.recovery-codes {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 0.5rem;
  padding: 1rem;
  background: #f9fafb;
  border-radius: 0.5rem;

  code {
    padding: 0.5rem;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 0.375rem;
    font-family: 'Monaco', 'Courier New', monospace;
    font-size: 0.875rem;
    text-align: center;
  }
}

.btn {
  padding: 0.5rem 1rem;
  border-radius: 0.375rem;
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  border: none;

  &.btn-sm {
    padding: 0.375rem 0.75rem;
    font-size: 0.8125rem;
  }

  &.btn-primary {
    background: #3b82f6;
    color: white;

    &:hover:not(:disabled) {
      background: #2563eb;
    }

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }

  &.btn-secondary {
    background: #f3f4f6;
    color: #374151;

    &:hover:not(:disabled) {
      background: #e5e7eb;
    }

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }

  &.btn-danger {
    background: #dc2626;
    color: white;

    &:hover:not(:disabled) {
      background: #b91c1c;
    }

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }

  &.btn-danger-outline {
    background: white;
    color: #dc2626;
    border: 1px solid #dc2626;

    &:hover {
      background: #fef2f2;
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
  padding: 1rem;
}

.modal-content {
  background: white;
  border-radius: 0.75rem;
  max-width: 500px;
  width: 100%;
  max-height: 90vh;
  overflow-y: auto;

  &.totp-setup-modal {
    max-width: 600px;
  }
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1.5rem;
  border-bottom: 1px solid #e5e7eb;

  h3 {
    font-size: 1.125rem;
    font-weight: 600;
    color: #111827;
  }

  .modal-close {
    width: 32px;
    height: 32px;
    border-radius: 0.375rem;
    background: transparent;
    border: none;
    cursor: pointer;
    color: #6b7280;
    display: flex;
    align-items: center;
    justify-content: center;

    &:hover {
      background: #f3f4f6;
    }
  }
}

.modal-body {
  padding: 1.5rem;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 1.5rem;
  border-top: 1px solid #e5e7eb;
}

.setup-step {
  margin-bottom: 2rem;

  &:last-child {
    margin-bottom: 0;
  }

  h4 {
    font-size: 1rem;
    font-weight: 600;
    margin-bottom: 0.5rem;
    color: #111827;
  }

  p {
    color: #6b7280;
    font-size: 0.875rem;
    margin-bottom: 1rem;
  }
}

.qr-code-container {
  display: flex;
  justify-content: center;
  padding: 1rem;
  background: white;
  border: 1px solid #e5e7eb;
  border-radius: 0.5rem;
  margin-bottom: 1rem;

  img {
    max-width: 200px;
  }
}

.secret-code {
  display: block;
  padding: 1rem;
  background: #f9fafb;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
  font-family: 'Monaco', 'Courier New', monospace;
  font-size: 0.875rem;
  word-break: break-all;
  margin-bottom: 1rem;
}

.totp-input {
  width: 100%;
  max-width: 200px;
  padding: 0.75rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-size: 1.25rem;
  font-family: 'Monaco', 'Courier New', monospace;
  letter-spacing: 0.5rem;
  text-align: center;

  &:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }
}

.form-group {
  margin-bottom: 1rem;

  label {
    display: block;
    font-weight: 500;
    margin-bottom: 0.5rem;
    color: #374151;
  }

  .form-input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    font-size: 0.875rem;

    &:focus {
      outline: none;
      border-color: #3b82f6;
      box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
    }
  }
}

.warning-text {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 1rem;
  background: #fef3c7;
  border: 1px solid #fbbf24;
  border-radius: 0.5rem;
  color: #92400e;
  font-size: 0.875rem;
  margin-bottom: 1rem;

  svg {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    margin-top: 0.125rem;
  }
}

.error-message {
  color: #dc2626;
  font-size: 0.875rem;
  margin-top: 0.5rem;
}

.success-message {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: #059669;
  font-size: 0.875rem;
  margin-bottom: 1rem;
  padding: 0.75rem;
  background: #d1fae5;
  border-radius: 0.5rem;
  
  svg {
    flex-shrink: 0;
    stroke: #059669;
  }
}

.text-muted {
  color: #6b7280;
  font-size: 0.875rem;
}

.toast {
  position: fixed;
  bottom: 2rem;
  right: 2rem;
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1rem 1.5rem;
  background: white;
  border-radius: 0.5rem;
  box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1);
  z-index: 2000;
  animation: slideIn 0.3s ease-out;

  &.toast-success {
    border-left: 4px solid #10b981;
    color: #047857;
  }

  svg {
    flex-shrink: 0;
  }
}

@keyframes slideIn {
  from {
    transform: translateX(100%);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}
</style>
