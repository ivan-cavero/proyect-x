<script setup lang="ts">
import { useAppStore } from '../stores/app'
const store = useAppStore()

const logColor = (t: string) => {
  if (t === 'system') return 'text-green-400'
  if (t === 'error') return 'text-red-400'
  return 'text-gray-500'
}
</script>

<template>
  <div class="p-6 space-y-6">
    <h3 class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Event Log</h3>
    <div class="bg-[#111111] border border-[#222222] rounded-xl p-4 font-mono text-[11px] max-h-[600px] overflow-y-auto">
      <div v-if="store.logs.length === 0" class="text-gray-600 text-center py-8">
        No events yet
      </div>
      <div v-for="(log, i) in store.logs" :key="i" class="flex gap-3 py-1">
        <span class="text-gray-600 w-16 shrink-0">{{ log.time }}</span>
        <span :class="logColor(log.type)">{{ log.message }}</span>
      </div>
    </div>
  </div>
</template>