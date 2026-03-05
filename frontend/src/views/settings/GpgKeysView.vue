<template>
  <div class="gpg-keys-page">
    <!-- 面包屑导航 -->
    <div class="breadcrumb">
      <router-link to="/-/profile">用户设置</router-link>
      <span class="separator">/</span>
      <span>GPG Keys</span>
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
      <h1>GPG Keys</h1>
      <p class="section-description">
        GPG 密钥用于验证 Git 提交的签名。添加您用于签署提交的 GPG 公钥后，您的已验证提交将显示"已验证"徽章。
      </p>
    </section>

    <!-- GPG 密钥列表 -->
    <div class="keys-card">
      <div class="keys-header">
        <div class="keys-title">
          <span>您的 GPG 密钥</span>
          <svg class="key-icon" viewBox="0 0 16 16" width="16" height="16" fill="none">
            <path d="M10.5 1a4.5 4.5 0 00-4.495 4.73A.5.5 0 015.5 6H4v1.5a.5.5 0 01-.5.5H2v1.5a.5.5 0 01-.5.5h-1v2a.5.5 0 00.5.5h2a.5.5 0 00.5-.5V11h1.5a.5.5 0 00.5-.5V9h1.5a.5.5 0 00.354-.146l.79-.79A4.5 4.5 0 1010.5 1z" stroke="currentColor" stroke-width="1.2"/>
            <circle cx="10.5" cy="4.5" r="1" stroke="currentColor" stroke-width="1.2"/>
          </svg>
          <span class="keys-count">{{ keys.length }}</span>
        </div>
        <button class="btn btn-add" @click="showAddModal = true">
          添加新密钥
        </button>
      </div>

      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <span>加载中...</span>
      </div>

      <div v-else-if="keys.length === 0" class="empty-state">
        <div class="empty-icon">
          <svg viewBox="0 0 64 64" width="64" height="64" fill="none">
            <circle cx="32" cy="32" r="28" fill="#e9d8fd" stroke="#c4b5fd" stroke-width="2"/>
            <path d="M42 22a10 10 0 00-10 10v2h-2a2 2 0 00-2 2v8a2 2 0 002 2h12a2 2 0 002-2v-8a2 2 0 00-2-2h-2v-2a4 4 0 10-8 0" stroke="#8b5cf6" stroke-width="2.5" fill="none"/>
          </svg>
        </div>
        <p class="empty-title">此帐户没有 GPG 密钥</p>
        <p class="empty-subtitle">添加 GPG 密钥以验证您的 Git 提交</p>
      </div>

      <div v-else class="keys-list">
        <div v-for="key in keys" :key="key.id" class="key-item">
          <div class="key-icon-wrapper" :class="{ verified: key.verified, revoked: key.revoked }">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M10.5 1a4.5 4.5 0 00-4.495 4.73A.5.5 0 015.5 6H4v1.5a.5.5 0 01-.5.5H2v1.5a.5.5 0 01-.5.5h-1v2a.5.5 0 00.5.5h2a.5.5 0 00.5-.5V11h1.5a.5.5 0 00.5-.5V9h1.5a.5.5 0 00.354-.146l.79-.79A4.5 4.5 0 1010.5 1z" stroke="currentColor" stroke-width="1.2"/>
              <circle cx="10.5" cy="4.5" r="1" stroke="currentColor" stroke-width="1.2"/>
            </svg>
          </div>
          <div class="key-info">
            <div class="key-header-row">
              <div class="key-id">{{ key.primary_key_id }}</div>
              <div class="key-badges">
                <span v-if="key.verified" class="badge badge-verified">已验证</span>
                <span v-else class="badge badge-unverified">未验证</span>
                <span v-if="key.revoked" class="badge badge-revoked">已撤销</span>
                <span v-if="isExpired(key)" class="badge badge-expired">已过期</span>
                <span class="badge badge-algo">{{ key.key_algorithm }} {{ key.key_size }}</span>
              </div>
            </div>
            <code class="key-fingerprint">{{ formatFingerprint(key.fingerprint) }}</code>
            <div class="key-emails">
              <svg class="email-icon" viewBox="0 0 16 16" width="12" height="12" fill="none">
                <rect x="1" y="3" width="14" height="10" rx="2" stroke="currentColor" stroke-width="1.2"/>
                <path d="M1 5l7 4 7-4" stroke="currentColor" stroke-width="1.2"/>
              </svg>
              <span v-for="(email, idx) in key.emails" :key="idx" class="email-tag">{{ email }}</span>
              <span v-if="key.emails.length === 0" class="no-emails">无关联邮箱</span>
            </div>
            <div class="key-meta">
              <span>创建于: {{ formatDate(key.key_created_at || key.created_at) }}</span>
              <span v-if="key.key_expires_at">·</span>
              <span v-if="key.key_expires_at">过期时间: {{ formatDate(key.key_expires_at) }}</span>
              <span>·</span>
              <span v-if="key.last_used_at">最后使用: {{ formatDate(key.last_used_at) }}</span>
              <span v-else class="never-used">从未使用</span>
            </div>
            <div v-if="key.subkeys && key.subkeys.length > 0" class="subkeys-section">
              <div class="subkeys-toggle" @click="toggleSubkeys(key.id)">
                <svg :class="{ expanded: expandedSubkeys.has(key.id) }" viewBox="0 0 16 16" width="12" height="12" fill="none">
                  <path d="M4 6l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                </svg>
                <span>{{ key.subkeys.length }} 个子密钥</span>
              </div>
              <div v-if="expandedSubkeys.has(key.id)" class="subkeys-list">
                <div v-for="subkey in key.subkeys" :key="subkey.id" class="subkey-item">
                  <span class="subkey-id">{{ subkey.key_id }}</span>
                  <span class="subkey-algo">{{ subkey.key_algorithm }}</span>
                  <span v-if="subkey.can_sign" class="subkey-cap">[签名]</span>
                  <span v-if="subkey.can_encrypt" class="subkey-cap">[加密]</span>
                </div>
              </div>
            </div>
          </div>
          <div class="key-actions">
            <button v-if="!key.revoked" class="btn btn-warning-outline" @click="confirmRevoke(key)">
              撤销
            </button>
            <button class="btn btn-danger-outline" @click="confirmDelete(key)">
              删除
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 添加密钥弹窗 -->
    <div v-if="showAddModal" class="modal-overlay" @click.self="showAddModal = false">
      <div class="modal-content modal-large">
        <div class="modal-header">
          <h3>添加 GPG 密钥</h3>
          <button class="modal-close" @click="showAddModal = false">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
        <form @submit.prevent="addKey">
          <div class="form-group">
            <label for="key-content">GPG 公钥</label>
            <textarea
              id="key-content"
              v-model="newKey"
              class="form-input mono"
              rows="10"
              placeholder="粘贴您的 ASCII-armored GPG 公钥
