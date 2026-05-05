import { describe, it, expect, vi, beforeEach } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import en from '@/locales/en.json'
import { CANONICAL_REMINDER_OFFSETS } from '@/types/userSettings'

// Mock subscription service so we can flip tier per-test.
const getMySubscriptionMock = vi.fn()
vi.mock('@/services/subscription', () => ({
  getMySubscription: (...args: unknown[]) => getMySubscriptionMock(...args),
}))

// Mock auth store — `isAuthenticated()` must return true so onMounted runs the
// settings fetch. Default an authenticated user; tests can override.
vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({ isAuthenticated: () => true }),
}))

// Stub useTheme so accent application is a no-op.
vi.mock('@/composables/useTheme', () => ({
  useTheme: () => ({ setAccent: vi.fn(), setIsPaid: vi.fn() }),
}))

// Stub applySettings to avoid real DOM mutations.
vi.mock('@/services/applySettings', () => ({
  applySettings: vi.fn(),
}))

// In-memory user settings store mock — capture values returned by fetchSettings.
const fetchSettingsMock = vi.fn()
const updateSettingsMock = vi.fn()
vi.mock('@/stores/userSettingsStore', () => ({
  useUserSettingsStore: () => ({
    fetchSettings: fetchSettingsMock,
    updateSettings: updateSettingsMock,
    getDefaultSettings: () => ({
      theme_preference: 'dark',
      language_preference: 'en',
      title_language_preference: 'English',
      accent_preference: 'coral',
      timezone: 'UTC',
      reminder_offsets_minutes: [30],
    }),
  }),
}))

// Stub the upgrade modal trigger.
const openUpgradeModalMock = vi.fn()
vi.mock('@/composables/useUpgradeInterrupt', () => ({
  useUpgradeInterrupt: () => ({ openUpgradeModal: openUpgradeModalMock }),
}))

import PreferencesTab from '../PreferencesTab.vue'

const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages: { en } })

// Spy that captures router.push calls without polluting other test files —
// per memory `feedback_test_history_pollution`, use createMemoryHistory.
const routerPushSpy = vi.fn()

function mountTab() {
  setActivePinia(createPinia())
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: { template: '<div />' } },
      { path: '/upgrade', component: { template: '<div />' } },
    ],
  })
  // Override push so we can assert without committing to a real navigation
  // (which would resolve the route component and then complete async).
  router.push = routerPushSpy as unknown as typeof router.push
  return mount(PreferencesTab, {
    global: {
      plugins: [i18n, router],
      stubs: {
        IconGlobe: true,
        IconInfo: true,
        // Real AccentPicker pulls in icons; stub for isolation.
        AccentPicker: true,
      },
    },
  })
}

beforeEach(() => {
  fetchSettingsMock.mockReset()
  updateSettingsMock.mockReset()
  getMySubscriptionMock.mockReset()
  openUpgradeModalMock.mockReset()
  routerPushSpy.mockReset()
})

