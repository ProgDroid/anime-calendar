<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import axios from 'axios'
import { useAuthStore } from '@/stores/auth'
import api from '@/config/api'
import { toastService } from '@/services/toastService'
import UiInput from '@/components/ui/UiInput.vue'
import UiButton from '@/components/ui/UiButton.vue'

defineOptions({ name: 'PasswordTab' })

interface User {
  username: string
  email: string
  is_oauth: boolean
}

const { t } = useI18n()
const router = useRouter()
const authStore = useAuthStore()

const user = ref<User | null>(null)
const loading = ref(true)
const error = ref<string | null>(null)
const currentPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const isUpdatingPassword = ref(false)

const fetchUserDetails = async () => {
  try {
    loading.value = true
    error.value = null
    const response = await api.get('/user/details')
    user.value = response.data
  } catch {
    error.value = t('userDetails.fetchFailed')
  } finally {
    loading.value = false
  }
}

const handleUpdatePassword = async (e: Event) => {
  e.preventDefault()

  if (newPassword.value !== confirmPassword.value) {
    error.value = t('userDetails.passwordsDontMatch')
    return
  }
  if (newPassword.value.length < 12) {
    error.value = t('userDetails.passwordLength', { min: 12 })
    return
  }
  if (!/(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[!@#$%^&*(),.?":{}|<>]).+/.test(newPassword.value)) {
    error.value = t('userDetails.passwordContent')
    return
  }
  if (newPassword.value === currentPassword.value) {
    error.value = t('userDetails.passwordMustBeDifferent')
    return
  }

  try {
    isUpdatingPassword.value = true
    error.value = null
    await api.post('/user/password', {
      current_password: currentPassword.value,
      new_password: newPassword.value,
    })
    currentPassword.value = ''
    newPassword.value = ''
    confirmPassword.value = ''
    toastService.success(t('userDetails.passwordUpdateSuccess'))
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 401) {
      error.value = t('userDetails.currentPasswordIncorrect')
    } else {
      error.value = t('userDetails.passwordUpdateFailed')
    }
  } finally {
    isUpdatingPassword.value = false
  }
}

onMounted(() => {
  if (!authStore.isAuthenticated()) {
    router.push('/login')
    return
  }
  fetchUserDetails()
})
</script>

<template>
  <section class="flex flex-col max-w-xl">
    <header class="mb-2">
      <p class="text-xs uppercase tracking-wider text-fg-2" data-testid="account-tab-eyebrow">{{ t('account.password.eyebrow') }}</p>
      <h2 class="text-3xl md:text-4xl font-medium tracking-tight mt-1 text-fg-1" data-testid="account-tab-heading">
        {{ t('account.password.headingLead') }}<span class="font-display italic"> {{ t('account.password.headingItalic') }}</span>
      </h2>
      <p class="text-sm text-fg-2 mt-2">{{ t('account.password.subtitle') }}</p>
    </header>
    <div v-if="loading" class="text-fg-2 mt-6">{{ t('userDetails.loading') }}</div>
    <div v-else-if="user && user.is_oauth" class="bg-bg-1 border border-line rounded-lg p-6 mt-6 text-fg-2" data-testid="password-oauth-notice">
      {{ t('userDetails.passwordOauthNotice') }}
    </div>
    <div v-else-if="user" class="bg-bg-1 border border-line rounded-lg p-6 mt-6 flex flex-col gap-4">
      <h3 class="text-lg font-medium text-fg-1">{{ t('userDetails.changePassword') }}</h3>
      <div v-if="error" class="text-danger-text" data-testid="password-error">{{ error }}</div>
      <form class="flex flex-col gap-4" @submit="handleUpdatePassword">
        <UiInput
          v-model="currentPassword"
          :label="t('userDetails.currentPassword')"
          type="password"
          autocomplete="current-password"
          required
        />
        <div>
          <UiInput
            v-model="newPassword"
            :label="t('userDetails.newPassword')"
            type="password"
            autocomplete="new-password"
            required
          />
          <p class="text-xs text-fg-2 mt-1">{{ t('userDetails.passwordHint') }}</p>
        </div>
        <UiInput
          v-model="confirmPassword"
          :label="t('userDetails.confirmPassword')"
          type="password"
          autocomplete="new-password"
          required
        />
        <div class="flex justify-end gap-3 mt-2">
          <UiButton
            variant="primary"
            type="submit"
            :loading="isUpdatingPassword"
            :disabled="isUpdatingPassword"
          >
            {{ isUpdatingPassword ? t('userDetails.passwordUpdating') : t('userDetails.passwordUpdate') }}
          </UiButton>
        </div>
      </form>
    </div>
  </section>
</template>
