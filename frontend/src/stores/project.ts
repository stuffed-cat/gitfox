import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/api'
import type { Project, ProjectStats, CreateProjectRequest } from '@/types'

export const useProjectStore = defineStore('project', () => {
  const projects = ref<Project[]>([])
  const currentProject = ref<Project | null>(null)
  const projectStats = ref<ProjectStats | null>(null)
  const loading = ref(false)

  async function fetchProjects(page = 1, perPage = 20) {
    loading.value = true
    try {
      projects.value = await api.projects.list(page, perPage)
    } finally {
      loading.value = false
    }
  }

  async function fetchProject(slug: string) {
    loading.value = true
    try {
      currentProject.value = await api.projects.get(slug)
      projectStats.value = await api.projects.getStats(slug)
    } finally {
      loading.value = false
    }
  }

  async function createProject(data: CreateProjectRequest) {
    const project = await api.projects.create(data)
    projects.value.unshift(project)
    return project
  }

  async function updateProject(slug: string, data: Partial<CreateProjectRequest>) {
    const project = await api.projects.update(slug, data)
    currentProject.value = project
    const index = projects.value.findIndex(p => p.slug === slug)
    if (index !== -1) {
      projects.value[index] = project
    }
    return project
  }

  async function deleteProject(slug: string) {
    await api.projects.delete(slug)
    projects.value = projects.value.filter(p => p.slug !== slug)
    if (currentProject.value?.slug === slug) {
      currentProject.value = null
    }
  }

  return {
    projects,
    currentProject,
    projectStats,
    loading,
    fetchProjects,
    fetchProject,
    createProject,
    updateProject,
    deleteProject
  }
})
