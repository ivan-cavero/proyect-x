<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import Card from './components/ui/Card.vue'
import Button from './components/ui/Button.vue'
import Badge from './components/ui/Badge.vue'

// ─── State ─────────────────────────────────────────────

const currentView = ref('chat')

interface Provider {
  name: string
  baseUrl: string
  apiKey: string
  models: string[]
}

interface Message {
  id: string
  role: 'user' | 'assistant' | 'system'
  content: string
  timestamp: Date
  model?: string
}

const providers = ref<Provider[]>([])
const activeProvider = ref('')
const activeModel = ref('')
const messages = ref<Message[]>([])
const inputMessage = ref('')
const isLoading = ref(false)
const showAddProvider = ref(false)
const newProvider = ref({ name: '', baseUrl: '', apiKey: '' })

// ─── Persistence ───────────────────────────────────────

onMounted(() => {
  const saved = localStorage.getItem('project-x-config')
  if (saved) {
    try {
      const config = JSON.parse(saved)
      providers.value = config.providers || []
      if (providers.value.length > 0 && !activeProvider.value) {
        activeProvider.value = providers.value[0].name
        activeModel.value = providers.value[0].models[0] || ''
      }
    } catch {}
  }
})

function saveConfig() {
  localStorage.setItem('project-x-config', JSON.stringify({
    providers: providers.value,
    activeProvider: activeProvider.value,
    activeModel: activeModel.value,
  }))
}

// ─── Providers ─────────────────────────────────────────

function addProvider() {
  if (!newProvider.value.name || !newProvider.value.baseUrl) return
  providers.value.push({
    name: newProvider.value.name,
    baseUrl: newProvider.value.baseUrl,
    apiKey: newProvider.value.apiKey,
    models: [],
  })
  if (!activeProvider.value) {
    activeProvider.value = newProvider.value.name
  }
  saveConfig()
  newProvider.value = { name: '', baseUrl: '', apiKey: '' }
  showAddProvider.value = false
}

function removeProvider(name: string) {
  providers.value = providers.value.filter(p => p.name !== name)
  if (activeProvider.value === name) {
    activeProvider.value = providers.value[0]?.name || ''
    activeModel.value = providers.value[0]?.models[0] || ''
  }
  saveConfig()
}

// ─── Chat ──────────────────────────────────────────────

async function sendMessage() {
  if (!inputMessage.value.trim() || isLoading.value || !activeProvider.value) return

  const provider = providers.value.find(p => p.name === activeProvider.value)
  if (!provider) return

  messages.value.push({
    id: Date.now().toString(),
    role: 'user',
    content: inputMessage.value,
    timestamp: new Date(),
  })

  inputMessage.value = ''
  isLoading.value = true

  try {
    const res = await fetch(`${provider.baseUrl}/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${provider.apiKey}`,
      },
      body: JSON.stringify({
        model: activeModel.value || provider.models[0],
        messages: messages.value.map(m => ({ role: m.role, content: m.content })),
      }),
    })

    if (!res.ok) throw new Error(`API ${res.status}: ${await res.text()}`)
    const data = await res.json()

    messages.value.push({
      id: Date.now().toString(),
      role: 'assistant',
      content: data.choices?.[0]?.message?.content || 'No response',
      timestamp: new Date(),
      model: data.model,
    })
  } catch (e: any) {
    messages.value.push({
      id: Date.now().toString(),
      role: 'system',
      content: `Error: ${e.message}`,
      timestamp: new Date(),
    })
  } finally {
    isLoading.value = false
  }
}

function clearChat() {
  messages.value = []
}

// ─── Navigation ────────────────────────────────────────

const navItems = [
  { id: 'chat', label: 'Chat', icon: '💬' },
  { id: 'agents', label: 'Agents', icon: '🤖' },
  { id: 'sessions', label: 'Sessions', icon: '📋' },
  { id: 'config', label: 'Config', icon: '⚙️' },
]

// ─── Auto-scroll ───────────────────────────────────────

