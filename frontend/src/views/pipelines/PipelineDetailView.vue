<template>
  <div class="pipeline-detail">
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <template v-else-if="pipeline">
      <!-- 确认对话框 -->
      <ConfirmDialog
        v-model="showCancelDialog"
        title="取消流水线"
        message="确定要取消此流水线吗？"
        type="warning"
        confirm-text="取消流水线"
        @confirm="confirmCancel"
      />
      
      <ConfirmDialog
        v-model="showDeleteDialog"
        title="删除流水线"
        message="确定要删除此流水线吗？删除后无法恢复。"
        type="danger"
        confirm-text="删除"
        @confirm="confirmDelete"
      />
      <div class="pipeline-header">
        <div class="header-main">
          <h2>
            <CiStatusIcon :status="pipeline.status" :size="20" />
            流水线 #{{ String(pipeline.id) }}
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
          <div v-if="pipeline.error_message" class="error-banner">
            <svg viewBox="0 0 16 16" width="16" height="16">
              <circle cx="8" cy="8" r="7" fill="#dd2b0e"/>
              <path d="M8 4v4M8 10v.5" stroke="white" stroke-width="2" stroke-linecap="round"/>
            </svg>
            {{ pipeline.error_message }}
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
          <button 
            class="btn btn-danger"
            @click="deletePipeline"
            title="删除流水线"
          >
            <svg viewBox="0 0 16 16" width="14" height="14" style="margin-right: 4px;">
              <path d="M2 4h12M6 2h4M6 6v6M10 6v6M4 4v9a1 1 0 001 1h6a1 1 0 001-1V4" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            删除
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
              <CiStatusIcon :status="job.status" :size="16" />
              <span class="job-name">{{ job.name }}</span>
              <span class="job-stage">{{ job.stage }}</span>
              <span class="job-duration" v-if="job.duration">{{ formatDuration(job.duration) }}</span>
              <svg class="expand-icon" viewBox="0 0 16 16" width="12" height="12">
                <path :d="expandedJob === job.id ? icons.chevronDown : icons.chevronRight" fill="currentColor" />
              </svg>
            </div>
            
            <div v-if="expandedJob === job.id" class="job-logs-container">
              <JobLogViewer
                v-if="props.project?.owner_name && props.project?.name && pipeline"
                :namespace="props.project.owner_name"
                :project="props.project.name"
                :pipeline-id="pipeline.id"
                :job-id="job.id"
                :job-name="job.name"
              />
            </div>
          </div>
        </div>
      </div>
    </template>
    
    <!-- Toast 通知 -->
    <ToastNotification ref="toast" />
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import api from '@/api'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import type { Project, Pipeline } from '@/types'
import { navIcons } from '@/navigation/icons'
import CiStatusIcon from '@/components/CiStatusIcon.vue'
import JobLogViewer from '@/components/JobLogViewer.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import ToastNotification from '@/components/ToastNotification.vue'

const icons = navIcons

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
const router = useRouter()

const loading = ref(false)
const pipeline = ref<Pipeline | null>(null)
const jobs = ref<PipelineJob[]>([])
const expandedJob = ref<string | null>(null)
const toast = ref<InstanceType<typeof ToastNotification> | null>(null)
const showCancelDialog = ref(false)
const showDeleteDialog = ref(false)

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

async function toggleJob(jobId: string) {
  if (expandedJob.value === jobId) {
    expandedJob.value = null
    return
  }
  
  expandedJob.value = jobId
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
  showCancelDialog.value = true
}

async function confirmCancel() {
  if (!props.project?.owner_name || !props.project?.name || !pipeline.value) return
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    await api.pipelines.cancel(path, pipeline.value.id)
    toast.value?.success('流水线已取消')
    loadPipeline()
  } catch (error) {
    console.error('Failed to cancel pipeline:', error)
    toast.value?.error('取消流水线失败')
  }
}

async function retryPipeline() {
  if (!props.project?.owner_name || !props.project?.name || !pipeline.value) return
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    await api.pipelines.retry(path, pipeline.value.id)
    toast.value?.success('流水线已重试')
    loadPipeline()
  } catch (error) {
    console.error('Failed to retry pipeline:', error)
    toast.value?.error('重试流水线失败')
  }
}

async function deletePipeline() {
  if (!props.project?.owner_name || !props.project?.name || !pipeline.value) return
  showDeleteDialog.value = true
}

async function confirmDelete() {
  if (!props.project?.owner_name || !props.project?.name || !pipeline.value) return
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    await api.pipelines.delete(path, String(pipeline.value.id))
    toast.value?.success('流水线已删除')
    // 删除成功，返回到流水线列表
    await router.push(`/${props.project.owner_name}/${props.project.name}/-/pipelines`)
  } catch (error) {
    console.error('Failed to delete pipeline:', error)
    toast.value?.error('删除流水线失败')
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

.error-banner {
  margin-top: $spacing-sm;
  padding: $spacing-sm $spacing-md;
  background: #fef2f2;
  border: 1px solid #fecaca;
  border-radius: $border-radius;
  color: #dd2b0e;
  font-size: $font-size-sm;
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  
  svg {
    flex-shrink: 0;
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

.job-logs-container {
  height: 500px;
  border-top: 1px solid $border-color;
}
</style>
