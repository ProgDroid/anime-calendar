<script setup lang="ts">
import { ref } from 'vue'
defineOptions({ name: 'RegisterPage' })
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import GoogleLoginButton from './GoogleLoginButton.vue'
import { i18n } from '@/plugins/i18n'

const { t } = i18n.global

const router = useRouter()
const authStore = useAuthStore()

const email = ref('')
const password = ref('')
const username = ref('')
const loading = ref(false)
const error = ref<string | null>(null)

const handleSubmit = async (e: Event) => {
  e.preventDefault()
  loading.value = true
  error.value = null

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
  <div class="min-h-[calc(100vh-6rem)] bg-base-200 flex items-center justify-center">
    <div class="card bg-base-100 w-full max-w-md shadow-xl">
      <div class="card-body">
        <h2 class="card-title">{{ $t('auth.register.title') }}</h2>
        <form @submit="handleSubmit" class="space-y-4">
          <div class="form-control w-full">
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
              <span class="label-text">{{ $t('auth.register.email') }}</span>
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
              <span class="label-text">{{ $t('auth.register.password') }}</span>
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

          <button
            type="submit"
            :disabled="loading"
            class="btn btn-primary w-full"
          >
            {{ loading ? $t('auth.register.registering') : $t('auth.register.submit') }}
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
            {{ $t('auth.register.noAccount') }}
            <router-link to="/login" class="link link-primary">
              {{ $t('auth.login.submit') }}
            </router-link>
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
