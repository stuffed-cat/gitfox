<template>
  <div class="project-settings-page">
    <div class="settings-header">
      <h2>Webhooks</h2>
      <p class="description">配置项目事件的 HTTP 回调</p>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <template v-else>
      <!-- 添加 Webhook -->
      <section class="settings-section">
        <h3>{{ editingWebhook ? '编辑 Webhook' : '添加 Webhook' }}</h3>
        
        <form @submit.prevent="saveWebhook" class="webhook-form">
          <div class="form-group">
            <label for="webhook-url">URL <span class="required">*</span></label>
            <input
              id="webhook-url"
              v-model="webhookForm.url"
              type="url"
              class="form-control"
              placeholder="https://example.com/webhook"
              required
            />
            <p class="form-help">收到事件时将向此 URL 发送 POST 请求</p>
          </div>
          
          <div class="form-group">
            <label for="webhook-secret">密钥 (可选)</label>
            <input
              id="webhook-secret"
              v-model="webhookForm.secret"
              type="password"
              class="form-control"
              placeholder="用于验证请求签名"
            />
            <p class="form-help">用于验证 Webhook 请求的 HMAC 签名</p>
          </div>
          
          <div class="form-group">
            <label>触发事件</label>
            <div class="event-checkboxes">
              <div class="event-group">
                <h4>推送事件</h4>
                <label class="checkbox-label">
                  <input type="checkbox" v-model="webhookForm.events" value="push">
                  Push 事件 - 代码推送到仓库
                </label>
                <label class="checkbox-label">
                  <input type="checkbox" v-model="webhookForm.events" value="tag_push">
                  Tag 推送 - 创建或删除标签
                </label>
              </div>
              
              <div class="event-group">
                <h4>合并请求</h4>
                <label class="checkbox-label">
                  <input type="checkbox" v-model="webhookForm.events" value="merge_request">
                  合并请求事件 - 创建、更新、合并或关闭
                </label>
              </div>
              
              <div class="event-group">
                <h4>议题</h4>
                <label class="checkbox-label">
                  <input type="checkbox" v-model="webhookForm.events" value="issue">
                  议题事件 - 创建、更新或关闭
                </label>
                <label class="checkbox-label">
                  <input type="checkbox" v-model="webhookForm.events" value="note">
                  评论事件 - 议题或合并请求的评论
                </label>
              </div>
              
              <div class="event-group">
                <h4>流水线</h4>
                <label class="checkbox-label">
                  <input type="checkbox" v-model="webhookForm.events" value="pipeline">
                  流水线事件 - 流水线状态变更
                </label>
                <label class="checkbox-label">
                  <input type="checkbox" v-model="webhookForm.events" value="job">
                  作业事件 - 作业状态变更
                </label>
              </div>
              
              <div class="event-group">
                <h4>其他</h4>
                <label class="checkbox-label">
                  <input type="checkbox" v-model="webhookForm.events" value="release">
                  发布事件 - 创建新版本
                </label>
                <label class="checkbox-label">
                  <input type="checkbox" v-model="webhookForm.events" value="wiki_page">
                  Wiki 事件 - 页面创建或更新
                </label>
              </div>
            </div>
          </div>
          
          <div class="form-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="webhookForm.ssl_verification">
              启用 SSL 验证
            </label>
            <p class="form-help">建议启用以确保安全通信</p>
          </div>
          
          <div class="form-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="webhookForm.is_active">
              启用此 Webhook
            </label>
          </div>
          
          <div class="form-actions">
            <button type="submit" class="btn btn-primary" :disabled="!webhookForm.url || webhookForm.events.length === 0">
              {{ editingWebhook ? '更新 Webhook' : '添加 Webhook' }}
            </button>
            <button v-if="editingWebhook" type="button" class="btn btn-outline" @click="cancelEdit">
              取消
            </button>
          </div>
        </form>
      </section>

      <!-- Webhook 列表 -->
      <section class="settings-section">
        <h3>已配置的 Webhooks</h3>
        
        <div class="webhook-list">
          <div v-for="webhook in webhooks" :key="webhook.id" class="webhook-item">
            <div class="webhook-info">
              <div class="webhook-url">
                <span class="status-indicator" :class="{ active: webhook.is_active }"></span>
                <code>{{ webhook.url }}</code>
              </div>
              <div class="webhook-events">
                <span v-for="event in webhook.events" :key="event" class="event-tag">
                  {{ eventLabels[event] || event }}
                </span>
              </div>
              <div class="webhook-meta">
                <span>创建于 {{ formatDate(webhook.created_at) }}</span>
                <span v-if="webhook.last_triggered_at">
                  · 最后触发: {{ formatDate(webhook.last_triggered_at) }}
                </span>
                <span v-if="webhook.last_response_code" :class="{ error: webhook.last_response_code >= 400 }">
                  · 响应: {{ webhook.last_response_code }}
                </span>
              </div>
            </div>
            
            <div class="webhook-actions">
              <button class="btn btn-outline btn-sm" @click="testWebhook(webhook)">
                测试
              </button>
              <button class="btn btn-outline btn-sm" @click="editWebhook(webhook)">
                编辑
              </button>
              <button class="btn btn-outline btn-sm" @click="viewRecentDeliveries(webhook)">
                历史
              </button>
              <button class="btn btn-danger btn-sm" @click="deleteWebhook(webhook.id)">
                删除
              </button>
            </div>
          </div>
          
          <div v-if="webhooks.length === 0" class="empty-state">
            <p>暂无配置 Webhooks</p>
            <p class="text-muted">添加 Webhook 以在项目事件发生时接收通知</p>
          </div>
        </div>
      </section>

      <!-- 最近投递记录模态框 -->
      <div v-if="showDeliveries" class="modal-overlay" @click.self="showDeliveries = false">
        <div class="modal-content modal-large">
          <div class="modal-header">
            <h3>投递历史</h3>
            <button class="btn-close" @click="showDeliveries = false">&times;</button>
          </div>
          
          <div class="delivery-list">
            <div v-for="delivery in recentDeliveries" :key="delivery.id" class="delivery-item">
              <div class="delivery-header" @click="toggleDeliveryDetail(delivery)">
                <span class="delivery-status" :class="delivery.response_code < 400 ? 'success' : 'error'">
                  {{ delivery.response_code }}
                </span>
                <span class="delivery-event">{{ eventLabels[delivery.event] || delivery.event }}</span>
                <span class="delivery-time">{{ formatDateTime(delivery.triggered_at) }}</span>
                <span class="delivery-duration">{{ delivery.duration_ms }}ms</span>
              </div>
              
              <div v-if="delivery.expanded" class="delivery-detail">
                <div class="detail-section">
                  <h4>请求头</h4>
                  <pre><code>{{ formatJson(delivery.request_headers) }}</code></pre>
                </div>
                <div class="detail-section">
                  <h4>请求体</h4>
                  <pre><code>{{ formatJson(delivery.request_body) }}</code></pre>
                </div>
                <div class="detail-section">
                  <h4>响应</h4>
                  <pre><code>{{ delivery.response_body }}</code></pre>
                </div>
                
                <button class="btn btn-outline btn-sm" @click="redeliverWebhook(delivery)">
                  重新投递
                </button>
              </div>
            </div>
            
            <div v-if="recentDeliveries.length === 0" class="empty-state">
              <p>暂无投递记录</p>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import api from '@/api'
