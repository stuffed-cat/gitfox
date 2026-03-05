<template>
  <div class="settings-page">
    <div class="page-header">
      <h1>已关联账号</h1>
      <p class="page-description">管理与第三方服务关联的账号，用于社交登录</p>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>加载中...</span>
    </div>

    <template v-else>
      <!-- Linked identities -->
      <div class="card">
        <div class="card-header">
          <h3>已关联的账号</h3>
        </div>
        <div class="card-body">
          <div v-if="identities.length === 0" class="empty-state-inline">
            <p>您还没有关联任何第三方账号</p>
          </div>
          <div v-else class="identities-list">
            <div v-for="identity in identities" :key="identity.id" class="identity-item">
              <div class="identity-icon">
                <component :is="getProviderIcon(identity.provider_type)" />
              </div>
              <div class="identity-info">
                <div class="identity-header">
                  <span class="identity-provider">{{ identity.provider_display_name }}</span>
                  <span class="identity-username">{{ identity.external_username || identity.external_email }}</span>
                </div>
                <div class="identity-meta">
                  <span v-if="identity.last_sign_in_at">
                    上次登录: {{ formatDate(identity.last_sign_in_at) }}
                  </span>
                  <span>
                    关联时间: {{ formatDate(identity.created_at) }}
                  </span>
                </div>
              </div>
              <div class="identity-actions">
                <button 
                  class="btn btn-danger btn-sm" 
                  @click="confirmUnlink(identity)"
                  :disabled="identities.length === 1 && !hasPassword"
                  :title="identities.length === 1 && !hasPassword ? '至少需要保留一种登录方式' : '解除关联'"
                >
                  解除关联
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Available providers to link -->
      <div class="card" v-if="availableProviders.length > 0">
        <div class="card-header">
          <h3>可关联的账号</h3>
        </div>
        <div class="card-body">
          <div class="providers-grid">
            <button 
              v-for="provider in availableProviders" 
              :key="provider.name"
              class="provider-link-btn"
              @click="linkProvider(provider)"
            >
              <component :is="getProviderIcon(provider.provider_type)" />
              <span>关联 {{ provider.display_name }}</span>
            </button>
          </div>
        </div>
      </div>
    </template>

    <!-- Confirm unlink modal -->
    <div v-if="unlinkingIdentity" class="modal-overlay" @click.self="unlinkingIdentity = null">
      <div class="modal-content">
        <div class="modal-header">
          <h3>确认解除关联</h3>
          <button class="modal-close" @click="unlinkingIdentity = null">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
        <div class="modal-body">
          <p>
            确定要解除与 <strong>{{ unlinkingIdentity.provider_display_name }}</strong> 
            ({{ unlinkingIdentity.external_username || unlinkingIdentity.external_email }}) 的关联吗？
          </p>
          <p class="text-muted">解除后您将无法使用该账号登录 GitFox。</p>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" @click="unlinkingIdentity = null">取消</button>
          <button class="btn btn-danger" @click="unlinkIdentity" :disabled="unlinking">
            {{ unlinking ? '处理中...' : '解除关联' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, h } from 'vue'
import api from '@/api'
import type { OAuthIdentity, OAuthProviderInfo } from '@/types'

const loading = ref(true)
const identities = ref<OAuthIdentity[]>([])
const providers = ref<OAuthProviderInfo[]>([])
const hasPassword = ref(true)
const unlinkingIdentity = ref<OAuthIdentity | null>(null)
const unlinking = ref(false)

// 已关联的提供商名称
const linkedProviderNames = computed(() => 
  identities.value.map(i => i.provider_name)
)

// 可关联的提供商（排除已关联的）
const availableProviders = computed(() => 
  providers.value.filter(p => !linkedProviderNames.value.includes(p.name))
)

onMounted(async () => {
  try {
    const [identitiesData, providersData, accountStatus] = await Promise.all([
      api.identities.list(),
      api.oauth.getProviders(),
      api.identities.getAccountStatus()
    ])
    identities.value = identitiesData
    providers.value = providersData
    hasPassword.value = accountStatus.has_password
  } catch (error) {
    console.error('Failed to load data:', error)
  } finally {
    loading.value = false
  }
})

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString('zh-CN')
}

