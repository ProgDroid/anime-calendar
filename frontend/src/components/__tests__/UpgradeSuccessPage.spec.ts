import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
import { defineComponent, h } from 'vue'

import en from '@/locales/en.json'

const enMessages = en

const getMySubscriptionMock = vi.fn()
vi.mock('@/services/subscription', () => ({
  getMySubscription: (...args: unknown[]) => getMySubscriptionMock(...args),
}))

const toastSuccessMock = vi.fn()
vi.mock('@/services/toastService', () => ({
  toastService: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
  },
}))

import UpgradeSuccessPage from '../UpgradeSuccessPage.vue'

const Stub = defineComponent({ render: () => h('div') })

function makeRouter(query: Record<string, string> = {}) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/upgrade/success', component: UpgradeSuccessPage },
      { path: '/my-calendars', component: Stub, name: 'MyCalendars' },
    ],
  })
  router.replace({ path: '/upgrade/success', query })
  return router
}

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

async function mountPage(query: Record<string, string> = {}) {
  const router = makeRouter(query)
  await router.isReady()
  const wrapper = mount(UpgradeSuccessPage, {
    global: { plugins: [i18n, router], stubs: { IconSparkle: true } },
  })
  return { wrapper, router }
}

describe('UpgradeSuccessPage', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    getMySubscriptionMock.mockReset()
    toastSuccessMock.mockReset()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('starts in the polling state and shows the polling copy', async () => {
    getMySubscriptionMock.mockResolvedValue({
      tier: 'free',
      status: null,
      current_period_end: null,
      cancel_at_period_end: false,
      trial_end: null,
    })
    // session_id is no longer forwarded (F2-28); the server resolves the
    // caller from JWT claims, so the entitlement read takes no argument.
    const { wrapper } = await mountPage()
    await flushPromises()
    expect(wrapper.text()).toContain(enMessages.upgrade.success.polling)
    expect(getMySubscriptionMock).toHaveBeenCalledWith()
  })

  it('flips to success and pushes a toast when the tier becomes paid', async () => {
    getMySubscriptionMock.mockResolvedValue({
      tier: 'paid',
      status: 'active',
      current_period_end: '2030-01-01T00:00:00',
      cancel_at_period_end: false,
      trial_end: null,
    })
    const { wrapper, router } = await mountPage({ session_id: 'cs_test_123' })
    await flushPromises()

    expect(wrapper.text()).toContain(enMessages.upgrade.success.heading)
    expect(toastSuccessMock).toHaveBeenCalledWith(enMessages.upgrade.success.toast)

    // Redirect happens after a short delay; advance the timer past it.
    await vi.advanceTimersByTimeAsync(900)
    await flushPromises()
    expect(router.currentRoute.value.path).toBe('/my-calendars')
  })

  it('flips to timeout state after MAX_ATTEMPTS unsuccessful polls', async () => {
    getMySubscriptionMock.mockResolvedValue({
      tier: 'free',
      status: null,
      current_period_end: null,
      cancel_at_period_end: false,
      trial_end: null,
    })
    const { wrapper } = await mountPage({ session_id: 'cs_test_123' })

    // 10 attempts with a 1-second gap between each = ~10 seconds.
    for (let i = 0; i < 10; i++) {
      await vi.advanceTimersByTimeAsync(1000)
      await flushPromises()
    }

    expect(wrapper.text()).toContain(enMessages.upgrade.success.timeout)
    expect(wrapper.find('[data-testid="upgrade-success-refresh"]').exists()).toBe(true)
  })
})
