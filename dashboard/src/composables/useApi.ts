/**
 * useApi — Backend API communication composable.
 *
 * All API calls go through the backend server, never directly to LLM providers.
 * The Vite dev server proxies /api/* to localhost:8080.
 */

const API_BASE = '/api'

export interface HealthStatus {
  status: string
  version: string
  uptime_seconds: number
}

export interface Project {
  id: string
  name: string
  created_at: string
}

export interface Session {
  id: string
  project_id: string
  status: string
  goal: string
  phase: string
  iteration: number
}

export interface TokenMetrics {
  total_input: number
  total_output: number
  total_tokens: number
  by_provider: Record<string, number>
  by_model: Record<string, number>
}

export interface ContextMetrics {
  avg_pressure: number
  max_pressure: number
  total_compressions: number
  active_sessions: number
}

export interface MetricsSummary {
  version: string
  uptime_seconds: number
  active_sessions: number
  total_tokens: number
  avg_asi_score: number
}

async function apiGet<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`)
  if (!res.ok) throw new Error(`API ${res.status}: ${res.statusText}`)
  return res.json()
}

async function apiPost<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  if (!res.ok) throw new Error(`API ${res.status}: ${res.statusText}`)
  return res.json()
}

export function useApi() {
  const getHealth = () => apiGet<HealthStatus>('/health')
  const getProjects = () => apiGet<Project[]>('/projects')
  const createProject = (name: string) => apiPost<Project>('/projects', { name })
  const getSessions = () => apiGet<Session[]>('/sessions')
  const getTokenMetrics = () => apiGet<TokenMetrics>('/metrics/tokens')
  const getContextMetrics = () => apiGet<ContextMetrics>('/metrics/context')
  const getMetricsSummary = () => apiGet<MetricsSummary>('/metrics/summary')

  return {
    getHealth,
    getProjects,
    createProject,
    getSessions,
    getTokenMetrics,
    getContextMetrics,
    getMetricsSummary,
  }
}
