<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useAppStore } from './stores/app'
import Sidebar from './components/Sidebar.vue'
import TopBar from './components/TopBar.vue'
import StatusView from './components/StatusView.vue'
import AgentsView from './components/AgentsView.vue'
import SessionsView from './components/SessionsView.vue'
import ConfigView from './components/ConfigView.vue'
import ContextView from './components/ContextView.vue'
import LogsView from './components/LogsView.vue'

const store = useAppStore()

onMounted(() => {
  store.connectWebSocket()
})

onUnmounted(() => {
  store.ws?.close()
})
</script>

<template>
  <div class="h-screen flex bg-black text-white overflow-hidden">
    <Sidebar />
    <div class="flex-1 flex flex-col min-w-0">
      <TopBar />
      <main class="flex-1 overflow-y-auto">
        <StatusView v-if="store.currentView === 'status'" />
        <AgentsView v-else-if="store.currentView === 'agents'" />
        <SessionsView v-else-if="store.currentView === 'sessions'" />
        <ConfigView v-else-if="store.currentView === 'config'" />
        <ContextView v-else-if="store.currentView === 'context'" />
        <LogsView v-else-if="store.currentView === 'logs'" />
        <div v-else class="p-8 text-center text-gray-500">
          <p class="text-lg">{{ store.currentView }}</p>
        </div>
      </main>
    </div>
  </div>
</template>