import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiModal from '@/components/ui/UiModal.vue'
import { nextTick } from 'vue'

describe('UiModal', () => {
  it('does not render dialog when open=false', () => {
    const w = mount(UiModal, { props: { open: false, ariaLabel: 'm' }, attachTo: document.body })
    expect(document.querySelector('[role="dialog"]')).toBeNull()
    w.unmount()
  })

  it('renders dialog with aria-modal when open=true', () => {
    const w = mount(UiModal, { props: { open: true, ariaLabel: 'My modal' }, attachTo: document.body })
    const d = document.querySelector('[role="dialog"]')
    expect(d).not.toBeNull()
    expect(d!.getAttribute('aria-modal')).toBe('true')
    expect(d!.getAttribute('aria-label')).toBe('My modal')
    w.unmount()
  })

  it('Esc emits close', async () => {
    const w = mount(UiModal, { props: { open: true, ariaLabel: 'm' }, attachTo: document.body })
    await nextTick()
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(w.emitted('close')).toBeTruthy()
    w.unmount()
  })

  it('scrim click respects closeOnScrim=false', async () => {
    const w = mount(UiModal, { props: { open: true, ariaLabel: 'm', closeOnScrim: false }, attachTo: document.body })
    await document.querySelector('[data-testid="scrim"]')!.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    expect(w.emitted('close')).toBeUndefined()
    w.unmount()
  })
})
