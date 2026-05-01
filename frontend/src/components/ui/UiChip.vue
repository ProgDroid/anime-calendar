<script setup lang="ts">
import { computed } from 'vue'
import { tv } from 'tailwind-variants'

interface Props {
  variant?: 'default' | 'anime' | 'manga' | 'success' | 'warning' | 'pro'
  size?: 'sm' | 'md'
}
const props = withDefaults(defineProps<Props>(), { variant: 'default', size: 'sm' })

defineOptions({ name: 'UiChip' })

const chip = tv({
  base: 'inline-flex items-center rounded-full font-medium',
  variants: {
    variant: {
      default: 'bg-bg-2 text-fg-2',
      anime: 'bg-accent-1-soft text-accent-1-text',
      manga: 'bg-accent-2/20 text-accent-2',
      success: 'bg-success/20 text-success',
      warning: 'bg-warning/20 text-warning',
      pro: 'bg-accent-1 text-bg-0',
    },
    size: { sm: 'h-5 px-2 text-xs', md: 'h-6 px-3 text-sm' },
  },
})
const classes = computed(() => chip({ variant: props.variant, size: props.size }))
</script>

<template>
  <span :class="classes"><slot /></span>
</template>
