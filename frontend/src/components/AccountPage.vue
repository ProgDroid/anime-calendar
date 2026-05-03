<script setup lang="ts">
import { computed, onMounted, ref, watchEffect } from 'vue'
import { useRoute, useRouter, RouterLink, RouterView } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { useViewportLayout } from '@/composables/useViewportLayout'
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
const { isMobile } = useViewportLayout()

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
  { name: 'account.subscription', label: t('account.tabs.subscription'), testid: 'account-tab-subscription', icon: IconSparkle },
  { name: 'account.password', label: t('account.tabs.password'), testid: 'account-tab-password', icon: IconLock },
  { name: 'account.danger', label: t('account.tabs.danger'), testid: 'account-tab-danger', icon: IconTrash },
])

const SECTIONS = [
  { name: 'account.profile', i18n: 'account.tabs.profile' },
  { name: 'account.preferences', i18n: 'account.tabs.preferences' },
  { name: 'account.subscription', i18n: 'account.tabs.subscription' },
  { name: 'account.password', i18n: 'account.tabs.password' },
  { name: 'account.danger', i18n: 'account.tabs.danger' },
] as const

const isAtSectionList = computed(
  () => route.path === '/account' || route.path === '/account/',
)

watchEffect(() => {
  if (!isMobile.value && isAtSectionList.value) {
    router.replace({ name: 'account.profile' })
  }
})

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
  <div>
    <!-- Mobile: sectioned list at /account -->
    <div
      v-if="isMobile && isAtSectionList"
      class="px-4 py-6"
      data-testid="account-section-list"
    >
      <h1 class="font-display text-3xl text-fg-1">
        {{ t('mobile.account.sectionsHeading') }}
      </h1>
      <ul
        class="mt-6 divide-y divide-line-soft overflow-hidden rounded-lg border border-line-soft bg-bg-1"
      >
        <li v-for="s in SECTIONS" :key="s.name">
          <RouterLink
            :to="{ name: s.name }"
            :data-testid="`account-section-${s.name}`"
            class="flex items-center justify-between px-4 py-4 text-fg-1 hover:bg-bg-2"
          >
            <span>{{ t(s.i18n) }}</span>
            <span class="text-fg-3" aria-hidden="true">›</span>
          </RouterLink>
        </li>
      </ul>
    </div>

    <!-- Mobile: sub-route view with back arrow -->
    <div
      v-else-if="isMobile"
      class="px-4 py-6"
      data-testid="account-subroute"
    >
      <RouterLink
        to="/account"
        class="-ml-2 mb-4 inline-flex items-center gap-1 px-2 py-1 text-sm text-fg-2"
        data-testid="account-back-arrow"
      >
        <span aria-hidden="true">‹</span> {{ t('mobile.account.backToList') }}
      </RouterLink>
      <RouterView />
    </div>

    <!-- Desktop: existing sidebar layout -->
    <div
      v-else
      class="grid grid-cols-1 md:grid-cols-[260px_1fr] gap-6 p-4"
      data-testid="account-desktop-layout"
    >
      <nav
        class="flex md:flex-col gap-1"
        data-testid="account-sidebar"
        :aria-label="t('account.tabs.label')"
      >
        <div class="flex items-center gap-3 px-3 py-2 mb-4 w-full" data-testid="account-sidebar-user">
          <div
            class="w-12 h-12 rounded-full flex items-center justify-center bg-accent-1/15 text-accent-1-text font-semibold text-base shrink-0 select-none"
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
        <RouterLink
          v-for="tab in tabs"
          :key="tab.name"
          :to="{ name: tab.name }"
          :data-testid="tab.testid"
          :aria-current="route.name === tab.name ? 'page' : undefined"
          class="flex items-center gap-2.5 px-3 py-2 rounded-md w-full text-left transition [&_svg]:w-3.5 [&_svg]:h-3.5 focus-visible:outline-2 focus-visible:outline-accent-1 focus-visible:outline-offset-2"
          :class="route.name === tab.name ? 'bg-bg-2 text-fg-1 font-semibold' : 'text-fg-1 font-medium hover:bg-bg-3'"
        >
          <component :is="tab.icon" />
          <span>{{ tab.label }}</span>
        </RouterLink>
      </nav>
      <section :aria-label="t('account.tabs.label')">
        <router-view />
      </section>
    </div>
  </div>
</template>
