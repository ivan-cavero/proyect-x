<script setup lang="ts">
import { useAppStore } from '../stores/app'
const store = useAppStore()
</script>

<template>
  <div class="p-6 space-y-6">
    <h3 class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Context Management</h3>
    <div class="bg-[#111111] border border-[#222222] rounded-xl p-5">
      <div class="text-sm font-medium text-white mb-3">Context Budget</div>
      <div class="space-y-3">
        <div class="flex justify-between text-[12px]">
          <span class="text-gray-500">Pressure</span>
          <span class="text-white font-mono">{{ (store.metrics.context_pressure * 100).toFixed(1) }}%</span>
        </div>
        <div class="w-full bg-[#222222] rounded-full h-2">
          <div class="bg-green-500 h-2 rounded-full transition-all"
            :style="{ width: (store.metrics.context_pressure * 100) + '%' }"></div>
        </div>
        <div class="flex justify-between text-[12px]">
          <span class="text-gray-500">ASI Score</span>
          <span class="text-white font-mono">{{ store.metrics.avg_asi.toFixed(1) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>