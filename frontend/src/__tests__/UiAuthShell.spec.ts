import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import UiAuthShell from '@/components/ui/UiAuthShell.vue'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  fallbackLocale: 'en',
  messages: { en, pt },
})

const mountShell = (options: Parameters<typeof mount>[1] = {}) =>
  mount(UiAuthShell, {
    ...options,
    global: { ...options.global, plugins: [i18n] },
  })

describe('UiAuthShell', () => {
  it('renders default slot content', () => {
    const wrapper = mountShell({
      slots: { default: '<p data-testid="slot-content">hello</p>' },
    })
    expect(wrapper.find('[data-testid="slot-content"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('hello')
  })

  it('renders the poster pane with the provided testid', () => {
    const wrapper = mountShell({
      props: { posterTestid: 'login-poster-collage' },
    })
    const poster = wrapper.find('[data-testid="login-poster-collage"]')
    expect(poster.exists()).toBe(true)
    expect(poster.classes()).toContain('hidden')
    expect(poster.classes()).toContain('lg:flex')
    expect(poster.classes()).toContain('relative')
    expect(poster.classes()).toContain('overflow-hidden')
  })

  it('omits the poster pane when posterTestid is not provided', () => {
    const wrapper = mountShell()
    expect(wrapper.find('[data-testid="auth-poster-stack"]').exists()).toBe(false)
  })

  it('applies the outer wrapper layout classes', () => {
    const wrapper = mountShell()
    const root = wrapper.element as HTMLElement
    expect(root.classList.contains('grid')).toBe(true)
    expect(root.classList.contains('lg:grid-cols-2')).toBe(true)
    expect(root.classList.contains('bg-bg-0')).toBe(true)
    expect(root.classList.contains('min-h-[calc(100vh-6rem)]')).toBe(true)
  })

  it('applies the card test id when cardTestid is provided', () => {
    const wrapper = mountShell({
      props: { cardTestid: 'verify-pending' },
      slots: { default: '<p>x</p>' },
    })
    expect(wrapper.find('[data-testid="verify-pending"]').exists()).toBe(true)
  })

  it('renders eyebrow, heading and subtitle when provided', () => {
    const wrapper = mountShell({
      props: {
        eyebrow: 'Welcome back',
        heading: 'Sign in',
        subtitle: 'Pick up where you left off.',
      },
    })
    expect(wrapper.find('[data-testid="auth-eyebrow"]').text()).toBe('Welcome back')
    expect(wrapper.find('[data-testid="auth-heading"]').text()).toBe('Sign in')
    expect(wrapper.find('[data-testid="auth-subtitle"]').text()).toBe(
      'Pick up where you left off.',
    )
  })
})
