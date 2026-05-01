<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'

defineOptions({ name: 'AccountPage' })

const route = useRoute()
const router = useRouter()
const { t } = useI18n()

const tabs = computed(() => [
  { name: 'account.profile', label: t('account.tabs.profile'), testid: 'account-tab-profile' },
  { name: 'account.preferences', label: t('account.tabs.preferences'), testid: 'account-tab-preferences' },
  { name: 'account.password', label: t('account.tabs.password'), testid: 'account-tab-password' },
  { name: 'account.danger', label: t('account.tabs.danger'), testid: 'account-tab-danger' },
])
</script>

<template>
  <div class="grid grid-cols-1 md:grid-cols-[200px_1fr] gap-6 p-4">
    <nav class="flex md:flex-col gap-1" data-testid="account-sidebar">
      <button
        v-for="tab in tabs"
        :key="tab.name"
        :data-testid="tab.testid"
        type="button"
        class="text-left px-3 py-2 rounded-md hover:bg-bg-3 transition"
        :class="route.name === tab.name ? 'bg-bg-2 font-medium text-fg-1' : 'text-fg-2'"
        @click="router.push({ name: tab.name })"
      >
        {{ tab.label }}
      </button>
    </nav>
    <main>
      <router-view />
    </main>
  </div>
</template>
