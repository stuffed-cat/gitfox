<template>
  <!-- 根据 entityType 渲染对应成员视图 -->
  <GroupMembersView v-if="entityType === 'group'" />
  <ProjectMembersSettingsView v-else :project="project" />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import GroupMembersView from './groups/GroupMembersView.vue'
import ProjectMembersSettingsView from './projects/settings/ProjectMembersSettingsView.vue'
import type { Project } from '@/types'

const props = defineProps<{
  project?: Project
}>()

const route = useRoute()

const entityType = computed(() => route.meta.entityType as string | undefined)
</script>