-----BEGIN PGP PUBLIC KEY BLOCK-----
...
-----END PGP PUBLIC KEY BLOCK-----"
              required
            ></textarea>
            <p class="form-hint">
              您可以使用 <code>gpg --armor --export your@email.com</code> 导出您的公钥
            </p>
          </div>

          <div v-if="addError" class="alert alert-error">
            {{ addError }}
          </div>

          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" @click="showAddModal = false">
              取消
            </button>
            <button type="submit" class="btn btn-primary" :disabled="adding">
              {{ adding ? '添加中...' : '添加密钥' }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 撤销确认弹窗 -->
    <div v-if="keyToRevoke" class="modal-overlay" @click.self="keyToRevoke = null">
      <div class="modal-content modal-warning">
        <h3>撤销 GPG 密钥</h3>
        <p>确定要撤销 GPG 密钥 <strong>{{ keyToRevoke.primary_key_id }}</strong>？</p>
        <p class="warning">撤销后，使用此密钥签名的提交将显示为"已撤销"状态。此操作不可逆。</p>
        <div class="modal-actions">
          <button class="btn btn-secondary" @click="keyToRevoke = null">取消</button>
          <button class="btn btn-warning" @click="revokeKey(keyToRevoke)">撤销密钥</button>
        </div>
      </div>
    </div>

    <!-- 删除确认弹窗 -->
    <div v-if="keyToDelete" class="modal-overlay" @click.self="keyToDelete = null">
      <div class="modal-content modal-danger">
        <h3>删除 GPG 密钥</h3>
        <p>确定要删除 GPG 密钥 <strong>{{ keyToDelete.primary_key_id }}</strong>？</p>
        <p class="warning">删除后，使用此密钥签名的提交将显示为"未知密钥"状态。</p>
        <div class="modal-actions">
          <button class="btn btn-secondary" @click="keyToDelete = null">取消</button>
          <button class="btn btn-danger" @click="deleteKey(keyToDelete)">删除密钥</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import apiClient from '@/api'

interface GpgKeySubkey {
  id: number
  key_id: string
  fingerprint: string
  key_algorithm: string
  key_size: number | null
  can_sign: boolean
  can_encrypt: boolean
  key_created_at: string | null
  key_expires_at: string | null
  revoked: boolean
}

interface GpgKey {
  id: number
  primary_key_id: string
  fingerprint: string
  key_algorithm: string
  key_size: number | null
  emails: string[]
  can_sign: boolean
  can_encrypt: boolean
  can_certify: boolean
  key_created_at: string | null
  key_expires_at: string | null
  verified: boolean
  revoked: boolean
  subkeys: GpgKeySubkey[]
  last_used_at: string | null
  created_at: string
}

const searchQuery = ref('')
const keys = ref<GpgKey[]>([])
const loading = ref(true)
const adding = ref(false)
const addError = ref('')
const keyToRevoke = ref<GpgKey | null>(null)
const keyToDelete = ref<GpgKey | null>(null)
const showAddModal = ref(false)
const expandedSubkeys = ref<Set<number>>(new Set())

const newKey = ref('')

const loadKeys = async () => {
  loading.value = true
  try {
    const response = await apiClient.client.get('/user/gpg_keys')
    keys.value = response.data
  } catch (error: any) {
    console.error('Failed to load GPG keys:', error)
  } finally {
    loading.value = false
  }
}

const addKey = async () => {
  adding.value = true
  addError.value = ''

  try {
    const response = await apiClient.client.post('/user/gpg_keys', {
      key: newKey.value.trim()
    })
    keys.value.unshift(response.data)
    newKey.value = ''
    showAddModal.value = false
  } catch (error: any) {
    addError.value = error.response?.data?.message || 'Failed to add GPG key'
  } finally {
    adding.value = false
  }
}

const confirmRevoke = (key: GpgKey) => {
  keyToRevoke.value = key
}

const revokeKey = async (key: GpgKey) => {
  try {
    await apiClient.client.post(`/user/gpg_keys/${key.id}/revoke`)
    // Update the key in the list
    const idx = keys.value.findIndex(k => k.id === key.id)
    if (idx >= 0) {
      keys.value[idx].revoked = true
    }
  } catch (error: any) {
    console.error('Failed to revoke GPG key:', error)
  } finally {
    keyToRevoke.value = null
  }
}

const confirmDelete = (key: GpgKey) => {
  keyToDelete.value = key
}

const deleteKey = async (key: GpgKey) => {
  try {
    await apiClient.client.delete(`/user/gpg_keys/${key.id}`)
    keys.value = keys.value.filter(k => k.id !== key.id)
  } catch (error: any) {
    console.error('Failed to delete GPG key:', error)
  } finally {
    keyToDelete.value = null
  }
}

const toggleSubkeys = (keyId: number) => {
  if (expandedSubkeys.value.has(keyId)) {
    expandedSubkeys.value.delete(keyId)
  } else {
    expandedSubkeys.value.add(keyId)
  }
}

const isExpired = (key: GpgKey): boolean => {
  if (!key.key_expires_at) return false
  return new Date(key.key_expires_at) < new Date()
}

const formatFingerprint = (fp: string): string => {
  // Format fingerprint as groups of 4 characters
  return fp.replace(/(.{4})/g, '$1 ').trim()
}

const formatDate = (dateStr: string | null) => {
  if (!dateStr) return '未知'
  const date = new Date(dateStr)
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'short',
    day: 'numeric'
  })
}

