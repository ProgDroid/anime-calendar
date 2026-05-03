<script setup lang="ts">
import { onMounted, onBeforeUnmount, watch, ref, nextTick } from 'vue'
import { lockBodyScroll, unlockBodyScroll } from '@/composables/useBodyScrollLock'

interface Props { open: boolean; closeOnScrim?: boolean; ariaLabel: string }
const props = withDefaults(defineProps<Props>(), { closeOnScrim: true })

defineOptions({ name: 'UiModal' })

const emit = defineEmits<{ (e: 'update:open', v: boolean): void; (e: 'close'): void }>()

const dialogEl = ref<HTMLElement | null>(null)
let invoker: HTMLElement | null = null
// Per-instance flag (script setup runs once per component instance) so we
// pair exactly one lockBodyScroll() with one unlockBodyScroll() per open
// cycle, even if the modal unmounts while still open.
let didLock = false

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'

function getFocusable(): HTMLElement[] {
  if (!dialogEl.value) return []
  return Array.from(dialogEl.value.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (el) => !el.hasAttribute('disabled'),
  )
}

function close() {
  emit('update:open', false)
  emit('close')
}

function onKey(ev: KeyboardEvent) {
  if (!props.open) return
  if (ev.key === 'Escape') {
    close()
    return
  }
  if (ev.key === 'Tab') {
    const focusable = getFocusable()
    if (focusable.length === 0) {
      ev.preventDefault()
      dialogEl.value?.focus()
      return
    }
    const first = focusable[0]!
    const last = focusable[focusable.length - 1]!
    const active = document.activeElement as HTMLElement | null
    if (ev.shiftKey) {
      if (active === first || !dialogEl.value?.contains(active)) {
        ev.preventDefault()
        last.focus()
      }
    } else {
      if (active === last || !dialogEl.value?.contains(active)) {
        ev.preventDefault()
        first.focus()
      }
    }
  }
}

function onScrimClick() {
  if (props.closeOnScrim) close()
}

onMounted(() => document.addEventListener('keydown', onKey))
onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKey)
  if (didLock) {
    unlockBodyScroll()
    didLock = false
  }
})

watch(
  () => props.open,
  async (v) => {
    if (v && !didLock) {
      lockBodyScroll()
      didLock = true
    } else if (!v && didLock) {
      unlockBodyScroll()
      didLock = false
    }
    if (v) {
      invoker = (document.activeElement as HTMLElement) ?? null
      await nextTick()
      const focusable = getFocusable()
      if (focusable.length > 0) {
        focusable[0]!.focus()
      } else {
        dialogEl.value?.focus()
      }
    } else if (invoker && typeof invoker.focus === 'function') {
      invoker.focus()
      invoker = null
    }
  },
)
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center">
      <div
        data-testid="scrim"
        class="absolute inset-0 bg-bg-0/70 backdrop-blur-sm transition-opacity duration-[var(--d-2)]"
        @click="onScrimClick"
      />
      <div
        ref="dialogEl"
        role="dialog"
        aria-modal="true"
        :aria-label="ariaLabel"
        tabindex="-1"
        class="relative bg-bg-1 border border-line rounded-lg shadow-lg max-w-md w-full mx-4 transition-all duration-[var(--d-2)] focus:outline-none"
      >
        <header v-if="$slots.header" class="p-4 border-b border-line-soft"><slot name="header" /></header>
        <div class="p-4"><slot /></div>
        <footer v-if="$slots.footer" class="p-4 border-t border-line-soft"><slot name="footer" /></footer>
      </div>
    </div>
  </Teleport>
</template>
