<template>
  <div class="admin-runners">
    <div class="page-header">
      <div class="header-content">
        <h1>CI/CD Runners</h1>
        <p class="page-description">管理系统级共享 Runner，可供所有项目使用</p>
      </div>
      <button class="btn btn-primary" @click="showCreateModal = true">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        创建 Runner
      </button>
    </div>

    <!-- Stats Cards -->
    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-icon stat-icon-total">
          <svg viewBox="0 0 24 24" width="24" height="24" fill="none">
            <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="2"/>
            <path d="M8 12l3 3 5-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <div class="stat-content">
          <div class="stat-label">总 Runners</div>
          <div class="stat-value">{{ runners.length }}</div>
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-icon stat-icon-online">
          <svg viewBox="0 0 24 24" width="24" height="24" fill="none">
            <circle cx="12" cy="12" r="3" fill="currentColor"/>
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        </div>
        <div class="stat-content">
          <div class="stat-label">在线</div>
          <div class="stat-value">{{ onlineRunners }}</div>
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-icon stat-icon-running">
          <svg viewBox="0 0 24 24" width="24" height="24" fill="none">
            <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="2"/>
            <path d="M10 8l6 4-6 4V8z" fill="currentColor"/>
          </svg>
        </div>
        <div class="stat-content">
          <div class="stat-label">正在运行</div>
          <div class="stat-value">{{ runningRunners }}</div>
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-icon stat-icon-idle">
          <svg viewBox="0 0 24 24" width="24" height="24" fill="none">
            <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="2"/>
            <rect x="9" y="9" width="6" height="6" rx="1" fill="currentColor"/>
          </svg>
        </div>
        <div class="stat-content">
          <div class="stat-label">空闲</div>
          <div class="stat-value">{{ idleRunners }}</div>
        </div>
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
            <circle cx="32" cy="32" r="28" fill="#f0f9ff" stroke="#bfdbfe" stroke-width="2"/>
            <rect x="20" y="24" width="24" height="16" rx="2" stroke="#3b82f6" stroke-width="2"/>
            <circle cx="26" cy="32" r="2" fill="#3b82f6"/>
            <circle cx="32" cy="32" r="2" fill="#3b82f6"/>
            <circle cx="38" cy="32" r="2" fill="#3b82f6"/>
          </svg>
        </div>
        <p class="empty-title">尚未配置任何 Runner</p>
        <p class="empty-description">创建系统级 Runner 可供所有项目的 CI/CD 使用</p>
        <button class="btn btn-primary" @click="showCreateModal = true">创建第一个 Runner</button>
      </div>

      <div v-else class="runners-list">
        <div class="list-header">
          <div class="search-box">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <circle cx="7" cy="7" r="4" stroke="currentColor" stroke-width="1.5"/>
              <path d="M10 10l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <input v-model="searchQuery" type="text" placeholder="搜索 Runner..." />
          </div>
          <div class="filter-group">
            <select v-model="statusFilter" class="filter-select">
              <option value="">全部状态</option>
              <option value="idle">空闲</option>
              <option value="running">运行中</option>
              <option value="offline">离线</option>
            </select>
            <select v-model="activeFilter" class="filter-select">
              <option value="">全部</option>
              <option value="active">已启用</option>
              <option value="inactive">已禁用</option>
            </select>
          </div>
        </div>

        <div class="runners-table">
          <table>
            <thead>
              <tr>
                <th>Runner</th>
                <th>状态</th>
                <th>标签</th>
                <th>版本</th>
                <th>最后联系</th>
                <th>配置</th>
                <th class="actions-col">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="runner in filteredRunners" :key="runner.id" :class="{ disabled: !runner.is_active }">
                <td>
                  <div class="runner-info">
                    <div class="runner-status-dot" :class="`status-${runner.status}`"></div>
                    <div>
                      <div class="runner-name">{{ runner.name }}</div>
                      <div class="runner-desc">{{ runner.description || '无描述' }}</div>
                    </div>
                  </div>
                </td>
                <td>
                  <span class="badge" :class="`badge-${runner.status}`">
                    {{ getStatusText(runner.status) }}
                  </span>
                  <span v-if="!runner.is_active" class="badge badge-disabled">已禁用</span>
                </td>
                <td>
                  <div class="tags-list">
                    <span v-for="tag in runner.tags" :key="tag" class="tag">{{ tag }}</span>
                    <span v-if="runner.run_untagged" class="tag tag-special">untagged</span>
                    <span v-if="runner.tags.length === 0 && !runner.run_untagged" class="text-muted">无标签</span>
                  </div>
                </td>
                <td>
                  <div class="runner-version">
                    <div v-if="runner.version">{{ runner.version }}</div>
                    <div v-if="runner.platform" class="text-muted small">{{ runner.platform }} / {{ runner.architecture }}</div>
                    <div v-if="!runner.version" class="text-muted">未连接</div>
                  </div>
                </td>
                <td>
                  <div v-if="runner.last_contact_at" class="last-contact">
                    {{ formatTime(runner.last_contact_at) }}
                  </div>
                  <div v-else class="text-muted">从未</div>
                </td>
                <td>
                  <div class="runner-config">
                    <div v-if="runner.locked" class="config-item" title="锁定到当前作用域">
                      <svg viewBox="0 0 16 16" width="14" height="14" fill="none">
                        <rect x="4" y="7" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
                        <path d="M6 7V5a2 2 0 114 0v2" stroke="currentColor" stroke-width="1.5"/>
                      </svg>
                      锁定
                    </div>
                    <div v-if="runner.maximum_timeout" class="config-item" :title="`最大超时: ${runner.maximum_timeout}秒`">
                      <svg viewBox="0 0 16 16" width="14" height="14" fill="none">
                        <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
                        <path d="M8 5v3l2 2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                      </svg>
                      {{ runner.maximum_timeout }}s
                    </div>
                  </div>
                </td>
                <td class="actions-col">
                  <div class="action-buttons">
                    <button class="btn btn-icon" @click="editRunner(runner)" title="编辑">
                      <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
                        <path d="M11.5 2.5l2 2-8 8H3.5v-2l8-8z" stroke="currentColor" stroke-width="1.2"/>
                      </svg>
                    </button>
                    <button class="btn btn-icon" @click="toggleActive(runner)" :title="runner.is_active ? '禁用' : '启用'">
                      <svg v-if="runner.is_active" viewBox="0 0 16 16" width="16" height="16" fill="none">
                        <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.2"/>
                        <path d="M6 8l2 2 3-4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
                      </svg>
                      <svg v-else viewBox="0 0 16 16" width="16" height="16" fill="none">
                        <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.2"/>
                        <path d="M6 6l4 4M10 6l-4 4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                      </svg>
                    </button>
                    <button class="btn btn-icon btn-danger" @click="confirmDelete(runner)" title="删除">
                      <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
                        <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                      </svg>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </template>

    <!-- Create Modal -->
    <div v-if="showCreateModal" class="modal-overlay" @click.self="closeCreateModal">
      <div class="modal-content modal-large">
        <div class="modal-header">
          <h3>创建系统级 Runner</h3>
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
              placeholder="例如: system-runner-1"
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
              placeholder="描述此 Runner 的用途和配置..."
            ></textarea>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="runner-tags">标签</label>
              <input
                id="runner-tags"
                v-model="tagInput"
                type="text"
                class="form-input"
                placeholder="输入标签后按回车添加"
                @keydown.enter.prevent="addTag"
              />
              <div v-if="createForm.tags && createForm.tags.length > 0" class="tags-display">
                <span v-for="(tag, index) in createForm.tags" :key="index" class="tag tag-removable">
                  {{ tag }}
                  <button type="button" @click="removeTag(index)" class="tag-remove">×</button>
                </span>
              </div>
              <p class="form-hint">用于匹配指定标签的 CI/CD 任务</p>
            </div>
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
              <span>锁定到系统级</span>
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
              placeholder="留空使用默认值"
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
              <span v-else>创建</span>
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- Token Display Modal -->
    <div v-if="showTokenModal && createdToken" class="modal-overlay" @click.self="closeTokenModal">
      <div class="modal-content">
        <div class="modal-header">
          <h3>Runner 创建成功</h3>
          <button class="modal-close" @click="closeTokenModal">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <div class="token-display">
          <div class="alert alert-warning">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M8 2L2 14h12L8 2z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
              <path d="M8 6v3M8 11v1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <div>
              <strong>请立即保存此 Token！</strong>
              <p>此 Token 仅显示一次，之后将无法再次查看。</p>
            </div>
          </div>

          <div class="token-info">
            <label>Runner 名称</label>
            <div class="info-value">{{ createdToken.runner.name }}</div>
          </div>

          <div class="token-info">
            <label>Runner Token</label>
            <div class="token-value">
              <code>{{ createdToken.token }}</code>
              <button class="btn btn-icon" @click="copyToken" :title="tokenCopied ? '已复制' : '复制'">
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
            <h4>使用此 Token 启动 Runner：</h4>
            <pre><code>gitfox-runner --url {{ serverUrl }} --token {{ createdToken.token }}</code></pre>
          </div>
        </div>

        <div class="modal-actions">
          <button class="btn btn-primary" @click="closeTokenModal">我已保存</button>
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
              <span>锁定到系统级</span>
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
          <h3>确认删除</h3>
          <button class="modal-close" @click="closeDeleteModal">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <div class="modal-body">
          <p>确定要删除 Runner <strong>{{ deletingRunner.name }}</strong> 吗？</p>
          <p class="text-muted">此操作无法撤销，已连接的 Runner 将无法继续工作。</p>

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
import { ref, computed, onMounted } from 'vue'
import { api } from '@/api'
import type { Runner, CreateRunnerRequest, UpdateRunnerRequest, CreateRunnerResponse } from '@/types'

