<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import UiToast from '@/components/ui/UiToast.vue'
import { activeToasts, toastService } from '@/services/toastService'
import { useViewportLayout } from '@/composables/useViewportLayout'

defineOptions({ name: 'UiToastHost' })

const route = useRoute()
const { isMobile } = useViewportLayout()

// Mirror UiBottomTabBar's visibility rule so toasts clear the tab bar on
// mobile, but keep the tight 16px offset everywhere else.
const bottomBarVisible = computed(
  () => isMobile.value && route.meta.bottomTabBar !== false,
)

const bottomOffset = computed(() =>
  bottomBarVisible.value
    ? 'calc(72px + env(safe-area-inset-bottom))'
    : '1rem',
)
</script>

<template>
  <Teleport to="body">
    <div
      class="fixed left-4 z-50 space-y-2 w-full max-w-xs"
      :style="{ bottom: bottomOffset }"
      aria-live="polite"
      aria-atomic="false"
    >
      <UiToast
        v-for="t in activeToasts"
        :key="t.id"
        :message="t.message"
        :variant="t.variant"
        :duration="0"
        @dismiss="toastService.hide(t.id)"
      />
    </div>
  </Teleport>
</template>
