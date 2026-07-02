<script setup lang="ts">
defineProps<{
  title?: string
  subtitle?: string
  padding?: boolean
  glow?: 'green' | 'amber' | 'crimson' | null
  hover?: boolean
}>()
</script>

<template>
  <div
    class="card"
    :class="{
      'card-glow': hover,
    }"
  >
    <!-- Accent line at top -->
    <div
      v-if="glow"
      class="accent-line"
      :class="{
        'accent-green': glow === 'green',
        'accent-amber': glow === 'amber',
        'accent-crimson': glow === 'crimson',
      }"
    />

    <div v-if="title" class="card-header">
      <div class="card-title">{{ title }}</div>
      <span v-if="subtitle" class="card-subtitle">{{ subtitle }}</span>
    </div>
    <div v-if="title" class="card-body">
      <slot />
    </div>
    <div v-else class="card-body">
      <slot />
    </div>
  </div>
</template>