import type { Project, Webhook } from '@/types'

interface WebhookDelivery {
  id: string
  webhook_id: string
  event: string
  request_headers: Record<string, string>
  request_body: object
  response_code: number
  response_body: string
  duration_ms: number
  triggered_at: string
  expanded?: boolean
}

interface ExtendedWebhook extends Webhook {
  last_triggered_at?: string
  last_response_code?: number
}

const props = defineProps<{
  project?: Project
}>()

const loading = ref(false)
const webhooks = ref<ExtendedWebhook[]>([])
const editingWebhook = ref<ExtendedWebhook | null>(null)
const showDeliveries = ref(false)
const recentDeliveries = ref<WebhookDelivery[]>([])

const webhookForm = reactive({
  url: '',
  secret: '',
  events: [] as string[],
  ssl_verification: true,
  is_active: true
})

const eventLabels: Record<string, string> = {
  push: 'Push',
  tag_push: 'Tag',
  merge_request: '合并请求',
  issue: '议题',
  note: '评论',
  pipeline: '流水线',
  job: '作业',
  release: '发布',
  wiki_page: 'Wiki'
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('zh-CN')
}

function formatDateTime(dateStr: string): string {
  return new Date(dateStr).toLocaleString('zh-CN')
}

function formatJson(obj: object): string {
  return JSON.stringify(obj, null, 2)
}

