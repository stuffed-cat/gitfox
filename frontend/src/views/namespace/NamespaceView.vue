<template>
  <div class="namespace-page">
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>
    
    <div v-else-if="notFound" class="not-found">
      <h1>404</h1>
      <p>用户或群组不存在</p>
      <router-link to="/" class="btn btn-primary">返回首页</router-link>
    </div>
    
    <template v-else>
      <!-- User/Group Header -->
      <div class="namespace-header">
        <div class="avatar">
          {{ namespace?.name?.charAt(0).toUpperCase() || '?' }}
        </div>
        <div class="info">
          <h1>{{ namespace?.name || route.params.namespace }}</h1>
          <p class="username">@{{ route.params.namespace }}</p>
        </div>
      </div>
      
      <!-- Tabs -->
      <div class="tabs">
        <button :class="['tab', { active: activeTab === 'projects' }]" @click="activeTab = 'projects'">
          项目
        </button>
        <button :class="['tab', { active: activeTab === 'activity' }]" @click="activeTab = 'activity'">
          活动
        </button>
      </div>
      
      <!-- Projects List -->
      <div v-if="activeTab === 'projects'" class="projects-section">
        <div v-if="projects.length === 0" class="empty-state">
          <p>暂无公开项目</p>
        </div>
        <div v-else class="project-list">
          <router-link
            v-for="project in projects"
            :key="project.id"
            :to="`/${route.params.namespace}/${project.slug}`"
            class="project-item"
          >
            <div class="project-avatar">{{ project.name.charAt(0).toUpperCase() }}</div>
            <div class="project-info">
              <h3>{{ project.slug }}</h3>
              <p>{{ project.description || '暂无描述' }}</p>
            </div>
          </router-link>
        </div>
      </div>
      
      <!-- Activity -->
      <div v-if="activeTab === 'activity'" class="activity-section">
        <div class="empty-state">
          <p>暂无活动</p>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import api from '@/api'

const route = useRoute()

const loading = ref(true)
const notFound = ref(false)
const namespace = ref<any>(null)
const projects = ref<any[]>([])
const activeTab = ref('projects')

async function loadNamespace() {
  loading.value = true
  notFound.value = false
  
  try {
    // 尝试获取用户或群组信息
    const response = await api.get(`/users/${route.params.namespace}`)
    namespace.value = response.data
    
    // 获取该用户的项目
    const projectsRes = await api.get(`/users/${route.params.namespace}/projects`)
    projects.value = projectsRes.data
  } catch (error: any) {
    if (error.response?.status === 404) {
      notFound.value = true
    }
  } finally {
    loading.value = false
  }
}

onMounted(loadNamespace)

watch(() => route.params.namespace, loadNamespace)
</script>

<style lang="scss" scoped>
.namespace-page {
  padding: $spacing-6;
  max-width: 1000px;
  margin: 0 auto;
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

.not-found {
  text-align: center;
  padding: $spacing-12;
  
  h1 { font-size: 64px; color: $text-muted; }
  p { color: $text-secondary; margin-bottom: $spacing-4; }
}

.namespace-header {
  display: flex;
  align-items: center;
  gap: $spacing-5;
  margin-bottom: $spacing-6;
  padding-bottom: $spacing-6;
  border-bottom: 1px solid $border-color;
  
  .avatar {
    width: 96px;
    height: 96px;
    border-radius: 50%;
    background: linear-gradient(135deg, $color-primary, $color-primary-dark);
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 36px;
    font-weight: 600;
  }
  
  .info {
    h1 {
      font-size: $text-2xl;
      font-weight: 600;
      margin-bottom: $spacing-1;
    }
    
    .username {
      color: $text-secondary;
    }
  }
}

.tabs {
  display: flex;
  gap: $spacing-1;
  border-bottom: 1px solid $border-color;
  margin-bottom: $spacing-6;
  
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

.empty-state {
  text-align: center;
  padding: $spacing-8;
  color: $text-secondary;
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
