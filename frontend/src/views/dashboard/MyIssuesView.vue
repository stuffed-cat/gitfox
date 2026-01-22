<template>
  <div class="my-issues-page">
    <div class="page-header">
      <h1>议题</h1>
    </div>
    
    <div class="tabs">
      <button :class="['tab', { active: activeTab === 'assigned' }]" @click="activeTab = 'assigned'">
        分配给我
      </button>
      <button :class="['tab', { active: activeTab === 'created' }]" @click="activeTab = 'created'">
        我创建的
      </button>
    </div>
    
    <div class="empty-state">
      <h3>暂无议题</h3>
      <p>当有议题分配给您时，它们会显示在这里</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const activeTab = ref('assigned')
</script>

<style lang="scss" scoped>
.my-issues-page {
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
  padding: $spacing-12;
  h3 { color: $text-primary; margin-bottom: $spacing-2; }
  p { color: $text-secondary; }
}
</style>
