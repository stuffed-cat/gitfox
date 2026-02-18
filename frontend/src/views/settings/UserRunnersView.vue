<template>
  <div class="user-runners">
    <div class="page-header">
      <div class="header-content">
        <h1>私有 Runners</h1>
        <p class="page-description">管理您的私有 Runner，仅供您自己的项目使用</p>
      </div>
      <button class="btn btn-primary" @click="showCreateModal = true">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        创建私有 Runner
      </button>
    </div>

    <div class="info-banner">
      <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
        <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
        <path d="M8 4v5M8 11v1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      </svg>
      <div>
        <strong>关于私有 Runners</strong>
        <p>私有 Runner 仅供您创建的项目使用，其他用户无法访问。适合处理敏感数据或特殊环境需求。</p>
      </div>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>加载中...</span>
    </div>

    <div v-else-if="error" class="error-state">
      <svg viewBox="0 0 64 64" width="64" height="64" fill="none">
        <circle cx="32" cy="32" r="28" fill="#fee" stroke="#f88" stroke-width="2"/>
        <path d="M24 24l16 16M40 24L24 40" stroke="#e33" stroke-width="3" stroke-linecap="round"/>
      </svg>
      <p class="error-title">加载失败</p>
      <p class="error-message">{{ error }}</p>
      <button class="btn btn-primary" @click="loadRunners">重试</button>
    </div>

    <template v-else>
      <div v-if="runners.length === 0" class="empty-state">
        <div class="empty-icon">
          <svg viewBox="0 0 64 64" width="64" height="64" fill="none">
            <circle cx="32" cy="32" r="28" fill="#f0fdf4" stroke="#86efac" stroke-width="2"/>
            <path d="M22 32l8 8 12-16" stroke="#10b981" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <p class="empty-title">尚未创建任何私有 Runner</p>
        <p class="empty-description">创建私有 Runner 用于您的个人项目 CI/CD</p>
        <button class="btn btn-primary" @click="showCreateModal = true">创建第一个 Runner</button>
      </div>

      <div v-else class="runners-grid">
        <div v-for="runner in runners" :key="runner.id" class="runner-card" :class="{ disabled: !runner.is_active }">
          <div class="runner-header">
            <div class="runner-title">
              <div class="runner-status-indicator" :class="`status-${runner.status}`"></div>
              <h3>{{ runner.name }}</h3>
              <span v-if="!runner.is_active" class="badge badge-disabled">已禁用</span>
            </div>
            <div class="runner-actions">
              <button class="btn btn-icon" @click="editRunner(runner)" title="编辑">
                <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
                  <path d="M11.5 2.5l2 2-8 8H3.5v-2l8-8z" stroke="currentColor" stroke-width="1.2"/>
                </svg>
              </button>
              <button class="btn btn-icon btn-danger" @click="confirmDelete(runner)" title="删除">
                <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
                  <path d="M4 5v8a1 1 0 001 1h6a1 1 0 001-1V5M3 5h10M6 3h4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                </svg>
              </button>
            </div>
          </div>

          <div class="runner-description">
            {{ runner.description || '无描述' }}
          </div>

          <div class="runner-stats-row">
            <div class="stat-item">
              <span class="stat-label">状态</span>
              <span class="badge" :class="`badge-${runner.status}`">
                {{ getStatusText(runner.status) }}
              </span>
            </div>
            <div class="stat-item">
              <span class="stat-label">最后联系</span>
              <span class="stat-value">{{ runner.last_contact_at ? formatTime(runner.last_contact_at) : '从未' }}</span>
            </div>
          </div>

          <div class="runner-tags">
            <span v-for="tag in runner.tags" :key="tag" class="tag">{{ tag }}</span>
            <span v-if="runner.run_untagged" class="tag tag-special">untagged</span>
            <span v-if="runner.tags.length === 0 && !runner.run_untagged" class="text-muted">无标签</span>
          </div>

          <div v-if="runner.version" class="runner-version">
            <svg viewBox="0 0 16 16" width="14" height="14" fill="none">
              <rect x="3" y="3" width="10" height="10" rx="1" stroke="currentColor" stroke-width="1.5"/>
              <path d="M6 6h4M6 8h3M6 10h4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
            </svg>
            {{ runner.version }} · {{ runner.platform }}/{{ runner.architecture }}
          </div>

          <div class="runner-footer">
            <span v-if="runner.locked" class="config-badge">
              <svg viewBox="0 0 16 16" width="12" height="12" fill="none">
                <rect x="4" y="7" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <path d="M6 7V5a2 2 0 114 0v2" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              已锁定
            </span>
            <span v-if="runner.maximum_timeout" class="config-badge">
              <svg viewBox="0 0 16 16" width="12" height="12" fill="none">
                <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
                <path d="M8 5v3l2 2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              {{ runner.maximum_timeout }}s
            </span>
          </div>
        </div>
      </div>
    </template>

    <!-- Create Modal -->
    <div v-if="showCreateModal" class="modal-overlay" @click.self="closeCreateModal">
      <div class="modal-content modal-large">
        <div class="modal-header">
          <h3>创建私有 Runner</h3>
          <button class="modal-close" @click="closeCreateModal">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <form @submit.prevent="createRunner">
          <div class="form-group">
            <label for="runner-name">Runner 名称 <span class="required">*</span></label>
            <input
              id="runner-name"
              v-model="createForm.name"
              type="text"
              class="form-input"
              placeholder="例如: my-mac-runner"
              required
            />
            <p class="form-hint">用于标识此 Runner 的唯一名称</p>
          </div>

          <div class="form-group">
            <label for="runner-description">描述</label>
            <textarea
              id="runner-description"
              v-model="createForm.description"
              class="form-input"
              rows="3"
              placeholder="描述此 Runner 的用途、配置和特殊说明..."
            ></textarea>
          </div>

          <div class="form-group">
            <label for="runner-tags">标签</label>
            <input
              id="runner-tags"
              v-model="tagInput"
              type="text"
              class="form-input"
              placeholder="输入标签后按回车添加（例如: macos, docker, gpu）"
              @keydown.enter.prevent="addTag"
            />
            <div v-if="createForm.tags && createForm.tags.length > 0" class="tags-display">
              <span v-for="(tag, index) in createForm.tags" :key="index" class="tag tag-removable">
                {{ tag }}
                <button type="button" @click="removeTag(index)" class="tag-remove">×</button>
              </span>
            </div>
            <p class="form-hint">用于匹配 .gitfox-ci.yml 中指定的 tags</p>
          </div>

          <div class="form-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="createForm.run_untagged" />
              <span>运行无标签任务</span>
            </label>
            <p class="form-hint">允许此 Runner 执行没有指定标签的任务</p>
          </div>

          <div class="form-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="createForm.locked" />
              <span>锁定到用户级</span>
            </label>
            <p class="form-hint">防止此 Runner 被其他作用域引用</p>
          </div>

          <div class="form-group">
            <label for="runner-timeout">最大超时时间（秒）</label>
            <input
              id="runner-timeout"
              v-model.number="createForm.maximum_timeout"
              type="number"
              class="form-input"
              min="60"
              max="86400"
              placeholder="默认 3600 秒（1小时）"
            />
            <p class="form-hint">单个任务的最大执行时间，超时后将被终止</p>
          </div>

          <div v-if="createError" class="alert alert-error">
            {{ createError }}
          </div>

          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" @click="closeCreateModal" :disabled="creating">
              取消
            </button>
            <button type="submit" class="btn btn-primary" :disabled="creating">
              <span v-if="creating">创建中...</span>
              <span v-else>创建 Runner</span>
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- Token Display Modal -->
    <div v-if="showTokenModal && createdToken" class="modal-overlay" @click.self="() => {}">
      <div class="modal-content">
        <div class="modal-header">
          <h3>✅ Runner 创建成功</h3>
        </div>

        <div class="token-display">
          <div class="alert alert-warning">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M8 2L2 14h12L8 2z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
              <path d="M8 6v3M8 11v1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <div>
              <strong>重要提示</strong>
              <p>此 Token 仅显示一次，关闭后将无法再次查看。请立即复制并妥善保存！</p>
            </div>
          </div>

          <div class="token-info">
            <label>Runner 名称</label>
            <div class="info-value">{{ createdToken.runner.name }}</div>
          </div>

          <div class="token-info">
            <label>认证 Token</label>
            <div class="token-value">
              <code>{{ createdToken.token }}</code>
              <button class="btn btn-icon" @click="copyToken" :title="tokenCopied ? '已复制！' : '复制 Token'">
                <svg v-if="!tokenCopied" viewBox="0 0 16 16" width="16" height="16" fill="none">
                  <rect x="5" y="5" width="8" height="8" rx="1" stroke="currentColor" stroke-width="1.5"/>
                  <path d="M3 11V3a1 1 0 011-1h8" stroke="currentColor" stroke-width="1.5"/>
                </svg>
                <svg v-else viewBox="0 0 16 16" width="16" height="16" fill="none">
                  <path d="M4 8l3 3 5-6" stroke="#10b981" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </button>
            </div>
          </div>

          <div class="setup-instructions">
            <h4>🚀 启动 Runner</h4>
            <p>在您的机器上运行以下命令：</p>
            <pre><code>gitfox-runner --url {{ serverUrl }} --token {{ createdToken.token }}</code></pre>
            
            <h4 style="margin-top: 20px;">📦 下载 Runner</h4>
            <p>如果尚未安装 gitfox-runner，请访问：</p>
            <pre><code>{{ serverUrl }}/downloads/gitfox-runner</code></pre>
          </div>
        </div>

        <div class="modal-actions">
          <button class="btn btn-primary btn-large" @click="closeTokenModal">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 8l3 3 5-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            我已保存 Token
          </button>
        </div>
      </div>
    </div>

    <!-- Edit Modal -->
    <div v-if="editingRunner" class="modal-overlay" @click.self="closeEditModal">
      <div class="modal-content modal-large">
        <div class="modal-header">
          <h3>编辑 Runner</h3>
          <button class="modal-close" @click="closeEditModal">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <form @submit.prevent="updateRunner">
          <div class="form-group">
            <label for="edit-runner-name">Runner 名称 <span class="required">*</span></label>
            <input
              id="edit-runner-name"
              v-model="editForm.name"
              type="text"
              class="form-input"
              required
            />
          </div>

          <div class="form-group">
            <label for="edit-runner-description">描述</label>
            <textarea
              id="edit-runner-description"
              v-model="editForm.description"
              class="form-input"
              rows="3"
            ></textarea>
          </div>

          <div class="form-group">
            <label for="edit-runner-tags">标签</label>
            <input
              id="edit-runner-tags"
              v-model="editTagInput"
              type="text"
              class="form-input"
              placeholder="输入标签后按回车添加"
              @keydown.enter.prevent="addEditTag"
            />
            <div v-if="editForm.tags && editForm.tags.length > 0" class="tags-display">
              <span v-for="(tag, index) in editForm.tags" :key="index" class="tag tag-removable">
                {{ tag }}
                <button type="button" @click="removeEditTag(index)" class="tag-remove">×</button>
              </span>
            </div>
          </div>

          <div class="form-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="editForm.run_untagged" />
              <span>运行无标签任务</span>
            </label>
          </div>

          <div class="form-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="editForm.locked" />
              <span>锁定到用户级</span>
            </label>
          </div>

          <div class="form-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="editForm.is_active" />
              <span>启用此 Runner</span>
            </label>
          </div>

          <div class="form-group">
            <label for="edit-runner-timeout">最大超时时间（秒）</label>
            <input
              id="edit-runner-timeout"
              v-model.number="editForm.maximum_timeout"
              type="number"
              class="form-input"
              min="60"
              max="86400"
            />
          </div>

          <div v-if="editError" class="alert alert-error">
            {{ editError }}
          </div>

          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" @click="closeEditModal" :disabled="updating">
              取消
            </button>
            <button type="submit" class="btn btn-primary" :disabled="updating">
              <span v-if="updating">保存中...</span>
              <span v-else>保存更改</span>
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- Delete Confirmation Modal -->
    <div v-if="deletingRunner" class="modal-overlay" @click.self="closeDeleteModal">
      <div class="modal-content">
        <div class="modal-header">
          <h3>⚠️ 确认删除</h3>
          <button class="modal-close" @click="closeDeleteModal">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <div class="modal-body">
          <p>确定要删除 Runner <strong>{{ deletingRunner.name }}</strong> 吗？</p>
          <div class="warning-box">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M8 2L2 14h12L8 2z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
              <path d="M8 6v3M8 11v1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <div>
              <strong>此操作无法撤销</strong>
              <ul>
                <li>已连接的 Runner 将无法继续工作</li>
                <li>正在运行的任务将被中断</li>
                <li>相关的日志和历史记录将保留</li>
              </ul>
            </div>
          </div>

          <div v-if="deleteError" class="alert alert-error">
            {{ deleteError }}
          </div>
        </div>

        <div class="modal-actions">
          <button class="btn btn-secondary" @click="closeDeleteModal" :disabled="deleting">
            取消
          </button>
          <button class="btn btn-danger" @click="deleteRunner" :disabled="deleting">
            <span v-if="deleting">删除中...</span>
            <span v-else>确认删除</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '@/api'