describe('PreferencesTab — reminders chip-list', () => {
  it(`renders ${CANONICAL_REMINDER_OFFSETS.length} reminder chips`, async () => {
    fetchSettingsMock.mockResolvedValue({
      theme_preference: 'dark',
      language_preference: 'en',
      title_language_preference: 'English',
      accent_preference: 'coral',
      timezone: 'UTC',
      reminder_offsets_minutes: [30],
    })
    getMySubscriptionMock.mockResolvedValue({ tier: 'paid' })
    const wrapper = mountTab()
    await flushPromises()
    const chips = wrapper.findAll('[data-testid^="reminder-chip-"]')
    expect(chips.length).toBe(CANONICAL_REMINDER_OFFSETS.length)
  })

  it('pro user can toggle a chip active', async () => {
    fetchSettingsMock.mockResolvedValue({
      theme_preference: 'dark',
      language_preference: 'en',
      title_language_preference: 'English',
      accent_preference: 'coral',
      timezone: 'UTC',
      reminder_offsets_minutes: [30],
    })
    getMySubscriptionMock.mockResolvedValue({ tier: 'paid' })
    const wrapper = mountTab()
    await flushPromises()
    const chip60 = wrapper.get('[data-testid="reminder-chip-60"]')
    expect(chip60.attributes('aria-pressed')).toBe('false')
    await chip60.trigger('click')
    expect(wrapper.get('[data-testid="reminder-chip-60"]').attributes('aria-pressed')).toBe('true')
  })

  it('pro user with 5 active sees inactive chips disabled', async () => {
    fetchSettingsMock.mockResolvedValue({
      theme_preference: 'dark',
      language_preference: 'en',
      title_language_preference: 'English',
      accent_preference: 'coral',
      timezone: 'UTC',
      reminder_offsets_minutes: [15, 30, 60, 120, 360],
    })
    getMySubscriptionMock.mockResolvedValue({ tier: 'paid' })
    const wrapper = mountTab()
    await flushPromises()
    // 720 is inactive; should be disabled at cap.
    expect(wrapper.get('[data-testid="reminder-chip-720"]').attributes('disabled')).toBeDefined()
    // An active one remains enabled (so user can deactivate).
    expect(wrapper.get('[data-testid="reminder-chip-30"]').attributes('disabled')).toBeUndefined()
  })

  it('clicking disabled chip at cap is no-op', async () => {
    fetchSettingsMock.mockResolvedValue({
      theme_preference: 'dark',
      language_preference: 'en',
      title_language_preference: 'English',
      accent_preference: 'coral',
      timezone: 'UTC',
      reminder_offsets_minutes: [15, 30, 60, 120, 360],
    })
    getMySubscriptionMock.mockResolvedValue({ tier: 'paid' })
    const wrapper = mountTab()
    await flushPromises()
    const chip720 = wrapper.get('[data-testid="reminder-chip-720"]')
    await chip720.trigger('click')
    expect(chip720.attributes('aria-pressed')).toBe('false')
  })

  it('free user sees pro-locked overlay with upgrade cta', async () => {
    fetchSettingsMock.mockResolvedValue({
      theme_preference: 'dark',
      language_preference: 'en',
      title_language_preference: 'English',
      accent_preference: 'coral',
      timezone: 'UTC',
      reminder_offsets_minutes: [30],
    })
    getMySubscriptionMock.mockResolvedValue({ tier: 'free' })
    const wrapper = mountTab()
    await flushPromises()
    expect(wrapper.find('[data-testid="reminders-pro-locked"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="reminders-upgrade-cta"]').exists()).toBe(true)
  })

  it('free user clicking upgrade cta navigates to /upgrade', async () => {
    fetchSettingsMock.mockResolvedValue({
      theme_preference: 'dark',
      language_preference: 'en',
      title_language_preference: 'English',
      accent_preference: 'coral',
      timezone: 'UTC',
      reminder_offsets_minutes: [30],
    })
    getMySubscriptionMock.mockResolvedValue({ tier: 'free' })
    const wrapper = mountTab()
    await flushPromises()
    await wrapper.get('[data-testid="reminders-upgrade-cta"]').trigger('click')
    expect(routerPushSpy).toHaveBeenCalledWith('/upgrade')
  })

  it('free user cannot toggle chips', async () => {
    fetchSettingsMock.mockResolvedValue({
      theme_preference: 'dark',
      language_preference: 'en',
      title_language_preference: 'English',
      accent_preference: 'coral',
      timezone: 'UTC',
      reminder_offsets_minutes: [30],
    })
    getMySubscriptionMock.mockResolvedValue({ tier: 'free' })
    const wrapper = mountTab()
    await flushPromises()
    const chip60 = wrapper.get('[data-testid="reminder-chip-60"]')
    expect(chip60.attributes('disabled')).toBeDefined()
    await chip60.trigger('click')
    expect(chip60.attributes('aria-pressed')).toBe('false')
  })
})
