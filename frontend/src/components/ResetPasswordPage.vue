<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import axios from 'axios'
import Wordmark from './shared/Wordmark.vue'
import UiInput from './ui/UiInput.vue'
import UiButton from './ui/UiButton.vue'

defineOptions({ name: 'ResetPasswordPage' })

const { t } = useI18n()
const route = useRoute()
const router = useRouter()

const token = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const loading = ref(false)
const errorMessage = ref('')
const success = ref(false)

onMounted(() => {
  const q = route.query.token
  if (!q || typeof q !== 'string') {
    router.replace('/forgot-password')
    return
  }
  token.value = q
})

async function handleSubmit(e: Event) {
  e.preventDefault()
  errorMessage.value = ''

  if (newPassword.value !== confirmPassword.value) {
    errorMessage.value = t('auth.resetPassword.passwordMismatch')
    return
  }

  loading.value = true
  try {
    await axios.post('/api/auth/reset-password', {
      token: token.value,
      new_password: newPassword.value,
    })
    success.value = true
    setTimeout(() => router.push('/login'), 2000)
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 400) {
      errorMessage.value = t('auth.resetPassword.invalidToken')
    } else {
      errorMessage.value = t('errors.generic')
    }
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="min-h-[calc(100vh-6rem)] flex bg-bg-1">
    <div data-testid="reset-poster-collage" class="hidden lg:block flex-1 bg-bg-2" />

    <div class="flex-1 flex items-center justify-center p-6">
      <div class="w-full max-w-[420px] flex flex-col gap-6 bg-bg-1 rounded-xl p-8 border border-line">
        <div class="flex flex-col gap-2">
          <Wordmark size="lg" />
          <h1 class="font-display text-4xl text-fg-1">
            {{ t('auth.resetPassword.title') }}
          </h1>
        </div>

        <div
          v-if="success"
          data-testid="reset-success"
          role="status"
          class="text-sm text-fg-1 bg-bg-2 border border-line rounded-md p-4"
        >
          {{ t('auth.resetPassword.successMessage') }}
        </div>

        <form
          v-else
          data-testid="reset-form"
          class="flex flex-col gap-4"
          @submit="handleSubmit"
        >
          <UiInput
            id="reset-password"
            v-model="newPassword"
            type="password"
            name="new-password"
            :label="t('auth.resetPassword.newPasswordLabel')"
            :placeholder="t('auth.resetPassword.newPasswordPlaceholder')"
            autocomplete="new-password"
            required
            :maxlength="128"
            data-testid="reset-password"
          />

          <UiInput
            id="reset-confirm"
            v-model="confirmPassword"
            type="password"
            name="confirm-password"
            :label="t('auth.resetPassword.confirmPasswordLabel')"
            :placeholder="t('auth.resetPassword.confirmPasswordPlaceholder')"
            autocomplete="new-password"
            required
            :maxlength="128"
            data-testid="reset-confirm"
          />

          <UiButton
            type="submit"
            variant="primary"
            size="lg"
            :loading="loading"
            :disabled="loading"
            data-testid="reset-submit"
          >
            {{ loading ? t('auth.resetPassword.resetting') : t('auth.resetPassword.submit') }}
          </UiButton>

          <div
            v-if="errorMessage"
            data-testid="reset-error"
            class="text-sm text-danger"
            role="alert"
          >
            {{ errorMessage }}
          </div>
        </form>

        <p class="text-center text-sm text-fg-2">
          <router-link
            to="/login"
            data-testid="reset-login-link"
            class="text-accent-1 hover:underline"
          >
            {{ t('auth.login.submit') }}
          </router-link>
        </p>
      </div>
    </div>
  </div>
</template>
