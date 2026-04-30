import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory } from 'vue-router'
import UserDetailsPage from '@/components/UserDetailsPage.vue'
import en from '@/locales/en.json'

vi.mock('@/config/api', () => ({
  default: { get: vi.fn(), put: vi.fn(), post: vi.fn(), delete: vi.fn() }
}))

vi.mock('@/services/applySettings', () => ({
  applySettings: vi.fn()
}))

import api from '@/config/api'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const router = createRouter({ history: createWebHistory(), routes: [{ path: '/:p*', component: UserDetailsPage }] })

const mockUser = { username: 'testuser', email: 'test@example.com', is_oauth: false }

// Shared pinia so auth state survives from beforeEach into mountPage
let pinia: ReturnType<typeof createPinia>

function mountPage() {
  // Seed auth state so onMounted's isAuthenticated() check returns true
  pinia.state.value['auth'] = { user: 'mock-user', name: '', user_avatar: '' }
  return mount(UserDetailsPage, {
    global: { plugins: [i18n, router, pinia] }
  })
}

describe('UserDetailsPage', () => {
  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    pinia.state.value['auth'] = { user: 'mock-user', name: '', user_avatar: '' }
    vi.clearAllMocks()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn().mockReturnValue(null),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    })
    vi.mocked(api.get).mockResolvedValue({ data: mockUser })
  })

  it('fetches and displays user details on mount', async () => {
    const wrapper = mountPage()
    await flushPromises()
    expect(api.get).toHaveBeenCalledWith('/user/details')
    // Username and email are rendered inside disabled <input> elements — check .value, not .text()
    const inputs = wrapper.findAll('input')
    const values = inputs.map(i => i.element.value)
    expect(values).toContain('testuser')
    expect(values).toContain('test@example.com')
  })

  it('shows error when details fetch fails', async () => {
    vi.mocked(api.get).mockRejectedValue(new Error('server error'))
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.find('.alert-error').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.userDetails.fetchFailed)
  })

  it('shows edit form when edit button is clicked', async () => {
    const wrapper = mountPage()
    await flushPromises()
    // Edit button is .btn-primary.w-full (the "Edit details" button)
    const editBtn = wrapper.find('.btn-primary.w-full')
    await editBtn.trigger('click')
    expect(wrapper.find('form').exists()).toBe(true)
  })

  it('submits PUT /user on update', async () => {
    vi.mocked(api.put).mockResolvedValue({ data: { ...mockUser, username: 'updated' } })
    const wrapper = mountPage()
    await flushPromises()
    await wrapper.find('.btn-primary.w-full').trigger('click')
    await wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(api.put).toHaveBeenCalledWith('/user', {
      username: 'testuser',
      email: 'test@example.com'
    })
  })

  it('shows error when passwords do not match', async () => {
    const wrapper = mountPage()
    await flushPromises()
    // Open the password change section via the "Change Password" toggle button.
    // Use button.btn-outline to avoid matching the RouterLink which also has .btn-outline.
    const outlineBtn = wrapper.find('button.btn-outline')
    await outlineBtn.trigger('click')
    const passwordInputs = wrapper.findAll('input[type="password"]')
    await passwordInputs[1]!.setValue('NewPass123!@')
    await passwordInputs[2]!.setValue('DifferentPass123!')
    // Submit the password form (last form)
    const forms = wrapper.findAll('form')
    await forms[forms.length - 1]!.trigger('submit')
    await wrapper.vm.$nextTick()
    expect(wrapper.find('.alert-error').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.userDetails.passwordsDontMatch)
  })

  it('shows error when new password is too short', async () => {
    const wrapper = mountPage()
    await flushPromises()
    await wrapper.find('button.btn-outline').trigger('click')
    const passwordInputs = wrapper.findAll('input[type="password"]')
    await passwordInputs[1]!.setValue('Short1!')
    await passwordInputs[2]!.setValue('Short1!')
    const forms = wrapper.findAll('form')
    await forms[forms.length - 1]!.trigger('submit')
    await wrapper.vm.$nextTick()
    expect(wrapper.find('.alert-error').exists()).toBe(true)
  })

  it('opens confirm modal when delete account is clicked', async () => {
    const wrapper = mountPage()
    await flushPromises()
    await wrapper.find('.btn-error').trigger('click')
    expect(wrapper.find('[data-testid="modal-box"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.userDetails.accountDeleteWarning)
  })

  it('calls DELETE /user and redirects on account delete confirm', async () => {
    vi.mocked(api.delete).mockResolvedValue({})
    const pushSpy = vi.spyOn(router, 'push')
    const wrapper = mountPage()
    await flushPromises()

    await wrapper.find('.btn-error').trigger('click')
    await wrapper.vm.$nextTick()
    await wrapper.find('[data-testid="confirm-btn"]').trigger('click')
    await flushPromises()

    expect(api.delete).toHaveBeenCalledWith('/user')
    expect(pushSpy).toHaveBeenCalledWith('/login')
  })
})
