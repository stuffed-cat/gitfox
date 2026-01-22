<template>
  <div class="explore-groups-page">
    <div class="page-header">
      <h1>探索群组</h1>
    </div>
    
    <div class="filter-bar">
      <input v-model="searchQuery" type="text" placeholder="搜索群组..." class="search-input" />
    </div>
    
    <div class="empty-state">
      <h3>暂无公开群组</h3>
      <p>创建第一个公开群组来开始协作</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const searchQuery = ref('')
</script>

<style lang="scss" scoped>
.explore-groups-page {
  padding: $spacing-6;
  max-width: 1200px;
  margin: 0 auto;
}

.page-header {
  margin-bottom: $spacing-6;
  h1 { font-size: $text-2xl; font-weight: 600; }
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

.empty-state {
  text-align: center;
  padding: $spacing-12;
  h3 { color: $text-primary; margin-bottom: $spacing-2; }
  p { color: $text-secondary; }
}
</style>
