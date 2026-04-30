<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import GoogleLoginButton from './GoogleLoginButton.vue'
import Wordmark from './shared/Wordmark.vue'
import UiInput from './ui/UiInput.vue'
import UiButton from './ui/UiButton.vue'
import { useI18n } from 'vue-i18n'

defineOptions({ name: 'LoginPage' })

const { t } = useI18n()

const router = useRouter()
const authStore = useAuthStore()

const email = ref('')
const password = ref('')
const username = ref('')
const isRegistering = ref(false)
const loading = ref(false)
const error = ref<string | null>(null)

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
    router.push('/my-calendars')
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
  <div class="min-h-[calc(100vh-6rem)] flex bg-bg-1">
    <div data-testid="login-poster-collage" class="hidden lg:block flex-1 bg-bg-2" />

    <div class="flex-1 flex items-center justify-center p-6">
      <div class="w-full max-w-[420px] flex flex-col gap-6 bg-bg-1 rounded-xl p-8 border border-line">
        <div class="flex flex-col gap-2">
          <Wordmark size="lg" />
          <h1 class="font-display text-4xl text-fg-1">
            {{ isRegistering ? t('auth.register.title') : t('auth.login.title') }}
          </h1>
          <p data-testid="login-tagline" class="text-fg-2 text-sm">
            {{ t('auth.login.tagline') }}
          </p>
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
          />

          <UiInput
            id="password"
            v-model="password"
            type="password"
            name="password"
            :label="t('auth.login.password')"
            :placeholder="t('auth.login.passwordPlaceholder')"
            autocomplete="current-password"
            required
            :maxlength="128"
            data-testid="login-password"
          />

          <div v-if="!isRegistering" class="flex justify-end -mt-1">
            <router-link
              to="/forgot-password"
              data-testid="login-forgot-link"
              class="text-sm text-accent-1 hover:underline"
            >
              {{ t('auth.forgotPassword.link') }}
            </router-link>
          </div>

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

          <div v-if="error" data-testid="login-error" class="text-sm text-danger" role="alert">
            {{ error }}
          </div>
        </form>

        <div class="flex items-center gap-3 text-fg-3 text-sm">
          <span class="flex-1 h-px bg-line" />
          <span>{{ t('auth.login.or') }}</span>
          <span class="flex-1 h-px bg-line" />
        </div>

        <div data-testid="login-google" class="flex flex-col items-stretch">
          <GoogleLoginButton />
        </div>

        <p class="text-center text-sm text-fg-2">
          {{ isRegistering ? t('auth.register.alreadyHaveAccount') : t('auth.register.noAccount') }}
          <button
            type="button"
            data-testid="login-register-link"
            class="text-accent-1 hover:underline ml-1"
            @click="toggleMode"
          >
            {{ isRegistering ? t('auth.login.submit') : t('auth.register.submit') }}
          </button>
        </p>
      </div>
    </div>
  </div>
</template>
