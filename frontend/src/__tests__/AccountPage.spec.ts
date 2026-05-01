import { describe, it, expect, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import { createPinia } from 'pinia'
import AccountPage from '@/components/AccountPage.vue'
import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      {
        path: '/account',
        component: AccountPage,
        children: [
          { path: '', redirect: { name: 'account.profile' } },
          { path: 'profile', name: 'account.profile', component: { template: '<div data-testid="profile-stub" />' } },
          { path: 'preferences', name: 'account.preferences', component: { template: '<div data-testid="preferences-stub" />' } },
          { path: 'password', name: 'account.password', component: { template: '<div data-testid="password-stub" />' } },
          { path: 'danger', name: 'account.danger', component: { template: '<div data-testid="danger-stub" />' } },
        ],
      },
    ],
  })
}

describe('AccountPage (shell)', () => {
  it('renders all 4 sidebar tabs', async () => {
    const router = makeRouter()
    router.push('/account/profile')
    await router.isReady()
    const wrapper = mount(AccountPage, { global: { plugins: [i18n, router, createPinia()] } })
    await flushPromises()
    expect(wrapper.find('[data-testid="account-sidebar"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="account-tab-profile"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="account-tab-preferences"]').exists()).toBe(true)
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
