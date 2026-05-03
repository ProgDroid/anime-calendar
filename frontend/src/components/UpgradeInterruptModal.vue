<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useViewportLayout } from '@/composables/useViewportLayout'
import UiModal from '@/components/ui/UiModal.vue'
import UiBottomSheet from '@/components/ui/UiBottomSheet.vue'
import UpgradeInterruptModalBody from '@/components/UpgradeInterruptModalBody.vue'

defineOptions({ name: 'UpgradeInterruptModal' })

interface Props {
  open: boolean
}
const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [boolean]
  close: []
}>()

const { t } = useI18n()
const { isMobile } = useViewportLayout()

// UiModal uses `:open` prop + `update:open` emit.
// UiBottomSheet uses `v-model` (modelValue / update:modelValue).
const modelValue = computed(() => props.open)

function onClose() {
  emit('update:open', false)
  emit('close')
}
</script>

<template>
  <!-- Mobile: bottom sheet -->
  <UiBottomSheet
    v-if="isMobile"
    :model-value="modelValue"
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
