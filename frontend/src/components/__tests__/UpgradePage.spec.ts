import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'

import en from '@/locales/en.json'
import pt from '@/locales/pt.json'

// Re-export under the names the assertions use; keeps the typed default
// import that vue-i18n likes while letting the test reference both locales
// by descriptive names.
const enMessages = en
const ptMessages = pt

const createCheckoutSessionMock = vi.fn()
vi.mock('@/services/subscription', () => ({
  createCheckoutSession: (...args: unknown[]) => createCheckoutSessionMock(...args),
}))

import UpgradePage from '../UpgradePage.vue'

function makeI18n(locale: 'en' | 'pt' = 'en') {
  return createI18n({ legacy: false, locale, fallbackLocale: 'en', messages: { en, pt } })
}

function mountPage(locale: 'en' | 'pt' = 'en') {
  return mount(UpgradePage, {
    global: {
      plugins: [makeI18n(locale)],
      stubs: {
        // Stub icons — they're SFCs that don't add to the assertion surface.
        IconCheck: true,
        IconSparkle: true,
      },
    },
  })
}

describe('UpgradePage', () => {
  const originalLocation = window.location

  beforeEach(() => {
    createCheckoutSessionMock.mockReset()
    // Replace window.location so we can assert redirects without navigating.
    Object.defineProperty(window, 'location', {
      writable: true,
      value: { href: '' } as Location,
    })
  })

  afterEach(() => {
    Object.defineProperty(window, 'location', { writable: true, value: originalLocation })
  })

  it('renders pricing eyebrow + heading and the monthly/annual toggle', () => {
    const wrapper = mountPage()
    expect(wrapper.text()).toContain(enMessages.pricing.eyebrow)
    expect(wrapper.text()).toContain(enMessages.pricing.heading)
    expect(wrapper.text()).toContain(enMessages.pricing.interval.monthly)
    expect(wrapper.text()).toContain(enMessages.pricing.interval.annual)
  })

  it('shows the savings chip only when annual is selected', async () => {
    const wrapper = mountPage()
    expect(wrapper.text()).not.toContain(enMessages.pricing.price.savings)

    await wrapper
      .findAll('button')
      .find((b) => b.text() === enMessages.pricing.interval.annual)
      ?.trigger('click')

    expect(wrapper.text()).toContain(enMessages.pricing.price.savings)
  })

  it('posts the selected interval to the checkout service and redirects to the returned url', async () => {
    createCheckoutSessionMock.mockResolvedValue({ url: 'https://checkout.stripe.com/c_1' })
    const wrapper = mountPage()

    // Switch to annual first, then click CTA.
    await wrapper
      .findAll('button')
      .find((b) => b.text() === enMessages.pricing.interval.annual)
      ?.trigger('click')

    await wrapper.find('[data-testid="upgrade-cta"]').trigger('click')
    await flushPromises()

    expect(createCheckoutSessionMock).toHaveBeenCalledTimes(1)
    expect(createCheckoutSessionMock).toHaveBeenCalledWith('annual')
    expect(window.location.href).toBe('https://checkout.stripe.com/c_1')
  })

  it('surfaces an error message when checkout creation fails', async () => {
    createCheckoutSessionMock.mockRejectedValue(new Error('boom'))
    const wrapper = mountPage()

    await wrapper.find('[data-testid="upgrade-cta"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain(enMessages.upgrade.errors.checkoutFailed)
    // window should not have been redirected
    expect(window.location.href).toBe('')
  })

  it('renders Portuguese copy when locale=pt', () => {
    const wrapper = mountPage('pt')
    expect(wrapper.text()).toContain(ptMessages.pricing.heading)
    expect(wrapper.text()).toContain(ptMessages.upgrade.cta.startTrial)
  })
})

describe('locale parity', () => {
  it('en + pt have the same upgrade.* and pricing.* key shape', () => {
    const collect = (obj: unknown, prefix = ''): string[] => {
      if (typeof obj !== 'object' || obj === null) return [prefix]
      return Object.entries(obj as Record<string, unknown>).flatMap(([k, v]) =>
        collect(v, prefix ? `${prefix}.${k}` : k),
      )
    }
    const enRecord = en as Record<string, unknown>
    const ptRecord = pt as Record<string, unknown>
    for (const ns of ['pricing', 'upgrade'] as const) {
      const enKeys = collect(enRecord[ns]).sort()
      const ptKeys = collect(ptRecord[ns]).sort()
      expect(ptKeys).toEqual(enKeys)
    }
  })
})

