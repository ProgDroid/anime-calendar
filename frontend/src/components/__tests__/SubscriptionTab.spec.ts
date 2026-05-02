import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
import { defineComponent, h } from 'vue'
import { setActivePinia, createPinia } from 'pinia'

import en from '@/locales/en.json'
import pt from '@/locales/pt.json'

const enMessages = en
const ptMessages = pt

const getMySubscriptionMock = vi.fn()
const createPortalSessionMock = vi.fn()

vi.mock('@/services/subscription', () => ({
  getMySubscription: (...args: unknown[]) => getMySubscriptionMock(...args),
  createPortalSession: (...args: unknown[]) => createPortalSessionMock(...args),
}))

// useAuthStore.isAuthenticated() must return true so onMounted fires the
// fetch. Stub the store rather than going through real Pinia state.
vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({
    isAuthenticated: () => true,
  }),
}))

import SubscriptionTab from '../account/SubscriptionTab.vue'

const Stub = defineComponent({ render: () => h('div') })

function makeRouter() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/account/subscription', component: SubscriptionTab },
      { path: '/upgrade', component: Stub, name: 'Upgrade' },
    ],
  })
  router.replace('/account/subscription')
  return router
}

async function mountTab(locale: 'en' | 'pt' = 'en') {
  setActivePinia(createPinia())
  const i18n = createI18n({ legacy: false, locale, fallbackLocale: 'en', messages: { en, pt } })
  const router = makeRouter()
  await router.isReady()
  const wrapper = mount(SubscriptionTab, {
    global: {
      plugins: [i18n, router],
      stubs: {
        IconSparkle: true,
      },
    },
  })
  return { wrapper, router }
}

