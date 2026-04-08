import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory } from 'vue-router'
import { createPinia } from 'pinia'
import CalendarPage from '@/components/CalendarPage.vue'
import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const router = createRouter({ history: createWebHistory(), routes: [{ path: '/', component: CalendarPage }] })
const pinia = createPinia()

describe('CalendarPage', () => {
  it('renders correctly', () => {
    const wrapper = mount(CalendarPage, {
      global: { plugins: [i18n, router, pinia] }
    })
    expect(wrapper.exists()).toBe(true)
  })
})
