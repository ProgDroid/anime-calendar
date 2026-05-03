<script setup lang="ts">
import { defineAsyncComponent } from 'vue'
import { useViewportLayout } from '@/composables/useViewportLayout'

defineOptions({ name: 'CalendarEditorView' })

const { isMobile } = useViewportLayout()

// Lazy-load each variant so users only download the one their viewport uses.
const Desktop = defineAsyncComponent(
  () => import('./CalendarEditorViewDesktop.vue'),
)
const Mobile = defineAsyncComponent(
  () => import('./CalendarEditorViewMobile.vue'),
)
</script>

<template>
  <component :is="isMobile ? Mobile : Desktop" />
</template>
