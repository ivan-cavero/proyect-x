<script setup lang="ts">
import { useAppStore } from '../stores/app'
import AgentTable from './AgentTable.vue'
import MetricsCards from './MetricsCards.vue'

const store = useAppStore()
</script>

<template>
  <div class="p-6 space-y-6">
    <MetricsCards />

    <section v-if="store.sessions.length > 0">
      <h3 class="text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-3">Active Sessions</h3>
      <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-3">
        <div v-for="s in store.sessions" :key="s.id"
          class="bg-[#111111] border border-[#222222] rounded-xl p-4 hover:border-[#333] transition-colors">
          <div class="flex items-center justify-between mb-2">
            <span class="text-[13px] font-medium text-white">{{ s.goal }}</span>
            <span class="text-[10px] px-2 py-0.5 rounded-full bg-green-500/10 text-green-400 border border-green-500/20">
              {{ s.phase }}
            </span>
          </div>
          <p class="text-[12px] text-gray-500 mb-3">{{ s.project_id }}</p>
          <div class="flex items-center gap-3 text-[11px] text-gray-400">
            <span>Iteration {{ s.iteration }}</span>
            <span>{{ s.agents.length }} agents</span>
          </div>
        </div>
      </div>
    </section>

    <section v-if="store.sessions.length > 0 && store.sessions[0].agents.length > 0">
      <h3 class="text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-3">Agents</h3>
      <AgentTable :agents="store.sessions[0]?.agents || []" />
    </section>

    <section v-if="store.sessions.length === 0" class="text-center py-20">
      <div class="text-4xl mb-3 text-gray-700">⬡</div>
      <p class="text-gray-500 text-sm">No active sessions</p>
      <p class="text-gray-600 text-xs mt-1">Run <code class="bg-[#1a1a1a] px-1.5 py-0.5 rounded text-gray-400">project-x run --goal "your goal"</code> to start</p>
    </section>
  </div>
</template>