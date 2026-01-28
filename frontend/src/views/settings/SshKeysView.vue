<template>
  <div class="settings-ssh-keys">
    <div class="page-header">
      <h1>SSH Keys</h1>
      <p class="text-secondary">
        SSH keys allow you to establish a secure connection between your computer and GitFox.
      </p>
    </div>

    <!-- Add new key form -->
    <div class="add-key-section card">
      <div class="card-header">
        <h3>Add an SSH key</h3>
      </div>
      <div class="card-body">
        <form @submit.prevent="addKey">
          <div class="form-group">
            <label for="key-title">Title</label>
            <input
              id="key-title"
              v-model="newKey.title"
              type="text"
              class="form-control"
              placeholder="e.g., My MacBook Pro"
              required
            />
            <small class="form-text text-muted">
              A descriptive label for this key
            </small>
          </div>

          <div class="form-group">
            <label for="key-content">Key</label>
            <textarea
              id="key-content"
              v-model="newKey.key"
              class="form-control"
              rows="6"
              placeholder="Paste your public SSH key here (starts with 'ssh-rsa', 'ssh-ed25519', etc.)"
              required
            ></textarea>
            <small class="form-text text-muted">
              Paste your public SSH key, which is usually found in
              <code>~/.ssh/id_ed25519.pub</code> or <code>~/.ssh/id_rsa.pub</code>
            </small>
          </div>

          <div class="form-actions">
            <button type="submit" class="btn btn-primary" :disabled="adding">
              <span v-if="adding">Adding...</span>
              <span v-else>Add key</span>
            </button>
          </div>

          <div v-if="addError" class="alert alert-danger mt-3">
            {{ addError }}
          </div>
        </form>
      </div>
    </div>

    <!-- Existing keys -->
    <div class="keys-list-section">
      <h3>Your SSH keys ({{ keys.length }})</h3>

      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <span>Loading SSH keys...</span>
      </div>

      <div v-else-if="keys.length === 0" class="empty-state">
        <div class="empty-icon">🔑</div>
        <p>You haven't added any SSH keys yet.</p>
        <p class="text-secondary">
          Add an SSH key to start pushing code to GitFox repositories.
        </p>
      </div>

      <div v-else class="keys-list">
        <div v-for="key in keys" :key="key.id" class="key-item card">
          <div class="key-info">
            <div class="key-header">
              <span class="key-icon">🔐</span>
              <span class="key-title">{{ key.title }}</span>
              <span class="key-type badge">{{ key.key_type }}</span>
            </div>
            <div class="key-details">
              <code class="key-fingerprint">{{ key.fingerprint }}</code>
              <div class="key-meta">
                <span v-if="key.last_used_at" class="last-used">
                  Last used: {{ formatDate(key.last_used_at) }}
                </span>
                <span v-else class="last-used never">
                  Never used
                </span>
                <span class="created-at">
                  Added: {{ formatDate(key.created_at) }}
                </span>
              </div>
            </div>
          </div>
          <div class="key-actions">
            <button
              class="btn btn-danger btn-sm"
              @click="confirmDelete(key)"
              :disabled="deleting === key.id"
            >
              <span v-if="deleting === key.id">Removing...</span>
              <span v-else>Remove</span>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Delete confirmation modal -->
    <div v-if="keyToDelete" class="modal-overlay" @click.self="keyToDelete = null">
      <div class="modal-content">
        <h3>Remove SSH Key</h3>
        <p>
          Are you sure you want to remove the SSH key
          <strong>"{{ keyToDelete.title }}"</strong>?
        </p>
        <p class="text-danger">
          You will not be able to push or pull from GitFox using this key.
        </p>
        <div class="modal-actions">
          <button class="btn btn-secondary" @click="keyToDelete = null">
            Cancel
          </button>
          <button class="btn btn-danger" @click="deleteKey(keyToDelete)">
            Remove key
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '@/api'

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
    const response = await api.get('/user/ssh_keys')
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
    const response = await api.post('/user/ssh_keys', {
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
    await api.delete(`/user/ssh_keys/${key.id}`)
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
.settings-ssh-keys {
  max-width: 900px;
  margin: 0 auto;
  padding: 24px;
}

.page-header {
  margin-bottom: 24px;

  h1 {
    margin: 0 0 8px 0;
    font-size: 24px;
    font-weight: 600;
  }

  .text-secondary {
    color: #666;
    margin: 0;
  }
}

.card {
  background: #fff;
  border: 1px solid #e1e4e8;
  border-radius: 8px;
  margin-bottom: 24px;
}

.card-header {
  padding: 16px 20px;
  border-bottom: 1px solid #e1e4e8;

  h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }
}

.card-body {
  padding: 20px;
}

.form-group {
  margin-bottom: 16px;

  label {
    display: block;
    margin-bottom: 6px;
    font-weight: 500;
    font-size: 14px;
  }

  .form-control {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 14px;
    font-family: inherit;

    &:focus {
      outline: none;
      border-color: #6366f1;
      box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
    }
  }

  textarea.form-control {
    font-family: 'Monaco', 'Menlo', monospace;
    font-size: 12px;
    resize: vertical;
  }

  .form-text {
    display: block;
    margin-top: 4px;
    font-size: 12px;
    color: #6b7280;

    code {
      background: #f3f4f6;
      padding: 2px 4px;
      border-radius: 4px;
      font-size: 11px;
    }
  }
}

.form-actions {
  margin-top: 20px;
}

.btn {
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;

  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
}

.btn-primary {
  background: #6366f1;
  color: white;

  &:hover:not(:disabled) {
    background: #4f46e5;
  }
}

.btn-secondary {
  background: #e5e7eb;
  color: #374151;

  &:hover:not(:disabled) {
    background: #d1d5db;
  }
}

.btn-danger {
  background: #ef4444;
  color: white;

  &:hover:not(:disabled) {
    background: #dc2626;
  }
}

.btn-sm {
  padding: 6px 12px;
  font-size: 13px;
}

.alert {
  padding: 12px 16px;
  border-radius: 6px;
  font-size: 14px;
}

.alert-danger {
  background: #fef2f2;
  color: #dc2626;
  border: 1px solid #fecaca;
}

.keys-list-section {
  h3 {
    font-size: 16px;
    font-weight: 600;
    margin-bottom: 16px;
  }
}

.loading-state,
.empty-state {
  text-align: center;
  padding: 40px 20px;
  background: #f9fafb;
  border-radius: 8px;
}

.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid #e5e7eb;
  border-top-color: #6366f1;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin: 0 auto 12px;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.empty-state {
  .empty-icon {
    font-size: 48px;
    margin-bottom: 16px;
  }

  p {
    margin: 0 0 8px 0;
  }
}

.keys-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.key-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
}

.key-info {
  flex: 1;
  min-width: 0;
}

.key-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;

  .key-icon {
    font-size: 18px;
  }

  .key-title {
    font-weight: 600;
    font-size: 15px;
  }

  .badge {
    padding: 2px 8px;
    background: #e0e7ff;
    color: #4338ca;
    border-radius: 12px;
    font-size: 11px;
    font-weight: 500;
  }
}

.key-details {
  .key-fingerprint {
    display: block;
    font-family: 'Monaco', 'Menlo', monospace;
    font-size: 12px;
    color: #6b7280;
    background: #f3f4f6;
    padding: 4px 8px;
    border-radius: 4px;
    margin-bottom: 8px;
  }

  .key-meta {
    display: flex;
    gap: 16px;
    font-size: 12px;
    color: #9ca3af;

    .last-used.never {
      color: #d97706;
    }
  }
}

.key-actions {
  margin-left: 16px;
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
  background: white;
  padding: 24px;
  border-radius: 12px;
  max-width: 400px;
  width: 90%;

  h3 {
    margin: 0 0 16px 0;
  }

  p {
    margin: 0 0 12px 0;
    color: #4b5563;
  }

  .text-danger {
    color: #dc2626;
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
