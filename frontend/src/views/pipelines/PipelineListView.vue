<template>
  <div class="pipeline-list">
    <div class="list-header">
      <h3>流水线</h3>
      <button class="btn btn-primary" @click="showTriggerModal = true">
        <svg class="btn-icon" viewBox="0 0 16 16" width="14" height="14">
          <path :d="icons.play" fill="currentColor" />
        </svg>
        运行流水线
      </button>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <div v-else-if="pipelines.length === 0" class="empty-state">
      <h3>暂无流水线</h3>
      <p>配置 CI/CD 流水线自动化构建和部署</p>
    </div>
    
    <div v-else class="pipelines">
      <div
        v-for="pipeline in pipelinesWithJobs"
        :key="pipeline.id"
        class="pipeline-card"
      >
        <router-link
          :to="`/${project?.owner_name}/${project?.name}/-/pipelines/${pipeline.id}`"
          class="pipeline-item"
        >
          <div class="pipeline-status">
            <svg class="status-icon" :class="pipeline.status" viewBox="0 0 16 16" width="20" height="20">
              <path :d="getStatusIconPath(pipeline.status)" fill="currentColor" />
            </svg>
          </div>
          <div class="pipeline-info">
            <div class="pipeline-title">
              流水线 #{{ pipeline.id.toString().substring(0, 8) }}
              <span class="ref">{{ pipeline.ref_name }}</span>
            </div>
            <div class="pipeline-meta">
              <code>{{ pipeline.commit_sha?.substring(0, 8) }}</code>
              <span class="separator">·</span>
              <span>{{ formatDate(pipeline.created_at) }}</span>
            </div>
          </div>
          <div class="pipeline-duration" v-if="pipeline.duration_seconds">
            <svg viewBox="0 0 16 16" width="12" height="12">
              <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5" fill="none" />
              <path d="M8 4v4l3 2" stroke="currentColor" stroke-width="1.5" fill="none" />
            </svg>
            {{ formatDuration(pipeline.duration_seconds) }}
          </div>
        </router-link>
        
        <!-- Job status overview -->
        <div v-if="pipeline.jobs && pipeline.jobs.length > 0" class="jobs-overview">
          <div
            v-for="job in pipeline.jobs"
            :key="job.id"
            class="job-badge"
            :class="getJobStatusClass(job)"
            :title="getJobStatusText(job)"
          >
            <svg class="job-icon" viewBox="0 0 16 16" width="12" height="12">
              <path :d="getJobIconPath(job)" fill="currentColor" />
            </svg>
            <span class="job-name">{{ job.name }}</span>
          </div>
        </div>
      </div>
    </div>
    
    <!-- Trigger Pipeline Modal -->
    <div v-if="showTriggerModal" class="modal-overlay" @click.self="showTriggerModal = false">
      <div class="modal">
        <div class="modal-header">
          <h3>运行流水线</h3>
          <button class="btn-close" @click="showTriggerModal = false">
            <svg viewBox="0 0 16 16" width="16" height="16">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="2" fill="none" />
            </svg>
          </button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label>选择分支或标签</label>
            <input
              v-model="triggerRef"
              type="text"
              class="form-input"
              placeholder="例如: master, main, develop"
              @keyup.enter="triggerPipeline"
            />
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn" @click="showTriggerModal = false">取消</button>
          <button class="btn btn-primary" @click="triggerPipeline" :disabled="!triggerRef">
            运行
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import api from '@/api'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import type { Project, Pipeline, PipelineJob } from '@/types'
import { navIcons } from '@/navigation/icons'

const icons = navIcons

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

interface PipelineWithJobs extends Pipeline {
  jobs?: PipelineJob[]
}

const props = defineProps<{
  project?: Project
}>()

const loading = ref(false)
const pipelines = ref<Pipeline[]>([])
const pipelinesWithJobs = ref<PipelineWithJobs[]>([])
const showTriggerModal = ref(false)
const triggerRef = ref('master')

