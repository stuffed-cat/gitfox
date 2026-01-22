<template>
  <div class="project-overview">
    <div class="stats-grid">
      <div class="stat-item">
        <span class="stat-value">{{ stats?.commits_count || 0 }}</span>
        <span class="stat-label">提交</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">{{ stats?.branches_count || 0 }}</span>
        <span class="stat-label">分支</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">{{ stats?.tags_count || 0 }}</span>
        <span class="stat-label">标签</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">{{ stats?.merge_requests_count || 0 }}</span>
        <span class="stat-label">合并请求</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">{{ stats?.members_count || 0 }}</span>
        <span class="stat-label">成员</span>
      </div>
    </div>
    
    <div class="quick-actions">
      <h3>快速操作</h3>
      <div class="actions-grid">
        <router-link :to="`/projects/${project?.slug}/files`" class="action-card">
          <span class="action-icon">📁</span>
          <span class="action-text">浏览文件</span>
        </router-link>
        <router-link :to="`/projects/${project?.slug}/merge-requests/new`" class="action-card">
          <span class="action-icon">🔀</span>
          <span class="action-text">新建合并请求</span>
        </router-link>
        <router-link :to="`/projects/${project?.slug}/branches`" class="action-card">
          <span class="action-icon">🌿</span>
          <span class="action-text">管理分支</span>
        </router-link>
        <router-link :to="`/projects/${project?.slug}/pipelines`" class="action-card">
          <span class="action-icon">⚡</span>
          <span class="action-text">查看流水线</span>
        </router-link>
      </div>
    </div>
    
    <div class="clone-url">
      <h3>克隆仓库</h3>
      <div class="url-box">
        <code>git clone {{ cloneUrl }}</code>
        <button class="btn btn-outline btn-sm" @click="copyUrl">复制</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Project, ProjectStats } from '@/types'

const props = defineProps<{
  project?: Project
  stats?: ProjectStats
}>()

const cloneUrl = computed(() => {
  if (!props.project) return ''
  return `${window.location.origin}/git/${props.project.slug}.git`
})

function copyUrl() {
  navigator.clipboard.writeText(cloneUrl.value)
}
</script>

<style lang="scss" scoped>
.project-overview {
  padding: $spacing-lg;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
  gap: $spacing-md;
  margin-bottom: $spacing-xl;
}

.stat-item {
  text-align: center;
  padding: $spacing-md;
  background: $bg-secondary;
  border-radius: $border-radius;
}

.stat-value {
  display: block;
  font-size: $font-size-xl;
  font-weight: 600;
  color: $primary-color;
}

.stat-label {
  color: $text-muted;
  font-size: $font-size-sm;
}

.quick-actions {
  margin-bottom: $spacing-xl;
  
  h3 {
    margin-bottom: $spacing-md;
  }
}

.actions-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: $spacing-md;
}

.action-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: $spacing-sm;
  padding: $spacing-lg;
  background: $bg-secondary;
  border-radius: $border-radius;
  text-decoration: none;
  color: $text-primary;
  transition: all $transition-fast;
  
  &:hover {
    background: rgba($primary-color, 0.1);
    transform: translateY(-2px);
  }
}

.action-icon {
  font-size: 24px;
}

.action-text {
  font-size: $font-size-sm;
}

.clone-url {
  h3 {
    margin-bottom: $spacing-md;
  }
}

.url-box {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  padding: $spacing-md;
  background: $bg-secondary;
  border-radius: $border-radius;
  
  code {
    flex: 1;
    background: transparent;
    padding: 0;
  }
}
</style>
