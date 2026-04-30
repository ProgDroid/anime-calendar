<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()

const status = ref<'verifying' | 'success' | 'error'>('verifying')

onMounted(async () => {
  const token = route.query.token as string | undefined
  if (!token) {
    status.value = 'error'
    return
  }

  try {
    await authStore.verifyEmail(token)
    status.value = 'success'
    // Brief pause so the user sees the success message, then redirect
    setTimeout(() => router.push('/my-calendars'), 1500)
  } catch {
    status.value = 'error'
  }
})
</script>

<template>
  <div class="min-h-[calc(100vh-6rem)] bg-base-200 flex items-center justify-center">
    <div class="card bg-base-100 w-full max-w-md shadow-xl">
      <div class="card-body items-center text-center gap-4">

        <span
          v-if="status === 'verifying'"
          class="loading loading-spinner loading-lg"
          data-testid="verifying-spinner"
        />
        <p v-if="status === 'verifying'">{{ $t('auth.verifyEmail.confirm.verifying') }}</p>

        <div v-if="status === 'success'" class="alert alert-success" data-testid="success-message">
          {{ $t('auth.verifyEmail.confirm.success') }}
        </div>

        <template v-if="status === 'error'">
          <div class="alert alert-error" data-testid="error-message">
            {{ $t('auth.verifyEmail.confirm.error') }}
          </div>
          <router-link to="/login" class="btn btn-primary btn-sm">
            {{ $t('auth.verifyEmail.confirm.loginLink') }}
          </router-link>
        </template>

      </div>
    </div>
  </div>
</template>
