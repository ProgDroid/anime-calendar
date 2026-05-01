<script setup lang="ts">
import { onMounted, onBeforeUnmount, computed } from 'vue'
import { tv } from 'tailwind-variants'

interface Props {
  message: string
  variant?: 'info' | 'success' | 'warning' | 'danger'
  duration?: number
}
const props = withDefaults(defineProps<Props>(), { variant: 'info', duration: 4000 })
defineOptions({ name: 'UiToast' })
const emit = defineEmits<{ (e: 'dismiss'): void }>()

let timer: ReturnType<typeof setTimeout> | null = null

const toast = tv({
  base: 'px-4 py-3 rounded-md shadow-md border',
  variants: {
    variant: {
      info: 'bg-bg-1 text-fg-1 border-line',
      success: 'bg-success/15 text-success border-success/30',
      warning: 'bg-warning/15 text-warning border-warning/30',
      danger: 'bg-danger/15 text-danger-text border-danger/30',
    },
  },
})
const classes = computed(() => toast({ variant: props.variant }))

onMounted(() => {
  if (props.duration > 0) {
    timer = setTimeout(() => emit('dismiss'), props.duration)
  }
})

onBeforeUnmount(() => { if (timer) clearTimeout(timer) })
</script>

<template>
  <div
    :role="props.variant === 'danger' || props.variant === 'warning' ? 'alert' : 'status'"
    :class="classes"
  >
    {{ message }}
  </div>
</template>
