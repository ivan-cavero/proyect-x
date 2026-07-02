<script setup lang="ts">
withDefaults(defineProps<{
  variant?: 'green' | 'amber' | 'crimson' | 'gray' | 'emerald'
  size?: 'sm' | 'md'
}>(), {
  variant: 'gray',
  size: 'md',
})
</script>

<template>
  <span
    class="badge"
    :class="{
      'badge-green': variant === 'green',
      'badge-emerald': variant === 'emerald',
      'badge-amber': variant === 'amber',
      'badge-crimson': variant === 'crimson',
      'badge-gray': variant === 'gray',
      'badge-sm': size === 'sm',
    }"
  >
    <slot />
  </span>
</template>

<style scoped>
.badge-sm {
  padding: 2px 6px;
  font-size: 10px;
}
</style>
