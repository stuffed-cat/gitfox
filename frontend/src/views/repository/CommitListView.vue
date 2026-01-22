<template>
  <div class="commit-list">
    <div class="list-header">
      <h3>提交历史</h3>
      <div class="branch-selector">
        <select v-model="currentBranch" class="form-control form-control-sm" @change="loadCommits">
          <option v-for="branch in branches" :key="branch.name" :value="branch.name">
            {{ branch.name }}
          </option>
        </select>
      </div>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <div v-else-if="commits.length === 0" class="empty-state">
      <h3>暂无提交记录</h3>
    </div>
    
    <div v-else class="commits">
      <div
        v-for="commit in commits"
        :key="commit.sha"
        class="commit-item"
      >
        <div class="commit-main">
          <router-link
            :to="`/projects/${project?.slug}/commits/${commit.sha}`"
            class="commit-message"
          >
            {{ commit.message.split('\n')[0] }}
          </router-link>
          <div class="commit-meta">
            <span class="author">{{ commit.author_name }}</span>
            <span class="separator">·</span>
            <span class="time">{{ formatDate(commit.committed_at) }}</span>
          </div>
        </div>
        <div class="commit-actions">
          <code class="commit-sha">{{ commit.sha.substring(0, 8) }}</code>
          <button class="btn btn-outline btn-sm" @click="copyCommit(commit.sha)" title="复制 SHA">
            📋
          </button>
        </div>
      </div>
    </div>
    
    <div v-if="hasMore" class="load-more">
      <button class="btn btn-outline" @click="loadMore" :disabled="loadingMore">
        {{ loadingMore ? '加载中...' : '加载更多' }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import api from '@/api'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import type { Project, Branch, Commit } from '@/types'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

const props = defineProps<{
  project?: Project
}>()

const loading = ref(false)
const loadingMore = ref(false)
const branches = ref<Branch[]>([])
const currentBranch = ref('')
const commits = ref<Commit[]>([])
const page = ref(1)
const hasMore = ref(false)

function formatDate(date: string) {
  return dayjs(date).fromNow()
}

function copyCommit(sha: string) {
  navigator.clipboard.writeText(sha)
}

async function loadBranches() {
  if (!props.project?.id) return
  try {
    const response = await api.getBranches(props.project.id)
    branches.value = response.data
    if (branches.value.length > 0 && !currentBranch.value) {
      currentBranch.value = props.project.default_branch || branches.value[0].name
    }
  } catch (error) {
    console.error('Failed to load branches:', error)
  }
}

async function loadCommits() {
  if (!props.project?.id || !currentBranch.value) return
  loading.value = true
  page.value = 1
  
  try {
    const response = await api.getCommits(props.project.id, {
      ref: currentBranch.value,
      page: 1,
      per_page: 20
    })
    commits.value = response.data
    hasMore.value = response.data.length === 20
  } catch (error) {
    console.error('Failed to load commits:', error)
    commits.value = []
  } finally {
    loading.value = false
  }
}

async function loadMore() {
  if (!props.project?.id || !currentBranch.value) return
  loadingMore.value = true
  page.value++
  
  try {
    const response = await api.getCommits(props.project.id, {
      ref: currentBranch.value,
      page: page.value,
      per_page: 20
    })
    commits.value.push(...response.data)
    hasMore.value = response.data.length === 20
  } catch (error) {
    console.error('Failed to load more commits:', error)
  } finally {
    loadingMore.value = false
  }
}

watch(() => props.project?.id, () => {
  loadBranches().then(() => loadCommits())
}, { immediate: true })
</script>

<style lang="scss" scoped>
.commit-list {
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

.branch-selector {
  select {
    width: auto;
    min-width: 150px;
  }
}

.commits {
  display: flex;
  flex-direction: column;
}

.commit-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-md;
  border-bottom: 1px solid $border-color;
  
  &:last-child {
    border-bottom: none;
  }
  
  &:hover {
    background: $bg-secondary;
  }
}

.commit-main {
  flex: 1;
  min-width: 0;
}

.commit-message {
  display: block;
  color: $text-primary;
  text-decoration: none;
  font-weight: 500;
  margin-bottom: $spacing-xs;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  
  &:hover {
    color: $primary-color;
  }
}

.commit-meta {
  font-size: $font-size-sm;
  color: $text-muted;
  
  .separator {
    margin: 0 $spacing-xs;
  }
}

.commit-actions {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  margin-left: $spacing-md;
}

.commit-sha {
  font-family: 'JetBrains Mono', monospace;
  font-size: $font-size-sm;
  background: $bg-secondary;
  padding: $spacing-xs $spacing-sm;
  border-radius: $border-radius;
}

.load-more {
  display: flex;
  justify-content: center;
  padding: $spacing-lg;
}
</style>
