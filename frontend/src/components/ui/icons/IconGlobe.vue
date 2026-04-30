<script setup lang="ts">
interface Props { ariaLabel?: string }
const props = defineProps<Props>()
defineOptions({ name: 'IconGlobe' })
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    width="24"
    height="24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
    :aria-hidden="!props.ariaLabel"
    :aria-label="props.ariaLabel"
    :role="props.ariaLabel ? 'img' : undefined"
  >
    <circle cx="12" cy="12" r="9" />
    <path d="M3 12h18M12 3a13 13 0 0 1 0 18M12 3a13 13 0 0 0 0 18" />
  </svg>
</template>
