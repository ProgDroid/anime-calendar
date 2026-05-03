import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'

function makeRouter(): Router {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/login', component: { template: '<div/>' }, meta: { bottomTabBar: false } },
      { path: '/my-calendars', component: { template: '<div/>' } },
      { path: '/calendar/:id', component: { template: '<div/>' } },
      { path: '/account', component: { template: '<div/>' } },
      { path: '/account/profile', component: { template: '<div/>' } },
      { path: '/upgrade', component: { template: '<div/>' } },
    ],
  })
}

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, pt } })

describe('UiBottomTabBar', () => {
  beforeEach(() => {
    vi.resetModules()
  })
  afterEach(() => {
    resetViewportMock()
  })

  async function mountAt(path: string, viewportPx: number) {
    await mockViewport(viewportPx)
    const { default: UiBottomTabBar } = await import('../UiBottomTabBar.vue')
    const router = makeRouter()
    await router.push(path)
    await router.isReady()
    return mount(UiBottomTabBar, { global: { plugins: [i18n, router] } })
  }

  it('hides on desktop viewports', async () => {
    const w = await mountAt('/my-calendars', 1280)
    expect(w.find('[data-testid="bottom-tab-bar"]').exists()).toBe(false)
  })

  it('hides on routes with meta.bottomTabBar === false', async () => {
    const w = await mountAt('/login', 390)
    expect(w.find('[data-testid="bottom-tab-bar"]').exists()).toBe(false)
  })

  it('renders 2 tabs on authenticated mobile routes', async () => {
    const w = await mountAt('/my-calendars', 390)
    expect(w.find('[data-testid="bottom-tab-bar"]').exists()).toBe(true)
    expect(w.findAll('[data-testid="bottom-tab"]')).toHaveLength(2)
  })

  it('highlights Library on /my-calendars', async () => {
    const w = await mountAt('/my-calendars', 390)
    const lib = w.get('[data-testid="bottom-tab-library"]')
    expect(lib.classes().some((c) => c.includes('text-accent-1'))).toBe(true)
  })

  it('highlights Library on /calendar/:id (editor is child of Library)', async () => {
    const w = await mountAt('/calendar/42', 390)
    const lib = w.get('[data-testid="bottom-tab-library"]')
    expect(lib.classes().some((c) => c.includes('text-accent-1'))).toBe(true)
  })

  it('highlights Account on /account/profile', async () => {
    const w = await mountAt('/account/profile', 390)
    const acc = w.get('[data-testid="bottom-tab-account"]')
    expect(acc.classes().some((c) => c.includes('text-accent-1'))).toBe(true)
  })
})
