import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import ConfirmModal from '@/components/shared/ConfirmModal.vue'
import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const mountOpts = { global: { plugins: [i18n] } }

describe('ConfirmModal', () => {
  it('renders title and message', () => {
    const wrapper = mount(ConfirmModal, {
      props: { title: 'Delete?', message: 'This cannot be undone.', open: true },
      ...mountOpts
    })
    expect(wrapper.text()).toContain('Delete?')
    expect(wrapper.text()).toContain('This cannot be undone.')
  })

  it('emits confirm when confirm button clicked', async () => {
    const wrapper = mount(ConfirmModal, {
      props: { title: 'Delete?', message: 'Sure?', open: true },
      ...mountOpts
    })
    await wrapper.find('[data-testid="confirm-btn"]').trigger('click')
    expect(wrapper.emitted('confirm')).toBeTruthy()
  })

  it('emits cancel when cancel button clicked', async () => {
    const wrapper = mount(ConfirmModal, {
      props: { title: 'Delete?', message: 'Sure?', open: true },
      ...mountOpts
    })
    await wrapper.find('[data-testid="cancel-btn"]').trigger('click')
    expect(wrapper.emitted('cancel')).toBeTruthy()
  })

  it('is hidden when open is false', () => {
    const wrapper = mount(ConfirmModal, {
      props: { title: 'Delete?', message: 'Sure?', open: false },
      ...mountOpts
    })
    expect(wrapper.find('[data-testid="modal-box"]').exists()).toBe(false)
  })
})
