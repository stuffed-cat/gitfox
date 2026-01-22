<template>
  <div class="project-overview">
    <!-- Project Header -->
    <div class="project-header">
      <div class="project-avatar">
        {{ project?.name.charAt(0).toUpperCase() }}
      </div>
      <div class="project-info">
        <h1>{{ project?.name }}</h1>
        <p class="project-path">{{ cloneUrl }}</p>
        <div class="project-badges">
          <span class="badge" :class="visibilityClass">
            <svg v-if="project?.visibility === 'private'" width="12" height="12" viewBox="0 0 16 16" fill="none">
              <rect x="4" y="7" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
              <path d="M6 7V5a2 2 0 014 0v2" stroke="currentColor" stroke-width="1.5"/>
            </svg>
            <svg v-else width="12" height="12" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
              <path d="M2 8h12M8 2a10 10 0 010 12M8 2a10 10 0 000 12" stroke="currentColor" stroke-width="1.5"/>
            </svg>
            {{ visibilityText }}
          </span>
        </div>
      </div>
      <div class="project-actions">
        <button class="btn btn-secondary">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M8 2l2.5 5 5.5.8-4 3.9.9 5.3L8 14.6l-4.9 2.4.9-5.3-4-3.9 5.5-.8L8 2z" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          收藏
        </button>
        <button class="btn btn-secondary">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="4" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="8" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="12" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
            <path d="M4 6v2a4 4 0 004 4M12 6v2a4 4 0 01-4 4" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          Fork
        </button>
      </div>
    </div>

    <!-- Clone Box -->
    <div class="clone-section">
      <div class="clone-tabs">
        <button 
          :class="['tab-btn', { active: cloneType === 'http' }]"
          @click="cloneType = 'http'"
        >HTTP</button>
        <button 
          :class="['tab-btn', { active: cloneType === 'ssh' }]"
          @click="cloneType = 'ssh'"
        >SSH</button>
      </div>
      <div class="clone-input">
        <input type="text" readonly :value="currentCloneUrl" />
        <button class="copy-btn" @click="copyUrl" :title="copied ? '已复制!' : '复制'">
          <svg v-if="!copied" width="16" height="16" viewBox="0 0 16 16" fill="none">
            <rect x="6" y="6" width="7" height="8" rx="1" stroke="currentColor" stroke-width="1.5"/>
            <path d="M10 6V4a1 1 0 00-1-1H4a1 1 0 00-1 1v6a1 1 0 001 1h2" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          <svg v-else width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M4 8l3 3 5-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
      </div>
      <p class="clone-hint">使用 Web IDE 进行修改 或 推送一个现有的 Git 仓库</p>
    </div>

    <!-- Stats & Info Grid -->
    <div class="content-grid">
      <!-- Main Content -->
      <div class="main-content">
        <!-- File Browser Placeholder -->
        <div class="file-browser card">
          <div class="file-header">
            <div class="branch-selector">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="4" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
                <circle cx="8" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/>
                <path d="M4 6v2a4 4 0 004 4" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>{{ project?.default_branch }}</span>
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path d="M3 4.5l3 3 3-3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
            </div>
            <div class="file-actions">
              <router-link :to="`${projectPath}/-/commits`" class="file-link">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
                  <path d="M8 5v3l2 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                </svg>
                {{ stats?.commits_count || 0 }} 次提交
              </router-link>
              <router-link :to="`${projectPath}/-/branches`" class="file-link">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <circle cx="4" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
                  <circle cx="12" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/>
                  <path d="M4 6v6a2 2 0 002 2h4" stroke="currentColor" stroke-width="1.5"/>
                </svg>
                {{ stats?.branches_count || 0 }} 个分支
              </router-link>
              <router-link :to="`${projectPath}/-/tags`" class="file-link">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <path d="M2 8V3a1 1 0 011-1h5l6 6-5 5-6-6z" stroke="currentColor" stroke-width="1.5"/>
                  <circle cx="5" cy="5" r="1" fill="currentColor"/>
                </svg>
                {{ stats?.tags_count || 0 }} 个标签
              </router-link>
            </div>
          </div>
          
          <div class="last-commit">
            <div class="commit-info">
              <span class="commit-author">最近一次提交</span>
              <span class="commit-message">{{ lastCommitMessage }}</span>
            </div>
            <div class="commit-meta">
              <span class="commit-sha">{{ lastCommitSha }}</span>
              <span class="commit-time">{{ lastCommitTime }}</span>
            </div>
          </div>
          
          <div class="file-list">
            <div class="file-item" v-for="file in files" :key="file.name">
              <svg v-if="file.type === 'tree'" width="16" height="16" viewBox="0 0 16 16" fill="none" class="icon-folder">
                <path d="M2 4a1 1 0 011-1h3l2 2h5a1 1 0 011 1v6a1 1 0 01-1 1H3a1 1 0 01-1-1V4z" fill="#fbbf24"/>
              </svg>
              <svg v-else width="16" height="16" viewBox="0 0 16 16" fill="none" class="icon-file">
                <path d="M4 2a1 1 0 00-1 1v10a1 1 0 001 1h8a1 1 0 001-1V6l-4-4H4z" stroke="currentColor" stroke-width="1.5"/>
                <path d="M9 2v4h4" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span class="file-name">{{ file.name }}</span>
              <span class="file-message">{{ file.lastCommit || 'Initial commit' }}</span>
              <span class="file-time">{{ file.updatedAt || '刚刚' }}</span>
            </div>
          </div>
        </div>
        
        <!-- README -->
        <div class="readme card" v-if="readme">
          <div class="readme-header">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M4 2a1 1 0 00-1 1v10a1 1 0 001 1h8a1 1 0 001-1V6l-4-4H4z" stroke="currentColor" stroke-width="1.5"/>
              <path d="M9 2v4h4" stroke="currentColor" stroke-width="1.5"/>
            </svg>
            <span>README.md</span>
          </div>
          <div class="readme-content" v-html="readme"></div>
        </div>
      </div>
      
      <!-- Sidebar -->
      <div class="sidebar">
        <!-- Project Info Card -->
        <div class="info-card card">
          <h3>项目信息</h3>
          <div class="info-list">
            <div class="info-item">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
                <path d="M8 5v3l2 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              <span>{{ stats?.commits_count || 0 }} 次提交</span>
            </div>
            <div class="info-item">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="4" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
                <circle cx="12" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/>
                <path d="M4 6v6a2 2 0 002 2h4" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>{{ stats?.branches_count || 0 }} 个分支</span>
            </div>
            <div class="info-item">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M2 8V3a1 1 0 011-1h5l6 6-5 5-6-6z" stroke="currentColor" stroke-width="1.5"/>
                <circle cx="5" cy="5" r="1" fill="currentColor"/>
              </svg>
              <span>{{ stats?.tags_count || 0 }} 个标签</span>
            </div>
            <div class="info-item">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="5" r="3" stroke="currentColor" stroke-width="1.5"/>
                <path d="M3 14c0-2.8 2.2-5 5-5s5 2.2 5 5" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>{{ stats?.members_count || 0 }} 名成员</span>
            </div>
          </div>
        </div>
        
        <!-- Quick Actions -->
        <div class="actions-card card">
          <h3>快速操作</h3>
          <div class="action-list">
            <router-link :to="`${projectPath}/-/new/main`" class="action-item">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              </svg>
              添加自述文件
            </router-link>
            <router-link :to="`${projectPath}/-/new/main?file_name=LICENSE`" class="action-item">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              </svg>
              添加LICENSE
            </router-link>
            <router-link :to="`${projectPath}/-/pipelines`" class="action-item">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              </svg>
              配置 CI/CD
            </router-link>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { Project, ProjectStats } from '@/types'

