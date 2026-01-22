<template>
  <div class="my-activity-page">
    <div class="page-header">
      <h1>活动</h1>
    </div>
    
    <div class="empty-state">
      <h3>暂无活动</h3>
      <p>您的最近活动将显示在这里</p>
    </div>
  </div>
</template>

<script setup lang="ts">
</script>

<style lang="scss" scoped>
.my-activity-page {
  padding: $spacing-6;
  max-width: 1200px;
  margin: 0 auto;
}

.page-header {
  margin-bottom: $spacing-6;
  h1 { font-size: $text-2xl; font-weight: 600; }
}

.empty-state {
  text-align: center;
  padding: $spacing-12;
  h3 { color: $text-primary; margin-bottom: $spacing-2; }
  p { color: $text-secondary; }
}
</style>
