<template>
  <div class="dashboard-page">
    <!-- Welcome Banner -->
    <div class="welcome-banner">
      <div class="welcome-content">
        <div class="today-label">今日概览</div>
        <h1 class="welcome-title">你好，{{ userName }}</h1>
      </div>
      <div class="welcome-actions">
        <router-link to="/projects/new" class="btn btn-primary">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
          新建项目
        </router-link>
      </div>
    </div>
    
    <!-- Info Banner -->
    <div class="info-banner">
      <div class="banner-icon">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
          <path d="M12 2L15.09 8.26L22 9.27L17 14.14L18.18 21.02L12 17.77L5.82 21.02L7 14.14L2 9.27L8.91 8.26L12 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </div>
      <div class="banner-content">
        <h3>欢迎使用新版首页</h3>
        <p>我们正在为您提供全新的方式来概览您的工作，方便您规划下一步。如果您想更改默认首页，可以<a href="#">更新您的用户偏好</a>。</p>
      </div>
      <button class="banner-close" @click="hideBanner">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
    
    <!-- Stats Cards -->
    <div class="stats-row">
      <div class="stats-card">
        <div class="stats-header">
          <span class="stats-title">合并请求</span>
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="4" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="12" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="8" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/>
            <path d="M4 6v2a4 4 0 004 4m4-6v2a4 4 0 01-4 4" stroke="currentColor" stroke-width="1.5"/>
          </svg>
        </div>
        <div class="stats-value">{{ stats.pendingReview }}</div>
        <div class="stats-label">等待您的审核</div>
        <div class="stats-time">刚刚</div>
      </div>
      
      <div class="stats-card">
        <div class="stats-header">
          <span class="stats-title">合并请求</span>
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="4" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="12" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="8" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/>
            <path d="M4 6v2a4 4 0 004 4m4-6v2a4 4 0 01-4 4" stroke="currentColor" stroke-width="1.5"/>
          </svg>
        </div>
        <div class="stats-value">{{ stats.assignedMR }}</div>
        <div class="stats-label">分配给您</div>
        <div class="stats-time">刚刚</div>
      </div>
      
      <div class="stats-card">
        <div class="stats-header">
          <span class="stats-title">议题</span>
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <rect x="2" y="2" width="12" height="12" rx="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="8" cy="8" r="2" fill="currentColor"/>
          </svg>
        </div>
        <div class="stats-value">{{ stats.assignedIssues }}</div>
        <div class="stats-label">分配给您</div>
        <div class="stats-time">刚刚</div>
      </div>
      
      <div class="stats-card">
        <div class="stats-header">
          <span class="stats-title">议题</span>
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <rect x="2" y="2" width="12" height="12" rx="2" stroke="currentColor" stroke-width="1.5"/>
            <circle cx="8" cy="8" r="2" fill="currentColor"/>
          </svg>
        </div>
        <div class="stats-value">{{ stats.authoredIssues }}</div>
        <div class="stats-label">由您创建</div>
        <div class="stats-time">刚刚</div>
      </div>
    </div>
    
    <div class="dashboard-grid">
      <!-- Main Content -->
      <div class="main-content">
        <!-- Items that need attention -->
        <div class="card">
          <div class="card-header">
            <h3>需要您关注的事项</h3>
            <select class="filter-select">
              <option>全部</option>
              <option>合并请求</option>
              <option>议题</option>
            </select>
          </div>
          <div class="card-body">
            <div v-if="attentionItems.length === 0" class="empty-state">
              <svg class="empty-icon" viewBox="0 0 64 64" fill="none">
                <circle cx="32" cy="32" r="28" stroke="currentColor" stroke-width="2"/>
                <path d="M32 20v16M32 44h.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              </svg>
              <h3>没有需要关注的事项</h3>
              <p>当有合并请求或议题需要您处理时，会显示在这里</p>
            </div>
            <div v-else class="attention-list">
              <div v-for="item in attentionItems" :key="item.id" class="attention-item">
                <div class="attention-icon" :class="item.type">
                  <svg v-if="item.type === 'mr'" width="16" height="16" viewBox="0 0 16 16" fill="none">
                    <circle cx="4" cy="4" r="2" stroke="currentColor" stroke-width="1.5"/>
                    <circle cx="8" cy="12" r="2" stroke="currentColor" stroke-width="1.5"/>
                  </svg>
                </div>
                <div class="attention-content">
                  <div class="attention-title">{{ item.title }}</div>
                  <div class="attention-meta">{{ item.project }} · {{ item.time }}</div>
                </div>
                <div class="attention-actions">
                  <button class="btn btn-ghost btn-sm" title="稍后提醒">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                      <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
                      <path d="M8 5v3l2 2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                    </svg>
                  </button>
                  <button class="btn btn-ghost btn-sm" title="标记完成">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                      <path d="M3 8l4 4 6-8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                  </button>
                </div>
              </div>
            </div>
          </div>
          <div class="card-footer">
            <a href="#" class="text-link">查看所有待办事项</a>
          </div>
        </div>
        
        <!-- Recent Activity -->
        <div class="card">
          <div class="card-header">
            <h3>最新动态</h3>
            <select class="filter-select">
              <option>您的动态</option>
              <option>关注的项目</option>
              <option>所有</option>
            </select>
          </div>
          <div class="card-body">
            <div v-if="activities.length === 0" class="empty-state">
              <svg class="empty-icon" viewBox="0 0 64 64" fill="none">
                <path d="M8 32h12l8-16 8 32 8-16h12" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <h3>暂无动态</h3>
              <p>您的项目活动会显示在这里</p>
            </div>
            <div v-else class="activity-list">
              <div v-for="activity in activities" :key="activity.id" class="activity-item">
                <span class="avatar avatar-sm">{{ activity.userInitial }}</span>
                <div class="activity-content">
                  <span class="activity-text">{{ activity.text }}</span>
                  <span class="activity-time">{{ activity.time }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <!-- Sidebar -->
      <div class="sidebar-content">
        <!-- Quick Access -->
        <div class="card">
          <div class="card-header">
            <h3>快速访问</h3>
            <div class="tabs-mini">
              <button class="tab-mini active">最近查看</button>
              <button class="tab-mini">项目</button>
            </div>
          </div>
          <div class="card-body">
            <div v-if="recentProjects.length === 0" class="quick-access-empty">
              <p>您最近访问的议题和合并请求将显示在这里</p>
            </div>
            <div v-else class="quick-access-list">
              <router-link 
                v-for="project in recentProjects" 
                :key="project.id" 
                :to="`/${project.owner_name || 'unknown'}/${project.name}`"
                class="quick-access-item"
              >
                <span class="project-avatar">{{ project.name.charAt(0) }}</span>
                <span class="project-name">{{ project.name }}</span>
              </router-link>
            </div>
          </div>
        </div>
        
        <!-- Feedback -->
        <div class="card feedback-card">
          <div class="card-header">
            <h3>分享您的反馈</h3>
          </div>
          <div class="card-body">
            <p>帮助我们改进新首页，分享您的想法和建议。</p>
            <a href="#" class="text-link">提出反馈</a>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { useProjectStore } from '@/stores/project'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

const authStore = useAuthStore()
const projectStore = useProjectStore()

const userName = computed(() => authStore.user?.display_name || authStore.user?.username || '用户')
const recentProjects = computed(() => projectStore.projects.slice(0, 5))

const stats = ref({
  pendingReview: 0,
  assignedMR: 0,
  assignedIssues: 0,
  authoredIssues: 0
})

const attentionItems = ref<any[]>([])
const activities = ref<any[]>([])

const showBanner = ref(true)

function hideBanner() {
  showBanner.value = false
}

onMounted(async () => {
  await projectStore.fetchProjects()
  stats.value = {
    pendingReview: 0,
    assignedMR: 0,
    assignedIssues: 0,
    authoredIssues: 0
  }
})
</script>

<style lang="scss" scoped>
.dashboard-page {
  padding: $spacing-6;
  max-width: 1400px;
  margin: 0 auto;
}

.welcome-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: $spacing-6;
}

