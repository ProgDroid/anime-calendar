<script setup lang="ts">
import { useI18n } from 'vue-i18n'

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
  <dialog v-if="open" class="modal modal-open">
    <div class="modal-box" data-testid="modal-box">
      <h3 class="font-bold text-lg">{{ title }}</h3>
      <p class="py-4">{{ message }}</p>
      <div class="modal-action">
        <button
          data-testid="cancel-btn"
          class="btn btn-ghost"
          @click="emit('cancel')"
        >
          {{ cancelLabel ?? t('app.cancel') }}
        </button>
        <button
          data-testid="confirm-btn"
          :class="['btn', danger ? 'btn-error' : 'btn-primary']"
          @click="emit('confirm')"
        >
          {{ confirmLabel ?? t('app.confirm') }}
        </button>
      </div>
    </div>
    <div class="modal-backdrop" @click="emit('cancel')" />
  </dialog>
</template>
