<script setup lang="ts">
import UiToast from '@/components/ui/UiToast.vue'
import { activeToasts, toastService } from '@/services/toastService'

defineOptions({ name: 'UiToastHost' })
</script>

<template>
  <Teleport to="body">
    <div class="fixed bottom-4 left-4 z-50 space-y-2 w-full max-w-xs">
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
