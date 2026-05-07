import { describe, it, expect, vi, beforeEach } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import { defineComponent, h } from 'vue'
import axios from 'axios'

import en from '@/locales/en.json'
import { sharingService } from '@/services/sharingService'
import type { InvitationPreview } from '@/types/sharing'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

const Stub = defineComponent({ render: () => h('div') })

const SAMPLE_PREVIEW: InvitationPreview = {
  calendar_name: 'Spring 2026',
  owner_display: 'alice',
  owner_avatar: null,
  item_count: 12,
  masked_email: 'b***@e***.com',
}

function makeRouter(token: string = 'abc123') {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/invite/:token', component: Stub },
      { path: '/my-calendars', component: Stub },
      { path: '/calendar/:id', component: Stub },
      { path: '/login', component: Stub },
      { path: '/register', component: Stub },
    ],
  })
  void router.push(`/invite/${token}`)
  return router
}

describe('InviteLandingPage', () => {
  let pinia: ReturnType<typeof createPinia>

  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    vi.restoreAllMocks()
  })

  async function mountPage(token = 'abc123') {
    const router = makeRouter(token)
    await router.isReady()
    const { default: InviteLandingPage } = await import('../InviteLandingPage.vue')
    const wrapper = mount(InviteLandingPage, {
      global: {
        plugins: [i18n, pinia, router],
      },
      attachTo: document.body,
    })
    return { wrapper, router }
  }

  it('shows loading then invalid state when preview rejects', async () => {
    vi.spyOn(sharingService, 'preview').mockRejectedValue(new Error('not found'))

    const { wrapper } = await mountPage()

    // Initially in loading state
    expect(wrapper.find('[data-testid="invite-loading"]').exists()).toBe(true)

    await flushPromises()

    expect(wrapper.find('[data-testid="invite-invalid"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="invite-loading"]').exists()).toBe(false)
    expect(wrapper.text()).toContain(en.sharing.landing.invalid)
  })

  it('shows unauth state when preview succeeds and user is not authenticated', async () => {
    vi.spyOn(sharingService, 'preview').mockResolvedValue(SAMPLE_PREVIEW)

    const { wrapper } = await mountPage()
    await flushPromises()

    // auth store user is '' by default (not authenticated)
    expect(wrapper.find('[data-testid="invite-unauth"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="invite-loading"]').exists()).toBe(false)
    expect(wrapper.text()).toContain(en.sharing.landing.unauth_heading)
    expect(wrapper.text()).toContain(en.sharing.landing.signIn)
    expect(wrapper.text()).toContain(en.sharing.landing.signUp)
  })

  it('shows ready state when preview succeeds and user is authenticated', async () => {
    vi.spyOn(sharingService, 'preview').mockResolvedValue(SAMPLE_PREVIEW)

    // Set auth state before mounting so onMounted sees isAuthenticated() = true
    const { useAuthStore } = await import('@/stores/auth')
    const authStore = useAuthStore()
    authStore.user = 'alice'

    const { wrapper } = await mountPage()
    await flushPromises()

    expect(wrapper.find('[data-testid="invite-ready"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="invite-loading"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="invite-accept"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="invite-decline"]').exists()).toBe(true)
  })

  it('shows mismatch state when accept returns 403', async () => {
    vi.spyOn(sharingService, 'preview').mockResolvedValue(SAMPLE_PREVIEW)
    const error = Object.assign(new Error('Forbidden'), {
      isAxiosError: true,
      response: { status: 403 },
    })
    vi.spyOn(axios, 'isAxiosError').mockImplementation(
      (e) => (e as { isAxiosError?: boolean }).isAxiosError === true,
    )
    vi.spyOn(sharingService, 'accept').mockRejectedValue(error)

    // Set auth state before mounting
    const { useAuthStore } = await import('@/stores/auth')
    const authStore = useAuthStore()
    authStore.user = 'bob'

    const { wrapper } = await mountPage()
    await flushPromises()

    expect(wrapper.find('[data-testid="invite-ready"]').exists()).toBe(true)

    await wrapper.find('[data-testid="invite-accept"]').trigger('click')
    await flushPromises()

    expect(wrapper.find('[data-testid="invite-mismatch"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(SAMPLE_PREVIEW.masked_email)
  })

  it('redirects to /calendars/:id when accept succeeds', async () => {
    vi.spyOn(sharingService, 'preview').mockResolvedValue(SAMPLE_PREVIEW)
    vi.spyOn(sharingService, 'accept').mockResolvedValue({ calendar_id: 42 })

    // Set auth state before mounting
    const { useAuthStore } = await import('@/stores/auth')
    const authStore = useAuthStore()
    authStore.user = 'alice'

    const { wrapper, router } = await mountPage()
    await flushPromises()

    expect(wrapper.find('[data-testid="invite-ready"]').exists()).toBe(true)

    const pushSpy = vi.spyOn(router, 'push')
    await wrapper.find('[data-testid="invite-accept"]').trigger('click')
    await flushPromises()

    expect(pushSpy).toHaveBeenCalledWith('/calendar/42')
  })
})