describe('SubscriptionTab', () => {
  const originalLocation = window.location

  beforeEach(() => {
    getMySubscriptionMock.mockReset()
    createPortalSessionMock.mockReset()
    Object.defineProperty(window, 'location', {
      writable: true,
      value: { href: '' } as Location,
    })
  })

  afterEach(() => {
    Object.defineProperty(window, 'location', { writable: true, value: originalLocation })
  })

  it('renders the eyebrow + heading + subtitle from i18n', async () => {
    getMySubscriptionMock.mockResolvedValue({
      tier: 'free',
      status: null,
      current_period_end: null,
      cancel_at_period_end: false,
      trial_end: null,
    })
    const { wrapper } = await mountTab()
    await flushPromises()
    expect(wrapper.text()).toContain(enMessages.account.subscription.eyebrow)
    expect(wrapper.text()).toContain(enMessages.account.subscription.subtitle)
  })

  it('shows the free-state surface and routes to /upgrade on CTA click', async () => {
    getMySubscriptionMock.mockResolvedValue({
      tier: 'free',
      status: null,
      current_period_end: null,
      cancel_at_period_end: false,
      trial_end: null,
    })
    const { wrapper, router } = await mountTab()
    await flushPromises()
    expect(wrapper.find('[data-testid="subscription-free"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(enMessages.account.subscription.freeState.title)
    await wrapper.find('[data-testid="subscription-upgrade-cta"]').trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.path).toBe('/upgrade')
  })

  it('shows renews-on for an active paid subscription that is not canceling', async () => {
    getMySubscriptionMock.mockResolvedValue({
      tier: 'paid',
      status: 'active',
      current_period_end: '2030-01-15T00:00:00',
      cancel_at_period_end: false,
      trial_end: null,
    })
    const { wrapper } = await mountTab()
    await flushPromises()
    expect(wrapper.find('[data-testid="subscription-paid"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="subscription-renews-on"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="subscription-cancels-on"]').exists()).toBe(false)
  })

  it('shows cancels-on copy when cancel_at_period_end is true', async () => {
    getMySubscriptionMock.mockResolvedValue({
      tier: 'paid',
      status: 'active',
      current_period_end: '2030-01-15T00:00:00',
      cancel_at_period_end: true,
      trial_end: null,
    })
    const { wrapper } = await mountTab()
    await flushPromises()
    expect(wrapper.find('[data-testid="subscription-cancels-on"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="subscription-renews-on"]').exists()).toBe(false)
  })

  it('renders the past-due banner when status=past_due', async () => {
    getMySubscriptionMock.mockResolvedValue({
      tier: 'paid',
      status: 'past_due',
      current_period_end: '2030-01-15T00:00:00',
      cancel_at_period_end: false,
      trial_end: null,
    })
    const { wrapper } = await mountTab()
    await flushPromises()
    expect(wrapper.find('[data-testid="subscription-past_due"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="subscription-past-due-banner"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(enMessages.account.subscription.pastDueBanner)
  })

  it('renders the trial banner + trial-end date when status=trialing', async () => {
    getMySubscriptionMock.mockResolvedValue({
      tier: 'paid',
      status: 'trialing',
      current_period_end: '2030-01-15T00:00:00',
      cancel_at_period_end: false,
      trial_end: '2030-01-08T00:00:00',
    })
    const { wrapper } = await mountTab()
    await flushPromises()
    expect(wrapper.find('[data-testid="subscription-trialing"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="subscription-trial-banner"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="subscription-trial-end"]').exists()).toBe(true)
  })

  it('opens the Stripe portal on Manage click — POSTs and redirects', async () => {
    getMySubscriptionMock.mockResolvedValue({
      tier: 'paid',
      status: 'active',
      current_period_end: '2030-01-15T00:00:00',
      cancel_at_period_end: false,
      trial_end: null,
    })
    createPortalSessionMock.mockResolvedValue({ url: 'https://billing.stripe.com/p/session_1' })
    const { wrapper } = await mountTab()
    await flushPromises()
    await wrapper.find('[data-testid="subscription-manage-cta"]').trigger('click')
    await flushPromises()
    expect(createPortalSessionMock).toHaveBeenCalledTimes(1)
    expect(window.location.href).toBe('https://billing.stripe.com/p/session_1')
  })

  it('surfaces a portal error when /api/stripe/portal fails', async () => {
    getMySubscriptionMock.mockResolvedValue({
      tier: 'paid',
      status: 'active',
      current_period_end: '2030-01-15T00:00:00',
      cancel_at_period_end: false,
      trial_end: null,
    })
    createPortalSessionMock.mockRejectedValue(new Error('boom'))
    const { wrapper } = await mountTab()
    await flushPromises()
    await wrapper.find('[data-testid="subscription-manage-cta"]').trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="subscription-portal-error"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(enMessages.account.subscription.errors.portalFailed)
    expect(window.location.href).toBe('')
  })

  it('shows the fetchFailed message when GET /api/subscription/me errors', async () => {
    getMySubscriptionMock.mockRejectedValue(new Error('500'))
    const { wrapper } = await mountTab()
    await flushPromises()
    expect(wrapper.find('[data-testid="subscription-error"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(enMessages.account.subscription.errors.fetchFailed)
  })

  it('renders Portuguese copy when locale=pt', async () => {
    getMySubscriptionMock.mockResolvedValue({
      tier: 'free',
      status: null,
      current_period_end: null,
      cancel_at_period_end: false,
      trial_end: null,
    })
    const { wrapper } = await mountTab('pt')
    await flushPromises()
    expect(wrapper.text()).toContain(ptMessages.account.subscription.freeState.title)
  })
})

describe('SubscriptionTab locale parity', () => {
  it('en + pt have the same account.subscription.* key shape', () => {
    const collect = (obj: unknown, prefix = ''): string[] => {
      if (typeof obj !== 'object' || obj === null) return [prefix]
      return Object.entries(obj as Record<string, unknown>).flatMap(([k, v]) =>
        collect(v, prefix ? `${prefix}.${k}` : k),
      )
    }
    const enRecord = (en as Record<string, unknown>).account as Record<string, unknown>
    const ptRecord = (pt as Record<string, unknown>).account as Record<string, unknown>
    const enKeys = collect(enRecord.subscription).sort()
    const ptKeys = collect(ptRecord.subscription).sort()
    expect(ptKeys).toEqual(enKeys)
  })
})
