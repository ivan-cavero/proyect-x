<script setup lang="ts">
import { ref } from 'vue'
import Icon from '../components/ui/Icon.vue'

const emit = defineEmits<{
  login: [token: string]
}>()

const token = ref('')
const error = ref('')
const isLoading = ref(false)

async function handleLogin() {
  if (!token.value.trim()) {
    error.value = 'Enter your access token'
    return
  }

  isLoading.value = true
  error.value = ''

  try {
    const response = await fetch('/api/health', {
      headers: { 'Authorization': `Bearer ${token.value}` }
    })

    if (response.ok) {
      localStorage.setItem('praxis-token', token.value)
      emit('login', token.value)
    } else {
      error.value = 'Invalid token or server unavailable'
    }
  } catch (fetchError) {
    localStorage.setItem('praxis-token', token.value)
    emit('login', token.value)
  } finally {
    isLoading.value = false
  }
}
</script>

<template>
  <div class="login-screen">

    <!-- Background Grid -->
    <div class="grid-bg" />

    <!-- Background Glow -->
    <div class="login-glow" />

    <!-- Login Card -->
    <div class="login-card">
      <!-- Logo -->
      <div class="text-center mb-8">
        <div class="logo-symbol">X</div>
        <h1 class="login-title">praxis</h1>
        <p class="login-subtitle">Neural Command Center</p>
      </div>

      <!-- Form -->
      <div class="login-form">
        <div>
          <label class="data-label block mb-1">Access Token</label>
          <div class="input-with-icon">
            <input
              v-model="token"
              type="password"
              placeholder="Paste your JWT token..."
              class="input"
              @keydown.enter="handleLogin"
            />
            <Icon name="send" :size="14" class="input-icon" />
          </div>
        </div>

        <!-- Error -->
        <div v-if="error" class="error-banner">
          <Icon name="alert" :size="12" />
          {{ error }}
        </div>

        <!-- Login Button -->
        <button
          @click="handleLogin"
          :disabled="isLoading || !token.trim()"
          class="btn btn-primary"
        >
          <Icon v-if="!isLoading" name="login" :size="14" />
          <span v-if="isLoading" class="loading-spinner" />
          <span>{{ isLoading ? 'Authenticating...' : 'Access System' }}</span>
        </button>

        <!-- Footer -->
        <div class="login-footer">
          <p>Token is stored locally in your browser</p>
        </div>
      </div>

      <!-- Skip (dev mode) -->
      <div class="text-center mt-4">
        <button
          @click="emit('login', 'dev-mode')"
          class="skip-link"
        >
          Skip authentication (dev mode)
        </button>
      </div>
    </div>
  </div>
</template>
