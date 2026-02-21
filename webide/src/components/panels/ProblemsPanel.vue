<template>
  <div class="problems-panel">
    <div v-if="problems.length === 0" class="empty-state">
      未检测到问题
    </div>
    <div v-else class="problems-list">
      <div 
        v-for="(problem, index) in problems" 
        :key="index"
        class="problem-item"
        :class="problem.severity"
      >
        <span class="severity-icon">
          <svg v-if="problem.severity === 'error'" viewBox="0 0 16 16" fill="currentColor">
            <circle cx="8" cy="8" r="6"/>
          </svg>
          <svg v-else viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 1l7 14H1L8 1z"/>
          </svg>
        </span>
        <span class="file">{{ problem.file }}</span>
        <span class="location">:{{ problem.line }}</span>
        <span class="message">{{ problem.message }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  problems: Array<{
    file: string
    line: number
    message: string
    severity: string
  }>
}>()
</script>

<style lang="scss" scoped>
.problems-panel {
  height: 100%;
  overflow-y: auto;
  padding: 8px;
}

.empty-state {
  color: var(--ide-text-muted);
  text-align: center;
  padding: 20px;
  font-size: 12px;
}

.problem-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  font-size: 12px;
  cursor: pointer;
  
  &:hover {
    background: var(--ide-surface-hover);
  }
  
  &.error {
    .severity-icon {
      color: var(--ide-danger);
    }
  }
  
  &.warning {
    .severity-icon {
      color: var(--ide-warning);
    }
  }
}

.severity-icon {
  width: 14px;
  height: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  
  svg {
    width: 10px;
    height: 10px;
  }
}

.file {
  color: var(--ide-primary);
}

.location {
  color: var(--ide-text-muted);
}

.message {
  color: var(--ide-text);
  flex: 1;
}
</style>
