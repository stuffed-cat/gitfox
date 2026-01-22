<template>
  <div class="merge-request-list">
    <div class="list-header">
      <h3>合并请求</h3>
      <div class="header-actions">
        <div class="filter-tabs">
          <button
            :class="{ active: filter === 'open' }"
            @click="filter = 'open'"
          >
            开放 ({{ openCount }})
          </button>
          <button
            :class="{ active: filter === 'merged' }"
            @click="filter = 'merged'"
          >
            已合并 ({{ mergedCount }})
          </button>
          <button
            :class="{ active: filter === 'closed' }"
            @click="filter = 'closed'"
          >
            已关闭 ({{ closedCount }})
          </button>
        </div>
        <router-link
          :to="`/projects/${project?.slug}/merge-requests/new`"
          class="btn btn-primary"
        >
          + 新建合并请求
        </router-link>
      </div>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <div v-else-if="filteredMRs.length === 0" class="empty-state">
      <h3>暂无{{ filterText }}的合并请求</h3>
      <p v-if="filter === 'open'">创建合并请求来合并代码更改</p>
    </div>
    
    <div v-else class="mr-list">
      <router-link
        v-for="mr in filteredMRs"
        :key="mr.id"
        :to="`/projects/${project?.slug}/merge-requests/${mr.iid}`"
        class="mr-item"
      >
        <div class="mr-main">
          <div class="mr-title">
            <span class="mr-status" :class="mr.status">{{ statusIcon(mr.status) }}</span>
            {{ mr.title }}
          </div>
          <div class="mr-meta">
            <span class="mr-id">#{{ mr.iid }}</span>
            <span class="separator">·</span>
            <span>{{ mr.source_branch }} → {{ mr.target_branch }}</span>
            <span class="separator">·</span>
            <span>{{ mr.author_name }}</span>
            <span class="separator">·</span>
            <span>{{ formatDate(mr.created_at) }}</span>
          </div>
        </div>
        <div class="mr-stats">
          <span v-if="mr.comments_count > 0" class="comments">
            💬 {{ mr.comments_count }}
          </span>
        </div>
      </router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import api from '@/api'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import type { Project, MergeRequest } from '@/types'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

const props = defineProps<{
  project?: Project
}>()

const loading = ref(false)
const mergeRequests = ref<MergeRequest[]>([])
const filter = ref<'open' | 'merged' | 'closed'>('open')

const filteredMRs = computed(() => {
  return mergeRequests.value.filter(mr => mr.status === filter.value)
})

const openCount = computed(() => mergeRequests.value.filter(mr => mr.status === 'open').length)
const mergedCount = computed(() => mergeRequests.value.filter(mr => mr.status === 'merged').length)
const closedCount = computed(() => mergeRequests.value.filter(mr => mr.status === 'closed').length)

const filterText = computed(() => {
  const map = { open: '开放', merged: '已合并', closed: '已关闭' }
  return map[filter.value]
})

function formatDate(date: string) {
  return dayjs(date).fromNow()
}

function statusIcon(status: string) {
  const map: Record<string, string> = {
    open: '🟢',
    merged: '🟣',
    closed: '🔴'
  }
  return map[status] || '⚪'
}

async function loadMergeRequests() {
  if (!props.project?.id) return
  loading.value = true
  
  try {
    const response = await api.getMergeRequests(props.project.id)
    mergeRequests.value = response.data
  } catch (error) {
    console.error('Failed to load merge requests:', error)
  } finally {
    loading.value = false
  }
}

watch(() => props.project?.id, () => {
  loadMergeRequests()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.merge-request-list {
  padding: $spacing-lg;
}

.list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: $spacing-lg;
  flex-wrap: wrap;
  gap: $spacing-md;
  
  h3 {
    margin: 0;
  }
}

.header-actions {
  display: flex;
  align-items: center;
  gap: $spacing-lg;
}

.filter-tabs {
  display: flex;
  background: $bg-secondary;
  border-radius: $border-radius;
  padding: 2px;
  
  button {
    padding: $spacing-xs $spacing-md;
    background: transparent;
    border: none;
    color: $text-secondary;
    cursor: pointer;
    border-radius: $border-radius;
    transition: all $transition-fast;
    
    &:hover {
      color: $text-primary;
    }
    
    &.active {
      background: $bg-primary;
      color: $text-primary;
      box-shadow: $shadow-sm;
    }
  }
}

.mr-list {
  display: flex;
  flex-direction: column;
}

.mr-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
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

.mr-main {
  flex: 1;
  min-width: 0;
}

.mr-title {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  font-weight: 500;
  margin-bottom: $spacing-xs;
  
  &:hover {
    color: $primary-color;
  }
}

.mr-status {
  font-size: 12px;
}

.mr-meta {
  font-size: $font-size-sm;
  color: $text-muted;
  
  .separator {
    margin: 0 $spacing-xs;
  }
  
  .mr-id {
    font-weight: 500;
    color: $text-secondary;
  }
}

.mr-stats {
  margin-left: $spacing-md;
  
  .comments {
    font-size: $font-size-sm;
    color: $text-muted;
  }
}
</style>
