<script setup lang="ts">
defineProps<{ title: string; body: string }>()
defineOptions({ name: 'UiEmptyState' })
</script>

<template>
  <div class="flex flex-col items-center justify-center text-center gap-4 p-8">
    <div v-if="$slots.illustration" class="opacity-80">
      <slot name="illustration" />
    </div>
    <h2 data-testid="empty-state-title" class="text-2xl font-display text-fg-1">{{ title }}</h2>
    <p data-testid="empty-state-body" class="text-base text-fg-2 max-w-prose">{{ body }}</p>
    <div v-if="$slots.action">
      <slot name="action" />
    </div>
  </div>
</template>
