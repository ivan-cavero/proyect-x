<script setup lang="ts">
import { useAppStore } from '../stores/app'
const store = useAppStore()
</script>

<template>
  <aside class="w-60 bg-[#111111] border-r border-[#222222] flex flex-col h-screen">
    <!-- Logo -->
    <div class="px-5 py-4 border-b border-[#222222]">
      <div class="flex items-center gap-3">
        <div class="w-2.5 h-2.5 rounded-full" :class="store.connected ? 'bg-green-500' : 'bg-gray-600'"></div>
        <span class="text-sm font-semibold tracking-wide">PROJECT-X</span>
      </div>
    </div>

    <!-- Navigation -->
    <nav class="flex-1 py-3 px-3 space-y-1">
      <button
        v-for="item in [
          { id: 'status', label: 'Status', icon: '◉' },
          { id: 'agents', label: 'Agents', icon: '⬡' },
          { id: 'sessions', label: 'Sessions', icon: '▦' },
          { id: 'context', label: 'Context', icon: '⊞' },
          { id: 'logs', label: 'Logs', icon: '▤' },
          { id: 'config', label: 'Config', icon: '⚙' },
        ]"
        :key="item.id"
        @click="store.currentView = item.id"
        class="w-full flex items-center gap-3 px-3 py-2.5 text-[13px] rounded-lg transition-all"
        :class="store.currentView === item.id
          ? 'bg-white/10 text-white'
          : 'text-gray-400 hover:bg-white/5 hover:text-gray-200'"
      >
        <span class="w-5 text-center text-sm opacity-70">{{ item.icon }}</span>
        <span>{{ item.label }}</span>
      </button>
    </nav>

    <!-- Bottom: connection status -->
    <div class="px-5 py-4 border-t border-[#222222]">
      <div class="flex items-center gap-2 text-xs text-gray-500">
        <div class="w-2 h-2 rounded-full" :class="store.connected ? 'bg-green-500' : 'bg-gray-600'"></div>
        <span>{{ store.connected ? 'Connected' : 'Disconnected' }}</span>
      </div>
    </div>
  </aside>
</template>