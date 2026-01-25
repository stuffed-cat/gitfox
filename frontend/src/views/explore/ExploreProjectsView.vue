<template>
  <div class="explore-projects-page">
    <div class="page-header">
      <h1>探索项目</h1>
    </div>
    
    <div class="tabs">
      <button :class="['tab', { active: activeTab === 'all' }]" @click="activeTab = 'all'">所有</button>
      <button :class="['tab', { active: activeTab === 'trending' }]" @click="activeTab = 'trending'">热门</button>
      <button :class="['tab', { active: activeTab === 'starred' }]" @click="activeTab = 'starred'">最多星标</button>
    </div>
    
    <div class="filter-bar">
      <input v-model="searchQuery" type="text" placeholder="搜索项目..." class="search-input" />
    </div>
    
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>
    
    <div v-else-if="projects.length === 0" class="empty-state">
      <h3>暂无公开项目</h3>
      <p>创建第一个公开项目来分享您的代码</p>
    </div>
    
    <div v-else class="project-list">
      <router-link
        v-for="project in filteredProjects"
        :key="project.id"
        :to="`/${project.owner_name}/${project.name}`"
        class="project-item"
      >
        <div class="project-avatar">{{ project.name.charAt(0).toUpperCase() }}</div>
        <div class="project-info">
          <h3>{{ project.owner_name }} / {{ project.name }}</h3>
          <p>{{ project.description || '暂无描述' }}</p>
        </div>
      </router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useProjectStore } from '@/stores/project'

const projectStore = useProjectStore()

const loading = computed(() => projectStore.loading)
const projects = computed(() => projectStore.projects)

const activeTab = ref('all')
const searchQuery = ref('')

const filteredProjects = computed(() => {
  if (!searchQuery.value) return projects.value
  const query = searchQuery.value.toLowerCase()
  return projects.value.filter(p => 
    p.name.toLowerCase().includes(query) || 
    p.name.toLowerCase().includes(query)
  )
})

onMounted(() => {
  projectStore.fetchProjects()
})
</script>

<style lang="scss" scoped>
.explore-projects-page {
  padding: $spacing-6;
  max-width: 1200px;
  margin: 0 auto;
}

.page-header {
  margin-bottom: $spacing-6;
  h1 { font-size: $text-2xl; font-weight: 600; }
}

.tabs {
  display: flex;
  gap: $spacing-1;
  border-bottom: 1px solid $border-color;
  margin-bottom: $spacing-4;
  
  .tab {
    padding: $spacing-3 $spacing-4;
    background: none;
    border: none;
    color: $text-secondary;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    
    &:hover { color: $text-primary; }
    &.active { color: $text-primary; border-bottom-color: $color-primary; }
  }
}

.filter-bar {
  margin-bottom: $spacing-4;
  
  .search-input {
    width: 100%;
    max-width: 400px;
    padding: $spacing-2 $spacing-3;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    background: $bg-primary;
    color: $text-primary;
  }
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: $spacing-12;
  
  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid $border-color;
    border-top-color: $color-primary;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }
}

.empty-state {
  text-align: center;
  padding: $spacing-12;
  h3 { color: $text-primary; margin-bottom: $spacing-2; }
  p { color: $text-secondary; }
}

.project-list {
  .project-item {
    display: flex;
    gap: $spacing-4;
    padding: $spacing-4;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    margin-bottom: $spacing-3;
    text-decoration: none;
    transition: all 0.2s;
    
    &:hover {
      border-color: $color-primary;
      background: $bg-secondary;
    }
    
    .project-avatar {
      width: 48px;
      height: 48px;
      border-radius: $radius-md;
      background: $color-primary;
      color: white;
      display: flex;
      align-items: center;
      justify-content: center;
      font-weight: 600;
    }
    
    .project-info {
      h3 { color: $text-primary; margin-bottom: $spacing-1; }
      p { color: $text-secondary; font-size: $text-sm; }
    }
  }
}

@keyframes spin { to { transform: rotate(360deg); } }
</style>
