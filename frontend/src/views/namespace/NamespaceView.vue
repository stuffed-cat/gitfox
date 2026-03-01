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
    
    <template v-else-if="namespaceType === 'group'">
      <!-- Group Profile -->
      <div class="namespace-header">
        <div class="avatar group-avatar-shape">
          {{ namespace?.name?.charAt(0).toUpperCase() || '?' }}
        </div>
        <div class="info">
          <div class="name-row">
            <h1>{{ namespace?.name || namespacePath }}</h1>
            <span v-if="namespace?.visibility" class="visibility-badge" :class="namespace.visibility">
              {{ namespace.visibility === 'public' ? '公开' : namespace.visibility === 'internal' ? '内部' : '私有' }}
            </span>
          </div>
          <p class="username">{{ namespace?.path || namespacePath }}</p>
          <p v-if="namespace?.description" class="description">{{ namespace.description }}</p>
        </div>
        <div class="header-actions">
          <router-link :to="`/${namespace?.path}/-/settings`" class="btn btn-outline">
            查看群组详情
          </router-link>
        </div>
      </div>
      
      <!-- Group Tabs -->
      <div class="tabs">
        <button :class="['tab', { active: activeTab === 'subgroups' }]" @click="activeTab = 'subgroups'">
          子群组与项目
        </button>
        <button :class="['tab', { active: activeTab === 'activity' }]" @click="activeTab = 'activity'">
          活动
        </button>
      </div>
      
      <div v-if="activeTab === 'subgroups'" class="projects-section">
        <div v-if="projects.length === 0 && subgroups.length === 0" class="empty-state">
          <p>暂无子群组或项目</p>
        </div>
        <div v-else class="project-list">
          <!-- Subgroups -->
          <router-link
            v-for="sg in subgroups"
            :key="'sg-' + sg.id"
            :to="`/${sg.path}`"
            class="project-item"
          >
            <div class="project-avatar group-color">{{ sg.name.charAt(0).toUpperCase() }}</div>
            <div class="project-info">
              <div class="item-title-row">
                <h3>{{ sg.name }}</h3>
                <span class="type-badge group-type">群组</span>
              </div>
              <p>{{ sg.description || '暂无描述' }}</p>
            </div>
          </router-link>
          <!-- Projects -->
          <router-link
            v-for="project in projects"
            :key="'p-' + project.id"
            :to="`/${namespacePath}/${project.name}`"
            class="project-item"
          >
            <div class="project-avatar">{{ project.name.charAt(0).toUpperCase() }}</div>
            <div class="project-info">
              <div class="item-title-row">
                <h3>{{ project.name }}</h3>
                <span class="type-badge project-type">项目</span>
              </div>
              <p>{{ project.description || '暂无描述' }}</p>
            </div>
          </router-link>
        </div>
      </div>
      
      <div v-if="activeTab === 'activity'" class="activity-section">
        <div class="empty-state">
          <p>暂无活动</p>
        </div>
      </div>
    </template>
    
    <template v-else>
      <!-- User Profile -->
      <div class="namespace-header">
        <div class="avatar" :class="{ 'has-image': namespace?.avatar_url }">
          <img v-if="namespace?.avatar_url" :src="namespace.avatar_url" :alt="namespace?.username" />
          <span v-else>{{ namespace?.name?.charAt(0).toUpperCase() || '?' }}</span>
        </div>
        <div class="info">
          <h1>{{ namespace?.name || namespace?.display_name || namespacePath }}</h1>
          <p class="username">@{{ namespacePath }}</p>
          <div v-if="namespace?.status_emoji || namespace?.status_message" class="user-status">
            <span v-if="namespace?.status_emoji" class="status-emoji">{{ namespace.status_emoji }}</span>
            <span v-if="namespace?.status_message" class="status-message">{{ namespace.status_message }}</span>
            <span v-if="namespace?.busy" class="busy-indicator">忙碌中</span>
          </div>
        </div>
      </div>
      
      <div class="tabs">
        <button :class="['tab', { active: activeTab === 'projects' }]" @click="activeTab = 'projects'">
          项目
        </button>
        <button :class="['tab', { active: activeTab === 'activity' }]" @click="activeTab = 'activity'">
          活动
        </button>
      </div>
      
      <div v-if="activeTab === 'projects'" class="projects-section">
        <div v-if="projects.length === 0" class="empty-state">
          <p>暂无公开项目</p>
        </div>
        <div v-else class="project-list">
          <router-link
            v-for="project in projects"
            :key="project.id"
            :to="`/${namespacePath}/${project.name}`"
            class="project-item"
          >
            <div class="project-avatar">{{ project.name.charAt(0).toUpperCase() }}</div>
            <div class="project-info">
              <h3>{{ project.name }}</h3>
              <p>{{ project.description || '暂无描述' }}</p>
            </div>
          </router-link>
        </div>
      </div>
      
      <div v-if="activeTab === 'activity'" class="activity-section">
        <div class="empty-state">
          <p>暂无活动</p>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, onUnmounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import { api } from '@/api'
import { useNamespaceStore } from '@/stores/namespace'
import type { Group } from '@/types'

const props = defineProps<{
  path?: string
}>()

const route = useRoute()
const namespaceStore = useNamespaceStore()

