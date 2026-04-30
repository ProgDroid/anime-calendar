<script setup lang="ts">
interface Props { ariaLabel?: string }
const props = defineProps<Props>()
defineOptions({ name: 'IconGoogle' })
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 48 48"
    width="24"
    height="24"
    :aria-hidden="!props.ariaLabel"
    :aria-label="props.ariaLabel"
    :role="props.ariaLabel ? 'img' : undefined"
  >
    <path fill="#FFC107" d="M43.6 20.5H42V20H24v8h11.3C33.7 32.4 29.3 35.5 24 35.5c-6.4 0-11.5-5.1-11.5-11.5S17.6 12.5 24 12.5c2.9 0 5.6 1.1 7.6 2.9l5.7-5.7C33.7 6.5 29.1 4.5 24 4.5 13.2 4.5 4.5 13.2 4.5 24S13.2 43.5 24 43.5c10.7 0 19.5-8.7 19.5-19.5 0-1.2-.1-2.3-.4-3.5z" />
    <path fill="#FF3D00" d="m6.3 14.1 6.6 4.8C14.7 15.2 19 12.5 24 12.5c2.9 0 5.6 1.1 7.6 2.9l5.7-5.7C33.7 6.5 29.1 4.5 24 4.5 16.3 4.5 9.7 8.4 6.3 14.1z" />
    <path fill="#4CAF50" d="M24 43.5c5 0 9.6-1.9 13.1-5l-6.1-5c-2 1.4-4.4 2.2-7 2.2-5.3 0-9.7-3.1-11.3-7.4l-6.5 5C9.6 39.3 16.2 43.5 24 43.5z" />
    <path fill="#1976D2" d="M43.6 20.5H42V20H24v8h11.3c-.8 2.4-2.4 4.4-4.3 5.6l6.1 5c-.4.4 6.4-4.7 6.4-14.6 0-1.2-.1-2.3-.4-3.5z" />
  </svg>
</template>
