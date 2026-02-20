<template>
  <div class="gitlab-pipeline-detail">
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
      
      <!-- GitLab 风格的顶部信息栏 -->
      <div class="pipeline-info-bar">
        <div class="pipeline-info-left">
          <CiStatusIcon :status="pipeline.status" :size="24" />
          <div class="pipeline-title-group">
            <h1 class="pipeline-commit-title">{{ pipeline.ref_name }}</h1>
            <div class="pipeline-meta-info">
              <span class="pipeline-number">#{{ pipeline.id }}</span>
              <span class="meta-separator">·</span>
              <code class="commit-sha">{{ pipeline.commit_sha?.substring(0, 8) }}</code>
              <span class="meta-separator">·</span>
              <span class="time-info">{{ formatDate(pipeline.created_at) }}</span>
              <span v-if="pipeline.duration_seconds" class="meta-separator">·</span>
              <span v-if="pipeline.duration_seconds" class="duration-info">
                <svg viewBox="0 0 16 16" width="14" height="14">
                  <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
                  <path d="M8 5v3l2 2" stroke="currentColor" stroke-width="1.5"/>
                </svg>
                {{ formatDuration(pipeline.duration_seconds) }}
              </span>
            </div>
          </div>
        </div>
        
        <div class="pipeline-actions-bar">
          <button 
            v-if="pipeline.status === 'running'" 
            class="action-btn cancel-btn"
            @click="cancelPipeline"
          >
            取消
          </button>
          <button 
            v-if="['failed', 'canceled'].includes(pipeline.status)"
            class="action-btn retry-btn"
            @click="retryPipeline"
          >
            <svg viewBox="0 0 16 16" width="16" height="16">
              <path d="M4 8a4 4 0 017.5-2M12 8a4 4 0 01-7.5 2" fill="none" stroke="currentColor" stroke-width="1.5"/>
              <path d="M11.5 4v2h-2M4.5 12v-2h2" fill="currentColor"/>
            </svg>
            重试
          </button>
          <button 
            class="action-btn default-btn"
            @click="deletePipeline"
          >
            删除
          </button>
        </div>
      </div>
      
      <!-- 错误信息 -->
      <div v-if="pipeline.error_message" class="error-alert-box">
        <svg viewBox="0 0 16 16" width="16" height="16">
          <circle cx="8" cy="8" r="7" fill="#dd2b0e"/>
          <path d="M8 4v4M8 10v.5" stroke="white" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <span>{{ pipeline.error_message }}</span>
      </div>
      
      <!-- GitLab 风格的横向阶段视图 -->
      <div class="pipeline-graph-container">
        <div class="stage-column" v-for="stage in stages" :key="stage">
          <div class="stage-header">
            <span class="stage-name">{{ stage }}</span>
          </div>
          
          <div class="stage-jobs">
            <div
              v-for="job in getJobsByStage(stage)"
              :key="job.id"
              class="job-card"
              :class="[`status-${job.status}`, { selected: expandedJob === job.id }]"
              @click="toggleJob(job.id)"
            >
              <div class="job-card-content">
                <CiStatusIcon :status="job.status" :size="16" />
                <span class="job-name-text">{{ job.name }}</span>
              </div>
              <div v-if="job.duration" class="job-duration-badge">
                {{ formatDuration(job.duration) }}
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <!-- 日志查看器（全屏模态框） -->
      <Teleport to="body">
        <Transition name="modal">
          <div v-if="expandedJob" class="job-log-modal"  @click.self="closeJobLog">
            <div class="job-log-dialog">
              <div class="job-log-header">
                <div class="job-log-title">
                  <CiStatusIcon :status="selectedJob?.status || 'pending'" :size="20" />
                  <span class="job-title-text">{{ selectedJob?.name || '' }}</span>
                  <span class="job-stage-label">{{ selectedJob?.stage }}</span>
                </div>
                <button class="close-btn" @click="closeJobLog">
                  <svg viewBox="0 0 16 16" width="20" height="20">
                    <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                  </svg>
                </button>
              </div>
              
              <div class="job-log-body">
                <JobLogViewer
                  v-if="props.project?.owner_name && props.project?.name && pipeline && selectedJob"
                  :namespace="props.project.owner_name"
                  :project="props.project.name"
                  :pipeline-id="pipeline.id"
                  :job-id="selectedJob.id"
                  :job-name="selectedJob.name"
                />
              </div>
            </div>
          </div>
        </Transition>
      </Teleport>
    </template>
    
    <!-- Toast 通知 -->
    <ToastNotification ref="toast" />
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import api from '@/api'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import type { Project, Pipeline } from '@/types'
import CiStatusIcon from '@/components/CiStatusIcon.vue'
import JobLogViewer from '@/components/JobLogViewer.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import ToastNotification from '@/components/ToastNotification.vue'

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

// 计算所有唯一的阶段
const stages = computed(() => {
  const stageSet = new Set(jobs.value.map(job => job.stage))
  return Array.from(stageSet)
})

// 根据阶段获取作业
const getJobsByStage = (stage: string) => {
  return jobs.value.filter(job => job.stage === stage)
}

