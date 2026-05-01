<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import GoogleLoginButton from './GoogleLoginButton.vue'
import UiAuthShell from './ui/UiAuthShell.vue'
import UiInput from './ui/UiInput.vue'
import UiButton from './ui/UiButton.vue'
import { useI18n } from 'vue-i18n'

defineOptions({ name: 'RegisterPage' })

const { t } = useI18n()

const router = useRouter()
const authStore = useAuthStore()

const email = ref('')
const password = ref('')
const confirmPassword = ref('')
const username = ref('')
const loading = ref(false)
const error = ref<string | null>(null)

const handleSubmit = async (e: Event) => {
  e.preventDefault()
  error.value = null

  if (password.value !== confirmPassword.value) {
    error.value = t('auth.register.passwordMismatch')
    return
  }

  loading.value = true
  try {
    await authStore.register(username.value, email.value, password.value)
    router.push({
      name: 'VerifyEmailPending',
      state: { email: email.value },
    })
  } catch {
    error.value = t('errors.generic')
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <UiAuthShell
    poster-testid="register-poster-collage"
    :eyebrow="t('auth.register.eyebrow')"
    :heading="t('auth.register.heading')"
    :subtitle="t('auth.register.subtitle')"
  >
    <p data-testid="register-tagline" class="-mt-4 text-fg-2 text-sm">
      {{ t('auth.register.tagline') }}
    </p>

    <form data-testid="register-form" class="flex flex-col gap-4" @submit="handleSubmit">
      <UiInput
        id="username"
        v-model="username"
        type="text"
        name="username"
        :label="t('auth.register.name')"
        :placeholder="t('auth.register.usernamePlaceholder')"
        autocomplete="username"
        required
        :maxlength="50"
        data-testid="register-username"
      />

      <UiInput
        id="email"
        v-model="email"
        type="email"
        name="email"
        :label="t('auth.register.email')"
        :placeholder="t('auth.login.emailPlaceholder')"
        autocomplete="email"
        required
        data-testid="register-email"
      />

      <UiInput
        id="password"
        v-model="password"
        type="password"
        name="password"
        :label="t('auth.register.password')"
        :placeholder="t('auth.login.passwordPlaceholder')"
        autocomplete="new-password"
        required
        :maxlength="128"
        data-testid="register-password"
      />

      <UiInput
        id="confirmPassword"
        v-model="confirmPassword"
        type="password"
        name="confirmPassword"
        :label="t('auth.register.confirmPassword')"
        :placeholder="t('auth.register.confirmPasswordPlaceholder')"
        autocomplete="new-password"
        required
        :maxlength="128"
        data-testid="register-confirm"
      />

      <UiButton
        type="submit"
        variant="primary"
        size="lg"
        :loading="loading"
        :disabled="loading"
        data-testid="register-submit"
      >
        {{ t('auth.register.submit') }}
      </UiButton>

      <div v-if="error" data-testid="register-error" class="text-sm text-danger" role="alert">
        {{ error }}
      </div>
    </form>

    <div class="flex items-center gap-3 text-fg-3 text-sm">
      <span class="flex-1 h-px bg-line" />
      <span>{{ t('auth.login.or') }}</span>
      <span class="flex-1 h-px bg-line" />
    </div>

    <div data-testid="register-google" class="flex flex-col items-stretch">
      <GoogleLoginButton />
    </div>

    <p class="text-center text-sm text-fg-2">
      {{ t('auth.register.alreadyHaveAccount') }}
      <router-link
        to="/login"
        data-testid="register-login-link"
        class="text-accent-1 hover:underline ml-1"
      >
        {{ t('auth.login.submit') }}
      </router-link>
    </p>
  </UiAuthShell>
</template>
