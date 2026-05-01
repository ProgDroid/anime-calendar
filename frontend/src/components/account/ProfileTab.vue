<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import api from '@/config/api'
import { toastService } from '@/services/toastService'
import UiInput from '@/components/ui/UiInput.vue'
import UiButton from '@/components/ui/UiButton.vue'

defineOptions({ name: 'ProfileTab' })

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
const isEditing = ref(false)
const updatedUsername = ref('')
const updatedEmail = ref('')

const userAvatar = computed(() => window.localStorage.getItem('avatar') || '')
const userName = computed(() => localStorage.getItem('name') || '')

const fetchUserDetails = async () => {
  try {
    loading.value = true
    error.value = null
    const response = await api.get('/user/details')
    user.value = response.data
    updatedUsername.value = response.data.username
    updatedEmail.value = response.data.email
  } catch {
    error.value = t('userDetails.fetchFailed')
  } finally {
    loading.value = false
  }
}

const handleUpdate = async (e: Event) => {
  e.preventDefault()
  try {
    const response = await api.put('/user', {
      username: updatedUsername.value,
      email: updatedEmail.value,
    })
    user.value = response.data
    isEditing.value = false
    authStore.user = response.data
    toastService.success(t('userDetails.updateSuccess'))
  } catch {
    error.value = t('userDetails.updateFailed')
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
  <section class="flex flex-col gap-6 max-w-xl">
    <header class="mb-2">
      <p class="text-xs uppercase tracking-wider text-fg-2" data-testid="account-tab-eyebrow">{{ t('account.profile.eyebrow') }}</p>
      <h2 class="text-3xl md:text-4xl font-medium tracking-tight mt-1 text-fg-1" data-testid="account-tab-heading">
        {{ t('account.profile.headingLead') }}<span class="font-display italic"> {{ t('account.profile.headingItalic') }}</span>
      </h2>
      <p class="text-sm text-fg-2 mt-2">{{ t('account.profile.subtitle') }}</p>
    </header>
    <div v-if="loading" class="text-fg-2">{{ t('userDetails.loading') }}</div>
    <div v-else-if="error" class="text-danger" data-testid="profile-error">{{ error }}</div>
    <template v-else-if="user">
      <div v-if="user.is_oauth" class="flex justify-center">
        <div class="w-24 h-24 rounded-full overflow-hidden bg-bg-2">
          <img v-if="userAvatar" :src="userAvatar" :alt="t('userDetails.avatarAlt', { name: userName })" class="w-full h-full object-cover" />
        </div>
      </div>

      <form v-if="isEditing" class="flex flex-col gap-4" @submit="handleUpdate">
        <UiInput
          v-model="updatedUsername"
          :label="t('userDetails.username')"
          type="text"
          required
        />
        <UiInput
          v-model="updatedEmail"
          :label="t('userDetails.email')"
          type="email"
          required
        />
        <div class="flex justify-end gap-3 mt-2">
          <UiButton variant="ghost" type="button" @click="isEditing = false">
            {{ t('userDetails.cancel') }}
          </UiButton>
          <UiButton variant="primary" type="submit">
            {{ t('userDetails.update') }}
          </UiButton>
        </div>
      </form>

      <div v-else class="flex flex-col gap-4">
        <UiInput
          :model-value="user.is_oauth ? userName : user.username"
          :label="t('userDetails.name')"
          type="text"
          disabled
        />
        <UiInput
          :model-value="user.email"
          :label="t('userDetails.email')"
          type="email"
          disabled
        />
        <div v-if="!user.is_oauth" class="flex justify-end mt-2">
          <UiButton variant="primary" @click="isEditing = true">
            {{ t('userDetails.editDetails') }}
          </UiButton>
        </div>
      </div>
    </template>
  </section>
</template>
