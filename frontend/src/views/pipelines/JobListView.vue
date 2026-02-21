<template>
  <div class="job-list-page">
    <div class="page-header">
      <div class="page-header-inner">
        <div class="filter-tabs">
          <button
            class="tab-btn"
            :class="{ active: activeTab === 'all' }"
            @click="activeTab = 'all'"
          >
            全部
            <span v-if="totalAll > 0" class="tab-badge">{{ totalAll }}</span>
          </button>
          <button
            class="tab-btn"
            :class="{ active: activeTab === 'finished' }"
            @click="activeTab = 'finished'"
          >
            已完成
          </button>
        </div>
      </div>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="loading-spinner"></div>
    </div>

    <div v-else-if="jobs.length === 0" class="empty-state">
      <div class="empty-icon">
        <svg viewBox="0 0 64 64" width="64" height="64" fill="none">
          <rect x="8" y="8" width="48" height="48" rx="6" stroke="currentColor" stroke-width="2"/>
          <path d="M20 24h24M20 32h16M20 40h12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
      </div>
      <h3>使用作业自动执行任务</h3>
      <p>作业是CI/CD流水线的基本组成部分。若要在CI/CD流水线中设置作业，请在您的项目中添加一个 <code>.gitfox/ci/</code> 配置文件。</p>
      <router-link :to="`/${project?.owner_name}/${project?.name}/-/pipelines`" class="btn btn-primary">
        创建CI/CD配置文件
      </router-link>
    </div>

    <div v-else class="jobs-table">
      <div
        v-for="job in filteredJobs"
        :key="job.id"
        class="job-row"
        @click="goToPipeline(job)"
      >
        <div class="job-status">
          <span class="status-badge" :class="job.status">
            <CiStatusIcon :status="job.status" :size="14" />
            {{ getStatusLabel(job.status) }}
          </span>
        </div>
        <div class="job-info">
          <span class="job-name">{{ job.name }}</span>
          <span class="job-stage">{{ job.stage }}</span>
        </div>
        <div class="job-pipeline">
          流水线 #{{ job.pipeline_id }}
        </div>
        <div class="job-duration">
          <span v-if="job.duration_seconds">{{ formatDuration(job.duration_seconds) }}</span>
          <span v-else>—</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useRouter } from 'vue-router'
import api from '@/api'
import type { Project, PipelineJob } from '@/types'
import CiStatusIcon from '@/components/CiStatusIcon.vue'

const props = defineProps<{ project?: Project }>()
const router = useRouter()

const loading = ref(false)
const activeTab = ref<'all' | 'finished'>('all')

interface JobWithPipeline extends PipelineJob {
  pipelineNumId: number | string
}

const jobs = ref<JobWithPipeline[]>([])

const filteredJobs = computed(() => {
  if (activeTab.value === 'finished') {
    return jobs.value.filter(j => ['success', 'failed', 'canceled', 'skipped'].includes(j.status))
  }
  return jobs.value
})

const totalAll = computed(() => jobs.value.length)

function getStatusLabel(status: string): string {
  const map: Record<string, string> = {
    pending: '等待中', running: '运行中', success: '已通过',
    failed: '已失败', canceled: '已取消', skipped: '已跳过'
  }
  return map[status] || status
}

// Reserved for future use
// function _getStatusIcon(status: string): string {
//   const map: Record<string, string> = {
//     pending: icons.statusPending, running: icons.statusRunning,
//     success: icons.statusSuccess, failed: icons.statusFailed,
//     canceled: icons.statusCanceled, skipped: icons.statusSkipped
//   }
//   return map[status] || icons.statusPending
// }

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}秒`
  return `${Math.floor(seconds / 60)}分${seconds % 60}秒`
}

function goToPipeline(job: JobWithPipeline) {
  if (!props.project) return
  router.push(`/${props.project.owner_name}/${props.project.name}/-/pipelines/${job.pipelineNumId}`)
}

async function loadJobs() {
  if (!props.project?.owner_name || !props.project?.name) return
  loading.value = true
  const path = { namespace: props.project.owner_name, project: props.project.name }
  try {
    const pipelines = await api.pipelines.list(path)
    const allJobs: JobWithPipeline[] = []
    for (const pipeline of pipelines.slice(0, 20)) {
      try {
        const result = await api.pipelines.get(path, pipeline.id)
        for (const job of (result.jobs || [])) {
          allJobs.push({ ...job, pipelineNumId: pipeline.id })
        }
      } catch {}
    }
    jobs.value = allJobs
  } catch (err) {
    console.error('Failed to load jobs:', err)
  } finally {
    loading.value = false
  }
}

watch([() => props.project?.owner_name, () => props.project?.name], () => {
  loadJobs()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.job-list-page { padding: 0; }

.page-header {
  border-bottom: 1px solid $border-color;
  padding: 0 16px;
}

.filter-tabs {
  display: flex;
  gap: 0;
}

.tab-btn {
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 12px 4px;
  margin-right: 20px;
  font-size: $font-size-sm;
  color: $text-secondary;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  transition: all 0.15s;

  &.active {
    color: $text-primary;
    border-bottom-color: $primary-color;
    font-weight: 600;
  }

  .tab-badge {
    background: $bg-tertiary;
    border-radius: 10px;
    padding: 1px 7px;
    font-size: 11px;
    color: $text-secondary;
  }
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: 60px;

  .loading-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid $border-color;
    border-top-color: $primary-color;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
}

@keyframes spin { to { transform: rotate(360deg); } }

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 24px;
  text-align: center;
  color: $text-secondary;

  .empty-icon {
    color: $text-muted;
    margin-bottom: 20px;
  }

  h3 {
    font-size: 18px;
    font-weight: 600;
    color: $text-primary;
    margin: 0 0 12px;
  }

  p {
    font-size: 14px;
    line-height: 1.6;
    max-width: 500px;
    margin: 0 0 24px;

    code {
      background: $bg-tertiary;
      padding: 1px 5px;
      border-radius: 4px;
      font-family: monospace;
      font-size: 13px;
    }
  }
}

.jobs-table {
  border-top: 1px solid $border-color;
}

.job-row {
  display: grid;
  grid-template-columns: 120px 1fr 140px 80px;
  align-items: center;
  padding: 10px 16px;
  border-bottom: 1px solid $border-color;
  cursor: pointer;
  transition: background 0.1s;

  &:hover { background: $bg-secondary; }
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 8px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;

  &.pending { background: #f0f0f0; color: #666; }
  &.running { background: #e8f0fd; color: #1a73e8; }
  &.success { background: #e6f4ea; color: #188038; }
  &.failed { background: #fce8e6; color: #d93025; }
  &.canceled { background: #f5f5f5; color: #80868b; }
  &.skipped { background: #f5f5f5; color: #80868b; }
}

.job-info {
  display: flex;
  flex-direction: column;
  gap: 2px;

  .job-name {
    font-size: 14px;
    font-weight: 500;
    color: $text-primary;
  }

  .job-stage {
    font-size: 12px;
    color: $text-muted;
  }
}

.job-pipeline {
  font-size: 13px;
  color: $text-secondary;
  font-family: $font-mono;
}

.job-duration {
  font-size: 13px;
  color: $text-secondary;
  text-align: right;
}
</style>
