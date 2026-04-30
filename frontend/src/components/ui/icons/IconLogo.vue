<script setup lang="ts">
interface Props { ariaLabel?: string }
const props = defineProps<Props>()
defineOptions({ name: 'IconLogo' })
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 32 32"
    width="24"
    height="24"
    fill="none"
    :aria-hidden="!props.ariaLabel"
    :aria-label="props.ariaLabel"
    :role="props.ariaLabel ? 'img' : undefined"
  >
    <circle cx="16" cy="16" r="14" stroke="currentColor" stroke-width="1.5" />
    <path d="M16 4 v24 M4 16 h24" stroke="currentColor" stroke-width="1" opacity="0.4" />
    <circle cx="16" cy="16" r="4" fill="currentColor" />
  </svg>
</template>
