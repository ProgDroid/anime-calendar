<script setup lang="ts">
import { onMounted, onBeforeUnmount, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { tv } from 'tailwind-variants'
import IconCheck from './icons/IconCheck.vue'
import IconX from './icons/IconX.vue'

interface Props {
  message: string
  variant?: 'info' | 'success' | 'warning' | 'danger'
  duration?: number
}
const props = withDefaults(defineProps<Props>(), { variant: 'info', duration: 4000 })
defineOptions({ name: 'UiToast' })
const emit = defineEmits<{ (e: 'dismiss'): void }>()

const { t } = useI18n()

let timer: ReturnType<typeof setTimeout> | null = null

// Solid surface + backdrop blur prevents the bleed-through that occurred
// with the previous translucent variant fills. Variant only colors the
// left accent strip, the icon badge, and the border tint.
const toast = tv({
  base: 'relative flex items-start gap-3 pl-4 pr-2 py-3 rounded-lg shadow-xl border bg-bg-1/95 text-fg-1 backdrop-blur-md ring-1 ring-black/5 overflow-hidden motion-safe:animate-toast-in',
  variants: {
    variant: {
      info: 'border-line',
      success: 'border-success/40',
      warning: 'border-warning/40',
      danger: 'border-danger/40',
    },
  },
})

const accentClass = computed(() => {
  switch (props.variant) {
    case 'success': return 'bg-success'
    case 'warning': return 'bg-warning'
    case 'danger': return 'bg-danger'
    default: return 'bg-accent-1'
  }
})

const iconWrapClass = computed(() => {
  switch (props.variant) {
    case 'success': return 'text-success bg-success/15'
    case 'warning': return 'text-warning bg-warning/15'
    case 'danger': return 'text-danger-text bg-danger/15'
    default: return 'text-fg-2 bg-bg-2'
  }
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
    <span
      aria-hidden="true"
      class="absolute left-0 top-0 bottom-0 w-1"
      :class="accentClass"
    />
    <span
      aria-hidden="true"
      class="mt-0.5 inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-full [&_svg]:w-3.5 [&_svg]:h-3.5"
      :class="iconWrapClass"
    >
      <IconCheck v-if="variant === 'success'" />
      <IconX v-else-if="variant === 'danger'" />
      <span v-else-if="variant === 'warning'" class="text-sm font-bold leading-none">!</span>
      <span v-else class="text-sm font-bold leading-none">i</span>
    </span>
    <p class="flex-1 text-sm font-medium leading-snug pt-0.5">{{ message }}</p>
    <button
      type="button"
      :aria-label="t('common.close')"
      class="-mr-1 -mt-1 ml-1 inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-fg-2 transition-colors hover:bg-bg-2 hover:text-fg-1 motion-reduce:transition-none [&_svg]:w-3.5 [&_svg]:h-3.5"
      @click="emit('dismiss')"
    >
      <IconX />
    </button>
  </div>
</template>
