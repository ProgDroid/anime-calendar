<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import api from '@/config/api'
import { toastService } from '@/services/toastService'
import UiInput from '@/components/ui/UiInput.vue'
import UiButton from '@/components/ui/UiButton.vue'
import IconMail from '@/components/ui/icons/IconMail.vue'

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
  <section class="flex flex-col max-w-xl">
    <header class="mb-2">
      <p class="text-xs uppercase tracking-wider text-fg-2" data-testid="account-tab-eyebrow">{{ t('account.profile.eyebrow') }}</p>
      <h2 class="text-3xl md:text-4xl font-medium tracking-tight mt-1 text-fg-1" data-testid="account-tab-heading">
        {{ t('account.profile.headingLead') }}<span class="font-display italic"> {{ t('account.profile.headingItalic') }}</span>
      </h2>
      <p class="text-sm text-fg-2 mt-2">{{ t('account.profile.subtitle') }}</p>
    </header>
    <div v-if="loading" class="text-fg-2 mt-6">{{ t('userDetails.loading') }}</div>
    <div v-else-if="error" class="text-danger mt-6" data-testid="profile-error">{{ error }}</div>
    <div v-else-if="user" class="bg-bg-1 border border-line rounded-lg p-6 mt-6 flex flex-col gap-6">
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
        <div class="flex flex-col gap-1">
          <label for="profile-email-edit" class="text-sm text-fg-2">{{ t('userDetails.email') }}</label>
          <div class="relative">
            <span
              class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-fg-2 [&_svg]:w-3.5 [&_svg]:h-3.5"
            >
              <IconMail />
            </span>
            <input
              id="profile-email-edit"
              v-model="updatedEmail"
              type="email"
              required
              class="w-full h-10 pl-9 pr-3 rounded-md bg-bg-1 text-fg-1 border border-line outline-none transition-shadow duration-[var(--d-1)] focus:border-accent-1 focus:shadow-[0_0_0_3px_var(--accent-1-soft)]"
            />
          </div>
        </div>
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
        <div class="flex flex-col gap-1">
          <label for="profile-email-view" class="text-sm text-fg-2">{{ t('userDetails.email') }}</label>
          <div class="relative">
            <span
              class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-fg-2 [&_svg]:w-3.5 [&_svg]:h-3.5"
            >
              <IconMail />
            </span>
            <input
              id="profile-email-view"
              :value="user.email"
              type="email"
              disabled
              class="w-full h-10 pl-9 pr-3 rounded-md bg-bg-1 text-fg-1 border border-line outline-none disabled:opacity-60"
            />
          </div>
        </div>
        <div v-if="!user.is_oauth" class="flex justify-end mt-2">
          <UiButton variant="primary" @click="isEditing = true">
            {{ t('userDetails.editDetails') }}
          </UiButton>
        </div>
      </div>
    </div>
  </section>
</template>
