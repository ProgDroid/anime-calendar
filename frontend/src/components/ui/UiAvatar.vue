<script setup lang="ts">
import { computed } from 'vue'
import { tv } from 'tailwind-variants'

interface Props {
  src?: string
  alt: string
  size?: 'sm' | 'md' | 'lg'
  fallback?: string
}
const props = withDefaults(defineProps<Props>(), { src: '', size: 'md', fallback: '' })

defineOptions({ name: 'UiAvatar' })

const avatar = tv({
  base: 'inline-flex items-center justify-center rounded-full overflow-hidden bg-bg-2 text-fg-2 font-medium select-none',
  variants: {
    size: { sm: 'w-6 h-6 text-xs', md: 'w-8 h-8 text-sm', lg: 'w-12 h-12 text-base' },
  },
})
const classes = computed(() => avatar({ size: props.size }))
</script>

<template>
  <span :class="classes" :aria-label="alt">
    <img v-if="src" :src="src" :alt="alt" class="w-full h-full object-cover">
    <span v-else>{{ fallback }}</span>
  </span>
</template>