import type { Runner, CreateRunnerRequest, UpdateRunnerRequest, CreateRunnerResponse } from '@/types'

const loading = ref(true)
const error = ref<string | null>(null)
const runners = ref<Runner[]>([])

const showCreateModal = ref(false)
const createForm = ref<CreateRunnerRequest>({
  name: '',
  description: '',
  tags: [],
  run_untagged: true,
  locked: false,
  maximum_timeout: undefined,
})
const tagInput = ref('')
const creating = ref(false)
const createError = ref<string | null>(null)

const showTokenModal = ref(false)
const createdToken = ref<CreateRunnerResponse | null>(null)
const tokenCopied = ref(false)
const serverUrl = ref(window.location.origin)

const editingRunner = ref<Runner | null>(null)
const editForm = ref<UpdateRunnerRequest>({})
const editTagInput = ref('')
const updating = ref(false)
const editError = ref<string | null>(null)

const deletingRunner = ref<Runner | null>(null)
const deleting = ref(false)
const deleteError = ref<string | null>(null)

async function loadRunners() {
  try {
    loading.value = true
    error.value = null
    runners.value = await api.runners.userList()
  } catch (err: any) {
    error.value = err.response?.data?.error || err.message || '加载失败'
  } finally {
    loading.value = false
  }
}

