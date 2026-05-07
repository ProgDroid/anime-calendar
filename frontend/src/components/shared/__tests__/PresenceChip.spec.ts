import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import PresenceChip from '../PresenceChip.vue'

// Stub auth store — user is viewer #1
vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({ user: 'Me' }),
}))

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      sharing: {
        presence: {
          justYou: 'Just you',
          youPlusN: 'You + {n} other | You + {n} others',
          nViewing: '{n} viewing',
        },
      },
    },
  },
})

const mountChip = (viewers: { user_id: number; display: string }[]) =>
  mount(PresenceChip, {
    props: { viewers },
    global: { plugins: [i18n] },
  })

describe('PresenceChip', () => {
  it('shows "Just you" when only the current user is present', () => {
    const wrapper = mountChip([{ user_id: 1, display: 'Me' }])
    expect(wrapper.find('[data-testid="presence-chip"]').text()).toBe('Just you')
  })

  it('shows "You + N others" when others are also present', () => {
    const wrapper = mountChip([
      { user_id: 1, display: 'Me' },
      { user_id: 2, display: 'Alice' },
    ])
    expect(wrapper.find('[data-testid="presence-chip"]').text()).toContain('You + 1')
  })

  it('shows "N viewing" when current user is not in the list', () => {
    const wrapper = mountChip([{ user_id: 2, display: 'Alice' }])
    expect(wrapper.find('[data-testid="presence-chip"]').text()).toBe('1 viewing')
  })

  it('popover is hidden by default', () => {
    const wrapper = mountChip([{ user_id: 1, display: 'Me' }])
    expect(wrapper.find('[data-testid="presence-popover"]').exists()).toBe(false)
  })

  it('opens popover on chip click', async () => {
    const wrapper = mountChip([{ user_id: 1, display: 'Me' }])
    await wrapper.find('[data-testid="presence-chip"]').trigger('click')
    expect(wrapper.find('[data-testid="presence-popover"]').exists()).toBe(true)
  })

  it('closes popover on second click', async () => {
    const wrapper = mountChip([{ user_id: 1, display: 'Me' }])
    await wrapper.find('[data-testid="presence-chip"]').trigger('click')
    await wrapper.find('[data-testid="presence-chip"]').trigger('click')
    expect(wrapper.find('[data-testid="presence-popover"]').exists()).toBe(false)
  })

  it('popover lists all viewer display names', async () => {
    const wrapper = mountChip([
      { user_id: 1, display: 'Me' },
      { user_id: 2, display: 'Alice' },
    ])
    await wrapper.find('[data-testid="presence-chip"]').trigger('click')
    const popover = wrapper.find('[data-testid="presence-popover"]')
    expect(popover.text()).toContain('Me')
    expect(popover.text()).toContain('Alice')
  })
})
