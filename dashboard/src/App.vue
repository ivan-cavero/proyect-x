<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useAppStore } from './stores/app'
import { useWebSocket } from './composables/useWebSocket'
import Icon from './components/ui/Icon.vue'
import Button from './components/ui/Button.vue'
import Input from './components/ui/Input.vue'
import LoginView from './views/LoginView.vue'
import SettingsView from './views/SettingsView.vue'

const store = useAppStore()
const ws = useWebSocket()
const currentView = ref<'overview' | 'projects' | 'settings'>('overview')
const isAuthenticated = ref(false)
const showCreateForm = ref(false)
const newProjectName = ref('')
const newProjectDesc = ref('')
const editingProjectId = ref<string | null>(null)
const configEditMode = ref(false)
const configEditContent = ref('')
const saving = ref(false)

let refreshInterval: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  const token = localStorage.getItem('praxis-token')
  if (token) { isAuthenticated.value = true; startApp() }
})

onUnmounted(() => { if (refreshInterval) clearInterval(refreshInterval) })

function handleLogin(token: string) {
  isAuthenticated.value = true
  localStorage.setItem('praxis-token', token)
  startApp()
}

function handleLogout() {
  isAuthenticated.value = false
  localStorage.removeItem('praxis-token')
  if (refreshInterval) clearInterval(refreshInterval)
}

function startApp() {
  store.refreshAll()
  refreshInterval = setInterval(() => store.refreshAll(), 10000)
}

const navItems = [
  { id: 'overview' as const, label: 'Overview', icon: 'dashboard' },
  { id: 'projects' as const, label: 'Projects', icon: 'list' },
  { id: 'settings' as const, label: 'Vault', icon: 'shield' },
]

const recentEvents = computed(() => ws.events.value.slice().reverse().slice(0, 50))

function formatTime(iso: string) {
  const d = new Date(iso)
  return d.toLocaleTimeString('en-US', { hour12: false })
}

