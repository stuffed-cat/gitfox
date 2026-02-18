<template>
  <div class="pipeline-list">
    <!-- 顶栏：标签页 + 操作 -->
    <div class="pipeline-toolbar">
      <div class="toolbar-tabs">
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'all' }"
          @click="activeTab = 'all'"
        >
          全部
          <span class="tab-count">{{ pipelines.length }}</span>
        </button>
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'running' }"
          @click="activeTab = 'running'"
        >
          运行中
          <span class="tab-count">{{ pipelines.filter(p => p.status === 'running').length }}</span>
        </button>
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'finished' }"
          @click="activeTab = 'finished'"
        >
          已完成
        </button>
      </div>
      <button class="btn btn-primary btn-sm" @click="showTriggerModal = true">
        <svg viewBox="0 0 16 16" width="14" height="14">
          <path :d="icons.play" fill="currentColor" />
        </svg>
        运行流水线
      </button>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="loading-spinner"></div>
    </div>

    <div v-else-if="filteredPipelines.length === 0" class="empty-state">
      <svg viewBox="0 0 48 48" width="48" height="48">
        <rect x="4" y="8" width="16" height="16" rx="2" stroke="currentColor" stroke-width="2" fill="none"/>
        <rect x="28" y="8" width="16" height="16" rx="2" stroke="currentColor" stroke-width="2" fill="none"/>
        <rect x="16" y="28" width="16" height="16" rx="2" stroke="currentColor" stroke-width="2" fill="none"/>
        <path d="M20 16h8M12 32h-4M36 32h4" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
      <h3>暂无流水线</h3>
      <p>在 <code>.gitfox/ci/</code> 目录下添加 YAML 配置文件以启用 CI/CD，提交后自动触发</p>
      <button class="btn btn-primary" @click="showTriggerModal = true">手动运行</button>
    </div>

    <div v-else class="pipelines-table">
      <!-- 流水线行 -->
      <div
        v-for="pipeline in filteredPipelines"
        :key="pipeline.id"
        class="pipeline-row"
      >
        <!-- 状态 -->
        <div class="col-status">
          <router-link
            :to="`/${project?.owner_name}/${project?.name}/-/pipelines/${pipeline.id}`"
            class="status-link"
            :title="getStatusLabel(pipeline.status)"
          >
            <CiStatusIcon :status="pipeline.status" :size="22" />
            <span class="status-label">{{ getStatusLabel(pipeline.status) }}</span>
          </router-link>
          <div v-if="pipeline.error_message" class="error-message" :title="pipeline.error_message">
            {{ pipeline.error_message }}
          </div>
        </div>

        <!-- 流水线信息 -->
        <div class="col-pipeline">
          <div class="pipeline-id">
            <router-link
              :to="`/${project?.owner_name}/${project?.name}/-/pipelines/${pipeline.id}`"
              class="pipeline-link"
            >
              #{{ String(pipeline.id).substring(0, 8) }}
            </router-link>
          </div>
          <div class="pipeline-trigger">
            <svg viewBox="0 0 16 16" width="12" height="12">
              <circle cx="5" cy="4" r="2" stroke="currentColor" stroke-width="1.5" fill="none"/>
              <circle cx="5" cy="12" r="2" stroke="currentColor" stroke-width="1.5" fill="none"/>
              <path d="M5 6v4" stroke="currentColor" stroke-width="1.5"/>
              <path d="M7 12h4a2 2 0 002-2V8" stroke="currentColor" stroke-width="1.5" fill="none"/>
              <circle cx="11" cy="6" r="2" stroke="currentColor" stroke-width="1.5" fill="none"/>
            </svg>
            <router-link
              :to="`/${project?.owner_name}/${project?.name}/-/commits/${pipeline.ref_name}`"
              class="ref-link"
            >
              {{ pipeline.ref_name }}
            </router-link>
          </div>
          <div class="pipeline-commit">
            <svg viewBox="0 0 16 16" width="12" height="12">
              <circle cx="8" cy="8" r="3" stroke="currentColor" stroke-width="1.5" fill="none"/>
              <path d="M1 8h4M11 8h4" stroke="currentColor" stroke-width="1.5"/>
            </svg>
            <router-link
              :to="`/${project?.owner_name}/${project?.name}/-/commit/${pipeline.commit_sha}`"
              class="sha-link"
            >
              {{ pipeline.commit_sha?.substring(0, 8) }}
            </router-link>
            <span class="pipeline-time">{{ formatDate(pipeline.created_at) }}</span>
          </div>
        </div>

        <!-- 阶段气泡 -->
        <div class="col-stages">
          <div v-if="pipeline.jobs && pipeline.jobs.length > 0" class="stages-bubbles">
            <div
              v-for="job in pipeline.jobs"
              :key="job.id"
              class="job-bubble"
              :class="getJobBubbleClass(job)"
              :title="getJobStatusText(job)"
            >
              <CiStatusIcon :status="getJobBubbleClass(job)" :size="14" />
            </div>
          </div>
          <span v-else class="no-stages">—</span>
        </div>

        <!-- 耗时 -->
        <div class="col-duration">
          <span v-if="pipeline.duration_seconds" class="duration">
            {{ formatDuration(pipeline.duration_seconds) }}
          </span>
          <span v-else class="duration-empty">—</span>
        </div>

        <!-- 操作 -->
        <div class="col-actions">
          <button
            v-if="pipeline.status === 'running'"
            class="action-btn cancel"
            title="取消"
            @click.prevent="cancelPipeline(pipeline.id)"
          >
            <svg viewBox="0 0 16 16" width="14" height="14">
              <rect x="4" y="4" width="8" height="8" rx="1" fill="currentColor"/>
            </svg>
          </button>
          <button
            v-else-if="['failed', 'canceled'].includes(pipeline.status)"
            class="action-btn retry"
            title="重试"
            @click.prevent="retryPipeline(pipeline.id)"
          >
            <svg viewBox="0 0 16 16" width="14" height="14">
              <path d="M2 8a6 6 0 006 6 6 6 0 006-6 6 6 0 00-6-6" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/>
              <path d="M2 4v4h4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <router-link
            :to="`/${project?.owner_name}/${project?.name}/-/pipelines/${pipeline.id}`"
            class="action-btn detail"
            title="查看详情"
          >
            <svg viewBox="0 0 16 16" width="14" height="14">
              <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </router-link>
        </div>
      </div>
    </div>

    <!-- 触发流水线弹窗 -->
    <div v-if="showTriggerModal" class="modal-overlay" @click.self="showTriggerModal = false">
      <div class="modal">
        <div class="modal-header">
          <h3>运行流水线</h3>
          <button class="btn-close" @click="showTriggerModal = false">
            <svg viewBox="0 0 16 16" width="16" height="16">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="2" fill="none"/>
            </svg>
          </button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label>分支或标签</label>
            <input
              v-model="triggerRef"
              type="text"
              class="form-input"
              placeholder="例如: master, main, develop"
              @keyup.enter="triggerPipeline"
              autofocus
            />
            <p class="form-hint">在指定的分支/标签上运行 CI/CD 流水线（需存在 .gitfox/ci/ 目录）</p>
            <p v-if="triggerError" class="form-error">{{ triggerError }}</p>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn" @click="showTriggerModal = false">取消</button>
          <button class="btn btn-primary" @click="triggerPipeline" :disabled="triggering || !triggerRef">
            {{ triggering ? '运行中...' : '运行流水线' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import api from '@/api'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import type { Project, Pipeline, PipelineJob } from '@/types'
import { navIcons } from '@/navigation/icons'
import CiStatusIcon from '@/components/CiStatusIcon.vue'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

const icons = navIcons

interface PipelineWithJobs extends Pipeline {
  jobs?: PipelineJob[]
}

const props = defineProps<{ project?: Project }>()

const loading = ref(false)
const pipelines = ref<Pipeline[]>([])
const pipelinesWithJobs = ref<PipelineWithJobs[]>([])
const activeTab = ref<'all' | 'running' | 'finished'>('all')
const showTriggerModal = ref(false)
const triggerRef = ref('master')
const triggering = ref(false)
const triggerError = ref('')

const filteredPipelines = computed(() => {
  if (activeTab.value === 'running') {
    return pipelinesWithJobs.value.filter(p => p.status === 'running')
  }
  if (activeTab.value === 'finished') {
    return pipelinesWithJobs.value.filter(p => ['success', 'failed', 'canceled', 'skipped'].includes(p.status))
  }
  return pipelinesWithJobs.value
})

function formatDate(date: string) {
  return dayjs(date).fromNow()
}

function formatDuration(seconds: number) {
  if (seconds < 60) return `${seconds} 秒`
  if (seconds < 3600) return `${Math.floor(seconds / 60)} 分 ${seconds % 60} 秒`
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return `${h} 时 ${m} 分`
}

function getStatusLabel(status: string): string {
  const map: Record<string, string> = {
    pending: '等待中',
    running: '运行中',
    success: '已通过',
    failed: '已失败',
    canceled: '已取消',
    skipped: '已跳过'
  }
  return map[status] || status
}

function getJobBubbleClass(job: PipelineJob): string {
  if (job.status === 'pending' && !job.runner_id) return 'blocked'
  if ((job.status === 'success' || job.status === 'failed') && job.allow_failure) return 'warning'
  return job.status
}

function getJobStatusText(job: PipelineJob): string {
  const statusText: Record<string, string> = {
    pending: '等待中',
    running: '运行中',
    success: '已成功',
    failed: '已失败',
    canceled: '已取消',
    skipped: '已跳过'
  }
  const label = job.status === 'pending' && !job.runner_id
    ? '阻塞中（无可用 Runner）'
    : ((job.status === 'success' || job.status === 'failed') && job.allow_failure
        ? `${statusText[job.status]}（允许失败）`
        : statusText[job.status] || job.status)
  return `${job.name}: ${label}`
}

async function loadPipelines() {
  if (!props.project?.owner_name || !props.project?.name) return
  loading.value = true
  const path = { namespace: props.project.owner_name, project: props.project.name }

  try {
    pipelines.value = await api.pipelines.list(path)
    const data: PipelineWithJobs[] = await Promise.all(
      pipelines.value.map(async (pipeline): Promise<PipelineWithJobs> => {
        try {
          const result = await api.pipelines.get(path, pipeline.id)
          return { ...pipeline, jobs: result.jobs || [] }
        } catch {
          return { ...pipeline, jobs: [] }
        }
      })
    )
    pipelinesWithJobs.value = data
  } catch (err) {
    console.error('Failed to load pipelines:', err)
  } finally {
    loading.value = false
  }
}

async function triggerPipeline() {
  if (!props.project?.owner_name || !props.project?.name || !triggerRef.value) return
  triggering.value = true
  triggerError.value = ''
  const path = { namespace: props.project.owner_name, project: props.project.name }
  try {
    await api.pipelines.trigger(path, triggerRef.value)
    showTriggerModal.value = false
    await loadPipelines()
  } catch (err: any) {
    const msg = err?.response?.data?.message || err?.message || '触发失败，请检查分支是否存在'
    triggerError.value = msg
    console.error('Failed to trigger pipeline:', err)
  } finally {
    triggering.value = false
  }
}

async function cancelPipeline(id: string) {
  if (!props.project?.owner_name || !props.project?.name) return
  const path = { namespace: props.project.owner_name, project: props.project.name }
  try {
    await api.pipelines.cancel(path, id)
    await loadPipelines()
  } catch (err) {
    console.error('Failed to cancel pipeline:', err)
  }
}

async function retryPipeline(id: string) {
  if (!props.project?.owner_name || !props.project?.name) return
  const path = { namespace: props.project.owner_name, project: props.project.name }
  try {
    await api.pipelines.retry(path, id)
    await loadPipelines()
  } catch (err) {
    console.error('Failed to retry pipeline:', err)
  }
}

watch([() => props.project?.owner_name, () => props.project?.name], () => {
  loadPipelines()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.pipeline-list {
  padding: 0;
}

// ── 工具栏 ──────────────────────────────────────────────────
.pipeline-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px 0;
}

.toolbar-tabs {
  display: flex;
  gap: 4px;
}

.tab-btn {
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 8px 4px;
  margin-right: 16px;
  font-size: $font-size-sm;
  color: $text-muted;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  transition: all 0.15s;
  border-radius: 0;

  &:hover {
    color: $text-primary;
  }

  &.active {
    color: $text-primary;
    font-weight: 500;
    border-bottom-color: $primary-color;
  }

  .tab-count {
    background: $bg-secondary;
    border-radius: 10px;
    padding: 1px 6px;
    font-size: 11px;
    min-width: 18px;
    text-align: center;
  }
}

.btn-sm {
  padding: 6px 12px;
  font-size: $font-size-sm;
  display: flex;
  align-items: center;
  gap: 6px;
}

// ── 空状态 ──────────────────────────────────────────────────
.loading-state {
  display: flex;
  justify-content: center;
  padding: 48px;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 64px 24px;
  color: $text-muted;

  svg {
    margin-bottom: 16px;
    opacity: 0.4;
  }

  h3 {
    margin: 0 0 8px;
    font-size: $font-size-lg;
    color: $text-primary;
  }

  p {
    margin: 0 0 20px;
    font-size: $font-size-sm;

    code {
      background: $bg-secondary;
      padding: 2px 6px;
      border-radius: 4px;
      font-size: $font-size-xs;
    }
  }
}

// ── 表格 ──────────────────────────────────────────────────────
.pipelines-table {
  border-top: 1px solid $border-color;
  margin-top: 0;
}

.pipeline-row {
  display: grid;
  grid-template-columns: 120px 1fr 180px 72px 72px;
  align-items: center;
}

.pipeline-row {
  padding: 12px 16px;
  border-bottom: 1px solid $border-color;
  transition: background 0.1s;

  &:last-child {
    border-bottom: none;
  }

  &:hover {
    background: $bg-secondary;
  }
}

// ── 状态列 ──────────────────────────────────────────────────
.col-status {
  .status-link {
    display: flex;
    align-items: center;
    gap: 8px;
    text-decoration: none;
  }

  .status-label {
    font-size: $font-size-sm;
    font-weight: 500;
  }

  .error-message {
    margin-top: 4px;
    font-size: 12px;
    color: #dd2b0e;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-svg {
    flex-shrink: 0;

    &.pending { color: #9ca3af; }
    &.running {
      color: $primary-color;
      animation: spin 1.5s linear infinite;
    }
    &.success { color: #22c55e; }
    &.failed { color: #ef4444; }
    &.canceled { color: #6b7280; }
    &.skipped { color: #9ca3af; }
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

// ── 流水线信息列 ────────────────────────────────────────────
.col-pipeline {
  min-width: 0;
  padding-right: 12px;
}

.pipeline-id {
  margin-bottom: 4px;

  .pipeline-link {
    font-family: monospace;
    font-weight: 600;
    font-size: $font-size-sm;
    color: $text-primary;
    text-decoration: none;

    &:hover {
      color: $primary-color;
      text-decoration: underline;
    }
  }
}

.pipeline-trigger,
.pipeline-commit {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: $text-muted;
  margin-top: 2px;
  overflow: hidden;

  svg {
    flex-shrink: 0;
  }
}

.ref-link {
  color: $primary-color;
  text-decoration: none;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 180px;
  font-weight: 500;
  font-size: 12px;

  &:hover { text-decoration: underline; }
}

.sha-link {
  font-family: monospace;
  font-size: 11px;
  color: $text-muted;
  text-decoration: none;

  &:hover { color: $primary-color; }
}

.pipeline-time {
  font-size: 11px;
  color: $text-muted;
  white-space: nowrap;
}

// ── 阶段气泡列 ──────────────────────────────────────────────
.col-stages {
  .stages-bubbles {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .no-stages {
    color: $text-muted;
    font-size: $font-size-sm;
  }
}

.job-bubble {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: default;
  border-radius: 50%;
  transition: transform 0.1s;

  &:hover { transform: scale(1.2); }
}

// ── 耗时列 ──────────────────────────────────────────────────
.col-duration {
  text-align: center;
  font-size: 12px;
  color: $text-muted;
  white-space: nowrap;

  .duration-empty { color: $text-muted; }
}

// ── 操作列 ──────────────────────────────────────────────────
.col-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 6px;
}

.action-btn {
  width: 28px;
  height: 28px;
  border-radius: 6px;
  border: 1px solid $border-color;
  background: $bg-primary;
  color: $text-muted;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.1s;
  text-decoration: none;

  &:hover {
    background: $bg-secondary;
    color: $text-primary;
  }

  &.cancel:hover {
    background: #fee2e2;
    border-color: #fca5a5;
    color: #dc2626;
  }

  &.retry:hover {
    background: #dbeafe;
    border-color: #93c5fd;
    color: $primary-color;
  }
}

// ── 弹窗 ────────────────────────────────────────────────────
.modal-overlay {
  position: fixed;
  inset: 0;
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
  max-width: 480px;
  width: 100%;
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 18px 24px;
  border-bottom: 1px solid $border-color;

  h3 {
    margin: 0;
    font-size: $font-size-lg;
    font-weight: 600;
  }
}

.btn-close {
  background: none;
  border: none;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  color: $text-muted;
  border-radius: 6px;

  &:hover { background: $bg-secondary; color: $text-primary; }
}

.modal-body {
  padding: 20px 24px;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 16px 24px;
  border-top: 1px solid $border-color;
}

.form-group {
  label {
    display: block;
    margin-bottom: 6px;
    font-weight: 500;
    font-size: $font-size-sm;
  }
}

.form-hint {
  margin: 6px 0 0;
  font-size: 12px;
  color: $text-muted;
}

.form-error {
  margin: 8px 0 0;
  font-size: 12px;
  color: #dd2b0e;
  background: #fef3f2;
  border: 1px solid #f5c6be;
  border-radius: 4px;
  padding: 6px 10px;
}

.form-input {
  width: 100%;
  padding: 9px 12px;
  border: 1px solid $border-color;
  border-radius: 6px;
  background: $bg-primary;
  color: $text-primary;
  font-size: $font-size-sm;
  box-sizing: border-box;

  &:focus {
    outline: none;
    border-color: $primary-color;
    box-shadow: 0 0 0 3px rgba($primary-color, 0.12);
  }

  &::placeholder { color: $text-muted; }
}
</style>

