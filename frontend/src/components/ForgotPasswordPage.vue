<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import axios from 'axios'

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
  <div class="min-h-screen flex items-center justify-center bg-base-200">
    <div class="card w-full max-w-md bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title">{{ t('auth.forgotPassword.title') }}</h2>

        <div v-if="submitted" class="alert alert-success">
          <span>{{ t('auth.forgotPassword.successMessage') }}</span>
        </div>

        <form v-else @submit="handleSubmit" class="space-y-4">
          <p class="text-sm text-base-content/70">
            {{ t('auth.forgotPassword.subtitle') }}
          </p>
          <div class="form-control">
            <label class="label mb-2">
              <span class="label-text">{{ t('auth.forgotPassword.emailLabel') }}</span>
            </label>
            <input
              v-model="email"
              type="email"
              required
              class="input input-bordered w-full"
              :placeholder="t('auth.forgotPassword.emailPlaceholder')"
            />
          </div>
          <button
            type="submit"
            class="btn btn-primary w-full"
            :disabled="loading"
          >
            {{ loading ? t('auth.forgotPassword.sending') : t('auth.forgotPassword.submit') }}
          </button>
        </form>

        <div class="text-center mt-2">
          <router-link to="/login" class="link link-primary text-sm">
            {{ t('auth.login.title') }}
          </router-link>
        </div>
      </div>
    </div>
  </div>
</template>
