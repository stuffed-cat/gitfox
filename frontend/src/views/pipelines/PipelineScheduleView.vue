<template>
  <div class="schedule-page">
    <div class="empty-state">
      <div class="empty-icon">
        <svg viewBox="0 0 100 100" width="100" height="100" fill="none">
          <circle cx="50" cy="50" r="40" fill="#f0ede8"/>
          <circle cx="50" cy="50" r="30" stroke="#c17d3c" stroke-width="2.5" fill="none"/>
          <path d="M50 28v22l12 8" stroke="#c17d3c" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M42 16a4 4 0 018 0" stroke="#c17d3c" stroke-width="2" stroke-linecap="round"/>
          <path d="M35 82a4 4 0 01-4-4" stroke="#c17d3c" stroke-width="2" stroke-linecap="round"/>
        </svg>
      </div>
      <h2>流水线计划</h2>
      <p>
        计划流水线会定期自动启动，例如每天或每周。流水线：
      </p>
      <ul class="feature-list">
        <li>针对特定分支或标签运行。</li>
        <li>可以有自定义的 CI/CD 变量。</li>
        <li>使用与计划所有者相同的项目权限运行。</li>
      </ul>
      <p class="doc-link">
        在<a href="#" @click.prevent>计划流水线文档</a>中了解更多信息。
      </p>
      <button class="btn btn-primary" @click="showCreateModal = true">
        创建新的流水线计划
      </button>
    </div>

    <!-- 创建计划弹窗（占位） -->
    <div v-if="showCreateModal" class="modal-overlay" @click.self="showCreateModal = false">
      <div class="modal">
        <div class="modal-header">
          <h3>创建流水线计划</h3>
          <button class="btn-close" @click="showCreateModal = false">
            <svg viewBox="0 0 16 16" width="16" height="16">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="2" fill="none"/>
            </svg>
          </button>
        </div>
        <div class="modal-body">
          <p style="color: var(--text-secondary, #666); font-size: 14px;">
            流水线计划功能正在开发中，请稍后再试。
          </p>
        </div>
        <div class="modal-footer">
          <button class="btn btn-primary" @click="showCreateModal = false">关闭</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { Project } from '@/types'

defineProps<{ project?: Project }>()
const showCreateModal = ref(false)
</script>

<style lang="scss" scoped>
.schedule-page { padding: 0; }

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 24px;
  text-align: center;
  max-width: 600px;
  margin: 0 auto;

  .empty-icon { margin-bottom: 20px; }

  h2 {
    font-size: 22px;
    font-weight: 600;
    color: $text-primary;
    margin: 0 0 12px;
  }

  p {
    font-size: 14px;
    color: $text-secondary;
    margin: 0 0 8px;
    line-height: 1.6;
    text-align: left;
    width: 100%;
    max-width: 420px;
  }

  .doc-link {
    margin: 12px 0 24px;
    a {
      color: $primary-color;
      text-decoration: none;
      &:hover { text-decoration: underline; }
    }
  }
}

.feature-list {
  list-style: disc;
  text-align: left;
  width: 100%;
  max-width: 420px;
  margin: 0 0 8px;
  padding-left: 20px;

  li {
    font-size: 14px;
    color: $text-secondary;
    line-height: 1.8;
  }
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: $bg-primary;
  border-radius: 8px;
  width: 480px;
  max-width: 90vw;
  box-shadow: 0 8px 32px rgba(0,0,0,0.12);

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid $border-color;

    h3 { margin: 0; font-size: 16px; font-weight: 600; }
  }

  .modal-body { padding: 20px; }

  .modal-footer {
    padding: 12px 20px;
    border-top: 1px solid $border-color;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
}

.btn-close {
  background: none; border: none; cursor: pointer;
  color: $text-muted; padding: 4px; border-radius: 4px;
  &:hover { background: $bg-secondary; }
}
</style>
