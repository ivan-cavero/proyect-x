<script setup lang="ts">
const model = defineModel<string>({ required: true })

withDefaults(defineProps<{
  type?: string
  placeholder?: string
  disabled?: boolean
}>(), {
  type: 'text',
})
</script>

<template>
  <input
    v-model="model"
    :type="type"
    :placeholder="placeholder"
    :disabled="disabled"
    class="input"
  />
</template>
