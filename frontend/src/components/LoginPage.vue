<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import GoogleLoginButton from './GoogleLoginButton.vue'
import { useI18n } from 'vue-i18n'

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
  <div class="min-h-[calc(100vh-6rem)] bg-base-200 flex items-center justify-center">
    <div class="card bg-base-100 w-full max-w-md shadow-xl">
      <div class="card-body">
        <h2 class="card-title">{{ isRegistering ? $t('auth.register.title') : $t('auth.login.title') }}</h2>
        <form @submit="handleSubmit" class="space-y-4">
          <div v-if="isRegistering" class="form-control w-full">
            <label class="label mb-2">
              <span class="label-text">{{ $t('auth.register.name') }}</span>
            </label>
            <input
              id="username"
              v-model="username"
              type="text"
              required
              class="input input-bordered w-full"
              :placeholder="$t('auth.register.usernamePlaceholder')"
              maxlength="50"
            />
          </div>

          <div class="form-control w-full">
            <label class="label mb-2">
              <span class="label-text">{{ $t('auth.login.email') }}</span>
            </label>
            <input
              id="email"
              v-model="email"
              type="email"
              required
              class="input input-bordered w-full"
              :placeholder="$t('auth.login.emailPlaceholder')"
            />
          </div>

          <div class="form-control w-full">
            <label class="label mb-2">
              <span class="label-text">{{ $t('auth.login.password') }}</span>
            </label>
            <input
              id="password"
              v-model="password"
              type="password"
              required
              class="input input-bordered w-full"
              :placeholder="$t('auth.login.passwordPlaceholder')"
              maxlength="128"
            />
          </div>

          <div v-if="!isRegistering" class="text-right -mt-2">
            <router-link to="/forgot-password" class="link link-primary text-sm">
              {{ $t('auth.forgotPassword.link') }}
            </router-link>
          </div>

          <button
            type="submit"
            :disabled="loading"
            class="btn btn-primary w-full"
          >
            {{ loading ? (isRegistering ? $t('auth.register.registering') : $t('auth.login.submit')) : (isRegistering ? $t('auth.register.submit') : $t('auth.login.submit')) }}
          </button>
          
          <div v-if="error" class="alert alert-error mt-2">
            {{ error }}
          </div>
        </form>
        
        <div class="divider">{{ $t('app.orContinueWith') }}</div>
        <div class="flex flex-col gap-3">
          <GoogleLoginButton />
        </div>
        
        <div class="card-actions justify-center mt-4">
          <p class="text-center">
            {{ isRegistering ? $t('auth.register.alreadyHaveAccount') : $t('auth.register.noAccount') }}
            <button @click="toggleMode" class="link link-primary" data-testid="toggle-mode">
              {{ isRegistering ? $t('auth.login.submit') : $t('auth.register.submit') }}
            </button>
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
