<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import axios from 'axios'

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
      new_password: newPassword.value
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
  <div class="min-h-screen flex items-center justify-center bg-base-200">
    <div class="card w-full max-w-md bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title">{{ t('auth.resetPassword.title') }}</h2>

        <div v-if="success" class="alert alert-success">
          <span>{{ t('auth.resetPassword.successMessage') }}</span>
        </div>

        <template v-else>
          <div v-if="errorMessage" class="alert alert-error">
            <span>{{ errorMessage }}</span>
          </div>

          <form @submit="handleSubmit" class="space-y-4">
            <div class="form-control">
              <label class="label mb-2">
                <span class="label-text">{{ t('auth.resetPassword.newPasswordLabel') }}</span>
              </label>
              <input
                v-model="newPassword"
                type="password"
                required
                class="input input-bordered w-full"
                :placeholder="t('auth.resetPassword.newPasswordPlaceholder')"
                minlength="12"
                maxlength="128"
              />
            </div>
            <div class="form-control">
              <label class="label mb-2">
                <span class="label-text">{{ t('auth.resetPassword.confirmPasswordLabel') }}</span>
              </label>
              <input
                v-model="confirmPassword"
                type="password"
                required
                class="input input-bordered w-full"
                :placeholder="t('auth.resetPassword.confirmPasswordPlaceholder')"
                minlength="12"
                maxlength="128"
              />
            </div>
            <button
              type="submit"
              class="btn btn-primary w-full"
              :disabled="loading"
            >
              {{ loading ? t('auth.resetPassword.resetting') : t('auth.resetPassword.submit') }}
            </button>
          </form>
        </template>
      </div>
    </div>
  </div>
</template>
