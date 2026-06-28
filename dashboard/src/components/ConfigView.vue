<script setup lang="ts">
import { useAppStore } from '../stores/app'
const store = useAppStore()
</script>

<template>
  <div class="p-6 space-y-6">
    <h3 class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Providers</h3>
    <div class="space-y-2">
      <div v-for="p in store.providers" :key="p.id"
        class="bg-[#111111] border border-[#222222] rounded-xl p-4">
        <div class="flex items-center justify-between">
          <span class="text-sm font-medium text-white">{{ p.name }}</span>
          <button @click="store.removeProvider(p.id)" class="text-xs text-gray-500 hover:text-red-400">✕</button>
        </div>
        <div class="text-[11px] text-gray-500 font-mono mt-1">{{ p.base_url }}</div>
        <div class="flex gap-1.5 mt-2 flex-wrap">
          <span v-for="m in p.models" :key="m"
            class="text-[10px] px-2 py-0.5 rounded bg-white/5 text-gray-400">{{ m }}</span>
        </div>
      </div>
    </div>
  </div>
</template>