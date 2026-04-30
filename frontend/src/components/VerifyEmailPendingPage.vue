<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import api from '@/config/api'
import Wordmark from './shared/Wordmark.vue'
import UiAuthShell from './ui/UiAuthShell.vue'
import UiButton from './ui/UiButton.vue'
import UiEmptyState from './ui/UiEmptyState.vue'

defineOptions({ name: 'VerifyEmailPendingPage' })

const { t } = useI18n()

// Email is passed via router navigation state from Register.vue
const email = (history.state?.email as string) ?? ''
const resending = ref(false)
const resendMessage = ref<string | null>(null)
const resendError = ref<string | null>(null)

const handleResend = async () => {
  resending.value = true
  resendMessage.value = null
  resendError.value = null

  try {
    await api.post('/auth/resend-verification', { email })
    resendMessage.value = t('auth.verifyEmail.pending.resendSuccess')
  } catch {
    resendError.value = t('auth.verifyEmail.pending.resendError')
  } finally {
    resending.value = false
  }
}
</script>

<template>
  <UiAuthShell poster-testid="verify-poster-collage" card-testid="verify-pending">
    <div class="flex flex-col gap-2">
      <Wordmark size="lg" />
    </div>

    <UiEmptyState
      :title="t('auth.verifyEmail.pending.title')"
      :body="t('auth.verifyEmail.pending.subtitle')"
    >
      <template #action>
        <div class="flex flex-col items-stretch gap-3 w-full">
          <UiButton
            v-if="email"
            variant="secondary"
            size="md"
            :loading="resending"
            :disabled="resending"
            data-testid="verify-resend"
            @click="handleResend"
          >
            {{
              resending
                ? t('auth.verifyEmail.pending.resending')
                : t('auth.verifyEmail.pending.resendButton')
            }}
          </UiButton>

          <p
            v-if="resendMessage"
            data-testid="verify-resend-success"
            role="status"
            class="text-sm text-fg-1 bg-bg-2 border border-line rounded-md p-3"
          >
            {{ resendMessage }}
          </p>
          <p
            v-if="resendError"
            data-testid="verify-resend-error"
            role="alert"
            class="text-sm text-danger"
          >
            {{ resendError }}
          </p>
        </div>
      </template>
    </UiEmptyState>

    <p class="text-center text-sm text-fg-2">
      <router-link
        to="/login"
        data-testid="verify-back-to-login"
        class="text-accent-1 hover:underline"
      >
        {{ t('auth.login.submit') }}
      </router-link>
    </p>
  </UiAuthShell>
</template>
