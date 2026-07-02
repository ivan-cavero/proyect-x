<script setup lang="ts">
defineProps<{
  label: string
  value: string | number
  sub?: string
  color?: 'green' | 'emerald' | 'amber' | 'crimson'
}>()
</script>

<template>
  <div class="metric-card">
    <div class="metric-label">{{ label }}</div>
    <div class="metric-value" :class="color">{{ value }}</div>
    <div v-if="sub" class="metric-sub">{{ sub }}</div>
  </div>
</template>
