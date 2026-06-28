import { ref } from 'vue'

const API_BASE = 'http://localhost:8080'

export function useApi() {
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function get<T>(endpoint: string): Promise<T | null> {
    try {
      loading.value = true
      const res = await fetch(`${API_BASE}${endpoint}`)
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const data = await res.json() as T
      return data
    } catch (e: any) {
      error.value = e.message
      return null
    } finally {
      loading.value = false
    }
  }

  async function post<T>(endpoint: string, body: unknown): Promise<T | null> {
    try {
      loading.value = true
      const res = await fetch(`${API_BASE}${endpoint}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const data = await res.json() as T
      return data
    } catch (e: any) {
      error.value = e.message
      return null
    } finally {
      loading.value = false
    }
  }

  return { loading, error, get, post }
}