function formatDate(iso: string) {
  const diff = Date.now() - new Date(iso).getTime()
  if (diff < 60000) return `${Math.floor(diff / 1000)}s`
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m`
  return `${Math.floor(diff / 3600000)}h`
}

function eventKindColor(kind: string): string {
  if (kind.includes('Phase') || kind.includes('Session')) return 'badge-cyan'
  if (kind.includes('Token')) return 'badge-amber'
  if (kind.includes('Drift') || kind.includes('Alert')) return 'badge-crimson'
  if (kind.includes('Tool')) return 'badge-emerald'
  return 'badge-gray'
}

async function handleCreateProject() {
  if (!newProjectName.value.trim()) return
  try {
    await store.createProject(newProjectName.value.trim(), newProjectDesc.value)
    newProjectName.value = ''
    newProjectDesc.value = ''
    showCreateForm.value = false
  } catch (creationError: any) { console.error(creationError) }
}

async function handleDeleteProject(id: string) {
  if (!confirm('Delete this project?')) return
  await store.deleteProject(id)
  editingProjectId.value = null
  configEditMode.value = false
}

function editProject(id: string) {
  store.selectProject(id)
  editingProjectId.value = id
  configEditMode.value = false
  // Sync config content from store once loaded
  if (store.activeConfig) {
    configEditContent.value = store.activeConfig.raw
  }
}

async function saveConfig() {
  if (!editingProjectId.value) return
  saving.value = true
  try {
    await store.saveProjectConfig(editingProjectId.value, configEditContent.value)
    configEditMode.value = false
  } finally { saving.value = false }
}
</script>

<template>
  <LoginView v-if="!isAuthenticated" @login="handleLogin" />

  <div v-else class="layout">
    <aside class="sidebar">
      <div class="sidebar-header">
        <div class="logo-mark">X</div>
        <div class="logo-text">
          <div class="brand">praxis</div>
          <div class="sub">NEURAL CMD</div>
        </div>
      </div>
      <nav class="sidebar-nav">
        <button v-for="item in navItems" :key="item.id"
          @click="currentView = item.id"
          class="nav-item" :class="{ active: currentView === item.id }">
          <Icon :name="item.icon" :size="18" class="nav-icon"
            :color="currentView === item.id ? 'var(--clr-primary)' : undefined" />
          <span class="nav-label">{{ item.label }}</span>
        </button>
      </nav>
      <div class="sidebar-footer">
        <button @click="handleLogout" class="nav-item">
          <Icon name="logout" :size="18" class="nav-icon" />
          <span class="nav-label">Disconnect</span>
        </button>
      </div>
    </aside>

    <div class="main-content">
      <header class="main-header">
        <div class="header-left">
          <span class="header-label">Sector</span>
          <span class="header-title">{{ navItems.find(n => n.id === currentView)?.label }}</span>
        </div>
        <div class="header-right">
          <span v-if="store.isLoading" class="status-indicator" style="color: var(--clr-amber)">SYNCING</span>
          <div class="status-indicator">
            <Icon :name="ws.connected.value ? 'wifi' : 'wifi-off'" :size="14"
              :color="ws.connected.value ? 'var(--clr-emerald)' : 'var(--clr-crimson)'" />
            <span>{{ ws.connected.value ? 'LIVE' : 'DARK' }}</span>
          </div>
          <span class="status-indicator" style="font-size: 11px; color: var(--clr-text-secondary)">v{{ store.version }}</span>
        </div>
      </header>

      <div class="main-body">

        <!-- ═══ OVERVIEW ═══ -->
        <div v-if="currentView === 'overview'" class="animate-fade">
          <div class="metrics-grid compact">
            <div class="metric-card">
              <div class="metric-label">System</div>
              <div class="metric-value" :class="store.isHealthy ? 'emerald' : 'amber'" style="font-size: 18px">
                {{ store.isHealthy ? 'ONLINE' : 'UNKNOWN' }}
              </div>
              <div class="metric-sub">{{ store.uptime }}</div>
            </div>
            <div class="metric-card">
              <div class="metric-label">Projects</div>
              <div class="metric-value cyan">{{ store.projects.length }}</div>
              <div class="metric-sub">total created</div>
            </div>
            <div class="metric-card">
              <div class="metric-label">Providers</div>
              <div class="metric-value emerald">6</div>
              <div class="metric-sub">default roles</div>
            </div>
            <div class="metric-card">
              <div class="metric-label">Vault</div>
              <div class="metric-value cyan">—</div>
              <div class="metric-sub">API keys</div>
            </div>
          </div>

          <div class="info-grid-3">
            <div class="card compact">
              <div class="card-header"><div class="card-title">Projects</div><div class="card-subtitle">{{ store.projects.length }} total</div></div>
              <div class="data-list">
                <div v-if="store.projects.length === 0" class="empty-state" style="padding: var(--space-lg)">
                  <Icon name="list" :size="24" class="empty-state-icon" />
                  <div class="empty-state-text">No projects yet</div>
                  <div class="empty-state-hint">Create one above or via CLI</div>
                </div>
                <div v-for="p in store.projects.slice(0, 5)" :key="p.id" class="data-row" style="padding: var(--space-sm) var(--space-md)">
                  <span class="data-cell" style="font-weight: 600; font-size: 12px; min-width: 120px">{{ p.name }}</span>
                  <span class="data-cell muted" style="font-size: 10px">{{ formatDate(p.created_at) }}</span>
                  <span class="badge badge-cyan" style="font-size: 9px">{{ p.forge_toml ? 'configured' : 'empty' }}</span>
                </div>
              </div>
            </div>

            <div class="card compact">
              <div class="card-header"><div class="card-title">Default Roles</div><div class="card-subtitle">per forge.toml</div></div>
              <div class="data-list">
                <div v-for="role in ['architect', 'coder', 'reviewer', 'security', 'tester', 'researcher']" :key="role"
                  class="data-row" style="padding: var(--space-sm) var(--space-md)">
                  <span class="data-cell" style="font-weight: 600; font-size: 12px; text-transform: capitalize; min-width: 100px">{{ role }}</span>
                  <span class="data-cell mono" style="font-size: 11px; color: var(--clr-primary)">configured</span>
                </div>
              </div>
            </div>

            <div class="card compact">
              <div class="card-header"><div class="card-title">Event Stream</div><div class="card-subtitle">last 20</div></div>
              <div class="data-list">
                <div v-if="recentEvents.length === 0" class="empty-state" style="padding: var(--space-lg)">
                  <Icon name="terminal" :size="24" class="empty-state-icon" />
                  <div class="empty-state-text">Awaiting events</div>
                </div>
                <div v-for="event in recentEvents.slice(0, 20)" :key="event.id"
                  class="data-row" style="padding: var(--space-sm) var(--space-md)">
                  <span class="data-cell mono muted" style="min-width: 50px; font-size: 10px">{{ formatTime(event.timestamp) }}</span>
                  <span class="badge" :class="eventKindColor(event.kind)" style="font-size: 9px; padding: 1px 6px">{{ event.kind }}</span>
                  <span class="data-cell mono" style="font-size: 10px; color: var(--clr-text-secondary)">{{ event.source }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- ═══ PROJECTS ═══ -->
        <div v-if="currentView === 'projects'" class="animate-fade">
          <div class="config-header">
            <div><h2 class="text-sm font-semibold tracking-wide mb-3">PROJECTS</h2><p class="text-xs" style="color: var(--clr-text-muted)">Managed centrally in AppData</p></div>
            <Button @click="showCreateForm = true" variant="cyan" size="sm"><Icon name="plus" :size="14" /> New Project</Button>
          </div>

          <!-- Create form -->
          <div v-if="showCreateForm" class="card compact" style="margin-bottom: var(--space-md)">
            <div class="card-body">
              <div class="input-row">
                <div class="input-group"><label class="data-label">Name</label><Input v-model="newProjectName" placeholder="my-app" /></div>
                <div class="input-group"><label class="data-label">Description</label><Input v-model="newProjectDesc" placeholder="Optional" /></div>
              </div>
              <div class="flex gap-2" style="margin-top: var(--space-md)">
                <Button @click="handleCreateProject" variant="cyan" size="sm" :disabled="!newProjectName.trim()">Create</Button>
                <Button @click="showCreateForm = false" variant="ghost" size="sm">Cancel</Button>
              </div>
            </div>
          </div>

          <div class="info-grid-3">
            <!-- Project list -->
            <div class="card compact" style="grid-column: span 1">
              <div class="card-header"><div class="card-title">All Projects</div><div class="card-subtitle">{{ store.projects.length }}</div></div>
              <div class="data-list">
                <div v-if="store.projects.length === 0" class="empty-state" style="padding: var(--space-2xl)">
                  <Icon name="list" :size="32" class="empty-state-icon" />
                  <div class="empty-state-text">No projects</div>
                  <div class="empty-state-hint">Click New Project to create one</div>
                </div>
                <button v-for="p in store.projects" :key="p.id" @click="editProject(p.id)"
                  class="data-row" style="width: 100%; text-align: left; background: none; border: none; cursor: pointer; padding: var(--space-sm) var(--space-md)"
                  :class="{ 'active-row': editingProjectId === p.id }">
                  <div style="flex: 1">
                    <div class="data-cell" style="font-weight: 600; font-size: 13px">{{ p.name }}</div>
                    <div class="data-cell mono muted" style="font-size: 10px">{{ formatDate(p.created_at) }}</div>
                  </div>
                  <Icon name="chevron-right" :size="14" style="color: var(--clr-text-muted)" />
                </button>
              </div>
            </div>

            <!-- Project detail / config -->
            <div v-if="editingProjectId && store.activeProject" class="card compact" style="grid-column: span 2">
              <div class="card-header" style="display: flex; justify-content: space-between; align-items: center">
                <div>
                  <div class="card-title">{{ store.activeProject.name }}</div>
                  <div class="card-subtitle">{{ store.activeProject.description || 'No description' }}</div>
                </div>
                <div class="flex gap-2">
                  <Button @click="handleDeleteProject(store.activeProject.id)" variant="danger" size="sm">Delete</Button>
                </div>
              </div>

              <!-- Config tabs -->
              <div class="config-tabs" style="padding: 0 var(--space-lg); border-bottom: 1px solid var(--clr-border-subtle)">
                <button @click="configEditMode = false" class="config-tab" :class="{ active: !configEditMode }">Summary</button>
                <button @click="configEditMode = true" class="config-tab" :class="{ active: configEditMode }">Forge Config</button>
              </div>

              <!-- Summary tab -->
              <div v-if="!configEditMode && store.activeConfig" class="card-body">
                <div class="info-grid-3" style="gap: var(--space-sm)">
                  <div>
                    <div class="data-label mb-1">Roles</div>
                    <div v-for="(role, name) in store.activeConfig.roles" :key="name" class="data-cell mono" style="font-size: 11px; margin-bottom: 4px">
                      {{ name }}: <span style="color: var(--clr-primary)">{{ role.model }}</span>
                    </div>
                  </div>
                  <div>
                    <div class="data-label mb-1">Providers</div>
                    <div v-for="(prov, name) in store.activeConfig.providers" :key="name" class="data-cell mono" style="font-size: 11px; margin-bottom: 4px; text-transform: capitalize">
                      {{ name }}: <span style="color: var(--clr-primary)">{{ prov.default_model }}</span>
                    </div>
                  </div>
                  <div>
                    <div class="data-label mb-1">Goals</div>
                    <div v-for="goal in store.activeConfig.goals" :key="goal.name" class="data-cell mono" style="font-size: 11px; margin-bottom: 4px">
                      {{ goal.name }}: {{ goal.agents.join(', ') }}
                    </div>
                    <div v-if="store.activeConfig.goals.length === 0" class="data-cell muted" style="font-size: 11px">No goals</div>
                  </div>
                </div>
                <div class="info-grid-3" style="gap: var(--space-sm); margin-top: var(--space-md)">
                  <div>
                    <div class="data-label mb-1">Limits</div>
                    <div class="data-cell mono" style="font-size: 11px">max iter: {{ store.activeConfig.limits.max_iterations_per_goal }}</div>
                    <div class="data-cell mono" style="font-size: 11px">ttl: {{ store.activeConfig.limits.session_ttl_seconds }}s</div>
                  </div>
                  <div>
                    <div class="data-label mb-1">Project</div>
                    <div class="data-cell mono" style="font-size: 11px">name: {{ store.activeConfig.project.name }}</div>
                    <div class="data-cell mono" style="font-size: 11px">version: {{ store.activeConfig.project.version }}</div>
                  </div>
                </div>
              </div>

              <!-- Raw config editor -->
              <div v-if="configEditMode" class="card-body" style="padding: 0">
                <textarea v-model="configEditContent" class="raw-editor" spellcheck="false" />
                <div class="flex gap-2" style="padding: var(--space-md)">
                  <Button @click="saveConfig" variant="cyan" size="sm" :disabled="saving">
                    <Icon v-if="!saving" name="check" :size="14" />
                    <Icon v-else name="loader" :size="14" class="animate-spin" />
                    {{ saving ? 'Saving...' : 'Save Config' }}
                  </Button>
                  <Button @click="configEditMode = false" variant="ghost" size="sm">Cancel</Button>
                </div>
              </div>
            </div>

            <!-- No selection -->
            <div v-if="!editingProjectId" class="card compact" style="grid-column: span 2">
              <div class="empty-state" style="padding: var(--space-3xl)">
                <Icon name="list" :size="40" class="empty-state-icon" />
                <div class="empty-state-text">Select a project</div>
                <div class="empty-state-hint">Click a project from the list to view and edit its configuration</div>
              </div>
            </div>
          </div>
        </div>

        <!-- ═══ SETTINGS (Vault) ═══ -->
        <div v-if="currentView === 'settings'" class="animate-fade">
          <div class="config-header">
            <div><h2 class="text-sm font-semibold tracking-wide mb-3">VAULT</h2><p class="text-xs" style="color: var(--clr-text-muted)">Global API keys (stored centrally in AppData)</p></div>
          </div>
          <SettingsView />
        </div>

      </div>
    </div>
  </div>
</template>

<style scoped>
.active-row {
  background: var(--clr-primary-glow) !important;
}

.raw-editor {
  width: 100%;
  min-height: 350px;
  background: var(--clr-bg);
  border: none;
  color: var(--clr-primary);
  font-family: var(--font-mono);
  font-size: 12px;
  padding: var(--space-lg);
  line-height: 1.6;
  resize: vertical;
  outline: none;
}
.raw-editor:focus { box-shadow: inset 0 0 0 1px var(--clr-primary); }
</style>