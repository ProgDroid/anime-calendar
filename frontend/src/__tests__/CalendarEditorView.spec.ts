import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import { createPinia } from 'pinia'
import CalendarEditorView from '@/components/calendar/CalendarEditorView.vue'
import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const router = createRouter({
  history: createMemoryHistory(),
  routes: [{ path: '/', component: CalendarEditorView }],
})
const pinia = createPinia()

describe('CalendarEditorView', () => {
  it('renders correctly', () => {
    const wrapper = mount(CalendarEditorView, {
      global: { plugins: [i18n, router, pinia] },
    })
    expect(wrapper.exists()).toBe(true)
  })
})
