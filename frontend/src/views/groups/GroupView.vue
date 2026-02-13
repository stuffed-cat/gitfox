<template>
  <div class="group-page">
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>
    
    <div v-else-if="error" class="error-state">
      <h2>加载群组失败</h2>
      <p>{{ error }}</p>
      <router-link to="/dashboard/groups" class="btn btn-primary">返回群组列表</router-link>
    </div>
    
    <template v-else-if="group">
      <div class="group-content">
        <router-view :group="group" :subgroups="subgroups" :projects="projects" :members="members" @refresh="loadGroup" />
      </div>
    </template>
    
    <div v-else class="empty-state">
      <h3>群组不存在</h3>
      <router-link to="/" class="btn btn-primary">返回首页</router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { api } from '@/api'
import type { Group, GroupMember, Project } from '@/types'

const route = useRoute()

const group = ref<Group | null>(null)
const subgroups = ref<Group[]>([])
const projects = ref<Project[]>([])
const members = ref<GroupMember[]>([])
const loading = ref(true)
const error = ref('')

async function loadGroup() {
  const groupPath = route.params.groupPath as string
  if (!groupPath) return
  
  loading.value = true
  error.value = ''
  
  try {
    const [groupData, subgroupsData, projectsData, membersData] = await Promise.all([
      api.groups.get(groupPath),
      api.groups.listSubgroups(groupPath).catch(() => []),
      api.groups.listProjects(groupPath).catch(() => []),
      api.groups.listMembers(groupPath).catch(() => []),
    ])
    
    group.value = groupData
    subgroups.value = subgroupsData
    projects.value = projectsData
    members.value = membersData
  } catch (e: any) {
    error.value = e.response?.data?.message || e.message || '加载群组失败'
    group.value = null
  } finally {
    loading.value = false
  }
}

watch(
  () => route.params.groupPath,
  () => loadGroup(),
  { immediate: true }
)
</script>

<style lang="scss" scoped>
.group-page {
  min-height: 100%;
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

.error-state {
  text-align: center;
  padding: $spacing-12;
  
  h2 { color: $text-primary; margin-bottom: $spacing-2; }
  p { color: $text-secondary; margin-bottom: $spacing-4; }
}

.empty-state {
  text-align: center;
  padding: $spacing-12;
  
  h3 { color: $text-primary; margin-bottom: $spacing-4; }
}

@keyframes spin { to { transform: rotate(360deg); } }
</style>
