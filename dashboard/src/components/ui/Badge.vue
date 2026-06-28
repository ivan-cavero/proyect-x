<script setup lang="ts">
defineProps<{
  variant?: 'green' | 'gray' | 'red' | 'amber'
  size?: 'sm' | 'md'
}>()
</script>

<template>
  <span class="inline-flex items-center rounded-full font-medium"
    :class="[
      size === 'sm' ? 'text-[10px] px-2 py-0.5' : 'text-xs px-2.5 py-1',
      variant === 'green' ? 'bg-green-500/10 text-green-400 border border-green-500/20' :
      variant === 'red' ? 'bg-red-500/10 text-red-400 border border-red-500/20' :
      variant === 'amber' ? 'bg-amber-500/10 text-amber-400 border border-amber-500/20' :
      'bg-white/5 text-gray-400 border border-[#2a2a2a]'
    ]">
    <slot />
  </span>
</template>