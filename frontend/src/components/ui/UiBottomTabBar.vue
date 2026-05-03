<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, RouterLink } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useViewportLayout } from '@/composables/useViewportLayout'
import IconCal from './icons/IconCal.vue'
import IconSettings from './icons/IconSettings.vue'

defineOptions({ name: 'UiBottomTabBar' })

const { t } = useI18n()
const route = useRoute()
const { isMobile } = useViewportLayout()

const visible = computed(
  () => isMobile.value && route.meta.bottomTabBar !== false,
)

const activeTab = computed<'library' | 'account' | null>(() => {
  const p = route.path
  if (p.startsWith('/calendar') || p.startsWith('/my-calendars')) return 'library'
  if (p.startsWith('/account')) return 'account'
  return null
})
</script>

<template>
  <nav
    v-if="visible"
    data-testid="bottom-tab-bar"
    :aria-label="t('mobile.tabBar.label')"
    class="fixed bottom-0 left-0 right-0 z-30 flex justify-around border-t border-line-soft bg-bg-0/80 pt-2 backdrop-blur-md"
    :style="{ paddingBottom: 'max(22px, calc(env(safe-area-inset-bottom) + 8px))' }"
  >
    <RouterLink
      to="/my-calendars"
      data-testid="bottom-tab"
      :aria-current="activeTab === 'library' ? 'page' : undefined"
      :class="[
        'flex flex-col items-center gap-1 px-4 py-1 text-[10px] font-medium',
        activeTab === 'library' ? 'text-accent-1-text' : 'text-fg-2',
      ]"
    >
      <span data-testid="bottom-tab-library" :class="activeTab === 'library' ? 'text-accent-1-text' : 'text-fg-2'">
        <IconCal />
      </span>
      {{ t('mobile.tabBar.library') }}
    </RouterLink>
    <RouterLink
      to="/account"
      data-testid="bottom-tab"
      :aria-current="activeTab === 'account' ? 'page' : undefined"
      :class="[
        'flex flex-col items-center gap-1 px-4 py-1 text-[10px] font-medium',
        activeTab === 'account' ? 'text-accent-1-text' : 'text-fg-2',
      ]"
    >
      <span data-testid="bottom-tab-account" :class="activeTab === 'account' ? 'text-accent-1-text' : 'text-fg-2'">
        <IconSettings />
      </span>
      {{ t('mobile.tabBar.account') }}
    </RouterLink>
  </nav>
</template>