async function loadWebhooks() {
  if (!props.project?.owner_name || !props.project?.name) return
  loading.value = true
  
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    const data = await api.webhooks.list(path)
    webhooks.value = data
  } catch (error) {
    console.error('Failed to load webhooks:', error)
  } finally {
    loading.value = false
  }
}

function resetForm() {
  webhookForm.url = ''
  webhookForm.secret = ''
  webhookForm.events = []
  webhookForm.ssl_verification = true
  webhookForm.is_active = true
  editingWebhook.value = null
}

async function saveWebhook() {
  if (!props.project?.owner_name || !props.project?.name) return
  if (!webhookForm.url || webhookForm.events.length === 0) return
  
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    if (editingWebhook.value) {
      await api.webhooks.update(path, editingWebhook.value.id, {
        url: webhookForm.url,
        secret: webhookForm.secret || undefined,
        events: webhookForm.events
      })
      alert('Webhook 已更新')
    } else {
      await api.webhooks.create(path, {
        url: webhookForm.url,
        secret: webhookForm.secret || undefined,
        events: webhookForm.events
      })
      alert('Webhook 已添加')
    }
    
    resetForm()
    loadWebhooks()
  } catch (error) {
    console.error('Failed to save webhook:', error)
    alert('保存失败')
  }
}

function editWebhook(webhook: ExtendedWebhook) {
  editingWebhook.value = webhook
  webhookForm.url = webhook.url
  webhookForm.secret = ''
  webhookForm.events = [...webhook.events]
  webhookForm.is_active = webhook.is_active
  webhookForm.ssl_verification = true
  
  // 滚动到表单
  window.scrollTo({ top: 0, behavior: 'smooth' })
}

function cancelEdit() {
  resetForm()
}

async function testWebhook(webhook: ExtendedWebhook) {
  if (!props.project?.owner_name || !props.project?.name) return
  
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    await api.webhooks.test(path, webhook.id)
    alert('测试请求已发送')
    loadWebhooks()
  } catch (error) {
    console.error('Failed to test webhook:', error)
    alert('测试失败')
  }
}

async function deleteWebhook(webhookId: string) {
  if (!props.project?.owner_name || !props.project?.name) return
  if (!confirm('确定要删除此 Webhook 吗？')) return
  
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    await api.webhooks.delete(path, webhookId)
    webhooks.value = webhooks.value.filter(w => w.id !== webhookId)
    alert('Webhook 已删除')
  } catch (error) {
    console.error('Failed to delete webhook:', error)
    alert('删除失败')
  }
}

async function viewRecentDeliveries(_webhook: ExtendedWebhook) {
  // TODO: 后端 API 尚未支持投递历史查询
  // 需要添加 GET /projects/{ns}/{proj}/hooks/{id}/deliveries 端点
  // recentDeliveries.value = await api.webhooks.listDeliveries(path, webhook.id)
  recentDeliveries.value = []
  showDeliveries.value = true
}

function toggleDeliveryDetail(delivery: WebhookDelivery) {
  delivery.expanded = !delivery.expanded
}

async function redeliverWebhook(_delivery: WebhookDelivery) {
  // TODO: 后端 API 尚未支持重新投递
  // 需要添加 POST /projects/{ns}/{proj}/hooks/{id}/deliveries/{delivery_id}/retry 端点
  alert('重新投递功能即将实现')
}

