<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { applySettings } from '@/services/applySettings'
import { useTheme } from '@/composables/useTheme'
import { toastService } from '@/services/toastService'
import { i18n } from '@/plugins/i18n'
import { CANONICAL_REMINDER_OFFSETS, type UserSettings, type Accent } from '@/types/userSettings'
import UiButton from '@/components/ui/UiButton.vue'
import UiSegmented from '@/components/ui/UiSegmented.vue'
import IconGlobe from '@/components/ui/icons/IconGlobe.vue'
import IconInfo from '@/components/ui/icons/IconInfo.vue'
import AccentPicker from '@/components/account/AccentPicker.vue'
import { useUpgradeInterrupt } from '@/composables/useUpgradeInterrupt'
import { getMySubscription, type Tier } from '@/services/subscription'

defineOptions({ name: 'PreferencesTab' })

const { t } = useI18n()
const router = useRouter()
const authStore = useAuthStore()
const userSettingsStore = useUserSettingsStore()
const { setAccent, setIsPaid } = useTheme()

const REMINDER_CAP = 5

const loading = ref(true)
const error = ref<string | null>(null)
const settings = ref<UserSettings>(userSettingsStore.getDefaultSettings())

// Pro accent gating: tier is fetched once on mount; AccentPicker uses
// `isPaid` to decide whether to apply or emit interrupt. Past_due users still
// read as paid here — the entitlement service grants access during dunning.
const tier = ref<Tier>('free')
const isPaid = computed(() => tier.value === 'paid')
const { openUpgradeModal } = useUpgradeInterrupt()

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

// Free user clicked a Pro accent. AccentPicker prevents the apply; here we
// just open the upgrade modal. The accent stays at its previous value.
const handleAccentInterrupt = () => {
  openUpgradeModal('pro_accent')
}

// Reminders: chip toggles edit local state; the existing Save button
// persists. Stored offsets are preserved across tier transitions but the
// server enforces a single 30-min VALARM at .ics emission for Free users.
const reminderOffsets = computed<number[]>(() => settings.value.reminder_offsets_minutes ?? [])
const activeRemindersCount = computed(() => reminderOffsets.value.length)
const remindersAtCap = computed(() => activeRemindersCount.value >= REMINDER_CAP)
const isReminderActive = (offset: number) => reminderOffsets.value.includes(offset)

const toggleReminderOffset = (offset: number) => {
  if (!isPaid.value) return
  const current = [...reminderOffsets.value]
  const idx = current.indexOf(offset)
  if (idx >= 0) {
    current.splice(idx, 1)
  } else if (current.length >= REMINDER_CAP) {
    return
  } else {
    current.push(offset)
    current.sort((a, b) => a - b)
  }
  settings.value.reminder_offsets_minutes = current
}

const formatReminderOffset = (minutes: number): string => {
  if (minutes < 60) return t('userSettings.reminders.offset.minutes', { n: minutes })
  if (minutes < 1440) return t('userSettings.reminders.offset.hours', { n: minutes / 60 }, minutes / 60)
  if (minutes < 10080) return t('userSettings.reminders.offset.days', { n: minutes / 1440 }, minutes / 1440)
  return t('userSettings.reminders.offset.weeks', { n: minutes / 10080 }, minutes / 10080)
}

const goToUpgrade = () => {
  router.push('/upgrade')
}

onMounted(async () => {
  if (!authStore.isAuthenticated()) return
  fetchUserSettings()
  // Tier fetch is fire-and-forget. If it fails, we default to free which is
  // the safer fallback (the backend gate would still 402 a Pro accent
  // attempt — see `feedback_billing_endpoint_no_request_body.md` for the
  // defense-in-depth pattern).
  try {
    const ent = await getMySubscription()
    tier.value = ent.tier
    // Tell useTheme so the rendered accent gets re-resolved if the user is
    // on a stale Pro preference (downgrade scenario).
    setIsPaid(ent.tier === 'paid')
  } catch {
    /* silent — defaults to free */
  }
})
</script>