const loading = ref(true)
const error = ref<string | null>(null)
const runners = ref<Runner[]>([])

const searchQuery = ref('')
const statusFilter = ref('')
const activeFilter = ref('')

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

const onlineRunners = computed(() => 
  runners.value.filter(r => r.status !== 'offline' && r.is_active).length
)
const runningRunners = computed(() => 
  runners.value.filter(r => r.status === 'running').length
)
const idleRunners = computed(() => 
  runners.value.filter(r => r.status === 'idle').length
)

const filteredRunners = computed(() => {
  return runners.value.filter(runner => {
    const matchesSearch = !searchQuery.value || 
      runner.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
      runner.description?.toLowerCase().includes(searchQuery.value.toLowerCase())
    
    const matchesStatus = !statusFilter.value || runner.status === statusFilter.value
    const matchesActive = !activeFilter.value || 
      (activeFilter.value === 'active' ? runner.is_active : !runner.is_active)
    
    return matchesSearch && matchesStatus && matchesActive
  })
})

async function loadRunners() {
  try {
    loading.value = true
    error.value = null
    runners.value = await api.runners.adminList()
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
  return `${seconds} 秒前`
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
    const response = await api.runners.adminCreate(createForm.value)
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
    await api.runners.adminUpdate(editingRunner.value.id, editForm.value)
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

async function toggleActive(runner: Runner) {
  try {
    await api.runners.adminUpdate(runner.id, { is_active: !runner.is_active })
    await loadRunners()
  } catch (err: any) {
    error.value = err.response?.data?.error || err.message || '操作失败'
  }
}

function confirmDelete(runner: Runner) {
  deletingRunner.value = runner
}

async function deleteRunner() {
  if (!deletingRunner.value) return
  
  try {
    deleting.value = true
    deleteError.value = null
    await api.runners.adminDelete(deletingRunner.value.id)
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

.admin-runners {
  padding: 24px;
  max-width: 1400px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;

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

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 16px;
  margin-bottom: 24px;
}

.stat-card {
  background: white;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  padding: 20px;
  display: flex;
  align-items: center;
  gap: 16px;

  .stat-icon {
    width: 48px;
    height: 48px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;

    &.stat-icon-total {
      background: #eff6ff;
      color: #3b82f6;
    }

    &.stat-icon-online {
      background: #f0fdf4;
      color: #10b981;
    }

    &.stat-icon-running {
      background: #fef3c7;
      color: #f59e0b;
    }

    &.stat-icon-idle {
      background: #f3f4f6;
      color: #6b7280;
    }
  }

  .stat-content {
    flex: 1;

    .stat-label {
      font-size: 14px;
      color: #6b7280;
      margin-bottom: 4px;
    }

    .stat-value {
      font-size: 24px;
      font-weight: 600;
      color: #1f2937;
    }
  }
}

.list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  gap: 16px;
  flex-wrap: wrap;

  .search-box {
    flex: 1;
    min-width: 300px;
    position: relative;
    display: flex;
    align-items: center;

    svg {
      position: absolute;
      left: 12px;
      color: #9ca3af;
    }

    input {
      width: 100%;
      padding: 8px 12px 8px 36px;
      border: 1px solid #d1d5db;
      border-radius: 6px;
      font-size: 14px;

      &:focus {
        outline: none;
        border-color: #3b82f6;
        box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
      }
    }
  }

  .filter-group {
    display: flex;
    gap: 8px;

    .filter-select {
      padding: 8px 12px;
      border: 1px solid #d1d5db;
      border-radius: 6px;
      font-size: 14px;
      background: white;

      &:focus {
        outline: none;
        border-color: #3b82f6;
      }
    }
  }
}

.runners-table {
  background: white;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  overflow: hidden;

  table {
    width: 100%;
    border-collapse: collapse;

    thead {
      background: #f9fafb;
      border-bottom: 1px solid #e5e7eb;

      th {
        padding: 12px 16px;
        text-align: left;
        font-size: 12px;
        font-weight: 600;
        color: #6b7280;
        text-transform: uppercase;
        letter-spacing: 0.05em;

        &.actions-col {
          width: 140px;
          text-align: center;
        }
      }
    }

    tbody {
      tr {
        border-bottom: 1px solid #f3f4f6;
        transition: background 0.15s;

        &:hover {
          background: #f9fafb;
        }

        &.disabled {
          opacity: 0.6;
        }

        &:last-child {
          border-bottom: none;
        }
      }

      td {
        padding: 16px;
        font-size: 14px;
        color: #374151;
        vertical-align: middle;

        &.actions-col {
          text-align: center;
        }
      }
    }
  }
}

.runner-info {
  display: flex;
  align-items: center;
  gap: 12px;

  .runner-status-dot {
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

  .runner-name {
    font-weight: 500;
    color: #1f2937;
  }

  .runner-desc {
    font-size: 13px;
    color: #6b7280;
    margin-top: 2px;
  }
}

.tags-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;

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
  .small {
    font-size: 12px;
    margin-top: 2px;
  }
}

.runner-config {
  display: flex;
  flex-direction: column;
  gap: 4px;

  .config-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: #6b7280;

    svg {
      flex-shrink: 0;
    }
  }
}

.action-buttons {
  display: flex;
  gap: 4px;
  justify-content: center;
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
    margin-left: 6px;
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
      font-weight: 500;
      color: #6b7280;
      margin-bottom: 6px;
    }

    .info-value {
      font-size: 15px;
      color: #1f2937;
      font-weight: 500;
    }

    .token-value {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 12px;
      background: #f9fafb;
      border: 1px solid #e5e7eb;
      border-radius: 6px;

      code {
        flex: 1;
        font-family: 'Monaco', 'Menlo', monospace;
        font-size: 13px;
        color: #1f2937;
        word-break: break-all;
      }
    }
  }

  .setup-instructions {
    margin-top: 20px;
    padding: 16px;
    background: #f9fafb;
    border-radius: 6px;

    h4 {
      margin: 0 0 12px 0;
      font-size: 14px;
      font-weight: 600;
      color: #374151;
    }

    pre {
      margin: 0;
      padding: 12px;
      background: #1f2937;
      border-radius: 4px;
      overflow-x: auto;

      code {
        color: #f3f4f6;
        font-family: 'Monaco', 'Menlo', monospace;
        font-size: 13px;
      }
    }
  }
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}

.text-muted {
  color: #9ca3af;
}

.loading-state,
.empty-state,
.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
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
