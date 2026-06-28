<script setup lang="ts">
import { useAppStore } from '../stores/app'
const store = useAppStore()
</script>

<template>
  <div class="p-6 space-y-6">
    <h3 class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Sessions</h3>
    <div class="space-y-2">
      <div v-for="s in store.sessions" :key="s.id"
        class="bg-[#111111] border border-[#222222] rounded-xl p-4 flex items-center justify-between">
        <div>
          <div class="text-sm font-medium text-white">{{ s.goal }}</div>
          <div class="text-[12px] text-gray-500">{{ s.project_id }} · {{ s.phase }} · Iter {{ s.iteration }}</div>
        </div>
        <div class="flex items-center gap-2">
          <span class="text-[10px] px-2 py-0.5 rounded-full bg-green-500/10 text-green-400">active</span>
        </div>
      </div>
      <div v-if="store.sessions.length === 0" class="text-center py-12 text-gray-500 text-sm">
        No active sessions
      </div>
    </div>
  </div>
</template>