const props = defineProps<{
  project?: Project
  stats?: ProjectStats
}>()

const cloneType = ref<'http' | 'ssh'>('http')
const copied = ref(false)

// GitLab 风格的项目路径
const projectPath = computed(() => {
  if (!props.project?.owner_name) return ''
  return `/${props.project.owner_name}/${props.project.slug}`
})

const cloneUrl = computed(() => {
  if (!props.project || !props.project.owner_name) return ''
  // 使用 namespace/project 格式，和 GitLab/GitHub 一致
  return `${window.location.origin}/${props.project.owner_name}/${props.project.slug}.git`
})

const currentCloneUrl = computed(() => {
  if (cloneType.value === 'ssh') {
    return `git@${window.location.hostname}:${props.project?.owner_name}/${props.project?.slug}.git`
  }
  return cloneUrl.value
})

const visibilityClass = computed(() => {
  const map: Record<string, string> = {
    public: 'badge-success',
    private: 'badge-secondary',
    internal: 'badge-info'
  }
  return map[props.project?.visibility || 'private'] || 'badge-secondary'
})

const visibilityText = computed(() => {
  const map: Record<string, string> = {
    public: '公开',
    private: '私有',
    internal: '内部'
  }
  return map[props.project?.visibility || 'private'] || '私有'
})