onMounted(() => {
  loadKeys()
})
</script>

<style lang="scss" scoped>
.gpg-keys-page {
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
  margin-bottom: 24px;
  
  h1 {
    font-size: 20px;
    font-weight: 600;
    color: #303030;
    margin: 0 0 8px 0;
  }
  
  .section-description {
    font-size: 14px;
    color: #737278;
    margin: 0;
    line-height: 1.5;
  }
}

.keys-card {
  border: 1px solid #dcdcde;
  border-radius: 4px;
  background: #fff;
}

.keys-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid #dcdcde;
  
  .keys-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 600;
    color: #303030;
    
    .key-icon {
      color: #737278;
    }
    
    .keys-count {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 20px;
      height: 20px;
      padding: 0 6px;
      font-size: 12px;
      font-weight: 500;
      background: #f0f0f2;
      color: #737278;
      border-radius: 10px;
    }
  }
}

.btn-add {
  padding: 8px 16px;
  font-size: 14px;
  font-weight: 500;
  color: #303030;
  background: #fff;
  border: 1px solid #dcdcde;
  border-radius: 4px;
  cursor: pointer;
  
  &:hover {
    background: #f0f0f2;
  }
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
  color: #737278;
  
  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid #dcdcde;
    border-top-color: #1f75cb;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: 12px;
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 64px 24px;
  text-align: center;
  
  .empty-icon {
    margin-bottom: 24px;
  }
  
  .empty-title {
    font-size: 16px;
    font-weight: 500;
    color: #303030;
    margin: 0 0 8px 0;
  }
  
  .empty-subtitle {
    font-size: 14px;
    color: #737278;
    margin: 0;
  }
}

