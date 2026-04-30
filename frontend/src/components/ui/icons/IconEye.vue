<script setup lang="ts">
interface Props { ariaLabel?: string }
const props = defineProps<Props>()
defineOptions({ name: 'IconEye' })
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    width="24"
    height="24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.6"
    stroke-linecap="round"
    stroke-linejoin="round"
    :aria-hidden="!props.ariaLabel"
    :aria-label="props.ariaLabel"
    :role="props.ariaLabel ? 'img' : undefined"
  >
    <path d="M2 12s4-8 10-8 10 8 10 8-4 8-10 8S2 12 2 12z" />
    <circle cx="12" cy="12" r="3" />
  </svg>
</template>