// Mock data for files
const files = ref([
  { name: 'src', type: 'tree', lastCommit: 'Add source files', updatedAt: '2小时前' },
  { name: '.gitignore', type: 'blob', lastCommit: 'Initial commit', updatedAt: '1天前' },
  { name: 'README.md', type: 'blob', lastCommit: 'Update README', updatedAt: '3小时前' },
  { name: 'package.json', type: 'blob', lastCommit: 'Add dependencies', updatedAt: '5小时前' },
])

const readme = ref('<h1>项目名称</h1><p>这是一个示例项目的 README 文件。</p>')
const lastCommitMessage = ref('Update README.md')
const lastCommitSha = ref('a1b2c3d')
const lastCommitTime = ref('3小时前')

async function copyUrl() {
  await navigator.clipboard.writeText(currentCloneUrl.value)
  copied.value = true
  setTimeout(() => {
    copied.value = false
  }, 2000)
}
</script>

<style lang="scss" scoped>
.project-overview {
  padding: $spacing-6;
  max-width: 1200px;
  margin: 0 auto;
}

.project-header {
  display: flex;
  align-items: flex-start;
  gap: $spacing-5;
  margin-bottom: $spacing-6;
  padding-bottom: $spacing-6;
  border-bottom: 1px solid $border-color;
}

.project-avatar {
  width: 64px;
  height: 64px;
  border-radius: $border-radius-lg;
  background: $brand-gradient;
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  font-weight: $font-weight-semibold;
  flex-shrink: 0;
}

.project-info {
  flex: 1;
  min-width: 0;
  
  h1 {
    font-size: $font-size-2xl;
    font-weight: $font-weight-semibold;
    margin: 0 0 $spacing-1;
  }
  
  .project-path {
    color: $text-secondary;
    font-size: $font-size-sm;
    margin: 0 0 $spacing-3;
    word-break: break-all;
  }
}

.project-badges {
  display: flex;
  gap: $spacing-2;
}

.badge {
  display: inline-flex;
  align-items: center;
  gap: $spacing-1;
  padding: $spacing-1 $spacing-2;
  font-size: $font-size-xs;
  font-weight: $font-weight-medium;
  border-radius: $border-radius-full;
}

.badge-success {
  background: $color-success-light;
  color: darken($color-success, 10%);
}

.badge-secondary {
  background: $gray-100;
  color: $gray-600;
}

.badge-info {
  background: $color-info-light;
  color: darken($color-info, 10%);
}

.project-actions {
  display: flex;
  gap: $spacing-3;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-2 $spacing-4;
  font-size: $font-size-sm;
  font-weight: $font-weight-medium;
  border-radius: $border-radius;
  border: 1px solid $border-color;
  background: $bg-primary;
  cursor: pointer;
  transition: all $transition-fast;
  
  &:hover {
    background: $bg-secondary;
    border-color: $gray-300;
  }
}

.btn-secondary {
  background: $bg-primary;
  color: $text-primary;
}

// Clone Section
.clone-section {
  background: $bg-secondary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
  padding: $spacing-4;
  margin-bottom: $spacing-6;
}

.clone-tabs {
  display: flex;
  gap: $spacing-1;
  margin-bottom: $spacing-3;
}

.tab-btn {
  padding: $spacing-2 $spacing-4;
  font-size: $font-size-sm;
  font-weight: $font-weight-medium;
  background: transparent;
  border: none;
  border-radius: $border-radius;
  cursor: pointer;
  color: $text-secondary;
  transition: all $transition-fast;
  
  &:hover {
    background: $bg-primary;
  }
  
  &.active {
    background: $bg-primary;
    color: $text-primary;
    box-shadow: $shadow-sm;
  }
}

.clone-input {
  display: flex;
  gap: $spacing-2;
  
  input {
    flex: 1;
    padding: $spacing-3;
    font-size: $font-size-sm;
    font-family: monospace;
    background: $bg-primary;
    border: 1px solid $border-color;
    border-radius: $border-radius;
  }
}

