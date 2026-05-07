<script setup lang="ts">
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import GoogleLoginButton from './GoogleLoginButton.vue'
import UiAuthShell from './ui/UiAuthShell.vue'
import UiInput from './ui/UiInput.vue'
import UiButton from './ui/UiButton.vue'
import IconMail from './ui/icons/IconMail.vue'
import IconLock from './ui/icons/IconLock.vue'
import IconEye from './ui/icons/IconEye.vue'
import UiCheckbox from './ui/UiCheckbox.vue'
import { useI18n } from 'vue-i18n'

defineOptions({ name: 'LoginPage' })

const { t } = useI18n()

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()

const email = ref('')
const password = ref('')
const username = ref('')
const isRegistering = ref(false)
const loading = ref(false)
const error = ref<string | null>(null)
const passwordVisible = ref(false)
// TODO: wire rememberMe through to authStore.login when backend supports it.
const rememberMe = ref(false)

const handleSubmit = async (e: Event) => {
  e.preventDefault()
  loading.value = true
  error.value = null

  try {
    if (isRegistering.value) {
      await authStore.register(username.value, email.value, password.value)
    } else {
      await authStore.login(email.value, password.value)
    }
    const redirect = route.query.redirect
    const safeRedirect = (() => {
      if (typeof redirect !== 'string' || redirect.length === 0) return '/my-calendars'
      try {
        const target = new URL(redirect, location.origin)
        if (target.origin === location.origin) {
          return target.pathname + target.search + target.hash
        }
      } catch {
        // invalid URL syntax; fall through to default
      }
      return '/my-calendars'
    })()
    router.push(safeRedirect)
  } catch {
    error.value = t('errors.generic')
  } finally {
    loading.value = false
  }
}

const toggleMode = () => {
  isRegistering.value = !isRegistering.value
  error.value = null
}
</script>

<template>
  <UiAuthShell
    poster-testid="login-poster-collage"
    :eyebrow="isRegistering ? t('auth.register.eyebrow') : t('auth.login.eyebrow')"
    :heading="isRegistering ? t('auth.register.heading') : t('auth.login.heading')"
    :subtitle="isRegistering ? t('auth.register.subtitle') : t('auth.login.subtitle')"
  >
    <p data-testid="login-tagline" class="-mt-4 text-fg-2 text-sm">
      {{ t('auth.login.tagline') }}
    </p>

    <div data-testid="login-google" class="flex flex-col items-stretch">
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

    <form data-testid="login-form" class="flex flex-col gap-4" @submit="handleSubmit">
      <UiInput
        v-if="isRegistering"
        id="username"
        v-model="username"
        type="text"
        name="username"
        :label="t('auth.register.name')"
        :placeholder="t('auth.register.usernamePlaceholder')"
        autocomplete="username"
        required
        :maxlength="50"
        data-testid="login-username"
      />

      <UiInput
        id="email"
        v-model="email"
        type="email"
        name="email"
        :label="t('auth.login.email')"
        :placeholder="t('auth.login.emailPlaceholder')"
        autocomplete="email"
        required
        data-testid="login-email"
      >
        <template #iconLeft><IconMail /></template>
      </UiInput>

      <div class="flex flex-col gap-1" data-testid="login-password">
        <div class="flex items-center justify-between">
          <label for="password" class="text-sm text-fg-2">{{ t('auth.login.password') }}</label>
          <router-link
            v-if="!isRegistering"
            to="/forgot-password"
            data-testid="login-forgot-link"
            class="text-sm text-accent-1-text hover:underline"
          >
            {{ t('auth.forgotPassword.link') }}
          </router-link>
        </div>
        <UiInput
          id="password"
          v-model="password"
          :type="passwordVisible ? 'text' : 'password'"
          name="password"
          :placeholder="t('auth.login.passwordPlaceholder')"
          autocomplete="current-password"
          required
          :maxlength="128"
        >
          <template #iconLeft><IconLock /></template>
          <template #iconRight>
            <button
              type="button"
              :aria-label="passwordVisible ? t('auth.login.hidePassword') : t('auth.login.showPassword')"
              data-testid="login-password-toggle"
              class="cursor-pointer hover:text-fg-1 inline-flex items-center justify-center w-9 h-9 rounded-sm focus-visible:outline-2 focus-visible:outline-accent-1 focus-visible:outline-offset-2"
              @click="passwordVisible = !passwordVisible"
            >
              <IconEye />
            </button>
          </template>
        </UiInput>
      </div>

      <UiCheckbox
        v-if="!isRegistering"
        v-model="rememberMe"
        :label="t('auth.login.rememberMe')"
        data-testid="login-remember"
      />

      <UiButton
        type="submit"
        variant="primary"
        size="lg"
        :loading="loading"
        :disabled="loading"
        data-testid="login-submit"
      >
        {{ isRegistering ? t('auth.register.submit') : t('auth.login.submit') }}
      </UiButton>

      <div v-if="error" data-testid="login-error" class="text-sm text-danger-text" role="alert">
        {{ error }}
      </div>
    </form>

    <p class="text-center text-sm text-fg-2">
      {{ isRegistering ? t('auth.register.alreadyHaveAccount') : t('auth.register.noAccount') }}
      <button
        type="button"
        data-testid="login-register-link"
        class="text-accent-1-text hover:underline ml-1"
        @click="toggleMode"
      >
        {{ isRegistering ? t('auth.login.submit') : t('auth.register.submit') }}
      </button>
    </p>
  </UiAuthShell>
</template>
