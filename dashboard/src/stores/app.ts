/**
 * App store — global application state.
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useApi, type HealthStatus, type Session, type MetricsSummary } from '../composables/useApi'

export const useAppStore = defineStore('app', () => {
  const api = useApi()

  // State
  const health = ref<HealthStatus | null>(null)
  const sessions = ref<Session[]>([])
  const metrics = ref<MetricsSummary | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Computed
  const isHealthy = computed(() => health.value?.status === 'ok')
  const uptime = computed(() => {
    if (!health.value) return '0m'
    const mins = Math.floor(health.value.uptime_seconds / 60)
    if (mins < 60) return `${mins}m`
    const hours = Math.floor(mins / 60)
    return `${hours}h ${mins % 60}m`
  })

  // Actions
  async function refreshHealth() {
    try {
      health.value = await api.getHealth()
    } catch (e: any) {
      health.value = null
      error.value = e.message
    }
  }

  async function refreshSessions() {
    try {
      sessions.value = await api.getSessions()
    } catch (e: any) {
      error.value = e.message
    }
  }

  async function refreshMetrics() {
    try {
      metrics.value = await api.getMetricsSummary()
    } catch (e: any) {
      error.value = e.message
    }
  }

  async function refreshAll() {
    loading.value = true
    error.value = null
    await Promise.all([refreshHealth(), refreshSessions(), refreshMetrics()])
    loading.value = false
  }

  return {
    health,
    sessions,
    metrics,
    loading,
    error,
    isHealthy,
    uptime,
    refreshHealth,
    refreshSessions,
    refreshMetrics,
    refreshAll,
  }
})
