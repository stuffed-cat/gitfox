<template>
  <div class="project-page">
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <template v-else-if="project">
      
      
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


.project-content {
  background: $bg-primary;
  border-radius: $border-radius;
  border: 1px solid $border-color;
}
</style>