function getProviderIcon(providerType: string) {
  const icons: Record<string, () => ReturnType<typeof h>> = {
    github: () => h('svg', { viewBox: '0 0 24 24', width: 24, height: 24, fill: 'currentColor' }, [
      h('path', { d: 'M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z' })
    ]),
    gitlab: () => h('svg', { viewBox: '0 0 24 24', width: 24, height: 24, fill: '#FC6D26' }, [
      h('path', { d: 'M23.955 13.587l-1.342-4.135-2.664-8.189a.455.455 0 00-.867 0L16.418 9.45H7.582L4.918 1.263a.455.455 0 00-.867 0L1.386 9.452.045 13.587a.924.924 0 00.331 1.023L12 23.054l11.624-8.443a.92.92 0 00.331-1.024' })
    ]),
    google: () => h('svg', { viewBox: '0 0 24 24', width: 24, height: 24 }, [
      h('path', { fill: '#4285F4', d: 'M23.745 12.27c0-.79-.07-1.54-.19-2.27h-11.3v4.51h6.47c-.29 1.48-1.14 2.73-2.4 3.58v3h3.86c2.26-2.09 3.56-5.17 3.56-8.82z' }),
      h('path', { fill: '#34A853', d: 'M12.255 24c3.24 0 5.95-1.08 7.93-2.91l-3.86-3c-1.08.72-2.45 1.16-4.07 1.16-3.13 0-5.78-2.11-6.73-4.96h-3.98v3.09C3.515 21.3 7.565 24 12.255 24z' }),
      h('path', { fill: '#FBBC05', d: 'M5.525 14.29c-.25-.72-.38-1.49-.38-2.29s.14-1.57.38-2.29V6.62h-3.98a11.86 11.86 0 000 10.76l3.98-3.09z' }),
      h('path', { fill: '#EA4335', d: 'M12.255 4.75c1.77 0 3.35.61 4.6 1.8l3.42-3.42C18.205 1.19 15.495 0 12.255 0c-4.69 0-8.74 2.7-10.71 6.62l3.98 3.09c.95-2.85 3.6-4.96 6.73-4.96z' })
    ]),
    azure_ad: () => h('svg', { viewBox: '0 0 24 24', width: 24, height: 24, fill: '#00A4EF' }, [
      h('path', { d: 'M11.4 24H0V12.6L11.4 0V24zM24 24H12.6V0L24 12.6V24z' })
    ]),
    bitbucket: () => h('svg', { viewBox: '0 0 24 24', width: 24, height: 24, fill: '#0052CC' }, [
      h('path', { d: 'M.778 1.213a.768.768 0 00-.768.892l3.263 19.81c.084.5.515.868 1.022.873H19.95a.772.772 0 00.77-.646l3.27-20.03a.768.768 0 00-.768-.891H.778zM14.52 15.53H9.522L8.17 8.466h7.561l-1.211 7.064z' })
    ])
  }
  return icons[providerType] || (() => h('svg', { viewBox: '0 0 24 24', width: 24, height: 24, fill: 'currentColor' }, [
    h('circle', { cx: 12, cy: 12, r: 10, stroke: 'currentColor', 'stroke-width': 2, fill: 'none' }),
    h('path', { d: 'M12 8v4l3 3', stroke: 'currentColor', 'stroke-width': 2, 'stroke-linecap': 'round' })
  ]))
}

function linkProvider(provider: OAuthProviderInfo) {
  // 跳转到 OAuth 授权页面，state 中标记是关联操作
  const state = btoa(JSON.stringify({ action: 'link', provider: provider.name }))
  window.location.href = `${provider.authorize_url}?state=${state}`
}

function confirmUnlink(identity: OAuthIdentity) {
  unlinkingIdentity.value = identity
}

async function unlinkIdentity() {
  if (!unlinkingIdentity.value) return
  
  unlinking.value = true
  try {
    await api.identities.unlink(unlinkingIdentity.value.id)
    identities.value = identities.value.filter(i => i.id !== unlinkingIdentity.value!.id)
    unlinkingIdentity.value = null
  } catch (error) {
    console.error('Failed to unlink identity:', error)
    alert('解除关联失败，请重试')
  } finally {
    unlinking.value = false
  }
}
</script>

<style lang="scss" scoped>
.settings-page {
  max-width: 800px;
  margin: 0 auto;
  padding: 24px;
}

.page-header {
  margin-bottom: 24px;

  h1 {
    font-size: 24px;
    font-weight: 600;
  }

  .page-description {
    color: var(--gl-text-secondary);
    margin: 0;
  }
}

.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 48px;
  color: var(--gl-text-secondary);
}

.card {
  background: var(--gl-background-color-default);
  border: 1px solid var(--gl-border-color-default);
  border-radius: 8px;
  margin-bottom: 16px;

  .card-header {
    padding: 16px 20px;
    border-bottom: 1px solid var(--gl-border-color-default);

    h3 {
      font-size: 16px;
      font-weight: 600;
      margin: 0;
    }
  }

  .card-body {
    padding: 20px;
  }
}

.empty-state-inline {
  text-align: center;
  padding: 24px;
  color: var(--gl-text-secondary);
}

.identities-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.identity-item {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px;
  background: var(--gl-background-color-subtle);
  border-radius: 8px;

  .identity-icon {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--gl-background-color-default);
    border-radius: 8px;
    flex-shrink: 0;
  }

  .identity-info {
    flex: 1;
    min-width: 0;

    .identity-header {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-bottom: 4px;

      .identity-provider {
        font-weight: 600;
      }

      .identity-username {
        color: var(--gl-text-secondary);
      }
    }

    .identity-meta {
      font-size: 12px;
      color: var(--gl-text-secondary);
      display: flex;
      gap: 16px;
    }
  }
}

.providers-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 12px;
}

.provider-link-btn {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: var(--gl-background-color-subtle);
  border: 1px solid var(--gl-border-color-default);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;

  &:hover {
    background: var(--gl-background-color-default);
    border-color: var(--gl-border-color-strong);
  }

  span {
    font-weight: 500;
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
  background: var(--gl-background-color-default);
  border-radius: 8px;
  width: 100%;
  max-width: 400px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--gl-border-color-default);

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
    color: var(--gl-text-secondary);

    &:hover {
      color: var(--gl-text-color-default);
    }
  }
}

.modal-body {
  padding: 20px;

  p {
    margin: 0 0 12px;

    &:last-child {
      margin-bottom: 0;
    }
  }

  .text-muted {
    color: var(--gl-text-secondary);
    font-size: 14px;
  }
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 16px 20px;
  border-top: 1px solid var(--gl-border-color-default);
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 8px 16px;
  border-radius: 4px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  border: none;

  &.btn-sm {
    padding: 6px 12px;
    font-size: 13px;
  }

  &.btn-secondary {
    background: var(--gl-background-color-subtle);
    color: var(--gl-text-color-default);

    &:hover {
      background: var(--gl-background-color-strong);
    }
  }

  &.btn-danger {
    background: var(--gl-background-color-danger);
    color: white;

    &:hover:not(:disabled) {
      background: var(--gl-background-color-danger-strong);
    }

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }
}

.spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--gl-border-color-default);
  border-top-color: var(--gl-text-color-default);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