// 优先使用 prop 传入的 path，否则使用 route.params.namespace
const namespacePath = computed(() => props.path || route.params.namespace as string)

const loading = ref(true)
const notFound = ref(false)
const namespaceType = ref<'user' | 'group'>('user')
const namespace = ref<any>(null)
const projects = ref<any[]>([])
const subgroups = ref<Group[]>([])
const activeTab = ref('projects')

async function loadNamespace() {
  loading.value = true
  notFound.value = false
  namespaceType.value = 'user'
  subgroups.value = []
  namespaceStore.clearNamespaceContext()
  
  const currentPath = namespacePath.value
  if (!currentPath) return
  
  try {
    // 先尝试获取群组
    try {
      const groupData = await api.groups.get(currentPath)
      namespace.value = groupData
      namespaceType.value = 'group'
      activeTab.value = 'subgroups'
      
      // 设置群组上下文到 store
      namespaceStore.setNamespaceContext('group', currentPath, groupData)
      
      // 获取子群组和项目
      const [sg, proj] = await Promise.all([
        api.groups.listSubgroups(currentPath).catch(() => []),
        api.groups.listProjects(currentPath).catch(() => [])
      ])
      subgroups.value = sg
      projects.value = proj
      return
    } catch {
      // Not a group, try user
    }
    
    // 尝试获取用户
    const response = await api.users.get(currentPath)
    namespace.value = response
    namespaceType.value = 'user'
    activeTab.value = 'projects'
    namespaceStore.setNamespaceContext('user', currentPath)
    
    const projectsRes = await api.projects.list()
    projects.value = projectsRes.filter((p: any) => p.owner_name === currentPath)
  } catch (error: any) {
    if (error.response?.status === 404) {
      notFound.value = true
    }
  } finally {
    loading.value = false
  }
}

onMounted(loadNamespace)
watch(namespacePath, loadNamespace)

onUnmounted(() => {
  namespaceStore.clearNamespaceContext()
})
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
    flex-shrink: 0;
    overflow: hidden;
    position: relative;
    
    img {
      width: 100%;
      height: 100%;
      object-fit: cover;
    }
    
    &.group-avatar-shape { border-radius: $radius-lg; }
  }
  
  .info {
    flex: 1;
    
    .name-row {
      display: flex;
      align-items: center;
      gap: $spacing-3;
      margin-bottom: $spacing-1;
    }
    
    h1 {
      font-size: $text-2xl;
      font-weight: 600;
    }
    
    .username {
      color: $text-secondary;
      margin-bottom: $spacing-1;
    }
    
    .user-status {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-top: $spacing-2;
      padding: 6px 10px;
      background: rgba($brand-primary, 0.05);
      border-radius: 4px;
      width: fit-content;
      
      .status-emoji {
        font-size: 16px;
      }
      
      .status-message {
        color: $text-primary;
        font-size: 12px;
      }
      
      .busy-indicator {
        font-size: 11px;
        color: $color-warning;
        font-weight: 500;
      }
    }
    
    .description {
      color: $text-secondary;
      font-size: $text-sm;
      line-height: 1.5;
    }
  }
  
  .header-actions {
    flex-shrink: 0;
  }
}

.visibility-badge {
  display: inline-block;
  padding: 2px $spacing-2;
  border-radius: $radius-full;
  font-size: $text-xs;
  font-weight: 500;
  
  &.public { background: #ddf4ff; color: #0969da; }
  &.internal { background: #fff8c5; color: #9a6700; }
  &.private { background: #ffebe9; color: #cf222e; }
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
    font-size: $text-sm;
    
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
  display: flex;
  flex-direction: column;
  gap: $spacing-3;
  
  .project-item {
    display: flex;
    gap: $spacing-4;
    padding: $spacing-4;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    text-decoration: none;
    transition: all 0.15s ease;
    
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
      flex-shrink: 0;
      
      &.group-color {
        background: linear-gradient(135deg, #e67e22, #d35400);
      }
    }
    
    .project-info {
      flex: 1;
      min-width: 0;
      
      .item-title-row {
        display: flex;
        align-items: center;
        gap: $spacing-2;
        margin-bottom: $spacing-1;
      }
      
      h3 { color: $text-primary; font-weight: 600; }
      p { color: $text-secondary; font-size: $text-sm; }
    }
  }
}

.type-badge {
  display: inline-block;
  padding: 1px $spacing-2;
  border-radius: $radius-full;
  font-size: $text-xs;
  font-weight: 500;
  
  &.group-type { background: #fff3e0; color: #e65100; }
  &.project-type { background: #e8f5e9; color: #2e7d32; }
}

.btn {
  display: inline-block;
  padding: $spacing-2 $spacing-4;
  border-radius: $radius-md;
  font-size: $text-sm;
  font-weight: 500;
  text-decoration: none;
  cursor: pointer;
  transition: all 0.2s;
  border: none;
}

.btn-primary {
  background: $color-primary;
  color: white;
  &:hover { background: $color-primary-dark; }
}

.btn-outline {
  background: transparent;
  color: $text-primary;
  border: 1px solid $border-color;
  &:hover { background: $bg-secondary; }
}

@keyframes spin { to { transform: rotate(360deg); } }
</style>