const chatContainer = ref<HTMLElement>()
watch(messages, () => {
  setTimeout(() => chatContainer.value?.scrollTo({ top: chatContainer.value.scrollHeight, behavior: 'smooth' }), 50)
}, { deep: true })
</script>

<template>
  <div class="h-screen flex bg-[#0a0a0a] text-[#e5e5e5] antialiased">

    <!-- ═══ SIDEBAR ═══ -->
    <aside class="bg-[#111111] border-r border-[#1f1f1f] flex flex-col w-64 shrink-0">
      <div class="h-14 border-b border-[#1f1f1f] flex items-center px-5 gap-3">
        <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-green-500 to-emerald-600 flex items-center justify-center text-white font-bold text-sm">X</div>
        <div>
          <div class="text-sm font-semibold text-white">Project-X</div>
          <div class="text-[10px] text-gray-500">v1.0.0</div>
        </div>
      </div>

      <nav class="flex-1 py-4 px-3 space-y-1">
        <button
          v-for="item in navItems"
          :key="item.id"
          @click="currentView = item.id"
          class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-all"
          :class="currentView === item.id
            ? 'bg-green-500/10 text-green-400'
            : 'text-gray-400 hover:bg-white/5 hover:text-white'"
        >
          <span class="text-lg w-6 text-center">{{ item.icon }}</span>
          <span>{{ item.label }}</span>
        </button>
      </nav>

      <div class="p-4 border-t border-[#1f1f1f] space-y-3">
        <div class="text-[10px] text-gray-500 uppercase tracking-wider mb-2">Active Model</div>
        <div v-if="activeModel" class="text-sm text-green-400 font-mono">{{ activeModel }}</div>
        <div v-else class="text-sm text-gray-600">No model selected</div>
        <div class="text-[10px] text-gray-600 mt-1">{{ activeProvider || 'No provider' }}</div>
      </div>
    </aside>

    <!-- ═══ MAIN CONTENT ═══ -->
    <div class="flex-1 flex flex-col min-w-0">

      <!-- Top Bar -->
      <header class="h-14 border-b border-[#1f1f1f] bg-[#111111]/50 backdrop-blur-sm flex items-center px-6 justify-between shrink-0">
        <span class="text-sm font-semibold">{{ navItems.find(n => n.id === currentView)?.label }}</span>
        <div class="flex items-center gap-4 text-xs text-gray-500">
          <span>Messages: {{ messages.length }}</span>
          <span class="w-2 h-2 rounded-full bg-green-500"></span>
        </div>
      </header>

      <!-- ═══ CHAT VIEW ═══ -->
      <div v-if="currentView === 'chat'" class="flex-1 flex flex-col min-h-0">
        <!-- Messages -->
        <div ref="chatContainer" class="flex-1 overflow-y-auto px-6 py-4 space-y-4">
          <div v-if="messages.length === 0" class="h-full flex items-center justify-center">
            <div class="text-center">
              <div class="text-5xl mb-4 opacity-30">💬</div>
              <div class="text-lg text-gray-400 mb-1">Start a conversation</div>
              <div class="text-sm text-gray-600">
                {{ activeProvider ? `Using ${activeModel} via ${activeProvider}` : 'Add a provider in Config first' }}
              </div>
            </div>
          </div>

          <div v-for="msg in messages" :key="msg.id" class="flex gap-3" :class="msg.role === 'user' ? 'justify-end' : ''">
            <div v-if="msg.role !== 'user'" class="w-8 h-8 rounded-full bg-blue-500/20 flex items-center justify-center text-sm shrink-0">🤖</div>
            <div class="max-w-2xl min-w-0">
              <div class="text-[10px] text-gray-500 mb-1">{{ msg.role }} · {{ msg.timestamp.toLocaleTimeString() }}</div>
              <div class="rounded-2xl px-4 py-3 text-sm leading-relaxed whitespace-pre-wrap"
                :class="msg.role === 'user'
                  ? 'bg-green-600/20 text-green-100 border border-green-600/30'
                  : msg.role === 'system'
                    ? 'bg-red-500/10 text-red-300 border border-red-500/20'
                    : 'bg-[#1a1a1a] text-gray-200 border border-[#2a2a2a]'"
              >{{ msg.content }}</div>
            </div>
            <div v-if="msg.role === 'user'" class="w-8 h-8 rounded-full bg-green-500/20 flex items-center justify-center text-sm shrink-0">👤</div>
          </div>

          <div v-if="isLoading" class="flex gap-3">
            <div class="w-8 h-8 rounded-full bg-blue-500/20 flex items-center justify-center text-sm">🤖</div>
            <div class="bg-[#1a1a1a] rounded-2xl px-4 py-3 border border-[#2a2a2a]">
              <div class="flex gap-1.5">
                <div class="w-2 h-2 bg-gray-500 rounded-full animate-bounce" style="animation-delay: 0s"></div>
                <div class="w-2 h-2 bg-gray-500 rounded-full animate-bounce" style="animation-delay: 0.15s"></div>
                <div class="w-2 h-2 bg-gray-500 rounded-full animate-bounce" style="animation-delay: 0.3s"></div>
              </div>
            </div>
          </div>
        </div>

        <!-- Input Bar -->
        <div class="border-t border-[#1f1f1f] px-6 py-4 shrink-0">
          <div class="flex gap-3 max-w-3xl">
            <input
              v-model="inputMessage"
              @keydown.enter="sendMessage"
              :placeholder="activeProvider ? `Message ${activeModel}...` : 'Configure a provider first'"
              :disabled="isLoading || !activeProvider"
              class="flex-1 bg-[#1a1a1a] border border-[#2a2a2a] rounded-xl px-4 py-3 text-sm text-white
                     placeholder-gray-500 focus:outline-none focus:border-green-500/50
                     disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            />
            <button @click="sendMessage" :disabled="isLoading || !inputMessage.trim() || !activeProvider"
              class="px-5 bg-green-600 hover:bg-green-500 disabled:bg-gray-700 disabled:text-gray-500
                     rounded-xl text-sm font-medium text-white transition-colors">
              Send
            </button>
            <button @click="clearChat" class="px-4 bg-[#1a1a1a] hover:bg-[#252525] rounded-xl text-gray-400 text-sm transition-colors border border-[#2a2a2a]">
              Clear
            </button>
          </div>
        </div>
      </div>

      <!-- ═══ AGENTS VIEW ═══ -->
      <div v-else-if="currentView === 'agents'" class="flex-1 overflow-y-auto px-6 py-6">
        <div class="max-w-3xl space-y-4">
          <h2 class="text-lg font-semibold">Agent Roles</h2>
          <div class="grid grid-cols-2 lg:grid-cols-3 gap-3">
            <Card v-for="role in ['Architect', 'Coder', 'Reviewer', 'Security', 'Tester', 'Researcher']" :key="role">
              <div class="space-y-2">
                <div class="font-medium text-white">{{ role }}</div>
                <div class="text-xs text-gray-500">{{ { Architect: 'System design, ADRs', Coder: 'Code generation', Reviewer: 'Code review', Security: 'Vulnerability scan', Tester: 'Test generation', Researcher: 'Web research' }[role] }}</div>
                <Badge variant="green" size="sm">{{ activeModel || 'No model' }}</Badge>
              </div>
            </Card>
          </div>
        </div>
      </div>

      <!-- ═══ SESSIONS VIEW ═══ -->
      <div v-else-if="currentView === 'sessions'" class="flex-1 overflow-y-auto px-6 py-6">
        <div class="max-w-3xl space-y-4">
          <h2 class="text-lg font-semibold">Sessions</h2>
          <Card v-if="messages.length > 0">
            <div class="flex items-center justify-between">
              <div>
                <div class="font-medium text-white">Current Chat</div>
                <div class="text-xs text-gray-500 mt-1">{{ messages.length }} messages</div>
              </div>
              <Badge variant="green">active</Badge>
            </div>
          </Card>
          <div v-else class="text-center py-16 text-gray-500">
            <div class="text-3xl mb-2 opacity-40">📋</div>
            <div class="text-sm">No sessions yet</div>
          </div>
        </div>
      </div>

      <!-- ═══ CONFIG VIEW ═══ -->
      <div v-else-if="currentView === 'config'" class="flex-1 overflow-y-auto px-6 py-6">
        <div class="max-w-2xl space-y-6">

          <!-- Active Provider -->
          <Card title="Active Provider">
            <div class="grid grid-cols-2 gap-4">
              <div>
                <label class="text-xs text-gray-500 block mb-1">Provider</label>
                <select v-model="activeProvider" @change="saveConfig"
                  class="w-full bg-[#1a1a1a] border border-[#2a2a2a] rounded-xl px-3 py-2.5 text-sm text-white focus:border-green-500/50">
                  <option value="">Select provider</option>
                  <option v-for="p in providers" :key="p.name" :value="p.name">{{ p.name }}</option>
                </select>
              </div>
              <div>
                <label class="text-xs text-gray-500 block mb-1">Model</label>
                <select v-model="activeModel" @change="saveConfig"
                  class="w-full bg-[#1a1a1a] border border-[#2a2a2a] rounded-xl px-3 py-2.5 text-sm text-white focus:border-green-500/50">
                  <option value="">Select model</option>
                  <option v-for="m in providers.find(p => p.name === activeProvider)?.models || []" :key="m" :value="m">{{ m }}</option>
                </select>
              </div>
            </div>
          </Card>

          <!-- Providers -->
          <div class="flex items-center justify-between">
            <h2 class="text-lg font-semibold">Providers</h2>
            <Button @click="showAddProvider = !showAddProvider">
              {{ showAddProvider ? 'Cancel' : '+ Add' }}
            </Button>
          </div>

          <div v-for="p in providers" :key="p.name" class="space-y-2">
            <Card>
              <div class="flex items-center justify-between mb-2">
                <div class="flex items-center gap-2">
                  <div class="w-2 h-2 rounded-full bg-green-500"></div>
                  <span class="font-medium">{{ p.name }}</span>
                </div>
                <button @click="removeProvider(p.name)" class="text-xs text-gray-500 hover:text-red-400">Remove</button>
              </div>
              <div class="text-xs text-gray-500 font-mono">{{ p.baseUrl }}</div>
              <div class="flex gap-2 mt-2 flex-wrap">
                <Badge v-for="m in p.models" :key="m" size="sm">{{ m }}</Badge>
                <span v-if="p.models.length === 0" class="text-xs text-gray-600">No models configured</span>
              </div>
            </Card>
          </div>

          <!-- Add Form -->
          <Card v-if="showAddProvider" title="Add Provider">
            <div class="space-y-3">
              <input v-model="newProvider.name" placeholder="Name (e.g., nan, deepseek)"
                class="w-full bg-[#1a1a1a] border border-[#2a2a2a] rounded-xl px-4 py-3 text-sm text-white placeholder-gray-500 focus:border-green-500/50" />
              <input v-model="newProvider.baseUrl" placeholder="Base URL (https://api.example.com/v1)"
                class="w-full bg-[#1a1a1a] border border-[#2a2a2a] rounded-xl px-4 py-3 text-sm text-white placeholder-gray-500 focus:border-green-500/50" />
              <input v-model="newProvider.apiKey" placeholder="API Key" type="password"
                class="w-full bg-[#1a1a1a] border border-[#2a2a2a] rounded-xl px-4 py-3 text-sm text-white placeholder-gray-500 focus:border-green-500/50" />
              <Button @click="addProvider" variant="primary">Save Provider</Button>
            </div>
          </Card>

          <div v-if="providers.length === 0 && !showAddProvider" class="text-center py-12 text-gray-500">
            <div class="text-4xl mb-3 opacity-30">🔌</div>
            <div class="text-sm">No providers configured</div>
          </div>
        </div>
      </div>

    </div>
  </div>
</template>