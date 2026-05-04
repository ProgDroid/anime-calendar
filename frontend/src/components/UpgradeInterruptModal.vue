<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useViewportLayout } from '@/composables/useViewportLayout'
import type { UpgradeReason } from '@/composables/useUpgradeInterrupt'
import UiModal from '@/components/ui/UiModal.vue'
import UiBottomSheet from '@/components/ui/UiBottomSheet.vue'
import UpgradeInterruptModalBody from '@/components/UpgradeInterruptModalBody.vue'

defineOptions({ name: 'UpgradeInterruptModal' })

interface Props {
  open: boolean
  /**
   * Drives heading + description copy in the body. Defaults to `pro_accent`
   * which matches the backend's behaviour when it omits the reason.
   */
  reason?: UpgradeReason
}
withDefaults(defineProps<Props>(), { reason: 'pro_accent' })
const emit = defineEmits<{
  'update:open': [boolean]
  close: []
}>()

const { t } = useI18n()
const { isMobile } = useViewportLayout()

function onClose() {
  emit('update:open', false)
  emit('close')
}
</script>

<template>
  <!-- Mobile: bottom sheet -->
  <UiBottomSheet
    v-if="isMobile"
    :model-value="open"
    :ariaLabel="t('interrupt.ariaLabel')"
    @update:model-value="(v) => { emit('update:open', v); if (!v) emit('close') }"
  >
    <UpgradeInterruptModalBody :reason="reason" @close="onClose" />
  </UiBottomSheet>

  <!-- Desktop: modal dialog -->
  <UiModal
    v-else
    :open="open"
    :ariaLabel="t('interrupt.ariaLabel')"
    @update:open="(v) => emit('update:open', v)"
    @close="emit('close')"
  >
    <UpgradeInterruptModalBody :reason="reason" @close="onClose" />
  </UiModal>
</template>
