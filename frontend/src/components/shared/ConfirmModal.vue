<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'

const { t } = useI18n()

defineProps<{
  title: string
  message: string
  open: boolean
  confirmLabel?: string
  cancelLabel?: string
  danger?: boolean
}>()

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()
</script>

<template>
  <UiModal :open="open" :ariaLabel="title" @close="emit('cancel')">
    <template #header>
      <h3 class="font-semibold text-lg text-fg-1">{{ title }}</h3>
    </template>
    <div data-testid="modal-box">
      <p class="text-fg-2">{{ message }}</p>
    </div>
    <template #footer>
      <div class="flex justify-end gap-2">
        <UiButton
          data-testid="cancel-btn"
          variant="ghost"
          @click="emit('cancel')"
        >
          {{ cancelLabel ?? t('app.cancel') }}
        </UiButton>
        <UiButton
          data-testid="confirm-btn"
          :variant="danger ? 'danger' : 'primary'"
          @click="emit('confirm')"
        >
          {{ confirmLabel ?? t('app.confirm') }}
        </UiButton>
      </div>
    </template>
  </UiModal>
</template>
