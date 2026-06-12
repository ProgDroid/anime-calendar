import { describe, it, expect, vi, beforeEach } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import en from '@/locales/en.json'

// ── api mock ─────────────────────────────────────────────────────────────────
const apiDeleteMock = vi.fn()
const apiPostMock = vi.fn()
vi.mock('@/config/api', () => ({
  default: {
    delete: (...args: unknown[]) => apiDeleteMock(...args),
    post: (...args: unknown[]) => apiPostMock(...args),
  },
}))

// ── axios isAxiosError mock ───────────────────────────────────────────────────
vi.mock('axios', () => {
  const post = vi.fn()
  const get = vi.fn()
  // Unified shape across all axios-mocking specs so a cross-file mock-registry
  // leak can't strip `axios.post` (→ "mockResolvedValue is not a function") or
  // swap the error predicate. Recognises either marker the specs use (F2-32).
  const isAxiosError = (
    err: unknown,
  ): err is { response?: { status?: number; data?: Record<string, unknown> } } =>
    typeof err === 'object' && err !== null && ('isAxiosError' in err || '__isAxiosError' in err)
  return { default: { post, get, isAxiosError }, isAxiosError }
})

// ── authStore mock ────────────────────────────────────────────────────────────
const logoutMock = vi.fn()
vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({
    userId: 42,
    logout: logoutMock,
  }),
}))

// ── userSettingsStore mock ────────────────────────────────────────────────────
const clearCacheMock = vi.fn()
const getDefaultSettingsMock = vi.fn().mockReturnValue({
  theme_preference: 'dark',
  language_preference: 'en',
  title_language_preference: 'English',
  accent_preference: 'coral',
  timezone: 'UTC',
  reminder_offsets_minutes: [30],
})
vi.mock('@/stores/userSettingsStore', () => ({
  useUserSettingsStore: () => ({
    clearCache: clearCacheMock,
    getDefaultSettings: getDefaultSettingsMock,
  }),
}))

// ── applySettings mock ────────────────────────────────────────────────────────
vi.mock('@/services/applySettings', () => ({
  applySettings: vi.fn(),
}))

// ── useUpgradeInterrupt mock (required by api.ts import chain) ────────────────
vi.mock('@/composables/useUpgradeInterrupt', () => ({
  useUpgradeInterrupt: () => ({ openUpgradeModal: vi.fn() }),
}))

import DangerZoneTab from '../DangerZoneTab.vue'

const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages: { en } })

function makeAxiosError(status: number, data: Record<string, unknown>) {
  return Object.assign(new Error('axios error'), {
    __isAxiosError: true,
    response: { status, data },
  })
}

function mountTab() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/', component: { template: '<div />' } }],
  })
  const pushSpy = vi.spyOn(router, 'push').mockResolvedValue(undefined)
  const wrapper = mount(DangerZoneTab, {
    global: {
      plugins: [i18n, createPinia(), router],
    },
    attachTo: document.body,
  })
  return { wrapper, pushSpy }
}

describe('DangerZoneTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    setActivePinia(createPinia())
  })

  // ── 204 happy path ──────────────────────────────────────────────────────────
  it('calls DELETE /user/:id and logs out on 204 success', async () => {
    apiDeleteMock.mockResolvedValueOnce({})
    const { wrapper, pushSpy } = mountTab()

    const confirmModal = wrapper.findComponent({ name: 'ConfirmModal' })
    await confirmModal.vm.$emit('confirm')
    await flushPromises()

    expect(apiDeleteMock).toHaveBeenCalledWith('/user/42')
    expect(logoutMock).toHaveBeenCalled()
    expect(pushSpy).toHaveBeenCalledWith('/login')
  })

  // ── 409 active subscription path ────────────────────────────────────────────
  it('shows active-subscription modal when delete returns 409 with active_subscription', async () => {
    apiDeleteMock.mockRejectedValueOnce(
      makeAxiosError(409, { error: 'active_subscription' }),
    )
    const { wrapper } = mountTab()

    const confirmModal = wrapper.findComponent({ name: 'ConfirmModal' })
    await confirmModal.vm.$emit('confirm')
    await flushPromises()

    // UiModal Teleports to document.body — query the body
    const title = document.body.querySelector('[data-testid="active-sub-title"]')
    expect(title).not.toBeNull()
    expect(title?.textContent?.trim()).toContain('Active subscription')

    const message = document.body.querySelector('[data-testid="active-sub-message"]')
    expect(message?.textContent).toContain('Cancel it via the Stripe portal')

    expect(logoutMock).not.toHaveBeenCalled()
  })

  // ── portal button navigates ──────────────────────────────────────────────────
  it('clicking Open Stripe portal calls POST /stripe/portal', async () => {
    apiDeleteMock.mockRejectedValueOnce(
      makeAxiosError(409, { error: 'active_subscription' }),
    )
    apiPostMock.mockResolvedValueOnce({ data: { url: 'https://billing.stripe.com/portal/test' } })

    const { wrapper } = mountTab()

    const confirmModal = wrapper.findComponent({ name: 'ConfirmModal' })
    await confirmModal.vm.$emit('confirm')
    await flushPromises()

    const portalBtn = document.body.querySelector<HTMLElement>('[data-testid="open-portal-button"]')
    expect(portalBtn).not.toBeNull()
    portalBtn!.click()
    await flushPromises()

    expect(apiPostMock).toHaveBeenCalledWith('/stripe/portal')
  })

  // ── generic delete failure (non-409) ────────────────────────────────────────
  it('shows generic error on unexpected delete failure', async () => {
    apiDeleteMock.mockRejectedValueOnce(makeAxiosError(500, { error: 'internal' }))
    const { wrapper } = mountTab()

    const confirmModal = wrapper.findComponent({ name: 'ConfirmModal' })
    await confirmModal.vm.$emit('confirm')
    await flushPromises()

    const err = wrapper.find('[data-testid="danger-error"]')
    expect(err.exists()).toBe(true)
    expect(document.body.querySelector('[data-testid="active-sub-modal"]')).toBeNull()
    expect(logoutMock).not.toHaveBeenCalled()
  })

  // ── portal-fetch failure surfaces inside the modal ──────────────────────────
  it('shows portal-failed message inside the modal when /stripe/portal fails', async () => {
    apiDeleteMock.mockRejectedValueOnce(
      makeAxiosError(409, { error: 'active_subscription' }),
    )
    apiPostMock.mockRejectedValueOnce(new Error('portal down'))

    const { wrapper } = mountTab()

    const confirmModal = wrapper.findComponent({ name: 'ConfirmModal' })
    await confirmModal.vm.$emit('confirm')
    await flushPromises()

    const portalBtn = document.body.querySelector<HTMLElement>('[data-testid="open-portal-button"]')
    portalBtn!.click()
    await flushPromises()

    const portalErr = document.body.querySelector('[data-testid="active-sub-error"]')
    expect(portalErr).not.toBeNull()
    expect(portalErr?.textContent).toContain('Could not open the Stripe portal')

    // Generic delete-error banner stays absent
    expect(wrapper.find('[data-testid="danger-error"]').exists()).toBe(false)
  })
})
