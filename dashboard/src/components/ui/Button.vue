<script setup lang="ts">
const props = withDefaults(defineProps<{
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger'
  size?: 'sm' | 'md' | 'lg'
  disabled?: boolean
  block?: boolean
}>(), {
  variant: 'ghost',
  size: 'md',
  block: false,
})
</script>

<template>
  <button
    class="btn"
    :class="{
      'btn-primary': variant === 'primary',
      'btn-secondary': variant === 'secondary',
      'btn-danger': variant === 'danger',
      'btn-ghost': variant === 'ghost',
      'btn-sm': size === 'sm',
      'btn-block': block,
    }"
    :disabled="disabled"
  >
    <slot />
  </button>
</template>
