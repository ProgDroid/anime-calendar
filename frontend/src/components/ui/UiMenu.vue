<script setup lang="ts">
import { ref, onBeforeUnmount } from 'vue';
import { tv } from 'tailwind-variants';

defineOptions({ name: 'UiMenu' });

interface Props {
  align?: 'left' | 'right';
}
const props = withDefaults(defineProps<Props>(), { align: 'right' });

const open = ref(false);
const root = ref<HTMLElement | null>(null);

const menu = tv({
  slots: {
    panel:
      'absolute z-20 mt-1 min-w-[12rem] rounded-md border border-line bg-bg-1 shadow-lg p-1 flex flex-col',
  },
  variants: {
    align: {
      left: { panel: 'left-0' },
      right: { panel: 'right-0' },
    },
  },
});

const classes = menu({ align: props.align });

function toggle() {
  open.value = !open.value;
}

function closeOnOutside(e: MouseEvent) {
  if (!root.value) return;
  if (!root.value.contains(e.target as Node)) open.value = false;
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') open.value = false;
}

document.addEventListener('mousedown', closeOnOutside);
onBeforeUnmount(() => document.removeEventListener('mousedown', closeOnOutside));
</script>

<template>
  <div ref="root" class="relative inline-block" @keydown="onKeydown">
    <span @click="toggle">
      <slot name="trigger" :open="open" />
    </span>
    <div
      v-if="open"
      :class="classes.panel()"
      role="menu"
      @click="open = false"
    >
      <slot />
    </div>
  </div>
</template>
