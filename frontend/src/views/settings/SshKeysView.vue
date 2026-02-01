<template>
  <div class="settings-page">
    <!-- 面包屑导航 -->
    <div class="breadcrumb">
      <router-link to="/-/profile">用户设置</router-link>
      <span class="separator">/</span>
      <span>SSH 密钥</span>
    </div>

    <!-- 页面标题 -->
    <div class="page-header">
      <h1>SSH 密钥</h1>
      <p class="description">
        SSH 密钥用于在您的电脑和 GitFox 建立安全连接。
      </p>
    </div>

    <!-- 添加新密钥 -->
    <div class="settings-content">
      <section class="form-section">
        <h2>添加 SSH 密钥</h2>
        <form @submit.prevent="addKey">
          <div class="form-group">
            <label for="key-title">标题</label>
            <input
              id="key-title"
              v-model="newKey.title"
              type="text"
              class="form-input"
              placeholder="例如：My MacBook Pro"
              required
            />
            <p class="form-hint">为此密钥添加一个描述性标签</p>
          </div>

          <div class="form-group">
            <label for="key-content">密钥</label>
            <textarea
              id="key-content"
              v-model="newKey.key"
              class="form-input mono"
              rows="4"
              placeholder="粘贴您的公钥（以 'ssh-rsa'、'ssh-ed25519' 等开头）"
              required
            ></textarea>
            <p class="form-hint">
              公钥通常在 <code>~/.ssh/id_ed25519.pub</code> 或 <code>~/.ssh/id_rsa.pub</code>
            </p>
          </div>

          <div class="form-actions-inline">
            <button type="submit" class="btn btn-primary" :disabled="adding">
              {{ adding ? '添加中...' : '添加密钥' }}
            </button>
          </div>

          <div v-if="addError" class="alert alert-error">
            {{ addError }}
          </div>
        </form>
      </section>
    </div>

    <!-- 现有密钥列表 -->
    <div class="keys-section">
      <div class="keys-header">
        <h2>您的 SSH 密钥</h2>
        <span class="keys-count">{{ keys.length }}</span>
      </div>

      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <span>加载中...</span>
      </div>

      <div v-else-if="keys.length === 0" class="empty-state">
        <div class="empty-icon">
          <svg viewBox="0 0 24 24" fill="none" width="64" height="64">
            <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 11-7.778 7.778 5.5 5.5 0 017.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <p class="empty-title">此帐户没有访问权限的 SSH 密钥</p>
        <p class="empty-description">添加 SSH 密钥以开始向 GitFox 仓库推送代码。</p>
      </div>

      <div v-else class="keys-list">
        <div v-for="key in keys" :key="key.id" class="key-item">
          <div class="key-icon">
            <svg viewBox="0 0 24 24" fill="none" width="20" height="20">
              <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 11-7.778 7.778 5.5 5.5 0 017.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <div class="key-info">
            <div class="key-title">{{ key.title }}</div>
            <code class="key-fingerprint">{{ key.fingerprint }}</code>
            <div class="key-meta">
              <span v-if="key.last_used_at">最后使用: {{ formatDate(key.last_used_at) }}</span>
              <span v-else class="never-used">从未使用</span>
              <span>添加于: {{ formatDate(key.created_at) }}</span>
            </div>
          </div>
          <div class="key-actions">
            <button
              class="btn btn-danger-outline btn-sm"
              @click="confirmDelete(key)"
              :disabled="deleting === key.id"
            >
              {{ deleting === key.id ? '删除中...' : '删除' }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 删除确认弹窗 -->
    <div v-if="keyToDelete" class="modal-overlay" @click.self="keyToDelete = null">
      <div class="modal-content">
        <h3>删除 SSH 密钥</h3>
        <p>确定要删除 SSH 密钥 <strong>"{{ keyToDelete.title }}"</strong>？</p>
        <p class="warning">删除后将无法使用此密钥向 GitFox 推送或拉取代码。</p>
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

interface SshKey {
  id: number
  title: string
  key_type: string
  fingerprint: string
  last_used_at: string | null
  expires_at: string | null
  created_at: string
}

const keys = ref<SshKey[]>([])
const loading = ref(true)
const adding = ref(false)
const deleting = ref<number | null>(null)
const addError = ref('')
const keyToDelete = ref<SshKey | null>(null)

const newKey = ref({
  title: '',
  key: ''
})

const loadKeys = async () => {
  loading.value = true
  try {
    const response = await apiClient.client.get('/user/ssh_keys')
    keys.value = response.data
  } catch (error: any) {
    console.error('Failed to load SSH keys:', error)
  } finally {
    loading.value = false
  }
}

const addKey = async () => {
  adding.value = true
  addError.value = ''

  try {
    const response = await apiClient.client.post('/user/ssh_keys', {
      title: newKey.value.title,
      key: newKey.value.key.trim()
    })
    keys.value.unshift(response.data)
    newKey.value = { title: '', key: '' }
  } catch (error: any) {
    addError.value = error.response?.data?.message || 'Failed to add SSH key'
  } finally {
    adding.value = false
  }
}

const confirmDelete = (key: SshKey) => {
  keyToDelete.value = key
}

const deleteKey = async (key: SshKey) => {
  deleting.value = key.id
  keyToDelete.value = null

  try {
    await apiClient.client.delete(`/user/ssh_keys/${key.id}`)
    keys.value = keys.value.filter(k => k.id !== key.id)
  } catch (error: any) {
    console.error('Failed to delete SSH key:', error)
  } finally {
    deleting.value = null
  }
}

const formatDate = (dateStr: string) => {
  const date = new Date(dateStr)
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  })
}

