import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface Provider {
  id: string
  name: string
  base_url: string
  api_key: string
  models: string[]
}

export interface Agent {
  id: string
  model: string
  status: 'running' | 'idle' | 'error'
  asi_score: number
  context_pressure: number
  tokens_used: number
  current_action: string
}

export interface Session {
  id: string
  project_id: string
  goal: string
  phase: string
  iteration: number
  agents: Agent[]
  started_at: string
}

export interface SystemMetrics {
  total_tokens: number
  active_sessions: number
  avg_asi: number
  context_pressure: number
  uptime_seconds: number
}

export const useAppStore = defineStore('app', () => {
  // ─── State ─────────────────────────────────────────────
  const connected = ref(false)
  const currentView = ref('status')
  const providers = ref<Provider[]>([])
  const sessions = ref<Session[]>([])
  const metrics = ref<SystemMetrics>({
    total_tokens: 0,
    active_sessions: 0,
    avg_asi: 100,
    context_pressure: 0,
    uptime_seconds: 0,
  })
  const logs = ref<Array<{ time: string; message: string; type: string }>>([])
  const ws = ref<WebSocket | null>(null)

  // ─── Computed ──────────────────────────────────────────
  const totalTokens = computed(() => {
    return sessions.value.reduce((sum, s) => {
      return sum + s.agents.reduce((a, agent) => a + agent.tokens_used, 0)
    }, 0)
  })

  const runningAgents = computed(() => {
    return sessions.value.reduce((count, s) => {
      return count + s.agents.filter(a => a.status === 'running').length
    }, 0)
  })

  // ─── Actions ───────────────────────────────────────────
  function connectWebSocket() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    ws.value = new WebSocket(`${protocol}//${window.location.host}/ws/global`)

    ws.value.onopen = () => {
      connected.value = true
      addLog('system', 'WebSocket connected')
    }

    ws.value.onclose = () => {
      connected.value = false
      addLog('system', 'WebSocket disconnected, reconnecting...')
      setTimeout(connectWebSocket, 3000)
    }

    ws.value.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)
        handleEvent(data)
      } catch {}
    }
  }

  function handleEvent(data: any) {
    if (data.type === 'metric_update') {
      Object.assign(metrics.value, data.payload)
    } else if (data.type === 'session_update') {
      // Update session
    } else if (data.type === 'agent_update') {
      // Update agent
    }
    addLog('system', JSON.stringify(data.type || 'event'))
  }

  function addLog(type: string, message: string) {
    const time = new Date().toLocaleTimeString('en-US', { hour12: false })
    logs.value.unshift({ time, message, type })
    if (logs.value.length > 100) logs.value.pop()
  }

  function addProvider(provider: Provider) {
    providers.value.push(provider)
  }

  function removeProvider(id: string) {
    providers.value = providers.value.filter(p => p.id !== id)
  }

  return {
    connected,
    currentView,
    providers,
    sessions,
    metrics,
    logs,
    ws,
    totalTokens,
    runningAgents,
    connectWebSocket,
    addLog,
    addProvider,
    removeProvider,
  }
})