<template>
  <div class="pipeline-detail">
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <template v-else-if="pipeline">
      <div class="pipeline-header">
        <div class="header-main">
          <h2>
            <span class="status-icon" :class="pipeline.status">{{ statusIcon(pipeline.status) }}</span>
            流水线 #{{ pipeline.id.substring(0, 8) }}
          </h2>
          <div class="pipeline-meta">
            <span class="ref">{{ pipeline.ref_name }}</span>
            <span class="separator">·</span>
            <code>{{ pipeline.commit_sha?.substring(0, 8) }}</code>
            <span class="separator">·</span>
            <span>{{ formatDate(pipeline.created_at) }}</span>
            <span v-if="pipeline.duration_seconds" class="separator">·</span>
            <span v-if="pipeline.duration_seconds">耗时 {{ formatDuration(pipeline.duration_seconds) }}</span>
          </div>
        </div>
        <div class="header-actions">
          <button 
            v-if="pipeline.status === 'running'" 
            class="btn btn-danger"
            @click="cancelPipeline"
          >
            取消
          </button>
          <button 
            v-if="['failed', 'canceled'].includes(pipeline.status)"
            class="btn btn-primary"
            @click="retryPipeline"
          >
            重试
          </button>
        </div>
      </div>
      
      <div class="jobs-section">
        <h3>任务</h3>
        
        <div class="jobs-list">
          <div
            v-for="job in jobs"
            :key="job.id"
            class="job-item"
            :class="{ expanded: expandedJob === job.id }"
          >
            <div class="job-header" @click="toggleJob(job.id)">
              <span class="status-icon" :class="job.status">{{ statusIcon(job.status) }}</span>
              <span class="job-name">{{ job.name }}</span>
              <span class="job-stage">{{ job.stage }}</span>
              <span class="job-duration" v-if="job.duration">{{ formatDuration(job.duration) }}</span>
              <span class="expand-icon">{{ expandedJob === job.id ? '▼' : '▶' }}</span>
            </div>
            
            <div v-if="expandedJob === job.id" class="job-logs">
              <pre v-if="jobLogs[job.id]">{{ jobLogs[job.id] }}</pre>
              <div v-else-if="loadingLogs" class="loading-logs">加载日志中...</div>
              <div v-else class="no-logs">暂无日志</div>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import api from '@/api'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import type { Project, Pipeline } from '@/types'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

interface PipelineJob {
  id: string
  name: string
  stage: string
  status: string
  duration?: number
  started_at?: string
  finished_at?: string
}

const props = defineProps<{
  project?: Project
}>()

const route = useRoute()

const loading = ref(false)
const loadingLogs = ref(false)
const pipeline = ref<Pipeline | null>(null)
const jobs = ref<PipelineJob[]>([])
const expandedJob = ref<string | null>(null)
const jobLogs = ref<Record<string, string>>({})

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

function statusIcon(status: string) {
  const map: Record<string, string> = {
    pending: '⏳',
    running: '🔄',
    success: '✅',
    failed: '❌',
    canceled: '⏹️',
    skipped: '⏭️'
  }
  return map[status] || '❓'
}

async function toggleJob(jobId: string) {
  if (expandedJob.value === jobId) {
    expandedJob.value = null
    return
  }
  
  expandedJob.value = jobId
  
  if (!jobLogs.value[jobId]) {
    await loadJobLogs(jobId)
  }
}

async function loadJobLogs(jobId: string) {
  if (!props.project?.owner_name || !props.project?.name || !pipeline.value) return
  loadingLogs.value = true
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    const result = await api.pipelines.getJobLog(path, pipeline.value.id, jobId)
    jobLogs.value[jobId] = result.log
  } catch (error) {
    console.error('Failed to load job logs:', error)
    jobLogs.value[jobId] = '加载日志失败'
  } finally {
    loadingLogs.value = false
  }
}

async function loadPipeline() {
  const pipelineId = route.params.pipelineId as string
  if (!props.project?.owner_name || !props.project?.name || !pipelineId) return
  
  loading.value = true
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    const result = await api.pipelines.get(path, pipelineId)
    pipeline.value = result.pipeline
    jobs.value = result.jobs
  } catch (error) {
    console.error('Failed to load pipeline:', error)
  } finally {
    loading.value = false
  }
}

async function cancelPipeline() {
  if (!props.project?.owner_name || !props.project?.name || !pipeline.value) return
  if (!confirm('确定要取消此流水线吗？')) return
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    await api.pipelines.cancel(path, pipeline.value.id)
    loadPipeline()
  } catch (error) {
    console.error('Failed to cancel pipeline:', error)
  }
}

async function retryPipeline() {
  if (!props.project?.owner_name || !props.project?.name || !pipeline.value) return
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    await api.pipelines.retry(path, pipeline.value.id)
    loadPipeline()
  } catch (error) {
    console.error('Failed to retry pipeline:', error)
  }
}

watch([() => props.project?.owner_name, () => props.project?.name, () => route.params.pipelineId], () => {
  loadPipeline()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.pipeline-detail {
  padding: $spacing-lg;
}

.pipeline-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: $spacing-lg;
}

.header-main {
  h2 {
    display: flex;
    align-items: center;
    gap: $spacing-sm;
    margin-bottom: $spacing-sm;
  }
}

.status-icon {
  &.running {
    animation: spin 1s linear infinite;
  }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.pipeline-meta {
  font-size: $font-size-sm;
  color: $text-muted;
  
  .separator {
    margin: 0 $spacing-xs;
  }
  
  .ref {
    color: $primary-color;
    font-weight: 500;
  }
  
  code {
    font-size: $font-size-xs;
  }
}

.header-actions {
  display: flex;
  gap: $spacing-sm;
}

.commit-info {
  padding: $spacing-md;
  background: $bg-secondary;
  border-radius: $border-radius;
  margin-bottom: $spacing-lg;
  font-size: $font-size-sm;
}

.jobs-section {
  h3 {
    margin-bottom: $spacing-md;
  }
}

.jobs-list {
  border: 1px solid $border-color;
  border-radius: $border-radius;
  overflow: hidden;
}

.job-item {
  border-bottom: 1px solid $border-color;
  
  &:last-child {
    border-bottom: none;
  }
}

.job-header {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  padding: $spacing-md;
  cursor: pointer;
  transition: background $transition-fast;
  
  &:hover {
    background: $bg-secondary;
  }
}

.job-name {
  flex: 1;
  font-weight: 500;
}

.job-stage {
  color: $text-muted;
  font-size: $font-size-sm;
  background: $bg-secondary;
  padding: 2px 8px;
  border-radius: $border-radius;
}

.job-duration {
  color: $text-muted;
  font-size: $font-size-sm;
}

.expand-icon {
  color: $text-muted;
  font-size: $font-size-xs;
}

.job-logs {
  background: $bg-dark;
  color: $text-light;
  max-height: 400px;
  overflow: auto;
  
  pre {
    margin: 0;
    padding: $spacing-md;
    font-family: 'JetBrains Mono', monospace;
    font-size: $font-size-xs;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-all;
  }
}

.loading-logs, .no-logs {
  padding: $spacing-lg;
  text-align: center;
  color: $text-muted;
}
</style>
