import { describe, it, expect, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import ConfirmModal from '@/components/shared/ConfirmModal.vue'
import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const mountOpts = { global: { plugins: [i18n] }, attachTo: document.body }

interface Props {
  title: string
  message: string
  open: boolean
  confirmLabel?: string
  cancelLabel?: string
  danger?: boolean
}

const mounted: ReturnType<typeof mount>[] = []

afterEach(() => {
  while (mounted.length) mounted.pop()?.unmount()
})

function mountModal(props: Props) {
  const wrapper = mount(ConfirmModal, { props, ...mountOpts })
  mounted.push(wrapper)
  return wrapper
}

describe('ConfirmModal', () => {
  it('renders title and message', () => {
    mountModal({ title: 'Delete?', message: 'This cannot be undone.', open: true })
    expect(document.body.textContent).toContain('Delete?')
    expect(document.body.textContent).toContain('This cannot be undone.')
  })

  it('emits confirm when confirm button clicked', async () => {
    const wrapper = mountModal({ title: 'Delete?', message: 'Sure?', open: true })
    const btn = document.body.querySelector('[data-testid="confirm-btn"]') as HTMLElement
    btn.click()
    await wrapper.vm.$nextTick()
    expect(wrapper.emitted('confirm')).toBeTruthy()
  })

  it('emits cancel when cancel button clicked', async () => {
    const wrapper = mountModal({ title: 'Delete?', message: 'Sure?', open: true })
    const btn = document.body.querySelector('[data-testid="cancel-btn"]') as HTMLElement
    btn.click()
    await wrapper.vm.$nextTick()
    expect(wrapper.emitted('cancel')).toBeTruthy()
  })

  it('is hidden when open is false', () => {
    mountModal({ title: 'Delete?', message: 'Sure?', open: false })
    expect(document.body.querySelector('[data-testid="modal-box"]')).toBeNull()
  })
})
