<template>
  <div class="settings-page">
    <div class="page-header">
      <div class="header-content">
        <h1>OAuth 应用</h1>
        <p class="page-description">
          创建 OAuth 应用允许第三方服务通过 GitFox 进行用户认证。
          应用可以请求访问用户的 GitFox 账户信息。
        </p>
      </div>
      <button class="btn btn-primary" @click="showCreateModal = true">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        创建应用
      </button>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>加载中...</span>
    </div>

    <template v-else>
      <div v-if="applications.length === 0" class="empty-state">
        <div class="empty-icon">
          <svg viewBox="0 0 64 64" width="64" height="64" fill="none">
            <rect x="8" y="12" width="48" height="40" rx="4" stroke="#93c5fd" stroke-width="2" fill="#dbeafe"/>
            <circle cx="32" cy="28" r="8" stroke="#3b82f6" stroke-width="2"/>
            <path d="M24 42h16" stroke="#3b82f6" stroke-width="2" stroke-linecap="round"/>
          </svg>
        </div>
        <p class="empty-title">还没有创建任何 OAuth 应用</p>
        <p class="empty-description">
          OAuth 应用允许第三方服务通过 GitFox 认证用户身份，
          并根据授权范围访问用户数据。
        </p>
        <button class="btn btn-primary" @click="showCreateModal = true">创建第一个应用</button>
      </div>

      <div v-else class="applications-list">
        <div v-for="app in applications" :key="app.id" class="application-card">
          <div class="app-icon">
            <img v-if="app.logo_url" :src="app.logo_url" :alt="app.name" />
            <svg v-else viewBox="0 0 24 24" width="32" height="32" fill="none">
              <rect x="3" y="3" width="18" height="18" rx="3" stroke="currentColor" stroke-width="1.5"/>
              <circle cx="12" cy="10" r="3" stroke="currentColor" stroke-width="1.5"/>
              <path d="M7 18c1-2 3-3 5-3s4 1 5 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </div>
          <div class="app-info">
            <div class="app-header">
              <h3>{{ app.name }}</h3>
              <span v-if="app.trusted" class="badge badge-trusted" title="受信任的应用跳过用户确认">受信任</span>
              <span v-if="!app.confidential" class="badge badge-public" title="公开客户端（SPA/移动应用）">公开</span>
            </div>
            <p v-if="app.description" class="app-description">{{ app.description }}</p>
            <div class="app-meta">
              <span><strong>Client ID:</strong> <code>{{ app.uid }}</code></span>
              <span v-if="app.homepage_url">
                <strong>主页:</strong> 
                <a :href="app.homepage_url" target="_blank" rel="noopener">{{ app.homepage_url }}</a>
              </span>
            </div>
            <div class="app-scopes">
              <span v-for="scope in app.scopes" :key="scope" class="scope-tag">{{ scope }}</span>
            </div>
          </div>
          <div class="app-actions">
            <button class="btn btn-icon" @click="editApplication(app)" title="编辑">
              <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
                <path d="M11.5 2.5l2 2-8 8H3.5v-2l8-8z" stroke="currentColor" stroke-width="1.2"/>
              </svg>
            </button>
            <button class="btn btn-icon" @click="confirmRegenerateSecret(app)" title="重新生成密钥">
              <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
                <path d="M2 8a6 6 0 0110.5-4M14 8a6 6 0 01-10.5 4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                <path d="M12.5 1v3h-3M3.5 15v-3h3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>
            <button class="btn btn-icon btn-danger" @click="confirmDelete(app)" title="删除">
              <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
                <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
            </button>
          </div>
        </div>
      </div>
    </template>

    <!-- Create/Edit Modal -->
    <div v-if="showCreateModal || editingApp" class="modal-overlay" @click.self="closeModal">
      <div class="modal-content modal-large">
        <div class="modal-header">
          <h3>{{ editingApp ? '编辑' : '创建' }} OAuth 应用</h3>
          <button class="modal-close" @click="closeModal">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <form @submit.prevent="saveApplication">
          <div class="modal-body">
            <div class="form-group">
              <label for="app-name">应用名称 <span class="required">*</span></label>
              <input
                id="app-name"
                v-model="form.name"
                type="text"
                class="form-input"
                placeholder="例如: My Awesome App"
                required
              />
            </div>

            <div class="form-group">
              <label for="app-redirect">回调 URL <span class="required">*</span></label>
              <textarea
                id="app-redirect"
                v-model="redirectUrisText"
                class="form-input"
                placeholder="https://your-app.com/oauth/callback&#10;每行一个 URL"
                rows="3"
                required
              ></textarea>
              <p class="form-hint">用户授权后将重定向到这些 URL。每行一个。</p>
            </div>

            <div class="form-row">
              <div class="form-group">
                <label for="app-homepage">主页 URL</label>
                <input
                  id="app-homepage"
                  v-model="form.homepage_url"
                  type="url"
                  class="form-input"
                  placeholder="https://your-app.com"
                />
              </div>
              <div class="form-group">
                <label for="app-logo">Logo URL</label>
                <input
                  id="app-logo"
                  v-model="form.logo_url"
                  type="url"
                  class="form-input"
                  placeholder="https://your-app.com/logo.png"
                />
              </div>
            </div>

            <div class="form-group">
              <label for="app-description">描述</label>
              <textarea
                id="app-description"
                v-model="form.description"
                class="form-input"
                placeholder="简要描述您的应用..."
                rows="2"
              ></textarea>
            </div>

            <div class="form-group">
              <label>权限范围</label>
              <div class="scopes-grid">
                <label v-for="scope in availableScopes" :key="scope.value" class="scope-checkbox">
                  <input type="checkbox" v-model="selectedScopes" :value="scope.value" />
                  <span class="scope-info">
                    <strong>{{ scope.value }}</strong>
                    <small>{{ scope.description }}</small>
                  </span>
                </label>
              </div>
            </div>

            <div class="form-group">
              <label class="checkbox-label">
                <input type="checkbox" v-model="form.confidential" />
                <span>机密客户端</span>
              </label>
              <p class="form-hint">
                机密客户端（服务器端应用）可以安全存储 client_secret。
                公开客户端（SPA、移动应用）不应存储 secret。
              </p>
            </div>
          </div>

          <div class="modal-footer">
            <button type="button" class="btn btn-secondary" @click="closeModal">取消</button>
            <button type="submit" class="btn btn-primary" :disabled="saving">
              {{ saving ? '保存中...' : (editingApp ? '保存' : '创建应用') }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- Secret Display Modal -->
    <div v-if="showSecretModal" class="modal-overlay" @click.self="showSecretModal = false">
      <div class="modal-content">
        <div class="modal-header">
          <h3>应用凭证</h3>
          <button class="modal-close" @click="showSecretModal = false">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
        <div class="modal-body">
          <div class="alert alert-warning">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M8 1L1 14h14L8 1z" stroke="currentColor" stroke-width="1.5"/>
              <path d="M8 6v4M8 11.5v.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <span>请立即复制并安全保存 Client Secret，关闭后将无法再次查看！</span>
          </div>
          
          <div class="credential-item">
            <label>Client ID</label>
            <div class="credential-value">
              <code>{{ newAppCredentials?.uid }}</code>
              <button class="btn btn-icon btn-sm" @click="copyToClipboard(newAppCredentials?.uid || '')" title="复制">
                <svg viewBox="0 0 16 16" width="14" height="14" fill="none">
                  <rect x="5" y="5" width="9" height="9" rx="1" stroke="currentColor" stroke-width="1.2"/>
                  <path d="M3 11V3a1 1 0 011-1h8" stroke="currentColor" stroke-width="1.2"/>
                </svg>
              </button>
            </div>
          </div>
          
          <div class="credential-item">
            <label>Client Secret</label>
            <div class="credential-value">
              <code>{{ newAppCredentials?.secret }}</code>
              <button class="btn btn-icon btn-sm" @click="copyToClipboard(newAppCredentials?.secret || '')" title="复制">
                <svg viewBox="0 0 16 16" width="14" height="14" fill="none">
                  <rect x="5" y="5" width="9" height="9" rx="1" stroke="currentColor" stroke-width="1.2"/>
                  <path d="M3 11V3a1 1 0 011-1h8" stroke="currentColor" stroke-width="1.2"/>
                </svg>
              </button>
            </div>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-primary" @click="showSecretModal = false">我已保存</button>
        </div>
      </div>
    </div>

    <!-- Delete Confirm Modal -->
    <div v-if="deletingApp" class="modal-overlay" @click.self="deletingApp = null">
      <div class="modal-content">
        <div class="modal-header">
          <h3>确认删除</h3>
          <button class="modal-close" @click="deletingApp = null">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
        <div class="modal-body">
          <p>确定要删除应用 <strong>{{ deletingApp.name }}</strong> 吗？</p>
          <p class="text-muted">删除后，所有使用此应用认证的用户将被登出，此操作不可撤销。</p>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" @click="deletingApp = null">取消</button>
          <button class="btn btn-danger" @click="deleteApplication" :disabled="deleting">
            {{ deleting ? '删除中...' : '确认删除' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import api from '@/api'
import type { OAuthApplication, OAuthApplicationWithSecret } from '@/types'

const loading = ref(true)
const saving = ref(false)
const deleting = ref(false)
const applications = ref<OAuthApplication[]>([])
const showCreateModal = ref(false)
const editingApp = ref<OAuthApplication | null>(null)
const deletingApp = ref<OAuthApplication | null>(null)
const showSecretModal = ref(false)
const newAppCredentials = ref<OAuthApplicationWithSecret | null>(null)

const form = reactive({
  name: '',
  redirect_uris: [] as string[],
  homepage_url: '',
  logo_url: '',
  description: '',
  confidential: true,
  scopes: [] as string[]
})

const redirectUrisText = computed({
  get: () => form.redirect_uris.join('\n'),
  set: (val: string) => {
    form.redirect_uris = val.split('\n').map(s => s.trim()).filter(s => s)
  }
})

const selectedScopes = computed({
  get: () => form.scopes,
  set: (val: string[]) => { form.scopes = val }
})

const availableScopes = [
  { value: 'read_user', description: '读取用户基本信息' },
  { value: 'read_api', description: '只读访问 API' },
  { value: 'api', description: '完全访问 API' },
  { value: 'read_repository', description: '读取仓库内容' },
  { value: 'write_repository', description: '写入仓库内容' },
  { value: 'openid', description: 'OpenID Connect 身份验证' },
  { value: 'profile', description: '读取用户详细资料' },
  { value: 'email', description: '读取用户邮箱' },
]

onMounted(async () => {
  try {
    applications.value = await api.oauthApplications.list()
  } catch (error) {
    console.error('Failed to load applications:', error)
  } finally {
    loading.value = false
  }
})

function resetForm() {
  form.name = ''
  form.redirect_uris = []
  form.homepage_url = ''
  form.logo_url = ''
  form.description = ''
  form.confidential = true
  form.scopes = ['read_user']
}

function closeModal() {
  showCreateModal.value = false
  editingApp.value = null
  resetForm()
}

function editApplication(app: OAuthApplication) {
  editingApp.value = app
  form.name = app.name
  form.redirect_uris = app.redirect_uris || []
  form.homepage_url = app.homepage_url || ''
  form.logo_url = app.logo_url || ''
  form.description = app.description || ''
  form.confidential = app.confidential
  form.scopes = app.scopes || ['read_user']
}

async function saveApplication() {
  saving.value = true
  try {
    if (editingApp.value) {
      const updated = await api.oauthApplications.update(editingApp.value.id, {
        name: form.name,
        redirect_uris: form.redirect_uris,
        homepage_url: form.homepage_url || undefined,
        logo_url: form.logo_url || undefined,
        description: form.description || undefined,
        confidential: form.confidential,
        scopes: form.scopes
      })
      const idx = applications.value.findIndex(a => a.id === editingApp.value!.id)
      if (idx !== -1) applications.value[idx] = updated
    } else {
      const created = await api.oauthApplications.create({
        name: form.name,
        redirect_uris: form.redirect_uris,
        homepage_url: form.homepage_url || undefined,
        logo_url: form.logo_url || undefined,
        description: form.description || undefined,
        confidential: form.confidential,
        scopes: form.scopes
      })
      applications.value.push(created)
      // 显示新创建的凭证
      newAppCredentials.value = created
      showSecretModal.value = true
    }
    closeModal()
  } catch (error) {
    console.error('Failed to save application:', error)
    alert('保存失败，请重试')
  } finally {
    saving.value = false
  }
}

function confirmDelete(app: OAuthApplication) {
  deletingApp.value = app
}

async function deleteApplication() {
  if (!deletingApp.value) return
  deleting.value = true
  try {
    await api.oauthApplications.delete(deletingApp.value.id)
    applications.value = applications.value.filter(a => a.id !== deletingApp.value!.id)
    deletingApp.value = null
  } catch (error) {
    console.error('Failed to delete application:', error)
    alert('删除失败，请重试')
  } finally {
    deleting.value = false
  }
}

function confirmRegenerateSecret(app: OAuthApplication) {
  if (!confirm(`确定要重新生成 "${app.name}" 的 Client Secret 吗？现有的 Secret 将立即失效。`)) return
  regenerateSecret(app)
}

async function regenerateSecret(app: OAuthApplication) {
  try {
    const result = await api.oauthApplications.regenerateSecret(app.id)
    newAppCredentials.value = result
    showSecretModal.value = true
  } catch (error) {
    console.error('Failed to regenerate secret:', error)
    alert('重新生成密钥失败，请重试')
  }
}

function copyToClipboard(text: string) {
  navigator.clipboard.writeText(text).then(() => {
    // 可以添加 toast 提示
  }).catch(err => {
    console.error('Failed to copy:', err)
  })
}
</script>

<style lang="scss" scoped>
@import '@/styles/variables';

.settings-page {
  max-width: 900px;
  margin: 0 auto;
  padding: 24px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
  gap: 16px;

  .header-content {
    flex: 1;
  }

  h1 {
    font-size: 24px;
    font-weight: 600;
    margin: 0 0 8px;
  }

  .page-description {
    color: $text-secondary;
    margin: 0;
    line-height: 1.5;
  }
}

.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 48px;
  color: $text-secondary;
}

.empty-state {
  text-align: center;
  padding: 48px 24px;
  background: $gray-100;
  border-radius: 8px;

  .empty-icon {
    margin-bottom: 16px;
  }

  .empty-title {
    font-size: 18px;
    font-weight: 600;
    margin: 0 0 8px;
  }

  .empty-description {
    color: $text-secondary;
    margin: 0 0 24px;
    max-width: 400px;
    margin-left: auto;
    margin-right: auto;
  }
}

.applications-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.application-card {
  display: flex;
  gap: 16px;
  padding: 20px;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: 8px;

  .app-icon {
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: $gray-100;
    border-radius: 8px;
    flex-shrink: 0;
    color: $text-secondary;

    img {
      width: 100%;
      height: 100%;
      object-fit: cover;
      border-radius: 8px;
    }
  }

  .app-info {
    flex: 1;
    min-width: 0;

    .app-header {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-bottom: 8px;

      h3 {
        font-size: 16px;
        font-weight: 600;
        margin: 0;
      }

      .badge {
        font-size: 11px;
        padding: 2px 6px;
        border-radius: 4px;

        &.badge-trusted {
          background: $color-success-light;
          color: darken($color-success, 15%);
        }

        &.badge-public {
          background: $color-warning-light;
          color: darken($color-warning, 15%);
        }
      }
    }

    .app-description {
      color: $text-secondary;
      font-size: 14px;
      margin: 0 0 8px;
    }

    .app-meta {
      font-size: 13px;
      color: $text-secondary;
      display: flex;
      flex-wrap: wrap;
      gap: 16px;
      margin-bottom: 8px;

      code {
        background: $gray-100;
        padding: 2px 6px;
        border-radius: 4px;
        font-size: 12px;
      }

      a {
        color: $brand-primary;
        text-decoration: none;

        &:hover {
          text-decoration: underline;
        }
      }
    }

    .app-scopes {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
    }

    .scope-tag {
      font-size: 11px;
      padding: 2px 8px;
      background: $gray-100;
      border-radius: 12px;
      color: $text-secondary;
    }
  }

  .app-actions {
    display: flex;
    gap: 4px;
    align-self: flex-start;
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
  background: $bg-primary;
  border-radius: 8px;
  width: 100%;
  max-width: 480px;
  max-height: 90vh;
  overflow-y: auto;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);

  &.modal-large {
    max-width: 600px;
  }
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid $border-color;
  position: sticky;
  top: 0;
  background: $bg-primary;

  h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .modal-close {
    background: none;
    border: none;
    padding: 4px;
    cursor: pointer;
    color: $text-secondary;

    &:hover {
      color: $text-primary;
    }
  }
}

.modal-body {
  padding: 20px;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 16px 20px;
  border-top: 1px solid $border-color;
  position: sticky;
  bottom: 0;
  background: $bg-primary;
}

.form-group {
  margin-bottom: 16px;

  label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    margin-bottom: 6px;

    .required {
      color: $color-danger;
    }
  }

  .form-hint {
    font-size: 12px;
    color: $text-secondary;
    margin-top: 4px;
  }
}

.form-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.form-input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid $border-color;
  border-radius: 4px;
  font-size: 14px;
  background: $bg-primary;
  color: $text-primary;

  &:focus {
    outline: none;
    border-color: $brand-primary;
    box-shadow: 0 0 0 3px rgba($brand-primary, 0.15);
  }
}