.keys-list {
  .key-item {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    padding: 16px 20px;
    border-bottom: 1px solid #dcdcde;
    
    &:last-child {
      border-bottom: none;
    }
  }
  
  .key-icon-wrapper {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #f0f0f2;
    border-radius: 4px;
    color: #737278;
    flex-shrink: 0;
    
    &.verified {
      background: #ddf4e4;
      color: #108548;
    }
    
    &.revoked {
      background: #fcf1ef;
      color: #dd2b0e;
    }
  }
  
  .key-info {
    flex: 1;
    min-width: 0;
  }
  
  .key-header-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 6px;
    flex-wrap: wrap;
  }
  
  .key-id {
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
    font-size: 14px;
    font-weight: 600;
    color: #303030;
  }
  
  .key-badges {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  
  .badge {
    font-size: 11px;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: 10px;
    
    &.badge-verified {
      background: #ddf4e4;
      color: #108548;
    }
    
    &.badge-unverified {
      background: #fdf1ce;
      color: #ab6100;
    }
    
    &.badge-revoked {
      background: #fcf1ef;
      color: #dd2b0e;
    }
    
    &.badge-expired {
      background: #f0f0f2;
      color: #737278;
    }
    
    &.badge-algo {
      background: #e9e9f0;
      color: #525159;
    }
  }
  
  .key-fingerprint {
    display: block;
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
    font-size: 12px;
    color: #737278;
    margin-bottom: 8px;
    word-break: break-all;
    letter-spacing: 0.5px;
  }
  
  .key-emails {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
    flex-wrap: wrap;
    
    .email-icon {
      color: #737278;
    }
    
    .email-tag {
      font-size: 12px;
      color: #1f75cb;
      background: #e8f0f8;
      padding: 2px 8px;
      border-radius: 4px;
    }
    
    .no-emails {
      font-size: 12px;
      color: #868686;
      font-style: italic;
    }
  }
  
  .key-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    font-size: 12px;
    color: #868686;
    
    .never-used {
      color: #ab6100;
    }
  }
  
  .key-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
  
  .subkeys-section {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px dashed #dcdcde;
  }
  
  .subkeys-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: #1f75cb;
    cursor: pointer;
    
    &:hover {
      text-decoration: underline;
    }
    
    svg {
      transition: transform 0.2s;
      
      &.expanded {
        transform: rotate(180deg);
      }
    }
  }
  
  .subkeys-list {
    margin-top: 8px;
    padding-left: 20px;
  }
  
  .subkey-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: #737278;
    margin-bottom: 4px;
    
    .subkey-id {
      font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
    }
    
    .subkey-algo {
      color: #868686;
    }
    
    .subkey-cap {
      color: #525159;
    }
  }
}

