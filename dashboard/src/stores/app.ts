/**
 * App store — all state centralized from AppData.
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useApi, type HealthStatus, type Project, type ProjectConfig } from '../composables/useApi'

export const useAppStore = defineStore('app', () => {
  const api = useApi()

  // Health
  const health = ref<HealthStatus | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  // Projects
  const projects = ref<Project[]>([])
  const activeProject = ref<Project | null>(null)
  const activeConfig = ref<ProjectConfig | null>(null)

  // Computed
  const isHealthy = computed(() => health.value?.status === 'ok')
  const version = computed(() => health.value?.version || '')

  const uptime = computed(() => {
    if (!health.value) return '0m'
    const s = health.value.uptime_seconds
    if (s < 3600) return `${Math.floor(s / 60)}m`
    return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`
  })

  // Actions
async function refreshAll() {
    isLoading.value = true
    error.value = null
    try {
      health.value = await api.getHealth()
      projects.value = await api.getProjects()
    } catch (caughtError: any) {
      error.value = caughtError.message
    }
    isLoading.value = false
  }

  async function selectProject(id: string) {
    try {
      activeProject.value = await api.getProject(id)
      activeConfig.value = await api.getProjectConfig(id)
    } catch (caughtError: any) {
      error.value = caughtError.message
    }
  }

  function clearActiveProject() {
    activeProject.value = null
    activeConfig.value = null
  }

  async function createProject(name: string, description = '') {
    const project = await api.createProject(name, description)
    projects.value = [...projects.value, project]
    return project
  }

  async function updateProject(id: string, data: { name?: string; description?: string; forge_toml?: string }) {
    const updated = await api.updateProject(id, data)
    const projectIndex = projects.value.findIndex(project => project.id === id)
    if (projectIndex >= 0) projects.value[projectIndex] = updated
    if (activeProject.value?.id === id) activeProject.value = updated
    return updated
  }

  async function deleteProject(id: string) {
    await api.deleteProject(id)
    projects.value = projects.value.filter(project => project.id !== id)
    if (activeProject.value?.id === id) clearActiveProject()
  }

  async function saveProjectConfig(id: string, config: string) {
    const result = await api.updateProjectConfig(id, config)
    activeConfig.value = result
    if (activeProject.value) {
      activeProject.value.forge_toml = config
    }
    return result
  }

  return {
    health, isLoading, error,
    projects, activeProject, activeConfig,
    isHealthy, version, uptime,
    refreshAll, selectProject, clearActiveProject,
    createProject, updateProject, deleteProject, saveProjectConfig,
  }
})