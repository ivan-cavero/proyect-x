<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useAppStore } from './stores/app'
import { useWebSocket } from './composables/useWebSocket'

const store = useAppStore()
const ws = useWebSocket()
const currentView = ref('overview')

// Auto-refresh every 10s
let refreshInterval: ReturnType<typeof setInterval> | null = null

onMounted(async () => {
  await store.refreshAll()
  refreshInterval = setInterval(() => store.refreshAll(), 10000)
})

onUnmounted(() => {
  if (refreshInterval) clearInterval(refreshInterval)
})

const navItems = [
  { id: 'overview', label: 'Overview', icon: '📊' },
  { id: 'sessions', label: 'Sessions', icon: '📋' },
  { id: 'agents', label: 'Agents', icon: '🤖' },
  { id: 'context', label: 'Context', icon: '🧠' },
  { id: 'events', label: 'Events', icon: '📝' },
  { id: 'config', label: 'Config', icon: '⚙️' },
]

function formatTime(iso: string) {
  return new Date(iso).toLocaleTimeString()
}
</script>

<template>
  <div class="h-screen flex bg-[#0a0a0a] text-[#e5e5e5] antialiased font-sans">

    <!-- ═══ SIDEBAR ═══ -->
    <aside class="bg-[#111111] border-r border-[#1f1f1f] flex flex-col w-64 shrink-0">
      <div class="h-14 border-b border-[#1f1f1f] flex items-center px-5 gap-3">
        <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-green-500 to-emerald-600 flex items-center justify-center text-white font-bold text-sm">X</div>
        <div>
          <div class="text-sm font-semibold text-white">Project-X</div>
          <div class="text-[10px] text-gray-500">v{{ store.health?.version || '?' }}</div>
        </div>
      </div>

      <nav class="flex-1 py-4 px-3 space-y-1">
        <button
          v-for="item in navItems"
          :key="item.id"
          @click="currentView = item.id"
          class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-all"
          :class="currentView === item.id
            ? 'bg-green-500/10 text-green-400'
            : 'text-gray-400 hover:bg-white/5 hover:text-white'"
        >
          <span class="text-lg w-6 text-center">{{ item.icon }}</span>
          <span>{{ item.label }}</span>
        </button>
      </nav>

      <div class="p-4 border-t border-[#1f1f1f] space-y-2">
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full" :class="ws.connected.value ? 'bg-green-500' : 'bg-red-500'"></span>
          <span class="text-xs text-gray-500">{{ ws.connected.value ? 'Connected' : 'Disconnected' }}</span>
        </div>
        <div class="text-[10px] text-gray-600">Uptime: {{ store.uptime }}</div>
        <div class="text-[10px] text-gray-600">Events: {{ ws.events.value.length }}</div>
      </div>
    </aside>

    <!-- ═══ MAIN CONTENT ═══ -->
    <div class="flex-1 flex flex-col min-w-0">
      <header class="h-14 border-b border-[#1f1f1f] bg-[#111111]/50 backdrop-blur-sm flex items-center px-6 justify-between shrink-0">
        <span class="text-sm font-semibold">{{ navItems.find(n => n.id === currentView)?.label }}</span>
        <div class="flex items-center gap-4 text-xs text-gray-500">
          <span v-if="store.loading" class="text-yellow-400">Loading...</span>
          <span v-if="store.error" class="text-red-400">{{ store.error }}</span>
        </div>
      </header>

      <!-- ═══ OVERVIEW ═══ -->
      <div v-if="currentView === 'overview'" class="flex-1 overflow-y-auto px-6 py-6">
        <div class="max-w-5xl space-y-6">
          <!-- Status Cards -->
          <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
            <div class="bg-[#111111] border border-[#1f1f1f] rounded-xl p-4">
              <div class="text-xs text-gray-500 mb-1">Status</div>
              <div class="text-lg font-semibold" :class="store.isHealthy ? 'text-green-400' : 'text-red-400'">
                {{ store.isHealthy ? 'Healthy' : 'Unknown' }}
              </div>
            </div>
            <div class="bg-[#111111] border border-[#1f1f1f] rounded-xl p-4">
              <div class="text-xs text-gray-500 mb-1">Sessions</div>
              <div class="text-lg font-semibold text-white">{{ store.sessions.length }}</div>
            </div>
            <div class="bg-[#111111] border border-[#1f1f1f] rounded-xl p-4">
              <div class="text-xs text-gray-500 mb-1">ASI Score</div>
              <div class="text-lg font-semibold" :class="(store.metrics?.avg_asi_score || 0) >= 80 ? 'text-green-400' : 'text-yellow-400'">
                {{ store.metrics?.avg_asi_score?.toFixed(0) || '100' }}%
              </div>
            </div>
            <div class="bg-[#111111] border border-[#1f1f1f] rounded-xl p-4">
              <div class="text-xs text-gray-500 mb-1">Tokens Used</div>
              <div class="text-lg font-semibold text-white">{{ (store.metrics?.total_tokens || 0).toLocaleString() }}</div>
            </div>
          </div>

          <!-- WebSocket Events -->
          <div class="bg-[#111111] border border-[#1f1f1f] rounded-xl">
            <div class="px-4 py-3 border-b border-[#1f1f1f] flex items-center justify-between">
              <span class="text-sm font-semibold">Live Events</span>
              <button @click="ws.clearEvents()" class="text-xs text-gray-500 hover:text-white">Clear</button>
            </div>
            <div class="max-h-64 overflow-y-auto">
              <div v-if="ws.events.value.length === 0" class="p-4 text-center text-gray-600 text-sm">
                Waiting for events...
              </div>
              <div v-for="event in ws.events.value.slice().reverse()" :key="event.id"
                class="px-4 py-2 border-b border-[#1a1a1a] text-xs font-mono">
                <span class="text-gray-600">{{ formatTime(event.timestamp) }}</span>
                <span class="text-blue-400 ml-2">{{ event.kind }}</span>
                <span class="text-gray-500 ml-2">{{ event.source }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ═══ SESSIONS ═══ -->
      <div v-else-if="currentView === 'sessions'" class="flex-1 overflow-y-auto px-6 py-6">
        <div class="max-w-3xl space-y-4">
          <h2 class="text-lg font-semibold">Sessions</h2>
          <div v-if="store.sessions.length === 0" class="text-center py-16 text-gray-500">
            <div class="text-3xl mb-2 opacity-40">📋</div>
            <div class="text-sm">No active sessions</div>
            <div class="text-xs text-gray-600 mt-1">Run a goal with the CLI to start a session</div>
          </div>
          <div v-for="session in store.sessions" :key="session.id"
            class="bg-[#111111] border border-[#1f1f1f] rounded-xl p-4">
            <div class="flex items-center justify-between">
              <div>
                <div class="font-medium text-white">{{ session.goal }}</div>
                <div class="text-xs text-gray-500 mt-1">Phase: {{ session.phase }} · Iteration: {{ session.iteration }}</div>
              </div>
              <span class="text-xs px-2 py-1 rounded-full"
                :class="session.status === 'active' ? 'bg-green-500/10 text-green-400' : 'bg-gray-500/10 text-gray-400'">
                {{ session.status }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- ═══ AGENTS ═══ -->
      <div v-else-if="currentView === 'agents'" class="flex-1 overflow-y-auto px-6 py-6">
        <div class="max-w-3xl space-y-4">
          <h2 class="text-lg font-semibold">Agent Roles</h2>
          <div class="grid grid-cols-2 lg:grid-cols-3 gap-3">
            <div v-for="role in ['Architect', 'Coder', 'Reviewer', 'Security', 'Tester', 'Researcher']" :key="role"
              class="bg-[#111111] border border-[#1f1f1f] rounded-xl p-4">
              <div class="font-medium text-white mb-1">{{ role }}</div>
              <div class="text-xs text-gray-500">
                {{ { Architect: 'System design, ADRs', Coder: 'Code generation', Reviewer: 'Code review', Security: 'Vulnerability scan', Tester: 'Test generation', Researcher: 'Web research' }[role] }}
              </div>
              <div class="mt-2 text-xs text-gray-600">Model: configured in forge.toml</div>
            </div>
          </div>
        </div>
      </div>

      <!-- ═══ CONTEXT ═══ -->
      <div v-else-if="currentView === 'context'" class="flex-1 overflow-y-auto px-6 py-6">
        <div class="max-w-3xl space-y-4">
          <h2 class="text-lg font-semibold">Context Management</h2>
          <div class="bg-[#111111] border border-[#1f1f1f] rounded-xl p-4">
            <div class="text-xs text-gray-500 mb-3">Context Budget — Active Sessions</div>
            <div v-if="store.sessions.length === 0" class="text-sm text-gray-600">No active sessions</div>
            <div v-else class="space-y-3">
              <div v-for="session in store.sessions" :key="session.id" class="space-y-1">
                <div class="text-xs text-white">{{ session.goal }}</div>
                <div class="h-2 bg-[#1a1a1a] rounded-full overflow-hidden">
                  <div class="h-full bg-green-500 rounded-full transition-all" :style="{ width: '0%' }"></div>
                </div>
                <div class="text-[10px] text-gray-600">Phase: {{ session.phase }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ═══ EVENTS ═══ -->
      <div v-else-if="currentView === 'events'" class="flex-1 overflow-y-auto px-6 py-6">
        <div class="max-w-3xl space-y-4">
          <div class="flex items-center justify-between">
            <h2 class="text-lg font-semibold">Event Log</h2>
            <div class="flex items-center gap-2">
              <span class="w-2 h-2 rounded-full" :class="ws.connected.value ? 'bg-green-500' : 'bg-red-500'"></span>
              <button @click="ws.clearEvents()" class="text-xs text-gray-500 hover:text-white">Clear</button>
            </div>
          </div>
          <div class="bg-[#111111] border border-[#1f1f1f] rounded-xl">
            <div v-if="ws.events.value.length === 0" class="p-8 text-center text-gray-600 text-sm">
              Waiting for events from EventBus...
            </div>
            <div v-for="event in ws.events.value.slice().reverse()" :key="event.id"
              class="px-4 py-3 border-b border-[#1f1f1f] text-xs font-mono">
              <div class="flex items-start gap-3">
                <span class="text-gray-600 shrink-0 w-20">{{ formatTime(event.timestamp) }}</span>
                <span class="text-blue-400 shrink-0 w-40">{{ event.kind }}</span>
                <span class="text-gray-500">{{ event.source }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ═══ CONFIG ═══ -->
      <div v-else-if="currentView === 'config'" class="flex-1 overflow-y-auto px-6 py-6">
        <div class="max-w-2xl space-y-6">
          <h2 class="text-lg font-semibold">Configuration</h2>

          <div class="bg-[#111111] border border-[#1f1f1f] rounded-xl p-4">
            <div class="text-xs text-gray-500 mb-2">Backend API</div>
            <div class="text-sm text-white">http://localhost:8080</div>
            <div class="text-xs text-gray-600 mt-1">Configure providers via forge.toml in your project</div>
          </div>

          <div class="bg-[#111111] border border-[#1f1f1f] rounded-xl p-4">
            <div class="text-xs text-gray-500 mb-2">Providers</div>
            <div class="text-xs text-gray-400">API keys are read from environment variables</div>
            <div class="mt-2 space-y-1 text-xs font-mono text-gray-500">
              <div>NAN_API_KEY=env:NAN_API_KEY</div>
              <div>OPENAI_API_KEY=env:OPENAI_API_KEY</div>
              <div>ANTHROPIC_API_KEY=env:ANTHROPIC_API_KEY</div>
              <div>GEMINI_API_KEY=env:GEMINI_API_KEY</div>
            </div>
          </div>

          <div class="bg-[#111111] border border-[#1f1f1f] rounded-xl p-4">
            <div class="text-xs text-gray-500 mb-2">WebSocket</div>
            <div class="flex items-center gap-2">
              <span class="w-2 h-2 rounded-full" :class="ws.connected.value ? 'bg-green-500' : 'bg-red-500'"></span>
              <span class="text-sm" :class="ws.connected.value ? 'text-green-400' : 'text-red-400'">
                {{ ws.connected.value ? 'Connected to EventBus' : 'Disconnected' }}
              </span>
            </div>
          </div>
        </div>
      </div>

    </div>
  </div>
</template>
