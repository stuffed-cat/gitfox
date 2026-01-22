<template>
  <div class="my-groups-page">
    <div class="page-header">
      <h1>群组</h1>
      <router-link to="/groups/new" class="btn btn-primary">新建群组</router-link>
    </div>
    
    <div class="filter-bar">
      <input v-model="searchQuery" type="text" placeholder="搜索群组..." class="search-input" />
    </div>
    
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>
    
    <div v-else-if="groups.length === 0" class="empty-state">
      <h3>暂无群组</h3>
      <p>创建或加入一个群组来开始协作</p>
      <router-link to="/groups/new" class="btn btn-primary">新建群组</router-link>
    </div>
    
    <div v-else class="group-list">
      <div v-for="group in filteredGroups" :key="group.id" class="group-item">
        <div class="group-avatar">{{ group.name.charAt(0).toUpperCase() }}</div>
        <div class="group-info">
          <h3>{{ group.name }}</h3>
          <p>{{ group.description || '暂无描述' }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

const loading = ref(false)
const searchQuery = ref('')
const groups = ref<any[]>([])

const filteredGroups = computed(() => {
  if (!searchQuery.value) return groups.value
  const query = searchQuery.value.toLowerCase()
  return groups.value.filter(g => g.name.toLowerCase().includes(query))
})
</script>

<style lang="scss" scoped>
.my-groups-page {
  padding: $spacing-6;
  max-width: 1200px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: $spacing-6;
  
  h1 {
    font-size: $text-2xl;
    font-weight: 600;
  }
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

.loading-state {
  display: flex;
  justify-content: center;
  padding: $spacing-12;
  
  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid $border-color;
    border-top-color: $color-primary;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }
}

.empty-state {
  text-align: center;
  padding: $spacing-12;
  
  h3 { color: $text-primary; margin-bottom: $spacing-2; }
  p { color: $text-secondary; margin-bottom: $spacing-4; }
}

.group-list {
  .group-item {
    display: flex;
    gap: $spacing-4;
    padding: $spacing-4;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    margin-bottom: $spacing-3;
    
    .group-avatar {
      width: 48px;
      height: 48px;
      border-radius: $radius-md;
      background: $color-primary;
      color: white;
      display: flex;
      align-items: center;
      justify-content: center;
      font-weight: 600;
    }
    
    .group-info {
      h3 { color: $text-primary; margin-bottom: $spacing-1; }
      p { color: $text-secondary; font-size: $text-sm; }
    }
  }
}

@keyframes spin { to { transform: rotate(360deg); } }
</style>
