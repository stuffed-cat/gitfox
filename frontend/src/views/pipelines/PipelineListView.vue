<template>
  <div class="pipeline-list">
    <div class="list-header">
      <h3>流水线</h3>
      <button class="btn btn-primary" @click="triggerPipeline">
        ▶ 运行流水线
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
      <router-link
        v-for="pipeline in pipelines"
        :key="pipeline.id"
        :to="`/${project?.owner_name}/${project?.name}/-/pipelines/${pipeline.id}`"
        class="pipeline-item"
      >
        <div class="pipeline-status">
          <span class="status-icon" :class="pipeline.status">
            {{ statusIcon(pipeline.status) }}
          </span>
        </div>
        <div class="pipeline-info">
          <div class="pipeline-title">
            流水线 #{{ pipeline.id.substring(0, 8) }}
            <span class="ref">{{ pipeline.ref_name }}</span>
          </div>
          <div class="pipeline-meta">
            <code>{{ pipeline.commit_sha?.substring(0, 8) }}</code>
            <span class="separator">·</span>
            <span>{{ formatDate(pipeline.created_at) }}</span>
          </div>
        </div>
        <div class="pipeline-duration" v-if="pipeline.duration_seconds">
          ⏱ {{ formatDuration(pipeline.duration_seconds) }}
        </div>
      </router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import api from '@/api'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import type { Project, Pipeline } from '@/types'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

const props = defineProps<{
  project?: Project
}>()

const loading = ref(false)
const pipelines = ref<Pipeline[]>([])

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

async function loadPipelines() {
  if (!props.project?.owner_name || !props.project?.name) return
  loading.value = true
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    pipelines.value = await api.pipelines.list(path)
  } catch (error) {
    console.error('Failed to load pipelines:', error)
  } finally {
    loading.value = false
  }
}

async function triggerPipeline() {
  if (!props.project?.owner_name || !props.project?.name) return
  
  const refName = prompt('请输入分支或标签名称', 'master')
  if (!refName) return
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    await api.pipelines.trigger(path, refName)
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
}

.pipeline-item {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  padding: $spacing-md;
  border-bottom: 1px solid $border-color;
  text-decoration: none;
  color: inherit;
  transition: background $transition-fast;
  
  &:last-child {
    border-bottom: none;
  }
  
  &:hover {
    background: $bg-secondary;
  }
}

.pipeline-status {
  width: 40px;
  text-align: center;
}

.status-icon {
  font-size: 20px;
  
  &.running {
    animation: spin 1s linear infinite;
  }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
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
</style>
