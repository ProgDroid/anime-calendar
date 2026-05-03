<script setup lang="ts">
import { onBeforeUnmount, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { lockBodyScroll, unlockBodyScroll } from '@/composables/useBodyScrollLock'
import IconX from './icons/IconX.vue'

defineOptions({ name: 'UiBottomSheet' })

interface Props {
  modelValue: boolean
}
const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()

const { t } = useI18n()

function close() {
  emit('update:modelValue', false)
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.modelValue) close()
}

// Per-instance flag (script setup runs once per component instance) so we
// pair exactly one lockBodyScroll() with one unlockBodyScroll() per open
// cycle, even if the watcher fires unexpectedly or the component unmounts
// while still open.
let didLock = false

watch(
  () => props.modelValue,
  (open) => {
    if (open && !didLock) {
      lockBodyScroll()
      document.addEventListener('keydown', onKeydown)
      didLock = true
    } else if (!open && didLock) {
      unlockBodyScroll()
      document.removeEventListener('keydown', onKeydown)
      didLock = false
    }
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  if (didLock) {
    unlockBodyScroll()
    document.removeEventListener('keydown', onKeydown)
    didLock = false
  }
})
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition-opacity duration-200 motion-reduce:transition-none motion-reduce:duration-0"
      enter-from-class="opacity-0"
      leave-active-class="transition-opacity duration-200 motion-reduce:transition-none motion-reduce:duration-0"
      leave-to-class="opacity-0"
    >
      <div
        v-if="modelValue"
        data-testid="bottom-sheet-backdrop"
        class="fixed inset-0 z-40 bg-black/40"
        @click="close"
      />
    </Transition>
    <Transition
      enter-active-class="transition-transform duration-300 motion-reduce:transition-none motion-reduce:duration-0"
      enter-from-class="translate-y-full"
      leave-active-class="transition-transform duration-200 motion-reduce:transition-none motion-reduce:duration-0"
      leave-to-class="translate-y-full"
    >
      <div
        v-if="modelValue"
        data-testid="bottom-sheet-panel"
        role="dialog"
        aria-modal="true"
        class="fixed bottom-0 left-0 right-0 z-50 rounded-t-[28px] border-t border-line-soft bg-bg-1 shadow-[0_-16px_50px_rgba(0,0,0,0.35)]"
        :style="{ paddingBottom: 'max(36px, calc(env(safe-area-inset-bottom) + 16px))' }"
        @click.stop
      >
        <!-- TODO(track-3-a11y): focus-trap inside panel + initial focus on open + restore focus on close. Track 2 a11y memory `feedback_jsdom_focusable_offsetparent` flags the jsdom Teleport pitfall to watch for in tests. -->
        <div
          data-testid="bottom-sheet-handle"
          aria-hidden="true"
          class="mx-auto mt-3 mb-4 h-1 w-9 rounded-full bg-line"
        />
        <button
          type="button"
          data-testid="bottom-sheet-close"
          :aria-label="t('common.close')"
          class="absolute right-4 top-4 rounded-full p-2 text-fg-2 hover:bg-bg-2 focus-visible:outline-2 focus-visible:outline-accent-1 focus-visible:outline-offset-2"
          @click="close"
        >
          <IconX />
        </button>
        <slot />
      </div>
    </Transition>
  </Teleport>
</template>
