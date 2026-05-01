<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import GoogleLoginButton from './GoogleLoginButton.vue'
import UiAuthShell from './ui/UiAuthShell.vue'
import UiInput from './ui/UiInput.vue'
import UiButton from './ui/UiButton.vue'
import IconMail from './ui/icons/IconMail.vue'
import IconLock from './ui/icons/IconLock.vue'
import IconEye from './ui/icons/IconEye.vue'
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
const passwordVisible = ref(false)
const confirmPasswordVisible = ref(false)

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

    <div data-testid="register-google" class="flex flex-col items-stretch">
      <GoogleLoginButton />
    </div>

    <div
      class="flex items-center gap-3 text-fg-2 text-sm"
      role="separator"
      aria-orientation="horizontal"
    >
      <span class="flex-1 h-px bg-line" aria-hidden="true" />
      <span>{{ t('auth.login.or') }}</span>
      <span class="flex-1 h-px bg-line" aria-hidden="true" />
    </div>

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
      >
        <template #iconLeft><IconMail /></template>
      </UiInput>

      <UiInput
        id="password"
        v-model="password"
        :type="passwordVisible ? 'text' : 'password'"
        name="password"
        :label="t('auth.register.password')"
        :placeholder="t('auth.login.passwordPlaceholder')"
        autocomplete="new-password"
        required
        :maxlength="128"
        data-testid="register-password"
      >
        <template #iconLeft><IconLock /></template>
        <template #iconRight>
          <button
            type="button"
            :aria-label="passwordVisible ? t('auth.login.hidePassword') : t('auth.login.showPassword')"
            data-testid="register-password-toggle"
            class="cursor-pointer hover:text-fg-1 inline-flex items-center justify-center w-9 h-9 rounded-sm focus-visible:outline-2 focus-visible:outline-accent-1 focus-visible:outline-offset-2"
            @click="passwordVisible = !passwordVisible"
          >
            <IconEye />
          </button>
        </template>
      </UiInput>

      <UiInput
        id="confirmPassword"
        v-model="confirmPassword"
        :type="confirmPasswordVisible ? 'text' : 'password'"
        name="confirmPassword"
        :label="t('auth.register.confirmPassword')"
        :placeholder="t('auth.register.confirmPasswordPlaceholder')"
        autocomplete="new-password"
        required
        :maxlength="128"
        data-testid="register-confirm"
      >
        <template #iconLeft><IconLock /></template>
        <template #iconRight>
          <button
            type="button"
            :aria-label="confirmPasswordVisible ? t('auth.login.hidePassword') : t('auth.login.showPassword')"
            data-testid="register-confirm-toggle"
            class="cursor-pointer hover:text-fg-1 inline-flex items-center justify-center w-9 h-9 rounded-sm focus-visible:outline-2 focus-visible:outline-accent-1 focus-visible:outline-offset-2"
            @click="confirmPasswordVisible = !confirmPasswordVisible"
          >
            <IconEye />
          </button>
        </template>
      </UiInput>

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

      <div v-if="error" data-testid="register-error" class="text-sm text-danger-text" role="alert">
        {{ error }}
      </div>
    </form>

    <p class="text-center text-sm text-fg-2">
      {{ t('auth.register.alreadyHaveAccount') }}
      <router-link
        to="/login"
        data-testid="register-login-link"
        class="text-accent-1-text hover:underline ml-1"
      >
        {{ t('auth.login.submit') }}
      </router-link>
    </p>
  </UiAuthShell>
</template>
