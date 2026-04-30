<script setup lang="ts">
interface Props { ariaLabel?: string }
const props = defineProps<Props>()
defineOptions({ name: 'IconX' })
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
    <path d="M18 6 6 18M6 6l12 12" />
  </svg>
</template>
