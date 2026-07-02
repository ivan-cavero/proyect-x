<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useAppStore } from './stores/app'
import Icon from './components/ui/Icon.vue'
import LoginView from './views/LoginView.vue'
import SettingsView from './views/SettingsView.vue'

const store = useAppStore()
const isAuthenticated = ref(false)
const currentView = ref<'chat' | 'settings'>('chat')
const selectedProject = ref<string | null>(null)
const chatMessage = ref('')
const selectedModel = ref('GLM-5.2')
const sidebarOpen = ref(false)
const isMobile = ref(false)

let refreshInterval: ReturnType<typeof setInterval> | null = null

function checkMobile() {
  isMobile.value = window.innerWidth < 768
  if (!isMobile.value) {
    sidebarOpen.value = false
  }
}

onMounted(() => {
  checkMobile()
  window.addEventListener('resize', checkMobile)
  
  const token = localStorage.getItem('praxis-token')
  if (token) {
    isAuthenticated.value = true
    startApp()
  }
})

onUnmounted(() => {
  window.removeEventListener('resize', checkMobile)
  if (refreshInterval) clearInterval(refreshInterval)
})

function handleLogin(token: string) {
  isAuthenticated.value = true
  localStorage.setItem('praxis-token', token)
  startApp()
}

function startApp() {
  store.refreshAll()
  refreshInterval = setInterval(() => store.refreshAll(), 10000)
}

const greeting = computed(() => {
  const hour = new Date().getHours()
  if (hour < 12) return 'Morning'
  if (hour < 18) return 'Afternoon'
  return 'Evening'
})

function selectProject(id: string) {
  selectedProject.value = id
  store.selectProject(id)
  if (isMobile.value) {
    sidebarOpen.value = false
  }
}

function handleSendMessage() {
  if (!chatMessage.value.trim()) return
  // TODO: Send message to backend
  chatMessage.value = ''
}

function toggleSidebar() {
  sidebarOpen.value = !sidebarOpen.value
}

function closeSidebar() {
  sidebarOpen.value = false
}
</script>

