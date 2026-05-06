import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'

// Mock sharingService so store actions don't hit the network
vi.mock('@/services/sharingService', () => ({
  sharingService: {
    listMembers: vi.fn().mockResolvedValue({ editors: [], pending: [], editor_cap: 5 }),
    invite: vi.fn().mockResolvedValue({ id: 1, invitee_email: 'a@b.com', sent_at: '2026-05-06T00:00:00', expires_at: '2026-05-13T00:00:00' }),
    revoke: vi.fn().mockResolvedValue(undefined),
    removeEditor: vi.fn().mockResolvedValue(undefined),
    resend: vi.fn().mockResolvedValue({ id: 1, invitee_email: 'a@b.com', sent_at: '2026-05-06T00:00:00', expires_at: '2026-05-13T00:00:00' }),
  },
}))

import MembersTab from '@/components/calendar/MembersTab.vue'
import { useSharingStore } from '@/stores/sharingStore'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, pt } })

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/', component: { template: '<div/>' } }],
  })
}

describe('MembersTab', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    document.body.textContent = ''
  })

  afterEach(() => {
    document.body.textContent = ''
  })

  it('free owner sees upgrade prompt, not members UI', async () => {
    const router = makeRouter()
    const wrapper = mount(MembersTab, {
      props: { calendarId: 1, isOwner: true, isPaid: false },
      global: { plugins: [i18n, router, createPinia()] },
    })
    await flushPromises()
    expect(wrapper.find('[data-testid="members-upgrade-prompt"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="members-tab"]').exists()).toBe(false)
  })

  it('paid owner sees members UI', async () => {
    const router = makeRouter()
    const wrapper = mount(MembersTab, {
      props: { calendarId: 1, isOwner: true, isPaid: true },
      global: { plugins: [i18n, router, createPinia()] },
    })
    await flushPromises()
    expect(wrapper.find('[data-testid="members-tab"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="invite-btn"]').exists()).toBe(true)
  })

  it('invite button opens modal', async () => {
    const router = makeRouter()
    const wrapper = mount(MembersTab, {
      props: { calendarId: 1, isOwner: true, isPaid: true },
      global: { plugins: [i18n, router, createPinia()] },
      attachTo: document.body,
    })
    await flushPromises()
    await wrapper.find('[data-testid="invite-btn"]').trigger('click')
    await flushPromises()
    // InviteEditorModal teleports to body when open=true
    const modal = document.body.querySelector('[data-testid="invite-email-input"]')
    expect(modal).not.toBeNull()
    wrapper.unmount()
  })

  it('remove editor calls store', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const sharingStore = useSharingStore()
    // Mock loadMembers so it doesn't overwrite the state we set below
    vi.spyOn(sharingStore, 'loadMembers').mockResolvedValue()
    sharingStore.editors = [
      { user_id: 42, display: 'Alice', email: 'alice@example.com', joined_at: '2026-01-01T00:00:00', suspended_at: null },
    ]
    sharingStore.pendingInvites = []
    sharingStore.editorCap = 5
    const removeEditorSpy = vi.spyOn(sharingStore, 'removeEditor').mockResolvedValue()

    const router = makeRouter()
    const wrapper = mount(MembersTab, {
      props: { calendarId: 1, isOwner: true, isPaid: true },
      global: { plugins: [i18n, router, pinia] },
    })
    await flushPromises()

    const removeBtn = wrapper.find('[data-testid="remove-editor-btn"]')
    expect(removeBtn.exists()).toBe(true)
    await removeBtn.trigger('click')
    await flushPromises()
    expect(removeEditorSpy).toHaveBeenCalledWith(1, 42)
  })

  it('revoke invite calls store', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const sharingStore = useSharingStore()
    vi.spyOn(sharingStore, 'loadMembers').mockResolvedValue()
    sharingStore.editors = []
    sharingStore.pendingInvites = [
      { id: 99, invitee_email: 'bob@example.com', sent_at: '2026-05-01T00:00:00', expires_at: '2026-05-08T00:00:00' },
    ]
    sharingStore.editorCap = 5
    const revokeSpy = vi.spyOn(sharingStore, 'revoke').mockResolvedValue()

    const router = makeRouter()
    const wrapper = mount(MembersTab, {
      props: { calendarId: 1, isOwner: true, isPaid: true },
      global: { plugins: [i18n, router, pinia] },
    })
    await flushPromises()

    const revokeBtn = wrapper.find('[data-testid="revoke-btn"]')
    expect(revokeBtn.exists()).toBe(true)
    await revokeBtn.trigger('click')
    await flushPromises()
    expect(revokeSpy).toHaveBeenCalledWith(1, 99)
  })
})
