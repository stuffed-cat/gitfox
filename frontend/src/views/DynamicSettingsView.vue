<template>
  <!-- 根据 entityType 渲染对应设置视图 -->
  <GroupSettingsView v-if="entityType === 'group'" />
  <ProjectGeneralSettingsView v-else :project="project" />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import GroupSettingsView from './groups/GroupSettingsView.vue'
import ProjectGeneralSettingsView from './projects/settings/ProjectGeneralSettingsView.vue'
import type { Project } from '@/types'

const props = defineProps<{
  project?: Project
}>()

const route = useRoute()

const entityType = computed(() => route.meta.entityType as string | undefined)
</script>
