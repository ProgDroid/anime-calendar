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

  it('moves focus to first focusable on open', async () => {
    const w = mount(UiModal, {
      props: { open: false, ariaLabel: 'm' },
      slots: {
        default: '<button data-testid="first">First</button><button data-testid="second">Second</button>',
      },
      attachTo: document.body,
    })
    await w.setProps({ open: true })
    await nextTick()
    await nextTick()
    const first = document.querySelector('[data-testid="first"]') as HTMLElement
    expect(document.activeElement).toBe(first)
    w.unmount()
  })

  it('Tab cycles focus inside the dialog', async () => {
    const w = mount(UiModal, {
      props: { open: false, ariaLabel: 'm' },
      slots: {
        default: '<button data-testid="a">A</button><button data-testid="b">B</button>',
      },
      attachTo: document.body,
    })
    await w.setProps({ open: true })
    await nextTick()
    await nextTick()
    const a = document.querySelector('[data-testid="a"]') as HTMLButtonElement
    const b = document.querySelector('[data-testid="b"]') as HTMLButtonElement
    b.focus()
    expect(document.activeElement).toBe(b)
    const ev = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true })
    document.dispatchEvent(ev)
    expect(document.activeElement).toBe(a)
    w.unmount()
  })

  it('restores focus to invoker on close', async () => {
    const invoker = document.createElement('button')
    invoker.textContent = 'Open'
    document.body.appendChild(invoker)
    invoker.focus()
    expect(document.activeElement).toBe(invoker)

    const w = mount(UiModal, {
      props: { open: false, ariaLabel: 'm' },
      slots: { default: '<button>Inside</button>' },
      attachTo: document.body,
    })
    await w.setProps({ open: true })
    await nextTick()
    await nextTick()
    await w.setProps({ open: false })
    await nextTick()
    expect(document.activeElement).toBe(invoker)
    document.body.removeChild(invoker)
    w.unmount()
  })
})
