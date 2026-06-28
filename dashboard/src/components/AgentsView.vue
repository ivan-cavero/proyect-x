<script setup lang="ts">
import { useAppStore } from '../stores/app'
import AgentTable from './AgentTable.vue'

const store = useAppStore()

const allAgents = () => {
  return store.sessions.flatMap(s => s.agents)
}
</script>

<template>
  <div class="p-6 space-y-6">
    <h3 class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">All Agents</h3>
    <AgentTable :agents="allAgents()" />
  </div>
</template>