function getStatusText(status: string): string {
  const statusMap: Record<string, string> = {
    idle: '空闲',
    running: '运行中',
    offline: '离线'
  }
  return statusMap[status] || status
}

function formatTime(time: string): string {
  const date = new Date(time)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const seconds = Math.floor(diff / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (days > 0) return `${days} 天前`
  if (hours > 0) return `${hours} 小时前`
  if (minutes > 0) return `${minutes} 分钟前`
  return `刚刚`
}

function addTag() {
  const tag = tagInput.value.trim()
  if (tag && !createForm.value.tags?.includes(tag)) {
    if (!createForm.value.tags) createForm.value.tags = []
    createForm.value.tags.push(tag)
    tagInput.value = ''
  }
}

function removeTag(index: number) {
  createForm.value.tags?.splice(index, 1)
}

function addEditTag() {
  const tag = editTagInput.value.trim()
  if (tag && !editForm.value.tags?.includes(tag)) {
    if (!editForm.value.tags) editForm.value.tags = []
    editForm.value.tags.push(tag)
    editTagInput.value = ''
  }
}

function removeEditTag(index: number) {
  editForm.value.tags?.splice(index, 1)
}

async function createRunner() {
  try {
    creating.value = true
    createError.value = null
    const response = await api.runners.userCreate(createForm.value)
    createdToken.value = response
    showCreateModal.value = false
    showTokenModal.value = true
    await loadRunners()
  } catch (err: any) {
    createError.value = err.response?.data?.error || err.message || '创建失败'
  } finally {
    creating.value = false
  }
}

function closeCreateModal() {
  showCreateModal.value = false
  createForm.value = {
    name: '',
    description: '',
    tags: [],
    run_untagged: true,
    locked: false,
    maximum_timeout: undefined,
  }
  tagInput.value = ''
  createError.value = null
}

function closeTokenModal() {
  showTokenModal.value = false
  createdToken.value = null
  tokenCopied.value = false
}

async function copyToken() {
  if (createdToken.value) {
    await navigator.clipboard.writeText(createdToken.value.token)
    tokenCopied.value = true
    setTimeout(() => { tokenCopied.value = false }, 2000)
  }
}

function editRunner(runner: Runner) {
  editingRunner.value = runner
  editForm.value = {
    name: runner.name,
    description: runner.description,
    tags: [...(runner.tags || [])],
    run_untagged: runner.run_untagged,
    locked: runner.locked,
    is_active: runner.is_active,
    maximum_timeout: runner.maximum_timeout,
  }
}

async function updateRunner() {
  if (!editingRunner.value) return
  
  try {
    updating.value = true
    editError.value = null
    await api.runners.userUpdate(editingRunner.value.id, editForm.value)
    await loadRunners()
    closeEditModal()
  } catch (err: any) {
    editError.value = err.response?.data?.error || err.message || '更新失败'
  } finally {
    updating.value = false
  }
}

function closeEditModal() {
  editingRunner.value = null
  editForm.value = {}
  editTagInput.value = ''
  editError.value = null
}

function confirmDelete(runner: Runner) {
  deletingRunner.value = runner
}

async function deleteRunner() {
  if (!deletingRunner.value) return
  
  try {
    deleting.value = true
    deleteError.value = null
    await api.runners.userDelete(deletingRunner.value.id)
    await loadRunners()
    closeDeleteModal()
  } catch (err: any) {
    deleteError.value = err.response?.data?.error || err.message || '删除失败'
  } finally {
    deleting.value = false
  }
}

function closeDeleteModal() {
  deletingRunner.value = null
  deleteError.value = null
}

onMounted(() => {
  loadRunners()
})
</script>

<style scoped lang="scss">
@import '@/styles/variables.scss';

.user-runners {
  padding: 24px;
  max-width: 1200px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 20px;

  .header-content {
    h1 {
      margin: 0 0 8px 0;
      font-size: 28px;
      font-weight: 600;
      color: #1f2937;
    }

    .page-description {
      margin: 0;
      color: #6b7280;
      font-size: 14px;
    }
  }
}

.info-banner {
  display: flex;
  gap: 12px;
  padding: 16px;
  background: #eff6ff;
  border: 1px solid #bfdbfe;
  border-radius: 8px;
  margin-bottom: 24px;

  svg {
    flex-shrink: 0;
    color: #3b82f6;
    margin-top: 2px;
  }

  div {
    flex: 1;

    strong {
      display: block;
      color: #1e40af;
      font-size: 14px;
      margin-bottom: 4px;
    }

    p {
      margin: 0;
      color: #1e3a8a;
      font-size: 13px;
      line-height: 1.5;
    }
  }
}

.runners-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
  gap: 20px;
}