watch([() => props.project?.owner_name, () => props.project?.name], () => {
  loadWebhooks()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.project-settings-page {
  padding: $spacing-lg;
  max-width: 900px;
}

.settings-header {
  margin-bottom: $spacing-xl;
  
  h2 {
    margin: 0 0 $spacing-xs 0;
  }
  
  .description {
    color: $text-muted;
    margin: 0;
  }
}

.settings-section {
  margin-bottom: $spacing-xl;
  padding-bottom: $spacing-xl;
  border-bottom: 1px solid $border-color;
  
  &:last-child {
    border-bottom: none;
  }
  
  h3 {
    margin-bottom: $spacing-md;
  }
}

.webhook-form {
  padding: $spacing-lg;
  background: $bg-secondary;
  border-radius: $border-radius;
}

.form-group {
  margin-bottom: $spacing-lg;
  
  label {
    display: block;
    margin-bottom: $spacing-xs;
    font-weight: 500;
    
    .required {
      color: $danger-color;
    }
  }
  
  .form-help {
    font-size: $font-size-sm;
    color: $text-muted;
    margin-top: $spacing-xs;
  }
}

.event-checkboxes {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: $spacing-lg;
}

.event-group {
  h4 {
    margin: 0 0 $spacing-sm 0;
    font-size: $font-size-sm;
    color: $text-muted;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
}

.checkbox-label {
  display: flex;
  align-items: flex-start;
  gap: $spacing-sm;
  margin-bottom: $spacing-sm;
  cursor: pointer;
  font-weight: normal !important;
  
  input {
    margin-top: 3px;
  }
}

.form-actions {
  display: flex;
  gap: $spacing-md;
  margin-top: $spacing-lg;
}

.webhook-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
}

.webhook-item {
  padding: $spacing-md;
  background: $bg-secondary;
  border-radius: $border-radius;
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: $spacing-md;
}

.webhook-info {
  flex: 1;
  min-width: 0;
}

.webhook-url {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  margin-bottom: $spacing-sm;
  
  .status-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: $text-muted;
    flex-shrink: 0;
    
    &.active {
      background: $success-color;
    }
  }
  
  code {
    font-family: monospace;
    font-size: $font-size-sm;
    word-break: break-all;
  }
}

.webhook-events {
  display: flex;
  flex-wrap: wrap;
  gap: $spacing-xs;
  margin-bottom: $spacing-sm;
}

.event-tag {
  font-size: $font-size-xs;
  padding: 2px 8px;
  background: $bg-tertiary;
  border-radius: 3px;
  color: $text-muted;
}

.webhook-meta {
  font-size: $font-size-sm;
  color: $text-muted;
  
  .error {
    color: $danger-color;
  }
}

.webhook-actions {
  display: flex;
  gap: $spacing-sm;
  flex-shrink: 0;
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
  background: $bg-primary;
  border-radius: $border-radius-lg;
  max-height: 80vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  
  &.modal-large {
    width: 90%;
    max-width: 800px;
  }
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-lg;
  border-bottom: 1px solid $border-color;
  
  h3 {
    margin: 0;
  }
  
  .btn-close {
    background: none;
    border: none;
    font-size: 24px;
    cursor: pointer;
    color: $text-muted;
    
    &:hover {
      color: $text-primary;
    }
  }
}

.delivery-list {
  padding: $spacing-lg;
  overflow-y: auto;
}

.delivery-item {
  border: 1px solid $border-color;
  border-radius: $border-radius;
  margin-bottom: $spacing-sm;
  overflow: hidden;
}

.delivery-header {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  padding: $spacing-md;
  cursor: pointer;
  
  &:hover {
    background: $bg-secondary;
  }
}

.delivery-status {
  font-size: $font-size-sm;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 3px;
  
  &.success {
    background: rgba($success-color, 0.2);
    color: $success-color;
  }
  
  &.error {
    background: rgba($danger-color, 0.2);
    color: $danger-color;
  }
}

.delivery-event {
  flex: 1;
}

.delivery-time {
  color: $text-muted;
  font-size: $font-size-sm;
}

.delivery-duration {
  color: $text-muted;
  font-size: $font-size-sm;
}

.delivery-detail {
  padding: $spacing-md;
  background: $bg-secondary;
  border-top: 1px solid $border-color;
}

.detail-section {
  margin-bottom: $spacing-md;
  
  h4 {
    margin: 0 0 $spacing-sm 0;
    font-size: $font-size-sm;
    color: $text-muted;
  }
  
  pre {
    margin: 0;
    padding: $spacing-sm;
    background: $bg-tertiary;
    border-radius: $border-radius;
    overflow-x: auto;
    max-height: 200px;
    
    code {
      font-family: monospace;
      font-size: $font-size-sm;
    }
  }
}

.empty-state {
  padding: $spacing-lg;
  text-align: center;
  color: $text-muted;
  
  .text-muted {
    font-size: $font-size-sm;
  }
}
</style>
