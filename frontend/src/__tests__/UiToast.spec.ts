import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import UiToast from '@/components/ui/UiToast.vue'
import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const mountOpts = { global: { plugins: [i18n] } }

describe('UiToast', () => {
  beforeEach(() => vi.useFakeTimers())

  it('renders message', () => {
    const w = mount(UiToast, { props: { message: 'hi' }, ...mountOpts })
    expect(w.text()).toContain('hi')
  })

  it('auto-dismisses after duration', async () => {
    const w = mount(UiToast, { props: { message: 'x', duration: 1000 }, ...mountOpts })
    await vi.advanceTimersByTimeAsync(1000)
    expect(w.emitted('dismiss')).toBeTruthy()
  })

  it('duration=0 stays sticky', async () => {
    const w = mount(UiToast, { props: { message: 'x', duration: 0 }, ...mountOpts })
    await vi.advanceTimersByTimeAsync(10000)
    expect(w.emitted('dismiss')).toBeUndefined()
  })

  it('danger variant uses danger class', () => {
    const w = mount(UiToast, { props: { message: 'x', variant: 'danger' }, ...mountOpts })
    expect(w.classes().some(c => c.includes('danger'))).toBe(true)
  })

  it('dismiss button emits dismiss', async () => {
    const w = mount(UiToast, { props: { message: 'x', duration: 0 }, ...mountOpts })
    await w.find('button').trigger('click')
    expect(w.emitted('dismiss')).toBeTruthy()
  })
})