function formatDate(date: string) {
  return dayjs(date).fromNow()
}

function formatDuration(seconds: number) {
  if (seconds < 60) return `${seconds}秒`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}分${seconds % 60}秒`
  const hours = Math.floor(seconds / 3600)
  const mins = Math.floor((seconds % 3600) / 60)
  return `${hours}时${mins}分`
}

function getStatusIconPath(status: string): string {
  const map: Record<string, string> = {
    pending: icons.statusPending,
    running: icons.statusRunning,
    success: icons.statusSuccess,
    failed: icons.statusFailed,
    canceled: icons.statusCanceled,
    skipped: icons.statusSkipped
  }
  return map[status] || icons.statusPending
}

function getJobIconPath(job: PipelineJob): string {
  if (job.status === 'pending' && !job.runner_id) {
    return icons.statusBlocked
  }
  if (job.status === 'pending') {
    return icons.statusPending
  }
  if (job.status === 'running') {
    return icons.statusRunning
  }
  if (job.status === 'success') {
    return job.allow_failure ? icons.statusWarning : icons.statusSuccess
  }
  if (job.status === 'failed') {
    return job.allow_failure ? icons.statusWarning : icons.statusFailed
  }
  if (job.status === 'canceled') {
    return icons.statusCanceled
  }
  return icons.statusPending
}

function getJobStatusClass(job: PipelineJob): string {
  if (job.status === 'pending' && !job.runner_id) {
    return 'job-blocked' // 阻塞中
  }
  if (job.status === 'pending') {
    return 'job-waiting' // 等待中
  }
  if (job.status === 'running') {
    return 'job-running'
  }
  if (job.status === 'success') {
    return job.allow_failure ? 'job-warning' : 'job-success'
  }
  if (job.status === 'failed') {
    return job.allow_failure ? 'job-warning' : 'job-failed'
  }
  if (job.status === 'canceled') {
    return 'job-canceled'
  }
  return 'job-unknown'
}

function getJobStatusText(job: PipelineJob): string {
  if (job.status === 'pending' && !job.runner_id) {
    return `${job.name} - 阻塞中（没有可用的 Runner）`
  }
  if (job.status === 'pending') {
    return `${job.name} - 等待中`
  }
  if (job.status === 'running') {
    return `${job.name} - 运行中`
  }
  if (job.status === 'success') {
    return job.allow_failure ? `${job.name} - 成功（允许失败）` : `${job.name} - 成功`
  }
  if (job.status === 'failed') {
    return job.allow_failure ? `${job.name} - 失败（允许失败）` : `${job.name} - 失败`
  }
  if (job.status === 'canceled') {
    return `${job.name} - 已取消`
  }
  return `${job.name} - 未知状态`
}

async function loadPipelines() {
  if (!props.project?.owner_name || !props.project?.name) return
  loading.value = true
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    pipelines.value = await api.pipelines.list(path)
    
    // Load jobs for each pipeline
    const pipelinesData = await Promise.all(
      pipelines.value.map(async (pipeline) => {
        try {
          const result = await api.pipelines.get(path, pipeline.id)
          return {
            ...pipeline,
            jobs: result.jobs || []
          }
        } catch (error) {
          console.error(`Failed to load jobs for pipeline ${pipeline.id}:`, error)
          return { ...pipeline, jobs: [] }
        }
      })
    )
    pipelinesWithJobs.value = pipelinesData
  } catch (error) {
    console.error('Failed to load pipelines:', error)
  } finally {
    loading.value = false
  }
}

async function triggerPipeline() {
  if (!props.project?.owner_name || !props.project?.name || !triggerRef.value) return
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    await api.pipelines.trigger(path, triggerRef.value)
    showTriggerModal.value = false
    loadPipelines()
  } catch (error) {
    console.error('Failed to trigger pipeline:', error)
    alert('触发流水线失败')
  }
}

watch([() => props.project?.owner_name, () => props.project?.name], () => {
  loadPipelines()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.pipeline-list {
  padding: $spacing-lg;
}

.list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: $spacing-lg;
  
  h3 {
    margin: 0;
  }
}

.pipelines {
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
}

.pipeline-card {
  border: 1px solid $border-color;
  border-radius: $border-radius;
  overflow: hidden;
  background: $bg-primary;
}

.pipeline-item {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  padding: $spacing-md;
  text-decoration: none;
  color: inherit;
  transition: background $transition-fast;
  
  &:hover {
    background: $bg-secondary;
  }
}

.pipeline-status {
  width: 40px;
  text-align: center;
}

.status-icon {
  font-size: 18px;
  font-weight: bold;
  
  &.pending { color: #888; }
  &.running { 
    color: $primary-color;
    animation: pulse 1.5s ease-in-out infinite;
  }
  &.success { color: #22c55e; }
  &.failed { color: #ef4444; }
  &.canceled { color: #6b7280; }
  &.skipped { color: #9ca3af; }
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.pipeline-info {
  flex: 1;
  min-width: 0;
}

.pipeline-title {
  font-weight: 500;
  margin-bottom: $spacing-xs;
  
  .ref {
    font-weight: normal;
    color: $primary-color;
    margin-left: $spacing-sm;
    font-size: $font-size-sm;
  }
}

.pipeline-meta {
  font-size: $font-size-sm;
  color: $text-muted;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  
  .separator {
    margin: 0 $spacing-xs;
  }
  
  code {
    font-size: $font-size-xs;
  }
}

.pipeline-duration {
  color: $text-muted;
  font-size: $font-size-sm;
  white-space: nowrap;
}

.jobs-overview {
  display: flex;
  flex-wrap: wrap;
  gap: $spacing-xs;
  padding: $spacing-sm $spacing-md;
  background: $bg-secondary;
  border-top: 1px solid $border-color;
}

.job-badge {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: $font-size-xs;
  font-weight: 500;
  cursor: default;
  
  &.job-blocked {
    background: #fee2e2;
    color: #991b1b;
    border: 1px solid #fca5a5;
  }
  
  &.job-waiting {
    background: #e5e7eb;
    color: #374151;
    border: 1px solid #d1d5db;
  }
  
  &.job-running {
    background: #dbeafe;
    color: #1e40af;
    border: 1px solid #93c5fd;
  }
  
  &.job-success {
    background: #dcfce7;
    color: #15803d;
    border: 1px solid #86efac;
  }
  
  &.job-warning {
    background: #fef3c7;
    color: #92400e;
    border: 1px solid #fcd34d;
  }
  
  &.job-failed {
    background: #fee2e2;
    color: #991b1b;
    border: 1px solid #fca5a5;
  }
  
  &.job-canceled {
    background: #f3f4f6;
    color: #6b7280;
    border: 1px solid #d1d5db;
  }
}

.job-icon {
  font-size: 12px;
  line-height: 1;
}

.job-name {
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
  max-width: 500px;
  width: 100%;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
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
  font-size: 28px;
  line-height: 1;
  color: $text-muted;
  cursor: pointer;
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  transition: all $transition-fast;
  
  &:hover {
    background: $bg-secondary;
    color: $text-primary;
  }
}

.modal-body {
  padding: 24px;
  overflow-y: auto;
  flex: 1;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: $spacing-sm;
  padding: 16px 24px;
  border-top: 1px solid $border-color;
}

.form-group {
  margin-bottom: 20px;
  
  &:last-child {
    margin-bottom: 0;
  }
  
  label {
    display: block;
    margin-bottom: 8px;
    font-weight: 500;
    color: $text-primary;
  }
}

.form-input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid $border-color;
  border-radius: 6px;
  background: $bg-primary;
  color: $text-primary;
  font-size: $font-size-base;
  transition: all $transition-fast;
  
  &:focus {
    outline: none;
    border-color: $primary-color;
    box-shadow: 0 0 0 3px rgba($primary-color, 0.1);
  }
  
  &::placeholder {
    color: $text-muted;
  }
}
</style>
