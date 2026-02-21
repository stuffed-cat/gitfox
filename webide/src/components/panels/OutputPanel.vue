<template>
  <div class="output-panel">
    <div class="output-list">
      <div v-if="logs.length === 0" class="empty-state">
        暂无输出
      </div>
      <div v-for="(log, index) in logs" :key="index" class="output-line">
        {{ log }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  logs: string[]
}>()
</script>

<style lang="scss" scoped>
.output-panel {
  height: 100%;
  overflow-y: auto;
  padding: 8px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
}

.empty-state {
  color: var(--ide-text-muted);
  text-align: center;
  padding: 20px;
}

.output-line {
  padding: 2px 0;
  color: var(--ide-text);
}
</style>
