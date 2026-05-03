<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useViewportLayout } from '@/composables/useViewportLayout'
import UiModal from '@/components/ui/UiModal.vue'
import UiBottomSheet from '@/components/ui/UiBottomSheet.vue'
import UpgradeInterruptModalBody from '@/components/UpgradeInterruptModalBody.vue'

defineOptions({ name: 'UpgradeInterruptModal' })

interface Props {
  open: boolean
}
defineProps<Props>()
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
    <UpgradeInterruptModalBody @close="onClose" />
  </UiBottomSheet>

  <!-- Desktop: modal dialog -->
  <UiModal
    v-else
    :open="open"
    :ariaLabel="t('interrupt.ariaLabel')"
    @update:open="(v) => emit('update:open', v)"
    @close="emit('close')"
  >
    <UpgradeInterruptModalBody @close="onClose" />
  </UiModal>
</template>
