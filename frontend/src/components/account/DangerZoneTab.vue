<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import axios from 'axios'
import { useAuthStore } from '@/stores/auth'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { applySettings } from '@/services/applySettings'
import api from '@/config/api'
import UiButton from '@/components/ui/UiButton.vue'
import ConfirmModal from '@/components/shared/ConfirmModal.vue'
import UiModal from '@/components/ui/UiModal.vue'

defineOptions({ name: 'DangerZoneTab' })

const { t } = useI18n()
const router = useRouter()
const authStore = useAuthStore()
const userSettingsStore = useUserSettingsStore()

const isDeleting = ref(false)
const isOpeningPortal = ref(false)
const error = ref<string | null>(null)
const portalError = ref<string | null>(null)
const confirmDeleteOpen = ref(false)
const activeSubModalOpen = ref(false)

const handleDelete = async () => {
  confirmDeleteOpen.value = false
  try {
    isDeleting.value = true
    await api.delete(`/user/${authStore.userId}`)
    authStore.logout()
    userSettingsStore.clearCache()
    applySettings(userSettingsStore.getDefaultSettings())
    router.push('/login')
  } catch (err) {
    if (
      axios.isAxiosError(err) &&
      err.response?.status === 409 &&
      err.response.data?.error === 'active_subscription'
    ) {
      portalError.value = null
      activeSubModalOpen.value = true
    } else {
      error.value = t('userDetails.accountDeleteFailed')
    }
  } finally {
    isDeleting.value = false
  }
}

const openStripePortal = async () => {
  try {
    isOpeningPortal.value = true
    portalError.value = null
    const res = await api.post<{ url: string }>('/stripe/portal')
    window.location.href = res.data.url
  } catch {
    portalError.value = t('account.danger.activeSubscription.portalFailed')
  } finally {
    isOpeningPortal.value = false
  }
}
</script>

<template>
  <section class="flex flex-col max-w-xl">
    <ConfirmModal
      :open="confirmDeleteOpen"
      :title="t('userDetails.accountDeleteButton')"
      :message="t('userDetails.accountDeleteWarning')"
      :confirm-label="t('userDetails.accountDeleteButton')"
      :danger="true"
      @confirm="handleDelete"
      @cancel="confirmDeleteOpen = false"
    />
    <UiModal
      :open="activeSubModalOpen"
      :aria-label="t('account.danger.activeSubscription.title')"
      data-testid="active-sub-modal"
      @close="activeSubModalOpen = false"
    >
      <template #header>
        <h2 class="text-base font-semibold text-fg-1" data-testid="active-sub-title">
          {{ t('account.danger.activeSubscription.title') }}
        </h2>
      </template>
      <p class="text-sm text-fg-2" data-testid="active-sub-message">
        {{ t('account.danger.activeSubscription.message') }}
      </p>
      <p
        v-if="portalError"
        class="text-sm text-danger-text mt-3"
        data-testid="active-sub-error"
      >
        {{ portalError }}
      </p>
      <template #footer>
        <UiButton
          variant="primary"
          :loading="isOpeningPortal"
          :disabled="isOpeningPortal"
          data-testid="open-portal-button"
          @click="openStripePortal"
        >
          {{ t('account.danger.activeSubscription.openPortal') }}
        </UiButton>
      </template>
    </UiModal>
    <header class="mb-2">
      <p class="text-xs uppercase tracking-wider text-danger-text" data-testid="account-tab-eyebrow">{{ t('account.danger.eyebrow') }}</p>
      <h1 class="text-3xl md:text-4xl font-medium tracking-tight mt-1 text-fg-1" data-testid="account-tab-heading">
        {{ t('account.danger.headingLead') }}<span class="font-display italic"> {{ t('account.danger.headingItalic') }}</span>
      </h1>
      <p class="text-sm text-fg-2 mt-2">{{ t('account.danger.subtitle') }}</p>
    </header>
    <div class="bg-bg-1 border border-danger/30 rounded-lg p-6 mt-6 flex flex-col gap-4">
      <p class="text-sm text-fg-2">{{ t('userDetails.accountDeleteWarning') }}</p>
      <div v-if="error" class="text-danger-text" data-testid="danger-error">{{ error }}</div>
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
    </div>
  </section>
</template>
