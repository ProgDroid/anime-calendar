<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import api from '@/config/api'
import { i18n } from '@/plugins/i18n'

const { t } = i18n.global
const router = useRouter()

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
  <div class="min-h-[calc(100vh-6rem)] bg-base-200 flex items-center justify-center">
    <div class="card bg-base-100 w-full max-w-md shadow-xl">
      <div class="card-body items-center text-center gap-4">
        <h2 class="card-title text-2xl">{{ $t('auth.verifyEmail.pending.title') }}</h2>
        <p class="text-base-content/70">{{ $t('auth.verifyEmail.pending.subtitle') }}</p>

        <div v-if="resendMessage" class="alert alert-success w-full">
          {{ resendMessage }}
        </div>
        <div v-if="resendError" class="alert alert-error w-full">
          {{ resendError }}
        </div>

        <button
          v-if="email"
          :disabled="resending"
          class="btn btn-outline btn-sm"
          data-testid="resend-button"
          @click="handleResend"
        >
          {{ resending ? $t('auth.verifyEmail.pending.resending') : $t('auth.verifyEmail.pending.resendButton') }}
        </button>
      </div>
    </div>
  </div>
</template>
