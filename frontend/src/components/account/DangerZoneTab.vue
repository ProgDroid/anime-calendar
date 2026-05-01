<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { applySettings } from '@/services/applySettings'
import api from '@/config/api'
import UiButton from '@/components/ui/UiButton.vue'
import ConfirmModal from '@/components/shared/ConfirmModal.vue'

defineOptions({ name: 'DangerZoneTab' })

const { t } = useI18n()
const router = useRouter()
const authStore = useAuthStore()
const userSettingsStore = useUserSettingsStore()

const isDeleting = ref(false)
const error = ref<string | null>(null)
const confirmDeleteOpen = ref(false)

const handleDelete = async () => {
  confirmDeleteOpen.value = false
  try {
    isDeleting.value = true
    await api.delete('/user')
    authStore.logout()
    userSettingsStore.clearCache()
    applySettings(userSettingsStore.getDefaultSettings())
    router.push('/login')
  } catch {
    error.value = t('userDetails.accountDeleteFailed')
  } finally {
    isDeleting.value = false
  }
}
</script>

<template>
  <section class="flex flex-col gap-4 max-w-xl">
    <ConfirmModal
      :open="confirmDeleteOpen"
      :title="t('userDetails.accountDeleteButton')"
      :message="t('userDetails.accountDeleteWarning')"
      :confirm-label="t('userDetails.accountDeleteButton')"
      :danger="true"
      @confirm="handleDelete"
      @cancel="confirmDeleteOpen = false"
    />
    <header class="mb-2">
      <p class="text-xs uppercase tracking-wider text-danger" data-testid="account-tab-eyebrow">{{ t('account.danger.eyebrow') }}</p>
      <h2 class="text-3xl md:text-4xl font-medium tracking-tight mt-1 text-fg-1" data-testid="account-tab-heading">
        {{ t('account.danger.headingLead') }}<span class="font-display italic"> {{ t('account.danger.headingItalic') }}</span>
      </h2>
      <p class="text-sm text-fg-2 mt-2">{{ t('account.danger.subtitle') }}</p>
    </header>
    <p class="text-sm text-fg-2">{{ t('userDetails.accountDeleteWarning') }}</p>
    <div v-if="error" class="text-danger" data-testid="danger-error">{{ error }}</div>
    <div class="flex justify-end">
      <UiButton
        variant="danger"
        :loading="isDeleting"
        :disabled="isDeleting"
        data-testid="delete-account-button"
        @click="confirmDeleteOpen = true"
      >
        {{ isDeleting ? t('userDetails.accountDeleting') : t('userDetails.accountDeleteButton') }}
      </UiButton>
    </div>
  </section>
</template>
