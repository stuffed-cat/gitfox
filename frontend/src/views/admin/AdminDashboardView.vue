<template>
  <div class="admin-dashboard">
    <div class="page-header">
      <h1>管理区域</h1>
      <p class="page-description">系统概览和管理</p>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <p>加载中...</p>
    </div>

    <div v-else-if="stats" class="stats-grid">
      <div class="stat-card">
        <div class="stat-icon users-icon">
          <svg width="24" height="24" viewBox="0 0 16 16" fill="none">
            <path d="M5 5m-2 0a2 2 0 104 0a2 2 0 10-4 0M11 5m-2 0a2 2 0 104 0a2 2 0 10-4 0M1 12a4 4 0 018 0M7 12a4 4 0 018 0" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </div>
        <div class="stat-info">
          <div class="stat-value">{{ stats.total_users }}</div>
          <div class="stat-label">总用户数</div>
          <div class="stat-detail">{{ stats.active_users }} 个活跃</div>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-icon projects-icon">
          <svg width="24" height="24" viewBox="0 0 16 16" fill="none">
            <path d="M2 3a1 1 0 011-1h10a1 1 0 011 1v10a1 1 0 01-1 1H3a1 1 0 01-1-1V3zM5 6h6M5 9h4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </div>
        <div class="stat-info">
          <div class="stat-value">{{ stats.total_projects }}</div>
          <div class="stat-label">总项目数</div>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-icon groups-icon">
          <svg width="24" height="24" viewBox="0 0 16 16" fill="none">
            <rect x="1" y="2" width="6" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
            <rect x="9" y="2" width="6" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
            <rect x="5" y="8" width="6" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
          </svg>
        </div>
        <div class="stat-info">
          <div class="stat-value">{{ stats.total_groups }}</div>
          <div class="stat-label">总群组数</div>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-icon admin-icon">
          <svg width="24" height="24" viewBox="0 0 16 16" fill="none">
            <path d="M8 1L2 4v4c0 4.5 2.5 7.5 6 9 3.5-1.5 6-4.5 6-9V4L8 1z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <div class="stat-info">
          <div class="stat-value">{{ stats.admin_count }}</div>
          <div class="stat-label">管理员数</div>
        </div>
      </div>
    </div>

    <div class="quick-actions">
      <h2>快捷操作</h2>
      <div class="actions-grid">
        <router-link to="/admin/users" class="action-card">
          <svg width="20" height="20" viewBox="0 0 16 16" fill="none">
            <path d="M5 5m-2 0a2 2 0 104 0a2 2 0 10-4 0M11 5m-2 0a2 2 0 104 0a2 2 0 10-4 0M1 12a4 4 0 018 0M7 12a4 4 0 018 0" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          <span>用户管理</span>
          <p>查看和管理所有用户账户</p>
        </router-link>
        <router-link to="/admin/projects" class="action-card">
          <svg width="20" height="20" viewBox="0 0 16 16" fill="none">
            <path d="M2 3a1 1 0 011-1h10a1 1 0 011 1v10a1 1 0 01-1 1H3a1 1 0 01-1-1V3zM5 6h6M5 9h4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          <span>项目管理</span>
          <p>管理系统中所有项目</p>
        </router-link>
        <router-link to="/admin/settings" class="action-card">
          <svg width="20" height="20" viewBox="0 0 16 16" fill="none">
            <path d="M8 8m-2 0a2 2 0 104 0a2 2 0 10-4 0M8 1v2M8 13v2M1 8h2M13 8h2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          <span>系统设置</span>
          <p>配置系统参数和偏好</p>
        </router-link>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import api from '@/api'
import type { SystemStats } from '@/types'

const stats = ref<SystemStats | null>(null)
const loading = ref(true)

onMounted(async () => {
  try {
    stats.value = await api.admin.getDashboard()
  } catch (err) {
    console.error('Failed to load admin dashboard:', err)
  } finally {
    loading.value = false
  }
})
</script>

<style lang="scss" scoped>
.admin-dashboard {
  max-width: 1200px;
  margin: 0 auto;
  padding: $spacing-6;
}

.page-header {
  margin-bottom: $spacing-6;

  h1 {
    font-size: $font-size-2xl;
    font-weight: $font-weight-bold;
    color: $text-primary;
    margin: 0 0 $spacing-2;
  }

  .page-description {
    color: $text-secondary;
    font-size: $font-size-base;
    margin: 0;
  }
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: $spacing-12;
  color: $text-secondary;

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid $border-color;
    border-top-color: $brand-primary;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: $spacing-4;
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: $spacing-4;
  margin-bottom: $spacing-8;
}

.stat-card {
  display: flex;
  align-items: center;
  gap: $spacing-4;
  padding: $spacing-5;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
  transition: border-color $transition-fast;

  &:hover {
    border-color: $border-color-dark;
  }
}

.stat-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  border-radius: $border-radius-lg;
  flex-shrink: 0;

  &.users-icon {
    background: rgba(59, 130, 246, 0.1);
    color: #3b82f6;
  }
  &.projects-icon {
    background: rgba(16, 133, 72, 0.1);
    color: #108548;
  }
  &.groups-icon {
    background: rgba(171, 97, 0, 0.1);
    color: #ab6100;
  }
  &.admin-icon {
    background: rgba(99, 102, 241, 0.1);
    color: #6366f1;
  }
}

.stat-info {
  min-width: 0;
}

.stat-value {
  font-size: $font-size-2xl;
  font-weight: $font-weight-bold;
  color: $text-primary;
  line-height: 1;
}

.stat-label {
  font-size: $font-size-sm;
  color: $text-secondary;
  margin-top: $spacing-1;
}

.stat-detail {
  font-size: $font-size-xs;
  color: $text-muted;
  margin-top: $spacing-1;
}

.quick-actions {
  h2 {
    font-size: $font-size-lg;
    font-weight: $font-weight-semibold;
    color: $text-primary;
    margin: 0 0 $spacing-4;
  }
}

.actions-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: $spacing-4;
}

.action-card {
  display: flex;
  flex-direction: column;
  gap: $spacing-2;
  padding: $spacing-5;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
  text-decoration: none;
  transition: all $transition-fast;

  &:hover {
    border-color: $brand-primary;
    box-shadow: $shadow-sm;
  }

  svg {
    color: $brand-primary;
  }

  span {
    font-size: $font-size-base;
    font-weight: $font-weight-semibold;
    color: $text-primary;
  }

  p {
    font-size: $font-size-sm;
    color: $text-secondary;
    margin: 0;
  }
}
</style>
