<script setup lang="ts">
import { onMounted, onBeforeUnmount, watch } from 'vue'

interface Props { open: boolean; closeOnScrim?: boolean; ariaLabel: string }
const props = withDefaults(defineProps<Props>(), { closeOnScrim: true })

defineOptions({ name: 'UiModal' })

const emit = defineEmits<{ (e: 'update:open', v: boolean): void; (e: 'close'): void }>()

function close() {
  emit('update:open', false)
  emit('close')
}

function onKey(ev: KeyboardEvent) {
  if (ev.key === 'Escape' && props.open) close()
}

function onScrimClick() {
  if (props.closeOnScrim) close()
}

onMounted(() => document.addEventListener('keydown', onKey))
onBeforeUnmount(() => document.removeEventListener('keydown', onKey))

watch(() => props.open, (v) => {
  document.body.style.overflow = v ? 'hidden' : ''
})
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
        role="dialog"
        aria-modal="true"
        :aria-label="ariaLabel"
        class="relative bg-bg-1 border border-line rounded-lg shadow-lg max-w-md w-full mx-4 transition-all duration-[var(--d-2)]"
      >
        <header v-if="$slots.header" class="p-4 border-b border-line-soft"><slot name="header" /></header>
        <div class="p-4"><slot /></div>
        <footer v-if="$slots.footer" class="p-4 border-t border-line-soft"><slot name="footer" /></footer>
      </div>
    </div>
  </Teleport>
</template>
