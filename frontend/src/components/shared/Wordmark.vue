<script setup lang="ts">
/* eslint-disable vue/multi-word-component-names -- "Wordmark" is the design-system brand component name */
defineOptions({ name: 'Wordmark' })
withDefaults(defineProps<{ size?: 'sm' | 'md' | 'lg' }>(), { size: 'md' })
</script>

<template>
  <span
    :data-size="size"
    class="font-display tracking-tight"
    :class="{
      'text-lg': size === 'sm',
      'text-2xl': size === 'md',
      'text-4xl': size === 'lg',
    }"
  >
    Anime Calendar
  </span>
</template>
