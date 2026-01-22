<template>
  <div class="dashboard">
    <h1>欢迎回来，{{ userName }}</h1>
    
    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-icon projects-icon">📁</div>
        <div class="stat-content">
          <div class="stat-value">{{ stats.projects }}</div>
          <div class="stat-label">项目</div>
        </div>
      </div>
      
      <div class="stat-card">
        <div class="stat-icon mr-icon">🔀</div>
        <div class="stat-content">
          <div class="stat-value">{{ stats.mergeRequests }}</div>
          <div class="stat-label">待处理合并请求</div>
        </div>
      </div>
      
      <div class="stat-card">
        <div class="stat-icon pipeline-icon">⚡</div>
        <div class="stat-content">
          <div class="stat-value">{{ stats.pipelines }}</div>
          <div class="stat-label">运行中流水线</div>
        </div>
      </div>
    </div>
    
    <div class="dashboard-content">
      <div class="recent-projects">
        <div class="section-header">
          <h2>最近项目</h2>
          <router-link to="/projects" class="btn btn-outline btn-sm">查看全部</router-link>
        </div>
        
        <div v-if="loading" class="loading">
          <div class="loading-spinner"></div>
        </div>
        
        <div v-else-if="projects.length === 0" class="empty-state">
          <h3>暂无项目</h3>
          <p>创建您的第一个项目开始使用</p>
          <router-link to="/projects/new" class="btn btn-primary">创建项目</router-link>
        </div>
        
        <div v-else class="project-list">
          <router-link
            v-for="project in projects"
            :key="project.id"
            :to="`/projects/${project.slug}`"
            class="project-item"
          >
            <div class="project-info">
              <h3>{{ project.name }}</h3>
              <p>{{ project.description || '暂无描述' }}</p>
            </div>
            <div class="project-meta">
              <span class="badge" :class="visibilityClass(project.visibility)">
                {{ visibilityText(project.visibility) }}
              </span>
              <span class="updated">{{ formatDate(project.updated_at) }}</span>
            </div>
          </router-link>
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
const projects = computed(() => projectStore.projects.slice(0, 5))
const loading = computed(() => projectStore.loading)

const stats = ref({
  projects: 0,
  mergeRequests: 0,
  pipelines: 0
})

function visibilityClass(visibility: string) {
  return {
    'badge-success': visibility === 'public',
    'badge-secondary': visibility === 'private',
    'badge-info': visibility === 'internal'
  }
}

function visibilityText(visibility: string) {
  const map: Record<string, string> = {
    public: '公开',
    private: '私有',
    internal: '内部'
  }
  return map[visibility] || visibility
}

function formatDate(date: string) {
  return dayjs(date).fromNow()
}

onMounted(async () => {
  await projectStore.fetchProjects()
  stats.value.projects = projectStore.projects.length
})
</script>

<style lang="scss" scoped>
.dashboard {
  h1 {
    font-size: $font-size-xxl;
    margin-bottom: $spacing-lg;
  }
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: $spacing-md;
  margin-bottom: $spacing-xl;
}

.stat-card {
  background: $bg-primary;
  border-radius: $border-radius;
  padding: $spacing-lg;
  display: flex;
  align-items: center;
  gap: $spacing-md;
  box-shadow: $shadow-sm;
  border: 1px solid $border-color;
}

.stat-icon {
  font-size: 32px;
}

.stat-value {
  font-size: $font-size-xl;
  font-weight: 600;
  color: $text-primary;
}

.stat-label {
  color: $text-muted;
  font-size: $font-size-sm;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: $spacing-md;
  
  h2 {
    font-size: $font-size-lg;
  }
}

.recent-projects {
  background: $bg-primary;
  border-radius: $border-radius;
  padding: $spacing-lg;
  box-shadow: $shadow-sm;
  border: 1px solid $border-color;
}

.project-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
}

.project-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-md;
  border-radius: $border-radius;
  text-decoration: none;
  color: inherit;
  border: 1px solid transparent;
  transition: all $transition-fast;
  
  &:hover {
    background: $bg-secondary;
    border-color: $border-color;
  }
}

.project-info {
  h3 {
    font-size: $font-size-base;
    margin-bottom: $spacing-xs;
    color: $primary-color;
  }
  
  p {
    color: $text-muted;
    font-size: $font-size-sm;
    margin: 0;
  }
}

.project-meta {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  
  .updated {
    color: $text-muted;
    font-size: $font-size-sm;
  }
}
</style>
