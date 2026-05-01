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
import UiSegmented from '@/components/ui/UiSegmented.vue'
import IconGlobe from '@/components/ui/icons/IconGlobe.vue'
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
  <section class="flex flex-col max-w-xl">
    <header class="mb-2">
      <p class="text-xs uppercase tracking-wider text-fg-2" data-testid="account-tab-eyebrow">{{ t('account.preferences.eyebrow') }}</p>
      <h2 class="text-3xl md:text-4xl font-medium tracking-tight mt-1 text-fg-1" data-testid="account-tab-heading">
        {{ t('account.preferences.headingLead') }}<span class="font-display italic"> {{ t('account.preferences.headingItalic') }}</span>
      </h2>
      <p class="text-sm text-fg-2 mt-2">{{ t('account.preferences.subtitle') }}</p>
    </header>
    <div v-if="loading" class="text-fg-2 mt-6">{{ t('app.loading') }}</div>
    <div v-else-if="error" class="text-danger mt-6" data-testid="preferences-error">{{ error }}</div>
    <div v-else class="bg-bg-1 border border-line rounded-lg p-6 mt-6 flex flex-col gap-6">
      <!-- Theme -->
      <div class="flex flex-col gap-2">
        <label class="text-sm text-fg-2">{{ t('userSettings.theme') }}</label>
        <UiSegmented
          v-model="settings.theme_preference"
          :options="[
            { value: 'light', label: t('userSettings.light') },
            { value: 'dark', label: t('userSettings.dark') },
          ]"
        />
      </div>

      <!-- Language -->
      <div class="flex flex-col gap-2">
        <label class="text-sm text-fg-2" for="language-select">{{ t('userSettings.language') }}</label>
        <div class="relative max-w-[280px]">
          <span
            class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-fg-2 [&_svg]:w-3.5 [&_svg]:h-3.5"
          >
            <IconGlobe />
          </span>
          <select
            id="language-select"
            v-model="settings.language_preference"
            class="w-full h-10 pl-9 pr-3 rounded-md bg-bg-1 text-fg-1 border border-line"
          >
            <option value="en">{{ t('userSettings.english') }}</option>
            <option value="pt">{{ t('userSettings.portuguese') }}</option>
          </select>
        </div>
      </div>

      <!-- Title Language -->
      <div class="flex flex-col gap-2">
        <label class="text-sm text-fg-2" for="title-language-select">{{ t('userSettings.titleLanguage') }}</label>
        <UiSegmented
          v-model="settings.title_language_preference"
          :options="[
            { value: 'English', label: t('userSettings.english') },
            { value: 'Romaji', label: t('userSettings.romaji') },
            { value: 'Native', label: t('userSettings.native') },
          ]"
        />
      </div>

      <!-- Timezone -->
      <div class="flex flex-col gap-2">
        <label class="text-sm text-fg-2" for="timezone-input">{{ t('userSettings.timezone') }}</label>
        <input
          id="timezone-input"
          v-model="settings.timezone"
          type="text"
          class="h-10 px-3 rounded-md bg-bg-1 text-fg-1 border border-line max-w-[280px]"
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
    </div>
  </section>
</template>