.runner-card {
  background: white;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  padding: 20px;
  transition: all 0.2s;

  &:hover {
    border-color: #3b82f6;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
  }

  &.disabled {
    opacity: 0.6;
    background: #f9fafb;
  }

  .runner-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 12px;

    .runner-title {
      display: flex;
      align-items: center;
      gap: 10px;
      flex: 1;

      .runner-status-indicator {
        width: 10px;
        height: 10px;
        border-radius: 50%;
        flex-shrink: 0;

        &.status-idle {
          background: #10b981;
          box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.2);
        }

        &.status-running {
          background: #f59e0b;
          box-shadow: 0 0 0 3px rgba(245, 158, 11, 0.2);
          animation: pulse 2s infinite;
        }

        &.status-offline {
          background: #9ca3af;
        }
      }

      h3 {
        margin: 0;
        font-size: 18px;
        font-weight: 600;
        color: #1f2937;
      }
    }

    .runner-actions {
      display: flex;
      gap: 4px;
    }
  }

  .runner-description {
    color: #6b7280;
    font-size: 14px;
    margin-bottom: 16px;
    line-height: 1.5;
    min-height: 42px;
  }

  .runner-stats-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin-bottom: 16px;
    padding-bottom: 16px;
    border-bottom: 1px solid #f3f4f6;

    .stat-item {
      .stat-label {
        display: block;
        font-size: 12px;
        color: #9ca3af;
        margin-bottom: 4px;
      }

      .stat-value {
        font-size: 14px;
        color: #374151;
        font-weight: 500;
      }
    }
  }

  .runner-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 12px;
    min-height: 28px;

    .tag {
      display: inline-block;
      padding: 4px 10px;
      background: #eff6ff;
      color: #1e40af;
      border-radius: 12px;
      font-size: 12px;
      font-weight: 500;

      &.tag-special {
        background: #fef3c7;
        color: #92400e;
      }
    }
  }

  .runner-version {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: #6b7280;
    margin-bottom: 12px;

    svg {
      flex-shrink: 0;
    }
  }

  .runner-footer {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;

    .config-badge {
      display: inline-flex;
      align-items: center;
      gap: 4px;
      padding: 4px 8px;
      background: #f3f4f6;
      color: #4b5563;
      border-radius: 6px;
      font-size: 12px;
      font-weight: 500;

      svg {
        flex-shrink: 0;
      }
    }
  }
}