.today-label {
  font-size: $font-size-sm;
  color: $text-secondary;
  margin-bottom: $spacing-1;
}

.welcome-title {
  font-size: $font-size-3xl;
  font-weight: $font-weight-semibold;
  color: $text-primary;
  margin: 0;
  
  display: flex;
  align-items: center;
  gap: $spacing-3;
}

.info-banner {
  display: flex;
  align-items: flex-start;
  gap: $spacing-4;
  padding: $spacing-5;
  background: linear-gradient(135deg, #e0e7ff 0%, #c7d2fe 100%);
  border-radius: $border-radius-lg;
  margin-bottom: $spacing-6;
  position: relative;
  
  .banner-icon {
    width: 40px;
    height: 40px;
    background: rgba(99, 102, 241, 0.1);
    border-radius: $border-radius;
    display: flex;
    align-items: center;
    justify-content: center;
    color: $brand-primary;
    flex-shrink: 0;
  }
  
  .banner-content {
    flex: 1;
    
    h3 {
      font-size: $font-size-lg;
      font-weight: $font-weight-semibold;
      color: $gray-900;
      margin: 0 0 $spacing-2;
    }
    
    p {
      color: $gray-700;
      margin: 0;
      font-size: $font-size-base;
      
      a {
        color: $brand-primary;
        text-decoration: underline;
      }
    }
  }
  
  .banner-close {
    position: absolute;
    top: $spacing-4;
    right: $spacing-4;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: $gray-600;
    cursor: pointer;
    border-radius: $border-radius;
    
    &:hover {
      background: rgba(0, 0, 0, 0.1);
    }
  }
}

.stats-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: $spacing-5;
  margin-bottom: $spacing-6;
  
  @media (max-width: $breakpoint-lg) {
    grid-template-columns: repeat(2, 1fr);
  }
  
  @media (max-width: $breakpoint-sm) {
    grid-template-columns: 1fr;
  }
}

