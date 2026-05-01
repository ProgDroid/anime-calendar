<script setup lang="ts">
import { ref, onBeforeUnmount, useId, nextTick, watch } from 'vue';
import { tv } from 'tailwind-variants';

defineOptions({ name: 'UiMenu' });

interface Props {
  align?: 'left' | 'right';
}
const props = withDefaults(defineProps<Props>(), { align: 'right' });

const open = ref(false);
const root = ref<HTMLElement | null>(null);
const panel = ref<HTMLElement | null>(null);
const panelId = useId();
let invoker: HTMLElement | null = null;

watch(open, (next, prev) => {
  if (next && !prev) {
    const active = document.activeElement;
    invoker = active instanceof HTMLElement ? active : null;
  } else if (!next && prev) {
    const target = invoker;
    invoker = null;
    if (target && document.contains(target)) {
      void nextTick(() => target.focus());
    }
  }
});

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

function getItems(): HTMLElement[] {
  if (!panel.value) return [];
  return Array.from(
    panel.value.querySelectorAll<HTMLElement>(
      '[role="menuitem"], a, button:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ),
  );
}

async function focusFirst() {
  await nextTick();
  getItems()[0]?.focus();
}

async function focusLast() {
  await nextTick();
  const items = getItems();
  items[items.length - 1]?.focus();
}

function toggle() {
  open.value = !open.value;
  if (open.value) void focusFirst();
}

function closeOnOutside(e: MouseEvent) {
  if (!root.value) return;
  if (!root.value.contains(e.target as Node)) open.value = false;
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    open.value = false;
    return;
  }
  if (!open.value) return;
  const items = getItems();
  if (items.length === 0) return;
  const active = document.activeElement as HTMLElement | null;
  const idx = active ? items.indexOf(active) : -1;
  if (e.key === 'ArrowDown') {
    e.preventDefault();
    items[(idx + 1) % items.length]!.focus();
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    items[idx <= 0 ? items.length - 1 : idx - 1]!.focus();
  } else if (e.key === 'Home') {
    e.preventDefault();
    items[0]!.focus();
  } else if (e.key === 'End') {
    e.preventDefault();
    items[items.length - 1]!.focus();
  }
}

function onTriggerKey(e: KeyboardEvent) {
  if (!open.value && (e.key === 'ArrowDown' || e.key === 'Enter' || e.key === ' ')) {
    e.preventDefault();
    open.value = true;
    void focusFirst();
  } else if (!open.value && e.key === 'ArrowUp') {
    e.preventDefault();
    open.value = true;
    void focusLast();
  }
}

document.addEventListener('mousedown', closeOnOutside);
onBeforeUnmount(() => document.removeEventListener('mousedown', closeOnOutside));
</script>

<template>
  <div ref="root" class="relative inline-block" @keydown="onKeydown">
    <span @click="toggle" @keydown="onTriggerKey">
      <slot name="trigger" :open="open" :panel-id="panelId" />
    </span>
    <div
      v-if="open"
      ref="panel"
      :id="panelId"
      :class="classes.panel()"
      role="menu"
      @click="open = false"
    >
      <slot />
    </div>
  </div>
</template>
