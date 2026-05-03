import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'
import UiBottomSheet from '../UiBottomSheet.vue'
import { _resetBodyScrollLockForTests } from '@/composables/useBodyScrollLock'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, pt } })

function mountSheet(modelValue: boolean) {
  return mount(UiBottomSheet, {
    props: { modelValue },
    slots: { default: '<p>hello</p>' },
    attachTo: document.body,
    global: { plugins: [i18n] },
  })
}

describe('UiBottomSheet', () => {
  beforeEach(() => {
    _resetBodyScrollLockForTests()
    document.body.replaceChildren()
    document.body.style.overflow = ''
  })
  afterEach(() => {
    document.body.replaceChildren()
    document.body.style.overflow = ''
  })

  it('does not render the panel when modelValue is false', () => {
    const w = mountSheet(false)
    expect(document.body.querySelector('[data-testid="bottom-sheet-panel"]')).toBeNull()
    w.unmount()
  })

  it('renders slot content when open and teleports to body', () => {
    const w = mountSheet(true)
    expect(document.body.querySelector('[data-testid="bottom-sheet-panel"]')).not.toBeNull()
    expect(document.body.textContent).toContain('hello')
    w.unmount()
  })

  it('emits update:modelValue=false when close button is clicked', async () => {
    const w = mountSheet(true)
    const closeBtn = document.body.querySelector(
      '[data-testid="bottom-sheet-close"]',
    ) as HTMLElement | null
    expect(closeBtn).not.toBeNull()
    closeBtn?.click()
    expect(w.emitted('update:modelValue')?.[0]).toEqual([false])
    w.unmount()
  })

  it('emits update:modelValue=false when backdrop is clicked', async () => {
    const w = mountSheet(true)
    const backdrop = document.body.querySelector(
      '[data-testid="bottom-sheet-backdrop"]',
    ) as HTMLElement | null
    expect(backdrop).not.toBeNull()
    backdrop?.click()
    expect(w.emitted('update:modelValue')?.[0]).toEqual([false])
    w.unmount()
  })

  it('emits update:modelValue=false when ESC is pressed', async () => {
    const w = mountSheet(true)
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(w.emitted('update:modelValue')?.[0]).toEqual([false])
    w.unmount()
  })

  it('locks body scroll while open', async () => {
    const w = mountSheet(true)
    expect(document.body.style.overflow).toBe('hidden')
    await w.setProps({ modelValue: false })
    expect(document.body.style.overflow).toBe('')
    w.unmount()
  })

  it('renders a decorative drag-handle visual', () => {
    const w = mountSheet(true)
    const handle = document.body.querySelector('[data-testid="bottom-sheet-handle"]')
    expect(handle).not.toBeNull()
    expect(handle?.getAttribute('aria-hidden')).toBe('true')
    w.unmount()
  })
})
