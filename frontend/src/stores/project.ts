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

  // 通过 namespace/name 获取项目
  async function fetchProject(namespace: string, name: string) {
    loading.value = true
    try {
      const path = { namespace, project: name }
      currentProject.value = await api.projects.get(path)
      if (currentProject.value) {
        projectStats.value = await api.projects.getStats(path)
      }
    } catch (error) {
      currentProject.value = null
      projectStats.value = null
      console.error('Failed to fetch project:', error)
    } finally {
      loading.value = false
    }
  }

  async function createProject(data: CreateProjectRequest) {
    const project = await api.projects.create(data)
    projects.value.unshift(project)
    return project
  }

  async function updateProject(namespace: string, name: string, data: Partial<CreateProjectRequest>) {
    const project = await api.projects.update({ namespace, project: name }, data)
    currentProject.value = project
    const index = projects.value.findIndex(p => p.name === name && p.owner_name === namespace)
    if (index !== -1) {
      projects.value[index] = project
    }
    return project
  }

  async function deleteProject(namespace: string, name: string) {
    await api.projects.delete({ namespace, project: name })
    projects.value = projects.value.filter(p => !(p.name === name && p.owner_name === namespace))
    if (currentProject.value?.name === name && currentProject.value?.owner_name === namespace) {
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
