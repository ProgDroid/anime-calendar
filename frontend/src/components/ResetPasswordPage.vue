<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import axios from 'axios'
import UiAuthShell from './ui/UiAuthShell.vue'
import UiInput from './ui/UiInput.vue'
import UiButton from './ui/UiButton.vue'
import IconLock from './ui/icons/IconLock.vue'
import IconEye from './ui/icons/IconEye.vue'

defineOptions({ name: 'ResetPasswordPage' })

const { t } = useI18n()
const route = useRoute()
const router = useRouter()

const token = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const loading = ref(false)
const errorMessage = ref('')
const success = ref(false)
const newPasswordVisible = ref(false)
const confirmPasswordVisible = ref(false)

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
      new_password: newPassword.value,
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
  <UiAuthShell
    poster-testid="reset-poster-collage"
    :eyebrow="t('auth.resetPassword.eyebrow')"
    :heading="t('auth.resetPassword.heading')"
    :subtitle="t('auth.resetPassword.subtitle')"
  >

    <div
      v-if="success"
      data-testid="reset-success"
      role="status"
      class="text-sm text-fg-1 bg-bg-2 border border-line rounded-md p-4"
    >
      {{ t('auth.resetPassword.successMessage') }}
    </div>

    <form v-else data-testid="reset-form" class="flex flex-col gap-4" @submit="handleSubmit">
      <UiInput
        id="reset-password"
        v-model="newPassword"
        :type="newPasswordVisible ? 'text' : 'password'"
        name="new-password"
        :label="t('auth.resetPassword.newPasswordLabel')"
        :placeholder="t('auth.resetPassword.newPasswordPlaceholder')"
        autocomplete="new-password"
        required
        :maxlength="128"
        data-testid="reset-password"
      >
        <template #iconLeft><IconLock /></template>
        <template #iconRight>
          <button
            type="button"
            :aria-label="newPasswordVisible ? t('auth.login.hidePassword') : t('auth.login.showPassword')"
            data-testid="reset-password-toggle"
            class="cursor-pointer hover:text-fg-1"
            @click="newPasswordVisible = !newPasswordVisible"
          >
            <IconEye />
          </button>
        </template>
      </UiInput>

      <UiInput
        id="reset-confirm"
        v-model="confirmPassword"
        :type="confirmPasswordVisible ? 'text' : 'password'"
        name="confirm-password"
        :label="t('auth.resetPassword.confirmPasswordLabel')"
        :placeholder="t('auth.resetPassword.confirmPasswordPlaceholder')"
        autocomplete="new-password"
        required
        :maxlength="128"
        data-testid="reset-confirm"
      >
        <template #iconLeft><IconLock /></template>
        <template #iconRight>
          <button
            type="button"
            :aria-label="confirmPasswordVisible ? t('auth.login.hidePassword') : t('auth.login.showPassword')"
            data-testid="reset-confirm-toggle"
            class="cursor-pointer hover:text-fg-1"
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
        data-testid="reset-submit"
      >
        {{ loading ? t('auth.resetPassword.resetting') : t('auth.resetPassword.submit') }}
      </UiButton>

      <div v-if="errorMessage" data-testid="reset-error" class="text-sm text-danger" role="alert">
        {{ errorMessage }}
      </div>
    </form>

    <p class="text-center text-sm text-fg-2">
      <router-link to="/login" data-testid="reset-login-link" class="text-accent-1 hover:underline">
        {{ t('auth.login.submit') }}
      </router-link>
    </p>
  </UiAuthShell>
</template>
