<template>
  <div class="ssh-keys-page">
    <!-- 面包屑导航 -->
    <div class="breadcrumb">
      <router-link to="/-/profile">用户设置</router-link>
      <span class="separator">/</span>
      <span>SSH Keys</span>
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
      <h1>SSH Keys</h1>
      <p class="section-description">
        SSH密钥用于在您的电脑和GitFox建立安全连接。SSH 指纹验证客户端是否连接到正确的主机。检查<a href="#">当前实例配置</a>。
      </p>
    </section>

    <!-- SSH 密钥列表 -->
    <div class="keys-card">
      <div class="keys-header">
        <div class="keys-title">
          <span>您的 SSH 密钥</span>
          <svg class="key-icon" viewBox="0 0 16 16" width="16" height="16" fill="none">
            <path d="M14 2l-1 1m-4 4a3 3 0 11-4.243 4.243A3 3 0 019 7zm0 0L11 5m0 0l2 2 2-2-2-2m-2 2l1-1" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
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
            <rect x="26" y="20" width="12" height="8" rx="2" fill="#8b5cf6"/>
            <path d="M28 28v16h8V28" stroke="#8b5cf6" stroke-width="2"/>
            <rect x="29" y="34" width="6" height="3" fill="#c4b5fd"/>
          </svg>
        </div>
        <p class="empty-title">此帐户没有访问权限的 SSH 密钥</p>
      </div>

      <div v-else class="keys-list">
        <div v-for="key in keys" :key="key.id" class="key-item">
          <div class="key-icon-wrapper">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M14 2l-1 1m-4 4a3 3 0 11-4.243 4.243A3 3 0 019 7zm0 0L11 5m0 0l2 2 2-2-2-2m-2 2l1-1" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <div class="key-info">
            <div class="key-title">{{ key.title }}</div>
            <code class="key-fingerprint">{{ key.fingerprint }}</code>
            <div class="key-meta">
              <span v-if="key.last_used_at">最后使用: {{ formatDate(key.last_used_at) }}</span>
              <span v-else class="never-used">从未使用</span>
              <span>·</span>
              <span>添加于: {{ formatDate(key.created_at) }}</span>
            </div>
          </div>
          <button class="btn btn-danger-outline" @click="confirmDelete(key)">
            删除
          </button>
        </div>
      </div>
    </div>

    <!-- 添加密钥弹窗 -->
    <div v-if="showAddModal" class="modal-overlay" @click.self="showAddModal = false">
      <div class="modal-content">
        <div class="modal-header">
          <h3>添加 SSH 密钥</h3>
          <button class="modal-close" @click="showAddModal = false">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
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

    <!-- 删除确认弹窗 -->
    <div v-if="keyToDelete" class="modal-overlay" @click.self="keyToDelete = null">
      <div class="modal-content modal-danger">
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

const searchQuery = ref('')
const keys = ref<SshKey[]>([])
const loading = ref(true)
const adding = ref(false)
const addError = ref('')
const keyToDelete = ref<SshKey | null>(null)
const showAddModal = ref(false)

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
    showAddModal.value = false
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
  try {
    await apiClient.client.delete(`/user/ssh_keys/${key.id}`)
    keys.value = keys.value.filter(k => k.id !== key.id)
  } catch (error: any) {
    console.error('Failed to delete SSH key:', error)
  } finally {
    keyToDelete.value = null
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
  loadKeys()
})
</script>

<style lang="scss" scoped>
.ssh-keys-page {
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
    
    a {
      color: #1f75cb;
      text-decoration: none;
      
      &:hover {
        text-decoration: underline;
      }
    }
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
  }
  
  .key-info {
    flex: 1;
    min-width: 0;
  }
  
  .key-title {
    font-size: 14px;
    font-weight: 600;
    color: #303030;
    margin-bottom: 4px;
  }
  
  .key-fingerprint {
    display: block;
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
    font-size: 12px;
    color: #737278;
    margin-bottom: 8px;
    word-break: break-all;
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
  
  &.btn-danger {
    background: #dd2b0e;
    color: white;
    
    &:hover {
      background: #c91c00;
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
  min-height: 80px;
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
