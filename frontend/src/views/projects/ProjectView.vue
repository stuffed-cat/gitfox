<template>
  <div class="project-page">
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <template v-else-if="project">
      <div class="project-header">
        <div class="project-info">
          <h1>{{ project.name }}</h1>
          <p v-if="project.description">{{ project.description }}</p>
        </div>
        <div class="project-actions">
          <span class="badge" :class="visibilityClass(project.visibility)">
            {{ visibilityText(project.visibility) }}
          </span>
        </div>
      </div>
      
      <nav class="project-nav">
        <router-link :to="projectPath" exact-active-class="active">
          概览
        </router-link>
        <router-link :to="`${projectPath}/-/tree/${project.default_branch}`" active-class="active">
          文件
        </router-link>
        <router-link :to="`${projectPath}/-/commits/${project.default_branch}`" active-class="active">
          提交
        </router-link>
        <router-link :to="`${projectPath}/-/branches`" active-class="active">
          分支
        </router-link>
        <router-link :to="`${projectPath}/-/tags`" active-class="active">
          标签
        </router-link>
        <router-link :to="`${projectPath}/-/merge_requests`" active-class="active">
          合并请求
        </router-link>
        <router-link :to="`${projectPath}/-/pipelines`" active-class="active">
          流水线
        </router-link>
        <router-link :to="`${projectPath}/-/settings`" active-class="active">
          设置
        </router-link>
      </nav>
      
      <div class="project-content">
        <router-view :project="project" :stats="stats" />
      </div>
    </template>
    
    <div v-else class="empty-state">
      <h3>项目不存在</h3>
      <router-link to="/" class="btn btn-primary">返回首页</router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useProjectStore } from '@/stores/project'

const route = useRoute()
const projectStore = useProjectStore()

const project = computed(() => projectStore.currentProject)
const stats = computed(() => projectStore.projectStats)
const loading = computed(() => projectStore.loading)

// GitLab 风格的项目路径: /{owner}/{repo}
const projectPath = computed(() => {
  if (!project.value) return ''
  return `/${project.value.owner_name}/${project.value.name}`
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

// 监听路由参数变化，通过 owner/repo 获取项目
watch(
  () => [route.params.owner, route.params.repo],
  ([owner, repo]) => {
    if (owner && repo && typeof owner === 'string' && typeof repo === 'string') {
      projectStore.fetchProject(owner, repo)
    }
  },
  { immediate: true }
)
</script>

<style lang="scss" scoped>
.project-page {
  h1 {
    font-size: $font-size-xxl;
    margin-bottom: $spacing-sm;
  }
}

.project-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: $spacing-lg;
  
  p {
    color: $text-muted;
    margin: 0;
  }
}

.project-nav {
  display: flex;
  border-bottom: 1px solid $border-color;
  margin-bottom: $spacing-lg;
  overflow-x: auto;
  
  a {
    padding: $spacing-sm $spacing-md;
    color: $text-secondary;
    text-decoration: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    white-space: nowrap;
    transition: all $transition-fast;
    
    &:hover {
      color: $text-primary;
    }
    
    &.active {
      color: $primary-color;
      border-bottom-color: $primary-color;
    }
  }
}

.project-content {
  background: $bg-primary;
  border-radius: $border-radius;
  border: 1px solid $border-color;
}
</style>