<template>
  <section class="flex flex-col max-w-xl">
    <header class="mb-2">
      <p class="text-xs uppercase tracking-wider text-fg-2" data-testid="account-tab-eyebrow">{{ t('account.preferences.eyebrow') }}</p>
      <h1 class="text-3xl md:text-4xl font-medium tracking-tight mt-1 text-fg-1" data-testid="account-tab-heading">
        {{ t('account.preferences.headingLead') }}<span class="font-display italic"> {{ t('account.preferences.headingItalic') }}</span>
      </h1>
      <p class="text-sm text-fg-2 mt-2">{{ t('account.preferences.subtitle') }}</p>
    </header>
    <div v-if="loading" class="text-fg-2 mt-6">{{ t('app.loading') }}</div>
    <div v-else-if="error" class="text-danger-text mt-6" data-testid="preferences-error">{{ error }}</div>
    <div v-else class="bg-bg-1 border border-line rounded-lg p-6 mt-6 flex flex-col gap-6">
      <!-- Theme -->
      <div class="flex flex-col gap-2">
        <label class="text-sm text-fg-2">{{ t('userSettings.theme') }}</label>
        <UiSegmented
          v-model="settings.theme_preference"
          :aria-label="t('userSettings.theme')"
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
          :aria-label="t('userSettings.titleLanguage')"
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
        <AccentPicker
          :model-value="settings.accent_preference"
          :is-paid="isPaid"
          @update:model-value="handleAccentChange"
          @interrupt="handleAccentInterrupt"
        />
        <p class="text-sm text-fg-2">{{ t('account.preferences.proNote') }}</p>
      </div>

      <!-- Reminders -->
      <section class="flex flex-col gap-3" data-testid="reminders-section">
        <header class="flex items-center gap-2">
          <h3 class="font-display text-xl text-fg-1">{{ t('userSettings.reminders.heading') }}</h3>
          <span class="text-sm text-fg-2">
            {{ t('userSettings.reminders.cap', { active: activeRemindersCount, max: REMINDER_CAP }) }}
          </span>
          <button
            type="button"
            class="text-fg-2 [&_svg]:w-3.5 [&_svg]:h-3.5"
            :title="t('userSettings.reminders.capExplanation')"
            :aria-label="t('userSettings.reminders.capExplanation')"
            data-testid="reminders-info-icon"
            @click.prevent
          >
            <IconInfo />
          </button>
        </header>

        <div
          v-if="!isPaid"
          class="rounded-md border border-line bg-bg-2 p-4"
          data-testid="reminders-pro-locked"
        >
          <p class="text-sm text-fg-2">{{ t('userSettings.reminders.proLockedMessage') }}</p>
          <UiButton
            variant="primary"
            size="sm"
            class="mt-2"
            data-testid="reminders-upgrade-cta"
            @click="goToUpgrade"
          >
            {{ t('userSettings.reminders.upgradeCta') }}
          </UiButton>
        </div>

        <ul
          class="flex flex-wrap gap-2"
          :aria-disabled="!isPaid"
          data-testid="reminders-chip-list"
        >
          <li v-for="offset in CANONICAL_REMINDER_OFFSETS" :key="offset">
            <button
              type="button"
              :class="[
                'h-8 px-3 text-sm rounded-full border transition-colors',
                isReminderActive(offset)
                  ? 'bg-accent-1 text-[var(--accent-1-fg)] border-accent-1'
                  : 'bg-bg-1 text-fg-2 border-line hover:text-fg-1',
                (!isPaid || (remindersAtCap && !isReminderActive(offset))) && 'opacity-50 cursor-not-allowed',
              ]"
              :disabled="!isPaid || (remindersAtCap && !isReminderActive(offset))"
              :aria-pressed="isReminderActive(offset)"
              :data-testid="`reminder-chip-${offset}`"
              @click="toggleReminderOffset(offset)"
            >
              {{ formatReminderOffset(offset) }}
            </button>
          </li>
        </ul>
      </section>

      <div class="flex justify-end pt-2">
        <UiButton variant="primary" @click="handleSave">
          {{ t('userSettings.save') }}
        </UiButton>
      </div>
    </div>
  </section>
</template>