.badge {
  display: inline-block;
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;

  &.badge-idle {
    background: #d1fae5;
    color: #065f46;
  }

  &.badge-running {
    background: #fed7aa;
    color: #92400e;
  }

  &.badge-offline {
    background: #f3f4f6;
    color: #4b5563;
  }

  &.badge-disabled {
    background: #fee2e2;
    color: #991b1b;
  }
}

.tags-display {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;

  .tag-removable {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px 4px 12px;

    .tag-remove {
      background: none;
      border: none;
      color: currentColor;
      font-size: 18px;
      line-height: 1;
      cursor: pointer;
      padding: 0;
      opacity: 0.6;
      transition: opacity 0.15s;

      &:hover {
        opacity: 1;
      }
    }
  }
}

.token-display {
  .token-info {
    margin: 16px 0;

    label {
      display: block;
      font-size: 13px;
      font-weight: 600;
      color: #6b7280;
      margin-bottom: 6px;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    .info-value {
      font-size: 16px;
      color: #1f2937;
      font-weight: 600;
    }

    .token-value {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 12px;
      background: #1f2937;
      border: 2px solid #374151;
      border-radius: 6px;

      code {
        flex: 1;
        font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
        font-size: 13px;
        color: #10b981;
        word-break: break-all;
        line-height: 1.6;
      }
    }
  }

  .setup-instructions {
    margin-top: 24px;
    padding: 20px;
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 8px;

    h4 {
      margin: 0 0 8px 0;
      font-size: 14px;
      font-weight: 600;
      color: #374151;
    }

    p {
      margin: 0 0 12px 0;
      font-size: 13px;
      color: #6b7280;
    }

    pre {
      margin: 0;
      padding: 12px;
      background: #1f2937;
      border-radius: 6px;
      overflow-x: auto;

      code {
        color: #e5e7eb;
        font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
        font-size: 12px;
      }
    }
  }
}

.warning-box {
  display: flex;
  gap: 12px;
  padding: 16px;
  background: #fef3c7;
  border: 1px solid #fbbf24;
  border-radius: 6px;
  margin-top: 16px;

  svg {
    flex-shrink: 0;
    color: #d97706;
    margin-top: 2px;
  }

  div {
    flex: 1;

    strong {
      display: block;
      color: #92400e;
      margin-bottom: 8px;
    }

    ul {
      margin: 0;
      padding-left: 20px;
      color: #78350f;
      font-size: 13px;

      li {
        margin-bottom: 4px;
      }
    }
  }
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.text-muted {
  color: #9ca3af;
  font-size: 13px;
}

.loading-state,
.empty-state,
.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 20px;
  background: white;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
}

.loading-state {
  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid #e5e7eb;
    border-top-color: #3b82f6;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: 16px;
  }

  span {
    color: #6b7280;
    font-size: 14px;
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.empty-state {
  .empty-icon {
    margin-bottom: 20px;
  }

  .empty-title {
    margin: 0 0 8px 0;
    font-size: 18px;
    font-weight: 600;
    color: #1f2937;
  }

  .empty-description {
    margin: 0 0 24px 0;
    color: #6b7280;
    font-size: 14px;
    text-align: center;
  }
}

.error-state {
  .error-title {
    margin: 16px 0 8px 0;
    font-size: 18px;
    font-weight: 600;
    color: #dc2626;
  }

  .error-message {
    margin: 0 0 24px 0;
    color: #6b7280;
    font-size: 14px;
  }
}

.btn-large {
  padding: 12px 24px;
  font-size: 15px;
  font-weight: 600;
}

// Modal styles
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
  padding: 24px;
}

.modal {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: 12px;
  max-width: 600px;
  width: 100%;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1);

  &.modal-small {
    max-width: 480px;
  }

  &.modal-token {
    max-width: 700px;
  }
}

