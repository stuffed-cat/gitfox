<template>
  <div class="project-list-page">
    <div class="page-header">
      <h1>项目列表</h1>
      <router-link to="/projects/new" class="btn btn-primary">
        + 新建项目
      </router-link>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <div v-else-if="projects.length === 0" class="empty-state">
      <h3>暂无项目</h3>
      <p>创建您的第一个项目开始使用 DevOps</p>
      <router-link to="/projects/new" class="btn btn-primary">创建项目</router-link>
    </div>
    
    <div v-else class="projects-grid">
      <router-link
        v-for="project in projects"
        :key="project.id"
        :to="`/projects/${project.slug}`"
        class="project-card"
      >
        <div class="project-card-header">
          <h3>{{ project.name }}</h3>
          <span class="badge" :class="visibilityClass(project.visibility)">
            {{ visibilityText(project.visibility) }}
          </span>
        </div>
        <p class="project-description">{{ project.description || '暂无描述' }}</p>
        <div class="project-card-footer">
          <span class="branch">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M5 3.25a.75.75 0 11-1.5 0 .75.75 0 011.5 0zm0 2.122a2.25 2.25 0 10-1.5 0v.878A2.25 2.25 0 005.75 8.5h1.5v2.128a2.251 2.251 0 101.5 0V8.5h1.5a2.25 2.25 0 002.25-2.25v-.878a2.25 2.25 0 10-1.5 0v.878a.75.75 0 01-.75.75h-4.5a.75.75 0 01-.75-.75v-.878zm6.25 4.378a.75.75 0 10-1.5 0 .75.75 0 001.5 0z"/>
            </svg>
            {{ project.default_branch }}
          </span>
          <span class="updated">更新于 {{ formatDate(project.updated_at) }}</span>
        </div>
      </router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useProjectStore } from '@/stores/project'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'

dayjs.extend(relativeTime)
dayjs.locale('zh-cn')

const projectStore = useProjectStore()

const projects = computed(() => projectStore.projects)
const loading = computed(() => projectStore.loading)

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

onMounted(() => {
  projectStore.fetchProjects()
})
</script>

<style lang="scss" scoped>
.project-list-page {
  h1 {
    font-size: $font-size-xxl;
  }
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: $spacing-xl;
}

.projects-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: $spacing-lg;
}

.project-card {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  padding: $spacing-lg;
  text-decoration: none;
  color: inherit;
  transition: all $transition-fast;
  
  &:hover {
    border-color: $primary-color;
    box-shadow: $shadow-md;
    transform: translateY(-2px);
  }
}

.project-card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: $spacing-sm;
  
  h3 {
    font-size: $font-size-lg;
    color: $primary-color;
    margin: 0;
  }
}

.project-description {
  color: $text-muted;
  font-size: $font-size-sm;
  margin-bottom: $spacing-md;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.project-card-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: $font-size-xs;
  color: $text-muted;
  
  .branch {
    display: flex;
    align-items: center;
    gap: $spacing-xs;
  }
}
</style>