// 获取选中的作业
const selectedJob = computed(() => {
  if (!expandedJob.value) return null
  return jobs.value.find(job => job.id === expandedJob.value) || null
})

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
  expandedJob.value = jobId
}

function closeJobLog() {
  expandedJob.value = null
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
.gitlab-pipeline-detail {
  background: #fafafa;
  min-height: 100vh;
  padding: 0;
}

.loading {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 400px;
}

// GitLab 顶部信息栏
.pipeline-info-bar {
  background: white;
  border-bottom: 1px solid #e5e5e5;
  padding: 16px 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.pipeline-info-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.pipeline-title-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.pipeline-commit-title {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  color: #303030;
}

.pipeline-meta-info {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: #666;
}

.pipeline-number {
  color: #1f75cb;
  font-weight: 500;
}

.meta-separator {
  color: #ccc;
}

.commit-sha {
  font-family: 'Monaco', 'Menlo', monospace;
  font-size: 13px;
  background: #f5f5f5;
  padding: 2px 6px;
  border-radius: 3px;
  color: #333;
}

.time-info,
.duration-info {
  display: flex;
  align-items: center;
  gap: 4px;
}

.pipeline-actions-bar {
  display: flex;
  gap: 8px;
}

.action-btn {
  padding: 8px 16px;
  border: 1px solid #dbdbdb;
  border-radius: 4px;
  background: white;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  gap: 6px;
  color: #303030;
  
  &:hover {
    background: #f5f5f5;
    border-color: #999;
  }
  
  &.retry-btn {
    color: #1f75cb;
    border-color: #1f75cb;
    
    &:hover {
      background: #e9f2ff;
    }
  }
  
  &.cancel-btn {
    color: #c83030;
    border-color: #c83030;
    
    &:hover {
      background: #feeceb;
    }
  }
}

.error-alert-box {
  margin: 16px 24px;
  padding: 12px 16px;
  background: #feeceb;
  border: 1px solid #f0b5af;
  border-left: 4px solid #dd2b0e;
  border-radius: 4px;
  display: flex;
  align-items: center;
  gap: 12px;
  color: #c12920;
  font-size: 14px;
}

// GitLab 横向阶段图
.pipeline-graph-container {
  padding: 32px 24px;
  display: flex;
  gap: 48px;
  overflow-x: auto;
  min-height: 400px;
}

.stage-column {
  display: flex;
  flex-direction: column;
  min-width: 200px;
}

.stage-header {
  padding: 12px 0;
  margin-bottom: 16px;
  border-bottom: 2px solid #e5e5e5;
}

.stage-name {
  font-size: 14px;
  font-weight: 600;
  color: #303030;
  text-transform: capitalize;
}

.stage-jobs {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

// 作业卡片 - GitLab风格
.job-card {
  background: white;
  border: 1px solid #d4d4d4;
  border-radius: 6px;
  padding: 12px 16px;
  cursor: pointer;
  transition: all 0.2s;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
  
  &:hover {
    border-color: #1f75cb;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    transform: translateY(-1px);
  }
  
  &.selected {
    border-color: #1f75cb;
    background: #f7fbff;
    box-shadow: 0 0 0 2px rgba(31, 117, 203, 0.1);
  }
  
  // 状态左侧边框
  &.status-success {
    border-left: 4px solid #108548;
  }
  
  &.status-failed {
    border-left: 4px solid #dd2b0e;
  }
  
  &.status-running {
    border-left: 4px solid #1f75cb;
    
    .job-card-content {
      animation: pulse 2s ease-in-out infinite;
    }
  }
  
  &.status-pending {
    border-left: 4px solid #999;
  }
  
  &.status-canceled {
    border-left: 4px solid #999;
    opacity: 0.7;
  }
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.7; }
}

.job-card-content {
  display: flex;
  align-items: center;
  gap: 10px;
}

.job-name-text {
  font-size: 14px;
  font-weight: 500;
  color: #303030;
}

.job-duration-badge {
  margin-top: 8px;
  font-size: 12px;
  color: #666;
  padding: 2px 8px;
  background: #f5f5f5;
  border-radius: 10px;
  display: inline-block;
}

// 日志查看器模态框
.job-log-modal {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(4px);
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.job-log-dialog {
  background: #1e1e1e;
  border-radius: 8px;
  width: 95%;
  max-width: 1400px;
  height: 85vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
}

.job-log-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  background: #252526;
  border-bottom: 1px solid #3e3e42;
  border-radius: 8px 8px 0 0;
}

.job-log-title {
  display: flex;
  align-items: center;
  gap: 12px;
  color: #d4d4d4;
}

.job-title-text {
  font-size: 16px;
  font-weight: 600;
}

.job-stage-label {
  padding: 4px 10px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
}

.close-btn {
  background: transparent;
  border: none;
  color: #ccc;
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
  
  &:hover {
    background: rgba(255, 255, 255, 0.1);
    color: white;
  }
}

.job-log-body {
  flex: 1;
  overflow: hidden;
}

// 模态框过渡动画
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.3s ease;
  
  .job-log-dialog {
    transition: transform 0.3s ease, opacity 0.3s ease;
  }
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
  
  .job-log-dialog {
    transform: scale(0.9) translateY(-20px);
    opacity: 0;
  }
}
</style>
