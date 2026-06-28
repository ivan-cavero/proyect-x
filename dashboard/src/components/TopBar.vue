<script setup lang="ts">
import { useAppStore } from '../stores/app'
const store = useAppStore()

function formatUptime(s: number): string {
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  return `${h}h ${m}m`
}
</script>

<template>
  <header class="h-12 border-b border-[#222222] bg-[#111111] flex items-center px-5 justify-between shrink-0">
    <div class="flex items-center gap-4">
      <span class="text-sm font-semibold">{{ store.currentView.toUpperCase() }}</span>
    </div>
    <div class="flex items-center gap-5 text-[11px] text-gray-500 font-mono">
      <span>UPTIME {{ formatUptime(store.metrics.uptime_seconds) }}</span>
      <span>SESSIONS {{ store.metrics.active_sessions }}</span>
      <span>TOKENS {{ store.totalTokens.toLocaleString() }}</span>
    </div>
  </header>
</template>