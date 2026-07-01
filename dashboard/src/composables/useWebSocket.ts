/**
 * useWebSocket — Real-time event streaming from backend EventBus.
 */

import { ref, onUnmounted } from 'vue'

export interface SystemEvent {
  id: string
  timestamp: string
  kind: string
  source: string
  metadata: Record<string, unknown>
}

export function useWebSocket() {
  const isConnected = ref(false)
  const events = ref<SystemEvent[]>([])
  const maxEvents = 200

  // Mutable state wrapped in refs so no `let` is needed
  const wsRef = ref<WebSocket | null>(null)
  const reconnectTimeoutRef = ref<ReturnType<typeof setTimeout> | null>(null)
  const reconnectDelayRef = ref(1000)

  function connect() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const host = window.location.host
    const url = `${protocol}//${host}/ws/global`

    const ws = new WebSocket(url)
    wsRef.value = ws

    ws.onopen = () => {
      isConnected.value = true
      reconnectDelayRef.value = 1000
    }

    ws.onmessage = (message) => {
      try {
        const data = JSON.parse(message.data) as SystemEvent
        events.value = [...events.value, data]
        if (events.value.length > maxEvents) {
          events.value = events.value.slice(-maxEvents)
        }
      } catch (parseError) {
        console.warn('[WS] Failed to parse event:', parseError)
      }
    }

    ws.onclose = () => {
      isConnected.value = false

      reconnectTimeoutRef.value = setTimeout(() => {
        reconnectDelayRef.value = Math.min(reconnectDelayRef.value * 2, 30000)
        connect()
      }, reconnectDelayRef.value)
    }

    ws.onerror = (connectionError) => {
      console.error('[WS] Error:', connectionError)
    }
  }

  function disconnect() {
    if (reconnectTimeoutRef.value) {
      clearTimeout(reconnectTimeoutRef.value)
      reconnectTimeoutRef.value = null
    }
    if (wsRef.value) {
      wsRef.value.close()
      wsRef.value = null
    }
  }

  function clearEvents() {
    events.value = []
  }

  // Auto-connect
  connect()

  onUnmounted(() => {
    disconnect()
  })

  return {
    connected: isConnected,
    events,
    disconnect,
    clearEvents,
  }
}