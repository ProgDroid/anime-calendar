<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { applySettings } from '@/services/applySettings'
import { useTheme } from '@/composables/useTheme'
import { toastService } from '@/services/toastService'
import { i18n } from '@/plugins/i18n'
import type { UserSettings, Accent } from '@/types/userSettings'
import UiButton from '@/components/ui/UiButton.vue'
import AccentPicker from '@/components/account/AccentPicker.vue'

defineOptions({ name: 'PreferencesTab' })

const { t } = useI18n()
const authStore = useAuthStore()
const userSettingsStore = useUserSettingsStore()
const { setAccent } = useTheme()

const loading = ref(true)
const error = ref<string | null>(null)
const settings = ref<UserSettings>(userSettingsStore.getDefaultSettings())

const fetchUserSettings = async () => {
  try {
    loading.value = true
    error.value = null
    settings.value = await userSettingsStore.fetchSettings()
  } catch {
    error.value = t('userSettings.fetchFailed')
  } finally {
    loading.value = false
  }
}

const handleSave = async () => {
  try {
    await userSettingsStore.updateSettings(settings.value)
    applySettings(settings.value)
    if (settings.value.language_preference) {
      i18n.global.locale.value = settings.value.language_preference
    }
    toastService.success(t('userSettings.updateSuccess'))
  } catch {
    error.value = t('userSettings.updateFailed')
  }
}

const handleAccentChange = (accent: Accent) => {
  settings.value.accent_preference = accent
  setAccent(accent)
}

onMounted(() => {
  if (!authStore.isAuthenticated()) return
  fetchUserSettings()
})
</script>

<template>
  <section class="flex flex-col gap-6 max-w-xl">
    <header class="mb-2">
      <p class="text-xs uppercase tracking-wider text-fg-2" data-testid="account-tab-eyebrow">{{ t('account.preferences.eyebrow') }}</p>
      <h2 class="text-3xl md:text-4xl font-medium tracking-tight mt-1 text-fg-1" data-testid="account-tab-heading">
        {{ t('account.preferences.headingLead') }}<span class="font-display italic"> {{ t('account.preferences.headingItalic') }}</span>
      </h2>
      <p class="text-sm text-fg-2 mt-2">{{ t('account.preferences.subtitle') }}</p>
    </header>
    <div v-if="loading" class="text-fg-2">{{ t('app.loading') }}</div>
    <div v-else-if="error" class="text-danger" data-testid="preferences-error">{{ error }}</div>
    <template v-else>
      <!-- Theme -->
      <div class="flex flex-col gap-2">
        <label class="text-sm text-fg-2">{{ t('userSettings.theme') }}</label>
        <div class="flex gap-4">
          <label class="flex items-center gap-2 cursor-pointer text-fg-1">
            <input
              v-model="settings.theme_preference"
              type="radio"
              name="theme"
              value="light"
              class="accent-accent-1"
            />
            <span>{{ t('userSettings.light') }}</span>
          </label>
          <label class="flex items-center gap-2 cursor-pointer text-fg-1">
            <input
              v-model="settings.theme_preference"
              type="radio"
              name="theme"
              value="dark"
              class="accent-accent-1"
            />
            <span>{{ t('userSettings.dark') }}</span>
          </label>
        </div>
      </div>

      <!-- Language -->
      <div class="flex flex-col gap-2">
        <label class="text-sm text-fg-2" for="language-select">{{ t('userSettings.language') }}</label>
        <select
          id="language-select"
          v-model="settings.language_preference"
          class="h-10 px-3 rounded-md bg-bg-1 text-fg-1 border border-line max-w-xs"
        >
          <option value="en">{{ t('userSettings.english') }}</option>
          <option value="pt">{{ t('userSettings.portuguese') }}</option>
        </select>
      </div>

      <!-- Title Language -->
      <div class="flex flex-col gap-2">
        <label class="text-sm text-fg-2" for="title-language-select">{{ t('userSettings.titleLanguage') }}</label>
        <select
          id="title-language-select"
          v-model="settings.title_language_preference"
          class="h-10 px-3 rounded-md bg-bg-1 text-fg-1 border border-line max-w-xs"
        >
          <option value="English">{{ t('userSettings.english') }}</option>
          <option value="Romaji">{{ t('userSettings.romaji') }}</option>
          <option value="Native">{{ t('userSettings.native') }}</option>
        </select>
      </div>

      <!-- Timezone -->
      <div class="flex flex-col gap-2">
        <label class="text-sm text-fg-2" for="timezone-input">{{ t('userSettings.timezone') }}</label>
        <input
          id="timezone-input"
          v-model="settings.timezone"
          type="text"
          class="h-10 px-3 rounded-md bg-bg-1 text-fg-1 border border-line max-w-xs"
          :placeholder="t('userSettings.timezonePlaceholder')"
        />
      </div>

      <!-- Accent -->
      <div class="flex flex-col gap-2">
        <label class="text-sm text-fg-2">{{ t('userSettings.accent') }}</label>
        <AccentPicker :model-value="settings.accent_preference" @update:model-value="handleAccentChange" />
        <p class="text-sm text-fg-2">{{ t('account.preferences.proNote') }}</p>
      </div>

      <div class="flex justify-end pt-2">
        <UiButton variant="primary" @click="handleSave">
          {{ t('userSettings.save') }}
        </UiButton>
      </div>
    </template>
  </section>
</template>
