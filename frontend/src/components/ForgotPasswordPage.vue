<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import axios from 'axios'
import Wordmark from './shared/Wordmark.vue'
import UiInput from './ui/UiInput.vue'
import UiButton from './ui/UiButton.vue'

defineOptions({ name: 'ForgotPasswordPage' })

const { t } = useI18n()

const email = ref('')
const loading = ref(false)
const submitted = ref(false)

async function handleSubmit(e: Event) {
  e.preventDefault()
  loading.value = true
  try {
    await axios.post('/api/auth/forgot-password', { email: email.value })
  } catch {
    // Always show the same success message — no user enumeration.
  } finally {
    loading.value = false
    submitted.value = true
  }
}
</script>

<template>
  <div class="min-h-[calc(100vh-6rem)] flex bg-bg-1">
    <div data-testid="forgot-poster-collage" class="hidden lg:block flex-1 bg-bg-2" />

    <div class="flex-1 flex items-center justify-center p-6">
      <div class="w-full max-w-[420px] flex flex-col gap-6 bg-bg-1 rounded-xl p-8 border border-line">
        <div class="flex flex-col gap-2">
          <Wordmark size="lg" />
          <h1 class="font-display text-4xl text-fg-1">
            {{ t('auth.forgotPassword.title') }}
          </h1>
          <p data-testid="forgot-tagline" class="text-fg-2 text-sm">
            {{ t('auth.forgotPassword.subtitle') }}
          </p>
        </div>

        <div
          v-if="submitted"
          data-testid="forgot-success"
          role="status"
          class="text-sm text-fg-1 bg-bg-2 border border-line rounded-md p-4"
        >
          {{ t('auth.forgotPassword.successMessage') }}
        </div>

        <form
          v-else
          data-testid="forgot-form"
          class="flex flex-col gap-4"
          @submit="handleSubmit"
        >
          <UiInput
            id="forgot-email"
            v-model="email"
            type="email"
            name="email"
            :label="t('auth.forgotPassword.emailLabel')"
            :placeholder="t('auth.forgotPassword.emailPlaceholder')"
            autocomplete="email"
            required
            data-testid="forgot-email"
          />

          <UiButton
            type="submit"
            variant="primary"
            size="lg"
            :loading="loading"
            :disabled="loading"
            data-testid="forgot-submit"
          >
            {{ loading ? t('auth.forgotPassword.sending') : t('auth.forgotPassword.submit') }}
          </UiButton>
        </form>

        <p class="text-center text-sm text-fg-2">
          <router-link
            to="/login"
            data-testid="forgot-login-link"
            class="text-accent-1 hover:underline"
          >
            {{ t('auth.login.submit') }}
          </router-link>
        </p>
      </div>
    </div>
  </div>
</template>
