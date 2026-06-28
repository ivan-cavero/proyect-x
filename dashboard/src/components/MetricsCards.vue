<script setup lang="ts">
import { useAppStore } from '../stores/app'
const store = useAppStore()

const cards = [
  { label: 'TOTAL TOKENS', key: 'total_tokens' as const, format: (v: number) => v.toLocaleString() },
  { label: 'ASI SCORE', key: 'avg_asi' as const, format: (v: number) => v.toFixed(1), color: (v: number) => v > 80 ? 'text-green-400' : v > 60 ? 'text-amber-400' : 'text-red-400' },
  { label: 'CONTEXT', key: 'context_pressure' as const, format: (v: number) => (v * 100).toFixed(0) + '%', color: (v: number) => v > 0.9 ? 'text-red-400' : v > 0.7 ? 'text-amber-400' : 'text-green-400' },
  { label: 'SESSIONS', key: 'active_sessions' as const, format: (v: number) => String(v) },
]
</script>

<template>
  <div class="grid grid-cols-2 lg:grid-cols-4 gap-3">
    <div
      v-for="card in cards"
      :key="card.label"
      class="bg-[#111111] border border-[#222222] rounded-xl p-4"
    >
      <div class="text-[10px] font-medium text-gray-500 uppercase tracking-wider mb-1">{{ card.label }}</div>
      <div
        class="text-2xl font-bold font-mono"
        :class="card.color ? card.color(store.metrics[card.key]) : 'text-white'"
      >
        {{ card.format(store.metrics[card.key]) }}
      </div>
    </div>
  </div>
</template>