.btn {
  padding: 8px 16px;
  font-size: 14px;
  font-weight: 500;
  border-radius: 4px;
  cursor: pointer;
  border: none;
  
  &.btn-primary {
    background: #1f75cb;
    color: white;
    
    &:hover:not(:disabled) {
      background: #1068bf;
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
      background: #f0f0f2;
    }
  }
  
  &.btn-warning {
    background: #ab6100;
    color: white;
    
    &:hover {
      background: #8f5000;
    }
  }
  
  &.btn-danger {
    background: #dd2b0e;
    color: white;
    
    &:hover {
      background: #c91c00;
    }
  }
  
  &.btn-warning-outline {
    background: transparent;
    color: #ab6100;
    border: 1px solid #dcdcde;
    
    &:hover {
      background: #fdf1ce;
      border-color: #ab6100;
    }
  }
  
  &.btn-danger-outline {
    background: transparent;
    color: #dd2b0e;
    border: 1px solid #dcdcde;
    
    &:hover {
      background: #fcf1ef;
      border-color: #dd2b0e;
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
  background: #fff;
  border-radius: 8px;
  padding: 24px;
  max-width: 500px;
  width: 90%;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
  
  &.modal-large {
    max-width: 600px;
  }
  
  h3 {
    font-size: 18px;
    font-weight: 600;
    margin: 0 0 16px 0;
    color: #303030;
  }
  
  p {
    font-size: 14px;
    color: #737278;
    margin: 0 0 12px 0;
    
    strong {
      color: #303030;
      font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
    }
  }
  
  .warning {
    color: #dd2b0e;
    font-size: 13px;
  }
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  
  h3 {
    margin: 0;
  }
}

.modal-close {
  background: none;
  border: none;
  cursor: pointer;
  color: #737278;
  padding: 4px;
  
  &:hover {
    color: #303030;
  }
}

.modal-actions {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
  margin-top: 20px;
}

.form-group {
  margin-bottom: 16px;
  
  label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    color: #303030;
    margin-bottom: 8px;
  }
}

.form-input {
  width: 100%;
  padding: 10px 12px;
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
  
  &.mono {
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
    font-size: 12px;
  }
}

textarea.form-input {
  resize: vertical;
  min-height: 200px;
}

.form-hint {
  margin: 6px 0 0 0;
  font-size: 12px;
  color: #737278;
  
  code {
    background: #f0f0f2;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
  }
}

.alert {
  padding: 12px 16px;
  border-radius: 4px;
  font-size: 14px;
  
  &.alert-error {
    background: #fcf1ef;
    border: 1px solid #dd2b0e;
    color: #dd2b0e;
  }
}
</style>