.stats-card {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
  padding: $spacing-5;
  
  .stats-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: $spacing-3;
    
    .stats-title {
      font-size: $font-size-sm;
      font-weight: $font-weight-medium;
      color: $text-secondary;
    }
    
    svg {
      color: $text-muted;
    }
  }
  
  .stats-value {
    font-size: $font-size-4xl;
    font-weight: $font-weight-bold;
    color: $text-primary;
    line-height: 1;
    margin-bottom: $spacing-1;
  }
  
  .stats-label {
    font-size: $font-size-sm;
    color: $text-secondary;
    margin-bottom: $spacing-2;
  }
  
  .stats-time {
    font-size: $font-size-xs;
    color: $text-muted;
  }
}

.dashboard-grid {
  display: grid;
  grid-template-columns: 1fr 320px;
  gap: $spacing-6;
  
  @media (max-width: $breakpoint-lg) {
    grid-template-columns: 1fr;
  }
}

.main-content {
  display: flex;
  flex-direction: column;
  gap: $spacing-6;
}

.sidebar-content {
  display: flex;
  flex-direction: column;
  gap: $spacing-5;
}

.card {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
  overflow: hidden;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: $spacing-4 $spacing-5;
  border-bottom: 1px solid $border-color;
  
  h3 {
    font-size: $font-size-base;
    font-weight: $font-weight-semibold;
    margin: 0;
  }
}

.card-body {
  padding: $spacing-5;
}

.card-footer {
  padding: $spacing-4 $spacing-5;
  border-top: 1px solid $border-color;
  text-align: center;
  
  .text-link {
    font-size: $font-size-sm;
    color: $text-link;
    
    &:hover {
      text-decoration: underline;
    }
  }
}

