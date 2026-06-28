<script setup lang="ts">
defineProps<{
  variant?: 'primary' | 'secondary' | 'danger'
  size?: 'sm' | 'md' | 'lg'
  disabled?: boolean
}>()
</script>

<template>
  <button
    class="inline-flex items-center justify-center rounded-xl font-medium transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-green-500/20 disabled:opacity-50 disabled:cursor-not-allowed"
    :class="[
      variant === 'primary' ? 'bg-green-600 hover:bg-green-500 text-white' :
      variant === 'danger' ? 'bg-red-600/20 hover:bg-red-600/30 text-red-400' :
      'bg-[#1a1a1a] hover:bg-[#252525] text-gray-300 border border-[#2a2a2a]',
      size === 'lg' ? 'px-6 py-3 text-sm' :
      size === 'sm' ? 'px-3 py-1.5 text-xs' :
      'px-4 py-2.5 text-sm'
    ]"
  >
    <slot />
  </button>
</template>