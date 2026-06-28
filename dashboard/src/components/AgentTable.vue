<script setup lang="ts">
import type { Agent } from '../stores/app'

defineProps<{ agents: Agent[] }>()

const statusColor = (s: string) => {
  if (s === 'running') return 'bg-green-500'
  if (s === 'idle') return 'bg-gray-500'
  if (s === 'error') return 'bg-red-500'
  return 'bg-gray-600'
}

const pressureColor = (p: number) => {
  if (p > 0.9) return 'text-red-400'
  if (p > 0.7) return 'text-amber-400'
  return 'text-green-400'
}

const asiColor = (s: number) => {
  if (s >= 80) return 'text-green-400'
  if (s >= 60) return 'text-amber-400'
  return 'text-red-400'
}

function formatTokens(n: number): string {
  if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M'
  if (n >= 1000) return (n / 1000).toFixed(1) + 'K'
  return String(n)
}
</script>

<template>
  <div class="bg-[#111111] border border-[#222222] rounded-xl overflow-hidden">
    <table class="w-full text-[12px]">
      <thead>
        <tr class="border-b border-[#222222] text-gray-500 text-[11px] uppercase tracking-wider">
          <th class="text-left px-4 py-3 font-medium">Agent</th>
          <th class="text-left px-4 py-3 font-medium">Model</th>
          <th class="text-left px-4 py-3 font-medium">Status</th>
          <th class="text-left px-4 py-3 font-medium">ASI</th>
          <th class="text-left px-4 py-3 font-medium">Context</th>
          <th class="text-left px-4 py-3 font-medium">Tokens</th>
          <th class="text-left px-4 py-3 font-medium">Action</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="a in agents"
          :key="a.id"
          class="border-b border-[#222222]/50 hover:bg-white/[0.02] transition-colors"
        >
          <td class="px-4 py-3 font-medium text-white">{{ a.id }}</td>
          <td class="px-4 py-3 text-gray-400 font-mono text-[11px]">{{ a.model }}</td>
          <td class="px-4 py-3">
            <span class="flex items-center gap-1.5">
              <span class="w-1.5 h-1.5 rounded-full" :class="statusColor(a.status)"></span>
              <span class="text-gray-400 capitalize">{{ a.status }}</span>
            </span>
          </td>
          <td class="px-4 py-3" :class="asiColor(a.asi_score)">
            {{ a.asi_score.toFixed(1) }}
          </td>
          <td class="px-4 py-3" :class="pressureColor(a.context_pressure)">
            {{ (a.context_pressure * 100).toFixed(0) }}%
          </td>
          <td class="px-4 py-3 text-gray-400 font-mono text-[11px]">{{ formatTokens(a.tokens_used) }}</td>
          <td class="px-4 py-3 text-gray-500 text-[11px]">{{ a.current_action || '—' }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>