<template>
  <LoginView v-if="!isAuthenticated" @login="handleLogin" />

  <div v-else class="layout">
    <!-- ═══ SIDEBAR OVERLAY (Mobile) ═══ -->
    <div 
      v-if="isMobile" 
      class="sidebar-overlay" 
      :class="{ visible: sidebarOpen }"
      @click="closeSidebar"
    />

    <!-- ═══ SIDEBAR ═══ -->
    <aside class="sidebar" :class="{ open: sidebarOpen }">
      <!-- Logo -->
      <div class="sidebar-header">
        <div class="logo-mark">P</div>
        <div class="logo-text">
          <div class="brand">praxis</div>
        </div>
      </div>

      <!-- Quick actions -->
      <div class="sidebar-nav">
        <button class="nav-item" @click="currentView = 'chat'">
          <Icon name="plus" :size="18" class="nav-icon" />
          <span class="nav-label">New task</span>
          <span class="nav-shortcut">Ctrl+N</span>
        </button>
        <button class="nav-item">
          <Icon name="search" :size="18" class="nav-icon" />
          <span class="nav-label">Search</span>
          <span class="nav-shortcut">Ctrl+K</span>
        </button>
        <button class="nav-item">
          <Icon name="code" :size="18" class="nav-icon" />
          <span class="nav-label">Skills</span>
        </button>
      </div>

      <!-- Tabs: Group / Project -->
      <div class="sidebar-tabs">
        <button class="sidebar-tab active">
          <Icon name="list" :size="14" />
          Group
        </button>
        <button class="sidebar-tab">
          <Icon name="folder" :size="14" />
          Project
        </button>
      </div>

      <!-- Project list -->
      <div class="sidebar-projects">
        <div v-if="store.projects.length === 0" class="project-hint">
          No projects yet
        </div>
        <template v-for="project in store.projects" :key="project.id">
          <div
            class="project-item"
            :class="{ active: selectedProject === project.id }"
            @click="selectProject(project.id)"
          >
            <Icon name="folder" :size="16" class="project-icon" />
            <span class="project-name">{{ project.name }}</span>
          </div>
          <div class="project-hint" v-if="selectedProject === project.id">
            No tasks yet
          </div>
        </template>
      </div>

      <!-- Settings nav -->
      <div class="sidebar-nav" style="border-top: 1px solid var(--border-subtle); padding-top: var(--space-2);">
        <button
          class="nav-item"
          :class="{ active: currentView === 'settings' }"
          @click="currentView = 'settings'"
        >
          <Icon name="settings" :size="18" class="nav-icon" />
          <span class="nav-label">Settings</span>
        </button>
      </div>

      <!-- Footer: User -->
      <div class="sidebar-footer">
        <div class="sidebar-user">
          <div class="user-avatar">I</div>
          <span class="user-name">Ivan</span>
          <div class="sidebar-footer-actions">
            <button class="sidebar-footer-btn" title="Remote control">
              <Icon name="phone" :size="16" />
            </button>
            <button class="sidebar-footer-btn" @click="currentView = 'settings'" title="Settings">
              <Icon name="settings" :size="16" />
            </button>
          </div>
        </div>
      </div>
    </aside>

    <!-- ═══ MAIN CONTENT ═══ -->
    <div class="main-content">
      <!-- ═══ MOBILE HEADER ═══ -->
      <header v-if="isMobile" class="mobile-header">
        <button class="mobile-menu-btn" @click="toggleSidebar">
          <Icon name="menu" :size="20" />
        </button>
        <span class="mobile-title">praxis</span>
        <button class="mobile-menu-btn" @click="currentView = 'settings'">
          <Icon name="settings" :size="20" />
        </button>
      </header>

      <!-- ═══ CHAT VIEW ═══ -->
      <template v-if="currentView === 'chat'">
        <!-- Greeting area -->
        <div class="main-greeting">
          <!-- Logo placeholder -->
          <svg class="greeting-logo" viewBox="0 0 120 120" fill="none" xmlns="http://www.w3.org/2000/svg">
            <rect x="20" y="40" width="80" height="50" rx="8" stroke="currentColor" stroke-width="2" fill="none"/>
            <path d="M40 40 L60 20 L80 40" stroke="currentColor" stroke-width="2" fill="none"/>
            <circle cx="60" cy="65" r="8" stroke="currentColor" stroke-width="2" fill="none"/>
          </svg>

          <!-- Greeting text -->
          <h1 class="greeting-text">Good {{ greeting }}, how can I help?</h1>
        </div>

        <!-- Chat input -->
        <div class="chat-input-container">
          <div class="chat-input-wrapper">
            <!-- Header: Project & Branch -->
            <div class="chat-input-header">
              <div class="chat-input-header-item">
                <Icon name="folder" :size="14" />
                <span>{{ store.activeProject?.name || 'No project' }}</span>
                <Icon name="chevron-right" :size="12" />
              </div>
              <div class="chat-input-header-item">
                <Icon name="code" :size="14" />
                <span>main</span>
                <Icon name="chevron-right" :size="12" />
              </div>
            </div>

            <!-- Textarea -->
            <textarea
              v-model="chatMessage"
              class="chat-input-textarea"
              placeholder="Ask praxis anything, @ for files, folders, or whiteboards, / for commands or agents, $ for skills, # for related conversations"
              @keydown.enter.exact="handleSendMessage"
              rows="2"
            />

            <!-- Footer: Actions & Model -->
            <div class="chat-input-footer">
              <div class="chat-input-actions">
                <button class="chat-input-action">
                  <Icon name="plus" :size="16" />
                </button>
                <button class="chat-input-action primary">
                  <Icon name="shield" :size="14" />
                  <span>Full access</span>
                  <Icon name="chevron-right" :size="12" />
                </button>
              </div>

              <div class="chat-input-model">
                <button class="chat-input-action">
                  <Icon name="circle" :size="14" />
                </button>
                <button class="chat-input-action">
                  <span>{{ selectedModel }}</span>
                  <Icon name="chevron-right" :size="12" />
                </button>
                <button class="chat-input-action">
                  <Icon name="settings" :size="14" />
                  <span>Max</span>
                  <Icon name="chevron-right" :size="12" />
                </button>
                <button
                  class="btn-icon-send"
                  :disabled="!chatMessage.trim()"
                  @click="handleSendMessage"
                >
                  <Icon name="send" :size="16" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </template>

      <!-- ═══ SETTINGS VIEW ═══ -->
      <template v-else-if="currentView === 'settings'">
        <SettingsView @back="currentView = 'chat'" />
      </template>
    </div>
  </div>
</template>

<style scoped>
/* Settings view transitions */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
