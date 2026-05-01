<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import api from '@/config/api'
import IconSparkle from '@/components/ui/icons/IconSparkle.vue'
import IconSettings from '@/components/ui/icons/IconSettings.vue'
import IconLock from '@/components/ui/icons/IconLock.vue'
import IconTrash from '@/components/ui/icons/IconTrash.vue'

defineOptions({ name: 'AccountPage' })

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const authStore = useAuthStore()

interface UserDetails {
  username: string
  email: string
  is_oauth: boolean
}

const userDetails = ref<UserDetails | null>(null)

const displayName = computed(() => userDetails.value?.username ?? authStore.user ?? '')
const displayEmail = computed(() => userDetails.value?.email ?? '')
const initials = computed(() => {
  const source = displayName.value.trim()
  if (!source) return '—'
  const parts = source.split(/\s+/).filter(Boolean)
  if (parts.length >= 2) return (parts[0]![0]! + parts[1]![0]!).toUpperCase()
  return source.slice(0, 2).toUpperCase()
})

const tabs = computed(() => [
  { name: 'account.profile', label: t('account.tabs.profile'), testid: 'account-tab-profile', icon: IconSparkle },
  { name: 'account.preferences', label: t('account.tabs.preferences'), testid: 'account-tab-preferences', icon: IconSettings },
  { name: 'account.password', label: t('account.tabs.password'), testid: 'account-tab-password', icon: IconLock },
  { name: 'account.danger', label: t('account.tabs.danger'), testid: 'account-tab-danger', icon: IconTrash },
])

onMounted(async () => {
  if (!authStore.isAuthenticated()) return
  try {
    const response = await api.get('/user/details')
    userDetails.value = response.data
  } catch {
    // Sidebar falls back to placeholders; tabs handle their own errors.
  }
})
</script>

<template>
  <div class="grid grid-cols-1 md:grid-cols-[260px_1fr] gap-6 p-4">
    <nav class="flex md:flex-col gap-1" data-testid="account-sidebar">
      <div class="flex items-center gap-3 px-3 py-2 mb-4 w-full" data-testid="account-sidebar-user">
        <div
          class="w-12 h-12 rounded-full flex items-center justify-center bg-accent-1/15 text-accent-1 font-semibold text-base shrink-0 select-none"
          aria-hidden="true"
        >
          {{ initials }}
        </div>
        <div class="min-w-0">
          <div class="text-sm font-semibold text-fg-1 truncate" data-testid="account-sidebar-name">
            {{ displayName || '—' }}
          </div>
          <div class="text-xs text-fg-2 truncate" data-testid="account-sidebar-email">
            {{ displayEmail || '—' }}
          </div>
        </div>
      </div>
      <button
        v-for="tab in tabs"
        :key="tab.name"
        :data-testid="tab.testid"
        type="button"
        class="flex items-center gap-2.5 px-3 py-2 rounded-md w-full text-left transition [&_svg]:w-3.5 [&_svg]:h-3.5"
        :class="route.name === tab.name ? 'bg-bg-2 text-fg-1 font-semibold' : 'text-fg-1 font-medium hover:bg-bg-3'"
        @click="router.push({ name: tab.name })"
      >
        <component :is="tab.icon" />
        <span>{{ tab.label }}</span>
      </button>
    </nav>
    <main>
      <router-view />
    </main>
  </div>
</template>