textarea.form-input {
  resize: vertical;
  min-height: 60px;
}

.scopes-grid {
  display: grid;
  gap: 8px;
}

.scope-checkbox {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 8px 12px;
  background: $gray-100;
  border-radius: 4px;
  cursor: pointer;

  &:hover {
    background: $gray-200;
  }

  input[type="checkbox"] {
    margin-top: 2px;
  }

  .scope-info {
    display: flex;
    flex-direction: column;

    strong {
      font-size: 13px;
    }

    small {
      font-size: 12px;
      color: $text-secondary;
    }
  }
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;

  input[type="checkbox"] {
    width: 16px;
    height: 16px;
  }
}

.credential-item {
  margin-bottom: 16px;

  label {
    display: block;
    font-size: 12px;
    font-weight: 500;
    color: $text-secondary;
    margin-bottom: 4px;
  }

  .credential-value {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: $gray-100;
    border-radius: 4px;

    code {
      flex: 1;
      font-size: 13px;
      word-break: break-all;
    }
  }
}

.alert {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px 16px;
  border-radius: 4px;
  margin-bottom: 16px;

  &.alert-warning {
    background: $color-warning-light;
    color: darken($color-warning, 15%);
  }

  svg {
    flex-shrink: 0;
    margin-top: 2px;
  }
}

.text-muted {
  color: $text-secondary;
  font-size: 14px;
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: 4px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  border: none;

  &.btn-sm {
    padding: 4px 8px;
  }

  &.btn-icon {
    padding: 6px;
    background: transparent;
    color: $text-secondary;

    &:hover {
      background: $gray-100;
      color: $text-primary;
    }

    &.btn-danger:hover {
      background: $color-danger-light;
      color: $color-danger;
    }
  }

  &.btn-primary {
    background: $brand-primary;
    color: white;

    &:hover:not(:disabled) {
      background: darken($brand-primary, 8%);
    }
  }

  &.btn-secondary {
    background: $gray-100;
    color: $text-primary;

    &:hover {
      background: $gray-200;
    }
  }

  &.btn-danger {
    background: $color-danger;
    color: white;

    &:hover:not(:disabled) {
      background: darken($color-danger, 8%);
    }
  }

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.spinner {
  width: 20px;
  height: 20px;
  border: 2px solid $border-color;
  border-top-color: $text-primary;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
