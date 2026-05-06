import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import axios from 'axios'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'

// Mock sharingService
vi.mock('@/services/sharingService', () => ({
  sharingService: {
    listMembers: vi.fn().mockResolvedValue({ editors: [], pending: [], editor_cap: 5 }),
    invite: vi.fn().mockResolvedValue({ id: 1, invitee_email: 'a@b.com', sent_at: '2026-05-06T00:00:00', expires_at: '2026-05-13T00:00:00' }),
    revoke: vi.fn().mockResolvedValue(undefined),
    removeEditor: vi.fn().mockResolvedValue(undefined),
    resend: vi.fn().mockResolvedValue({ id: 1, invitee_email: 'a@b.com', sent_at: '2026-05-06T00:00:00', expires_at: '2026-05-13T00:00:00' }),
  },
}))

import InviteEditorModal from '@/components/calendar/InviteEditorModal.vue'
import { useSharingStore } from '@/stores/sharingStore'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, pt } })

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/', component: { template: '<div/>' } }],
  })
}

function fillEmail(value: string) {
  const inputEl = document.body.querySelector('[data-testid="invite-email-input"] input') as HTMLInputElement | null
  if (inputEl) {
    inputEl.value = value
    inputEl.dispatchEvent(new Event('input'))
  }
}

function clickSubmit() {
  const btn = document.body.querySelector('[data-testid="invite-submit-btn"]') as HTMLButtonElement | null
  btn?.click()
}

describe('InviteEditorModal', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    // clear any leftover teleport targets
    document.body.textContent = ''
  })

  afterEach(() => {
    document.body.textContent = ''
  })

  it('renders email input and submit when open', async () => {
    const router = makeRouter()
    mount(InviteEditorModal, {
      props: { open: true, calendarId: 42 },
      global: { plugins: [i18n, router, createPinia()] },
      attachTo: document.body,
    })
    await flushPromises()
    const input = document.body.querySelector('[data-testid="invite-email-input"]')
    expect(input).not.toBeNull()
  })

  it('submit with invalid email shows error', async () => {
    const router = makeRouter()
    mount(InviteEditorModal, {
      props: { open: true, calendarId: 42 },
      global: { plugins: [i18n, router, createPinia()] },
      attachTo: document.body,
    })
    await flushPromises()

    fillEmail('notanemail')
    await flushPromises()
    clickSubmit()
    await flushPromises()

    const errorEl = document.body.querySelector('[role="alert"]')
    expect(errorEl?.textContent).toContain(en.sharing.errors.invalid_email)
  })

  it('submit with valid email calls sharingStore.invite', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const sharingStore = useSharingStore()
    const inviteSpy = vi.spyOn(sharingStore, 'invite').mockResolvedValue({
      id: 1, invitee_email: 'a@b.com', sent_at: '2026-05-06T00:00:00', expires_at: '2026-05-13T00:00:00',
    })

    const router = makeRouter()
    mount(InviteEditorModal, {
      props: { open: true, calendarId: 42 },
      global: { plugins: [i18n, router, pinia] },
      attachTo: document.body,
    })
    await flushPromises()

    fillEmail('a@b.com')
    await flushPromises()
    clickSubmit()
    await flushPromises()

    expect(inviteSpy).toHaveBeenCalledWith(42, 'a@b.com')
  })

  it('409 editor_cap_reached shows inline error', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const sharingStore = useSharingStore()

    const axiosError = new axios.AxiosError(
      'Conflict',
      '409',
      undefined,
      undefined,
      {
        status: 409,
        data: { error: 'editor_cap_reached' },
        statusText: 'Conflict',
        headers: {},
        config: {} as never,
      },
    )
    vi.spyOn(sharingStore, 'invite').mockRejectedValue(axiosError)

    const router = makeRouter()
    mount(InviteEditorModal, {
      props: { open: true, calendarId: 42 },
      global: { plugins: [i18n, router, pinia] },
      attachTo: document.body,
    })
    await flushPromises()

    fillEmail('a@b.com')
    await flushPromises()
    clickSubmit()
    await flushPromises()

    const errorEl = document.body.querySelector('[role="alert"]')
    expect(errorEl?.textContent).toContain(en.sharing.errors.editor_cap_reached)
  })

  it('success closes the modal', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const sharingStore = useSharingStore()
    vi.spyOn(sharingStore, 'invite').mockResolvedValue({
      id: 1, invitee_email: 'a@b.com', sent_at: '2026-05-06T00:00:00', expires_at: '2026-05-13T00:00:00',
    })

    const router = makeRouter()
    const wrapper = mount(InviteEditorModal, {
      props: { open: true, calendarId: 42 },
      global: { plugins: [i18n, router, pinia] },
      attachTo: document.body,
    })
    await flushPromises()

    fillEmail('a@b.com')
    await flushPromises()
    clickSubmit()
    await flushPromises()

    const emitted = wrapper.emitted('update:open')
    expect(emitted).toBeTruthy()
    expect(emitted![emitted!.length - 1]).toEqual([false])
  })
})
