<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import axios from 'axios'
import UiAuthShell from './ui/UiAuthShell.vue'
import UiInput from './ui/UiInput.vue'
import UiButton from './ui/UiButton.vue'
import IconMail from './ui/icons/IconMail.vue'

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
  <UiAuthShell
    poster-testid="forgot-poster-collage"
    :eyebrow="t('auth.forgotPassword.eyebrow')"
    :heading="t('auth.forgotPassword.heading')"
    :subtitle="t('auth.forgotPassword.subtitle')"
  >
    <span data-testid="forgot-tagline" class="sr-only">{{
      t('auth.forgotPassword.subtitle')
    }}</span>

    <div
      v-if="submitted"
      data-testid="forgot-success"
      role="status"
      class="text-sm text-fg-1 bg-bg-2 border border-line rounded-md p-4"
    >
      {{ t('auth.forgotPassword.successMessage') }}
    </div>

    <form v-else data-testid="forgot-form" class="flex flex-col gap-4" @submit="handleSubmit">
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
      >
        <template #iconLeft><IconMail /></template>
      </UiInput>

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
  </UiAuthShell>
</template>
