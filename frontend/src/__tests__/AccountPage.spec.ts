import { describe, it, expect, vi, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory, type Router } from 'vue-router'
import { createPinia } from 'pinia'
import AccountPage from '@/components/AccountPage.vue'
import en from '@/locales/en.json'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

function makeRouter(component: unknown = AccountPage): Router {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      {
        path: '/account',
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        component: component as any,
        children: [
          { path: 'profile', name: 'account.profile', component: { template: '<div data-testid="profile-stub" />' } },
          { path: 'preferences', name: 'account.preferences', component: { template: '<div data-testid="preferences-stub" />' } },
          { path: 'subscription', name: 'account.subscription', component: { template: '<div data-testid="subscription-stub" />' } },
          { path: 'password', name: 'account.password', component: { template: '<div data-testid="password-stub" />' } },
          { path: 'danger', name: 'account.danger', component: { template: '<div data-testid="danger-stub" />' } },
        ],
      },
    ],
  })
}

describe('AccountPage (shell)', () => {
  it('renders sidebar avatar block with initials and placeholder fields when unauth', async () => {
    const router = makeRouter()
    router.push('/account/profile')
    await router.isReady()
    const wrapper = mount(AccountPage, { global: { plugins: [i18n, router, createPinia()] } })
    await flushPromises()
    expect(wrapper.find('[data-testid="account-sidebar-user"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="account-sidebar-name"]').text()).toBe('—')
    expect(wrapper.find('[data-testid="account-sidebar-email"]').text()).toBe('—')
  })

  it('renders all 5 sidebar tabs', async () => {
    const router = makeRouter()
    router.push('/account/profile')
    await router.isReady()
    const wrapper = mount(AccountPage, { global: { plugins: [i18n, router, createPinia()] } })
    await flushPromises()
    expect(wrapper.find('[data-testid="account-sidebar"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="account-tab-profile"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="account-tab-preferences"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="account-tab-subscription"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="account-tab-password"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="account-tab-danger"]').exists()).toBe(true)
  })

  it('renders profile outlet on /account/profile', async () => {
    const router = makeRouter()
    router.push('/account/profile')
    await router.isReady()
    const wrapper = mount(AccountPage, { global: { plugins: [i18n, router, createPinia()] } })
    await flushPromises()
    expect(wrapper.find('[data-testid="profile-stub"]').exists()).toBe(true)
  })

  it.each([
    ['account-tab-profile', 'account.profile'],
    ['account-tab-preferences', 'account.preferences'],
    ['account-tab-subscription', 'account.subscription'],
    ['account-tab-password', 'account.password'],
    ['account-tab-danger', 'account.danger'],
  ])('clicking %s pushes to %s', async (testid, name) => {
    const router = makeRouter()
    router.push('/account/profile')
    await router.isReady()
    const pushSpy = vi.spyOn(router, 'push')
    const wrapper = mount(AccountPage, { global: { plugins: [i18n, router, createPinia()] } })
    await flushPromises()
    await wrapper.find(`[data-testid="${testid}"]`).trigger('click')
    expect(pushSpy).toHaveBeenCalledWith({ name })
  })

  it.each([
    ['/account/profile', 'account-tab-profile'],
    ['/account/preferences', 'account-tab-preferences'],
    ['/account/subscription', 'account-tab-subscription'],
    ['/account/password', 'account-tab-password'],
    ['/account/danger', 'account-tab-danger'],
  ])('sidebar highlight tracks the active route at %s', async (path, activeTestid) => {
    const router = makeRouter()
    router.push(path)
    await router.isReady()
    const wrapper = mount(AccountPage, { global: { plugins: [i18n, router, createPinia()] } })
    await flushPromises()
    const activeBtn = wrapper.find(`[data-testid="${activeTestid}"]`)
    expect(activeBtn.classes().some((c) => c.includes('bg-bg-2'))).toBe(true)
  })
})

describe('AccountPage layout', () => {
  afterEach(() => {
    resetViewportMock()
  })

  it('renders the desktop sidebar layout above the breakpoint', async () => {
    await mockViewport(1280)
    vi.resetModules()
    const { default: Comp } = await import('@/components/AccountPage.vue')
    const router = makeRouter(Comp)
    router.push('/account/profile')
    await router.isReady()
    const wrapper = mount(Comp, { global: { plugins: [i18n, router, createPinia()] } })
    await flushPromises()
    expect(wrapper.find('[data-testid="account-desktop-layout"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="account-sidebar"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="account-section-list"]').exists()).toBe(false)
  })

  it('renders the section list at /account on mobile', async () => {
    await mockViewport(390)
    vi.resetModules()
    const { default: Comp } = await import('@/components/AccountPage.vue')
    const router = makeRouter(Comp)
    router.push('/account')
    await router.isReady()
    const wrapper = mount(Comp, { global: { plugins: [i18n, router, createPinia()] } })
    await flushPromises()
    expect(wrapper.find('[data-testid="account-section-list"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="account-desktop-layout"]').exists()).toBe(false)
  })

  it('renders the sub-route view + back arrow at /account/profile on mobile', async () => {
    await mockViewport(390)
    vi.resetModules()
    const { default: Comp } = await import('@/components/AccountPage.vue')
    const router = makeRouter(Comp)
    router.push('/account/profile')
    await router.isReady()
    const wrapper = mount(Comp, { global: { plugins: [i18n, router, createPinia()] } })
    await flushPromises()
    expect(wrapper.find('[data-testid="account-subroute"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="account-back-arrow"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="account-desktop-layout"]').exists()).toBe(false)
  })
})
