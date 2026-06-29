/**
 * useWebSocket — Real-time event streaming from backend EventBus.
 *
 * Connects to /ws/global and receives all system events as JSON.
 * Auto-reconnects on disconnect with exponential backoff.
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
  const connected = ref(false)
  const events = ref<SystemEvent[]>([])
  const maxEvents = 200

  let ws: WebSocket | null = null
  let reconnectTimeout: ReturnType<typeof setTimeout> | null = null
  let reconnectDelay = 1000

  function connect() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const host = window.location.host
    const url = `${protocol}//${host}/ws/global`

    ws = new WebSocket(url)

    ws.onopen = () => {
      connected.value = true
      reconnectDelay = 1000
      console.log('[WS] Connected')
    }

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as SystemEvent
        events.value.push(data)
        // Keep only last N events
        if (events.value.length > maxEvents) {
          events.value = events.value.slice(-maxEvents)
        }
      } catch (e) {
        console.warn('[WS] Failed to parse event:', e)
      }
    }

    ws.onclose = () => {
      connected.value = false
      console.log(`[WS] Disconnected, reconnecting in ${reconnectDelay}ms...`)

      reconnectTimeout = setTimeout(() => {
        reconnectDelay = Math.min(reconnectDelay * 2, 30000)
        connect()
      }, reconnectDelay)
    }

    ws.onerror = (error) => {
      console.error('[WS] Error:', error)
    }
  }

  function disconnect() {
    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout)
    }
    if (ws) {
      ws.close()
      ws = null
    }
  }

  function clearEvents() {
    events.value = []
  }

  // Auto-connect
  connect()

  // Cleanup on unmount
  onUnmounted(() => {
    disconnect()
  })

  return {
    connected,
    events,
    disconnect,
    clearEvents,
  }
}