onMounted(() => {
  loadKeys()
})
</script>

<style lang="scss" scoped>
.settings-page {
  padding: 24px 32px;
  max-width: 900px;
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  margin-bottom: 16px;
  
  a {
    color: var(--text-secondary, #8b949e);
    text-decoration: none;
    
    &:hover {
      color: var(--text-link, #58a6ff);
      text-decoration: underline;
    }
  }
  
  .separator {
    color: var(--text-secondary, #8b949e);
  }
  
  span:last-child {
    color: var(--text-primary, #c9d1d9);
  }
}

.page-header {
  margin-bottom: 24px;
  
  h1 {
    font-size: 24px;
    font-weight: 600;
    margin: 0 0 8px 0;
    color: var(--text-primary, #c9d1d9);
  }
  
  .description {
    font-size: 14px;
    color: var(--text-secondary, #8b949e);
    margin: 0;
  }
}

.settings-content {
  background: var(--bg-secondary, #161b22);
  border: 1px solid var(--border-color, #30363d);
  border-radius: 6px;
  margin-bottom: 24px;
}

.form-section {
  padding: 24px;
  
  h2 {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 16px 0;
    color: var(--text-primary, #c9d1d9);
  }
}

.form-group {
  margin-bottom: 16px;
  
  label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary, #c9d1d9);
    margin-bottom: 8px;
  }
}

.form-input {
  width: 100%;
  max-width: 480px;
  padding: 8px 12px;
  font-size: 14px;
  color: var(--text-primary, #c9d1d9);
  background: var(--bg-primary, #0d1117);
  border: 1px solid var(--border-color, #30363d);
  border-radius: 6px;
  
  &:focus {
    outline: none;
    border-color: var(--color-primary, #58a6ff);
    box-shadow: 0 0 0 3px rgba(88, 166, 255, 0.15);
  }
  
  &::placeholder {
    color: var(--text-muted, #484f58);
  }
  
  &.mono {
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
    font-size: 12px;
  }
}

textarea.form-input {
  resize: vertical;
  min-height: 80px;
  max-width: 100%;
}

.form-hint {
  margin: 6px 0 0 0;
  font-size: 12px;
  color: var(--text-secondary, #8b949e);
  
  code {
    background: var(--bg-tertiary, #21262d);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
  }
}

.form-actions-inline {
  margin-top: 16px;
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 8px 16px;
  font-size: 14px;
  font-weight: 500;
  border-radius: 6px;
  cursor: pointer;
  border: none;
  transition: background 0.2s;
  
  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  
  &.btn-primary {
    background: var(--color-success, #238636);
    color: white;
    
    &:hover:not(:disabled) {
      background: var(--color-success-hover, #2ea043);
    }
  }
  
  &.btn-secondary {
    background: var(--bg-tertiary, #21262d);
    border: 1px solid var(--border-color, #30363d);
    color: var(--text-primary, #c9d1d9);
    
    &:hover:not(:disabled) {
      background: var(--bg-secondary, #30363d);
    }
  }
  
  &.btn-danger {
    background: #da3633;
    color: white;
    
    &:hover:not(:disabled) {
      background: #f85149;
    }
  }
  
  &.btn-danger-outline {
    background: transparent;
    border: 1px solid var(--border-color, #30363d);
    color: #f85149;
    
    &:hover:not(:disabled) {
      background: rgba(248, 81, 73, 0.1);
      border-color: #f85149;
    }
  }
  
  &.btn-sm {
    padding: 4px 12px;
    font-size: 12px;
  }
}

.alert {
  margin-top: 16px;
  padding: 12px 16px;
  border-radius: 6px;
  font-size: 14px;
  
  &.alert-error {
    background: rgba(248, 81, 73, 0.15);
    border: 1px solid #f85149;
    color: #f85149;
  }
}

.keys-section {
  background: var(--bg-secondary, #161b22);
  border: 1px solid var(--border-color, #30363d);
  border-radius: 6px;
}

.keys-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px 24px;
  border-bottom: 1px solid var(--border-color, #30363d);
  
  h2 {
    font-size: 14px;
    font-weight: 600;
    margin: 0;
    color: var(--text-primary, #c9d1d9);
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
    background: var(--bg-tertiary, #21262d);
    color: var(--text-secondary, #8b949e);
    border-radius: 10px;
  }
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
  color: var(--text-secondary, #8b949e);
  
  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-color, #30363d);
    border-top-color: var(--text-link, #58a6ff);
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
    width: 96px;
    height: 96px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-tertiary, #21262d);
    border-radius: 50%;
    margin-bottom: 24px;
    color: var(--text-secondary, #8b949e);
  }
  
  .empty-title {
    font-size: 16px;
    font-weight: 500;
    color: var(--text-primary, #c9d1d9);
    margin: 0 0 8px 0;
  }
  
  .empty-description {
    font-size: 14px;
    color: var(--text-secondary, #8b949e);
    margin: 0;
  }
}

.keys-list {
  .key-item {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    padding: 16px 24px;
    border-bottom: 1px solid var(--border-color, #30363d);
    
    &:last-child {
      border-bottom: none;
    }
  }
  
  .key-icon {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-tertiary, #21262d);
    border-radius: 6px;
    color: var(--text-secondary, #8b949e);
  }
  
  .key-info {
    flex: 1;
    min-width: 0;
  }
  
  .key-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary, #c9d1d9);
    margin-bottom: 4px;
  }
  
  .key-fingerprint {
    display: block;
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
    font-size: 12px;
    color: var(--text-secondary, #8b949e);
    margin-bottom: 8px;
    word-break: break-all;
  }
  
  .key-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    font-size: 12px;
    color: var(--text-muted, #484f58);
    
    .never-used {
      color: #d29922;
    }
  }
  
  .key-actions {
    flex-shrink: 0;
  }
}

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
}

.modal-content {
  background: var(--bg-secondary, #161b22);
  border: 1px solid var(--border-color, #30363d);
  border-radius: 12px;
  padding: 24px;
  max-width: 400px;
  width: 90%;
  
  h3 {
    font-size: 18px;
    font-weight: 600;
    margin: 0 0 16px 0;
    color: var(--text-primary, #c9d1d9);
  }
  
  p {
    font-size: 14px;
    color: var(--text-secondary, #8b949e);
    margin: 0 0 12px 0;
    
    strong {
      color: var(--text-primary, #c9d1d9);
    }
  }
  
  .warning {
    color: #f85149;
    font-size: 13px;
  }
}

.modal-actions {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>