.copy-btn {
  padding: $spacing-3;
  background: $brand-primary;
  color: white;
  border: none;
  border-radius: $border-radius;
  cursor: pointer;
  transition: all $transition-fast;
  
  &:hover {
    background: darken($brand-primary, 10%);
  }
}

.clone-hint {
  margin-top: $spacing-3;
  font-size: $font-size-sm;
  color: $text-secondary;
}

// Content Grid
.content-grid {
  display: grid;
  grid-template-columns: 1fr 280px;
  gap: $spacing-6;
  
  @media (max-width: $breakpoint-lg) {
    grid-template-columns: 1fr;
  }
}

.card {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
  overflow: hidden;
}

// File Browser
.file-browser {
  margin-bottom: $spacing-6;
}

.file-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-4;
  border-bottom: 1px solid $border-color;
  flex-wrap: wrap;
  gap: $spacing-3;
}

.branch-selector {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-2 $spacing-3;
  background: $bg-secondary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  font-size: $font-size-sm;
  font-weight: $font-weight-medium;
  cursor: pointer;
  
  &:hover {
    border-color: $brand-primary;
  }
}

.file-actions {
  display: flex;
  gap: $spacing-4;
}

.file-link {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  font-size: $font-size-sm;
  color: $text-secondary;
  text-decoration: none;
  
  &:hover {
    color: $brand-primary;
  }
}

.last-commit {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-3 $spacing-4;
  background: $bg-secondary;
  border-bottom: 1px solid $border-color;
  font-size: $font-size-sm;
}

.commit-info {
  display: flex;
  align-items: center;
  gap: $spacing-3;
  
  .commit-author {
    color: $text-secondary;
  }
  
  .commit-message {
    color: $text-primary;
  }
}

.commit-meta {
  display: flex;
  align-items: center;
  gap: $spacing-4;
  color: $text-secondary;
  
  .commit-sha {
    font-family: monospace;
    background: $bg-tertiary;
    padding: 2px 6px;
    border-radius: $border-radius-sm;
  }
}

.file-list {
  .file-item {
    display: grid;
    grid-template-columns: auto 1fr 2fr auto;
    align-items: center;
    gap: $spacing-3;
    padding: $spacing-3 $spacing-4;
    border-bottom: 1px solid $border-color;
    font-size: $font-size-sm;
    
    &:last-child {
      border-bottom: none;
    }
    
    &:hover {
      background: $bg-secondary;
    }
  }
}

.icon-folder {
  color: #fbbf24;
}

.icon-file {
  color: $text-muted;
}

.file-name {
  font-weight: $font-weight-medium;
  color: $text-primary;
}

.file-message {
  color: $text-secondary;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file-time {
  color: $text-muted;
  white-space: nowrap;
}

// README
.readme {
  .readme-header {
    display: flex;
    align-items: center;
    gap: $spacing-2;
    padding: $spacing-3 $spacing-4;
    border-bottom: 1px solid $border-color;
    font-size: $font-size-sm;
    font-weight: $font-weight-medium;
    color: $text-secondary;
  }
  
  .readme-content {
    padding: $spacing-5;
    
    h1 {
      font-size: $font-size-xl;
      margin-bottom: $spacing-4;
    }
    
    p {
      color: $text-secondary;
      line-height: 1.6;
    }
  }
}

// Sidebar
.sidebar {
  .card {
    margin-bottom: $spacing-4;
    
    h3 {
      padding: $spacing-4;
      margin: 0;
      font-size: $font-size-sm;
      font-weight: $font-weight-semibold;
      color: $text-primary;
      border-bottom: 1px solid $border-color;
    }
  }
}

.info-list {
  padding: $spacing-3;
}

.info-item {
  display: flex;
  align-items: center;
  gap: $spacing-3;
  padding: $spacing-2;
  font-size: $font-size-sm;
  color: $text-secondary;
  
  svg {
    color: $text-muted;
  }
}

.action-list {
  padding: $spacing-2;
}

.action-item {
  display: flex;
  align-items: center;
  gap: $spacing-3;
  padding: $spacing-3;
  font-size: $font-size-sm;
  color: $brand-primary;
  text-decoration: none;
  border-radius: $border-radius;
  
  &:hover {
    background: $bg-secondary;
  }
}
</style>
