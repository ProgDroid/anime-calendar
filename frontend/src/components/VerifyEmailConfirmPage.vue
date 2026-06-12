<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import UiAuthShell from './ui/UiAuthShell.vue'
import UiEmptyState from './ui/UiEmptyState.vue'

defineOptions({ name: 'VerifyEmailConfirmPage' })

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()

const status = ref<'verifying' | 'success' | 'error'>('verifying')

// Stored so an unmount within the redirect window cancels the timer. Without
// this, the orphaned setTimeout fires router.push on a torn-down router — in
// Vitest's reused fork process that surfaces as a cross-file "reading 'push'"
// unhandled error that fails whichever spec is running at the time.
let redirectTimer: number | null = null

function clearRedirect() {
  if (redirectTimer !== null) {
    window.clearTimeout(redirectTimer)
    redirectTimer = null
  }
}

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
    redirectTimer = window.setTimeout(() => router.push('/my-calendars'), 1500)
  } catch {
    status.value = 'error'
  }
})

onBeforeUnmount(clearRedirect)
</script>

<template>
  <UiAuthShell
    poster-testid="verify-poster-collage"
    :eyebrow="t('auth.verifyEmail.confirm.eyebrow')"
  >
    <div
      v-if="status === 'verifying'"
      data-testid="verify-spinner"
      role="status"
      aria-live="polite"
      class="flex flex-col items-center gap-4 p-8"
    >
      <span
        class="inline-block w-8 h-8 border-2 border-fg-3 border-t-accent-1 rounded-full animate-spin"
      />
      <p class="text-sm text-fg-2">{{ t('auth.verifyEmail.confirm.verifying') }}</p>
    </div>

    <div v-else data-testid="verify-confirm-status">
      <UiEmptyState
        v-if="status === 'success'"
        :title="t('auth.verifyEmail.confirm.successTitle')"
        :body="t('auth.verifyEmail.confirm.success')"
      >
        <template #action>
          <router-link
            to="/my-calendars"
            data-testid="verify-go-to-calendars"
            class="text-accent-1-text hover:underline text-sm"
          >
            {{ t('app.myCalendars') }}
          </router-link>
        </template>
      </UiEmptyState>

      <UiEmptyState
        v-else
        :title="t('auth.verifyEmail.confirm.errorTitle')"
        :body="t('auth.verifyEmail.confirm.error')"
      >
        <template #action>
          <router-link
            to="/login"
            data-testid="verify-back-to-login"
            class="text-accent-1-text hover:underline text-sm"
          >
            {{ t('auth.verifyEmail.confirm.loginLink') }}
          </router-link>
        </template>
      </UiEmptyState>
    </div>
  </UiAuthShell>
</template>
