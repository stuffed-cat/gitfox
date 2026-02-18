<template>
  <div class="artifacts-page">
    <div class="artifacts-header">
      <div class="artifacts-stats">
        产物总大小 0 B <span class="separator">·</span> Total artifacts count {{ artifacts.length }}
      </div>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="loading-spinner"></div>
    </div>

    <div v-else-if="artifacts.length === 0" class="artifacts-table">
      <div class="table-header">
        <span class="col-check"></span>
        <span class="col-name">产物</span>
        <span class="col-job">作业</span>
        <span class="col-size">大小</span>
        <span class="col-created">创建于</span>
      </div>
      <div class="empty-rows">
        <!-- 空表格，GitLab 风格是显示表头但无行 -->
      </div>
    </div>

    <div v-else class="artifacts-table">
      <div class="table-header">
        <span class="col-check">
          <input type="checkbox" />
        </span>
        <span class="col-name">产物</span>
        <span class="col-job">作业</span>
        <span class="col-size">大小</span>
        <span class="col-created">创建于</span>
      </div>
      <div v-for="artifact in artifacts" :key="artifact.id" class="artifact-row">
        <span class="col-check"><input type="checkbox" /></span>
        <span class="col-name">
          <svg viewBox="0 0 16 16" width="14" height="14">
            <path :d="icons.file" fill="currentColor"/>
          </svg>
          {{ artifact.name }}
        </span>
        <span class="col-job">{{ artifact.job_name }}</span>
        <span class="col-size">{{ formatSize(artifact.size) }}</span>
        <span class="col-created">{{ formatDate(artifact.created_at) }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import type { Project } from '@/types'
import { navIcons } from '@/navigation/icons'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

const props = defineProps<{ project?: Project }>()
const icons = navIcons

interface Artifact {
  id: number
  name: string
  job_name: string
  size: number
  created_at: string
}

const loading = ref(false)
const artifacts = ref<Artifact[]>([])

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

function formatDate(date: string): string {
  return dayjs(date).fromNow()
}

// 目前暂无产物 API，显示空状态
watch([() => props.project?.owner_name, () => props.project?.name], () => {
  loading.value = false
  artifacts.value = []
}, { immediate: true })
</script>

<style lang="scss" scoped>
.artifacts-page { padding: 0; }

.artifacts-header {
  padding: 16px;
  border-bottom: 1px solid $border-color;
  font-size: 14px;
  color: $text-secondary;

  .separator { margin: 0 6px; }
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

.artifacts-table {
  border-top: 1px solid $border-color;
}

.table-header {
  display: grid;
  grid-template-columns: 36px 1fr 200px 100px 160px;
  padding: 8px 16px;
  background: $bg-secondary;
  border-bottom: 1px solid $border-color;
  font-size: 12px;
  font-weight: 600;
  color: $text-secondary;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.artifact-row {
  display: grid;
  grid-template-columns: 36px 1fr 200px 100px 160px;
  align-items: center;
  padding: 10px 16px;
  border-bottom: 1px solid $border-color;
  font-size: 13px;

  &:hover { background: $bg-secondary; }

  .col-name {
    display: flex;
    align-items: center;
    gap: 8px;
    color: $primary-color;
    cursor: pointer;
  }
}

.col-check { display: flex; align-items: center; }
.col-size, .col-created { color: $text-secondary; }
</style>
