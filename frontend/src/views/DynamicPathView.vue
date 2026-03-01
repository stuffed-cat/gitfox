<template>
  <!-- 根据路由 meta.entityType 渲染对应视图 -->
  <NamespaceView v-if="isNamespace" :path="fullPath" />
  <ProjectView v-else />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import ProjectView from './projects/ProjectView.vue'
import NamespaceView from './namespace/NamespaceView.vue'

const route = useRoute()

const isNamespace = computed(() => {
  const type = route
  .meta.entityType as string | undefined
  return type === 'group' || type === 'user'
})

const fullPath = computed(() => route.meta.fullPath as string || '')
</script>