.filter-select {
  padding: $spacing-1 $spacing-3;
  font-size: $font-size-sm;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  background: $bg-primary;
  color: $text-primary;
  cursor: pointer;
  
  &:focus {
    outline: none;
    border-color: $brand-primary;
  }
}

.empty-state {
  text-align: center;
  padding: $spacing-8 $spacing-4;
  
  .empty-icon {
    width: 48px;
    height: 48px;
    margin: 0 auto $spacing-4;
    color: $text-muted;
  }
  
  h3 {
    font-size: $font-size-lg;
    font-weight: $font-weight-medium;
    color: $text-primary;
    margin: 0 0 $spacing-2;
  }
  
  p {
    color: $text-secondary;
    margin: 0;
  }
}

.attention-list {
  display: flex;
  flex-direction: column;
}

.attention-item {
  display: flex;
  align-items: center;
  gap: $spacing-3;
  padding: $spacing-3;
  border-radius: $border-radius;
  
  &:hover {
    background: $bg-secondary;
  }
}

.attention-icon {
  width: 32px;
  height: 32px;
  border-radius: $border-radius-full;
  display: flex;
  align-items: center;
  justify-content: center;
  
  &.mr {
    background: $color-info-light;
    color: $color-info;
  }
  
  &.issue {
    background: $color-success-light;
    color: $color-success;
  }
}

.attention-content {
  flex: 1;
  min-width: 0;
}

.attention-title {
  font-size: $font-size-base;
  font-weight: $font-weight-medium;
  color: $text-primary;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.attention-meta {
  font-size: $font-size-sm;
  color: $text-secondary;
}

.attention-actions {
  display: flex;
  gap: $spacing-1;
  opacity: 0;
  transition: opacity $transition-fast;
  
  .attention-item:hover & {
    opacity: 1;
  }
}

.activity-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-3;
}

.activity-item {
  display: flex;
  align-items: flex-start;
  gap: $spacing-3;
}

.activity-content {
  flex: 1;
}

.activity-text {
  font-size: $font-size-base;
  color: $text-primary;
}

.activity-time {
  font-size: $font-size-sm;
  color: $text-muted;
  margin-left: $spacing-2;
}

.tabs-mini {
  display: flex;
  gap: $spacing-1;
}

.tab-mini {
  padding: $spacing-1 $spacing-3;
  font-size: $font-size-sm;
  color: $text-secondary;
  background: transparent;
  border: none;
  border-radius: $border-radius;
  cursor: pointer;
  
  &:hover {
    background: $bg-secondary;
    color: $text-primary;
  }
  
  &.active {
    background: $bg-tertiary;
    color: $text-primary;
    font-weight: $font-weight-medium;
  }
}

.quick-access-empty {
  text-align: center;
  padding: $spacing-4;
  
  p {
    color: $text-muted;
    font-size: $font-size-sm;
    margin: 0;
  }
}

.quick-access-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-2;
}

.quick-access-item {
  display: flex;
  align-items: center;
  gap: $spacing-3;
  padding: $spacing-2;
  border-radius: $border-radius;
  text-decoration: none;
  color: $text-primary;
  
  &:hover {
    background: $bg-secondary;
  }
}

.project-avatar {
  width: 32px;
  height: 32px;
  border-radius: $border-radius;
  background: $brand-gradient;
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: $font-weight-semibold;
  font-size: $font-size-sm;
}

.project-name {
  font-size: $font-size-base;
  color: $text-primary;
}

.feedback-card {
  .card-body {
    p {
      color: $text-secondary;
      font-size: $font-size-sm;
      margin: 0 0 $spacing-3;
    }
    
    .text-link {
      font-size: $font-size-sm;
      color: $text-link;
      
      &:hover {
        text-decoration: underline;
      }
    }
  }
}
</style>

