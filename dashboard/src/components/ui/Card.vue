<script setup lang="ts">
defineProps<{
  title?: string
  padding?: boolean
}>()
</script>

<template>
  <div class="bg-[#111111] border border-[#1f1f1f] rounded-2xl"
       :class="padding !== false ? 'p-5' : ''">
    <div v-if="title" class="text-[11px] font-medium text-gray-500 uppercase tracking-wider mb-3">{{ title }}</div>
    <slot />
  </div>
</template>