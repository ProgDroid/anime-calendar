<script setup lang="ts">
import { computed } from 'vue'
import { tv } from 'tailwind-variants'

interface Props {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger'
  size?: 'sm' | 'md' | 'lg'
  loading?: boolean
  disabled?: boolean
  type?: 'button' | 'submit'
}
const props = withDefaults(defineProps<Props>(), {
  variant: 'primary',
  size: 'md',
  loading: false,
  disabled: false,
  type: 'button',
})

defineOptions({ name: 'UiButton' })

const emit = defineEmits<{ (e: 'click', ev: MouseEvent): void }>()

const button = tv({
  base: 'inline-flex items-center justify-center gap-2 font-medium transition-all rounded-md select-none disabled:opacity-50 disabled:pointer-events-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-1-soft hover:-translate-y-[0.5px] active:translate-y-0',
  variants: {
    variant: {
      primary: 'bg-accent-1 text-bg-0 hover:shadow-[0_6px_18px_var(--accent-1-glow)]',
      secondary: 'bg-bg-2 text-fg-1 border border-line',
      ghost: 'bg-transparent text-fg-1 hover:bg-bg-2',
      danger: 'bg-danger text-bg-0',
    },
    size: {
      sm: 'h-8 px-3 text-sm',
      md: 'h-10 px-4 text-base',
      lg: 'h-12 px-6 text-md',
    },
  },
})

const classes = computed(() => button({ variant: props.variant, size: props.size }))

function onClick(ev: MouseEvent) {
  if (props.disabled || props.loading) return
  emit('click', ev)
}
</script>

<template>
  <button :class="classes" :type="type" :disabled="disabled || loading" @click="onClick">
    <span v-if="loading" data-testid="spinner" class="inline-block w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin" />
    <slot v-else name="icon-left" />
    <slot />
    <slot name="icon-right" />
  </button>
</template>
