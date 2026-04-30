<script setup lang="ts">
interface Props { ariaLabel?: string }
const props = defineProps<Props>()
defineOptions({ name: 'IconSearch' })
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    width="24"
    height="24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    :aria-hidden="!props.ariaLabel"
    :aria-label="props.ariaLabel"
    :role="props.ariaLabel ? 'img' : undefined"
  >
    <circle cx="11" cy="11" r="7" />
    <path d="m20 20-3.5-3.5" />
  </svg>
</template>
