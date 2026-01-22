<template>
  <div class="my-todos-page">
    <div class="page-header">
      <h1>待办事项</h1>
      <button class="btn btn-outline" disabled>全部标记为已完成</button>
    </div>
    
    <div class="tabs">
      <button :class="['tab', { active: activeTab === 'pending' }]" @click="activeTab = 'pending'">
        待处理 <span class="count">0</span>
      </button>
      <button :class="['tab', { active: activeTab === 'done' }]" @click="activeTab = 'done'">
        已完成
      </button>
    </div>
    
    <div class="empty-state">
      <h3>一切就绪！</h3>
      <p>您没有待处理的待办事项</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const activeTab = ref('pending')
</script>

<style lang="scss" scoped>
.my-todos-page {
  padding: $spacing-6;
  max-width: 1200px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: $spacing-6;
  h1 { font-size: $text-2xl; font-weight: 600; }
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
    display: flex;
    align-items: center;
    gap: $spacing-2;
    
    &:hover { color: $text-primary; }
    &.active { color: $text-primary; border-bottom-color: $color-primary; }
    
    .count {
      background: $bg-tertiary;
      padding: 2px 6px;
      border-radius: $radius-full;
      font-size: $text-xs;
    }
  }
}

.empty-state {
  text-align: center;
  padding: $spacing-12;
  h3 { color: $text-primary; margin-bottom: $spacing-2; }
  p { color: $text-secondary; }
}
</style>
