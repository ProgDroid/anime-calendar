<script setup lang="ts">
defineOptions({ name: 'UiAuthShell' })

defineProps<{
  /** Test id applied to the left poster collage pane. Pane is hidden when omitted. */
  posterTestid?: string
  /** Optional test id applied to the inner card wrapper. */
  cardTestid?: string
}>()
</script>

<template>
  <div class="min-h-[calc(100vh-6rem)] flex bg-bg-1">
    <div v-if="posterTestid" :data-testid="posterTestid" class="hidden lg:block flex-1 bg-bg-2" />

    <div class="flex-1 flex items-center justify-center p-6">
      <div
        :data-testid="cardTestid"
        class="w-full max-w-[420px] flex flex-col gap-6 bg-bg-1 rounded-xl p-8 border border-line"
      >
        <slot />
      </div>
    </div>
  </div>
</template>