.modal-header {
  padding: 24px;
  border-bottom: 1px solid $border-color;
  display: flex;
  justify-content: space-between;
  align-items: center;

  h2 {
    font-size: 20px;
    font-weight: 600;
    margin: 0;
    color: $text-primary;
  }

  .btn-close {
    background: transparent;
    border: none;
    font-size: 28px;
    color: $text-secondary;
    cursor: pointer;
    padding: 0;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;

    &:hover {
      background: $bg-secondary;
      color: $text-primary;
    }
  }
}

.modal-body {
  padding: 24px;
  overflow-y: auto;
}

.modal-footer {
  padding: 16px 24px;
  border-top: 1px solid $border-color;
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.form-group {
  margin-bottom: 20px;

  label {
    display: block;
    margin-bottom: 8px;
    font-weight: 500;
    color: $text-primary;
    font-size: 14px;
  }

  .form-input {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid $border-color;
    border-radius: 6px;
    font-size: 14px;
    background: $bg-secondary;
    color: $text-primary;
    transition: border-color 0.2s;

    &:focus {
      outline: none;
      border-color: $primary-color;
    }
  }

  textarea.form-input {
    resize: vertical;
    min-height: 80px;
  }

  .form-help {
    margin: 6px 0 0 0;
    font-size: 13px;
    color: $text-secondary;
  }
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-weight: 400;

  input[type="checkbox"] {
    width: 18px;
    height: 18px;
    cursor: pointer;
  }
}

.tags-input {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 8px;
  border: 1px solid $border-color;
  border-radius: 6px;
  background: $bg-secondary;
  min-height: 44px;

  &:focus-within {
    border-color: $primary-color;
  }

  .tag {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    background: $bg-tertiary;
    border: 1px solid $border-color;
    border-radius: 4px;
    font-size: 13px;
    color: $text-primary;

    .tag-remove {
      background: transparent;
      border: none;
      color: $text-secondary;
      cursor: pointer;
      padding: 0;
      font-size: 16px;
      line-height: 1;

      &:hover {
        color: #ef4444;
      }
    }
  }

  input {
    flex: 1;
    min-width: 120px;
    border: none;
    background: transparent;
    outline: none;
    font-size: 14px;
    color: $text-primary;
  }
}

.token-display {
  position: relative;
  background: $bg-tertiary;
  border: 1px solid $border-color;
  border-radius: 6px;
  padding: 16px;
  margin-bottom: 24px;

  code {
    display: block;
    font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
    font-size: 13px;
    word-break: break-all;
    color: $text-primary;
    padding-right: 80px;
  }

  .btn-copy {
    position: absolute;
    top: 12px;
    right: 12px;
    padding: 6px 12px;
    background: $bg-secondary;
    border: 1px solid $border-color;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: $text-primary;
    transition: all 0.2s;

    &:hover {
      background: $bg-secondary;
    }

    &.copied {
      background: rgba(16, 185, 129, 0.1);
      border-color: #10b981;
      color: #10b981;
    }
  }
}

.setup-instructions {
  h3 {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 12px 0;
    color: $text-primary;
  }

  ol {
    margin: 0;
    padding-left: 24px;

    li {
      margin-bottom: 12px;
      color: $text-secondary;
      line-height: 1.6;

      pre {
        margin-top: 8px;
        padding: 12px;
        background: $bg-tertiary;
        border: 1px solid $border-color;
        border-radius: 4px;
        overflow-x: auto;

        code {
          font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
          font-size: 12px;
          color: $text-primary;
        }
      }
    }
  }
}
</style>
