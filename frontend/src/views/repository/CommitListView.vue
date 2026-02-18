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
            :to="`/${project?.owner_name}/${project?.name}/-/commit/${commit.sha}`"
            class="commit-message"
          >
            {{ commit.message.split('\n')[0] }}
          </router-link>
          <div class="commit-meta">
            <span class="author">{{ commit.author_name }}</span>
            <span class="separator">·</span>
            <span class="time">{{ formatCommitDate(commit.committed_date) }}</span>
          </div>
        </div>
        <div class="commit-actions">
          <code class="commit-sha">{{ commit.sha.substring(0, 8) }}</code>
          <button class="btn btn-outline btn-sm" @click="copyCommit(commit.sha)" title="复制 SHA">
            <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5">
              <rect x="5" y="2" width="9" height="11" rx="1"/>
              <path d="M5 4H3a1 1 0 00-1 1v9a1 1 0 001 1h8a1 1 0 001-1v-2"/>
            </svg>
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
import type { Project, BranchInfo, CommitInfo } from '@/types'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

const props = defineProps<{
  project?: Project
}>()

const loading = ref(false)
const loadingMore = ref(false)
const branches = ref<BranchInfo[]>([])
const currentBranch = ref('')
const commits = ref<CommitInfo[]>([])
const page = ref(1)
const hasMore = ref(false)

function formatCommitDate(timestamp?: number) {
  if (!timestamp) return '-'
  return dayjs.unix(timestamp).fromNow()
}

function copyCommit(sha: string) {
  navigator.clipboard.writeText(sha)
}

async function loadBranches() {
  if (!props.project?.owner_name || !props.project?.name) return
  const path = { namespace: props.project.owner_name, project: props.project.name }
  try {
    branches.value = await api.branches.list(path)
    if (branches.value.length > 0 && !currentBranch.value) {
      // 使用API返回的默认分支或第一个分支
      const defaultBranch = branches.value.find(b => b.is_default)
      currentBranch.value = defaultBranch?.name || branches.value[0].name
    }
  } catch (error) {
    console.error('Failed to load branches:', error)
  }
}

async function loadCommits() {
  if (!props.project?.owner_name || !props.project?.name || !currentBranch.value) return
  loading.value = true
  page.value = 1
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    commits.value = await api.commits.list(path, currentBranch.value, undefined, 1, 20)
    hasMore.value = commits.value.length === 20
  } catch (error) {
    console.error('Failed to load commits:', error)
    commits.value = []
  } finally {
    loading.value = false
  }
}

async function loadMore() {
  if (!props.project?.owner_name || !props.project?.name || !currentBranch.value) return
  loadingMore.value = true
  page.value++
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    const moreCommits = await api.commits.list(path, currentBranch.value, undefined, page.value, 20)
    commits.value.push(...moreCommits)
    hasMore.value = moreCommits.length === 20
  } catch (error) {
    console.error('Failed to load more commits:', error)
  } finally {
    loadingMore.value = false
  }
}

watch([() => props.project?.owner_name, () => props.project?.name], () => {
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
