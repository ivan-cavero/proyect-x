<script setup lang="ts">
defineProps<{
  modelValue?: string
  placeholder?: string
  type?: string
}>()

defineEmits(['update:modelValue'])
</script>

<template>
  <input
    :type="type || 'text'"
    :value="modelValue"
    @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    :placeholder="placeholder"
    class="w-full bg-[#1a1a1a] border border-[#2a2a2a] rounded-xl px-4 py-3 text-sm text-white placeholder-gray-500
           focus:outline-none focus:border-green-500/50 transition-colors duration-200"
  />
</template>