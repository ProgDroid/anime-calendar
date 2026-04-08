import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import CalendarSettingsForm from '@/components/calendar/CalendarSettingsForm.vue'
import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const mountOpts = { global: { plugins: [i18n] } }

describe('CalendarSettingsForm', () => {
  it('renders the name input with provided value', () => {
    const wrapper = mount(CalendarSettingsForm, {
      props: { name: 'My Calendar', language: 'english', loading: false, canSubmit: true },
      ...mountOpts
    })
    expect((wrapper.find('input[type="text"]').element as HTMLInputElement).value).toBe('My Calendar')
  })

  it('emits update:name when name changes', async () => {
    const wrapper = mount(CalendarSettingsForm, {
      props: { name: '', language: 'english', loading: false, canSubmit: true },
      ...mountOpts
    })
    await wrapper.find('input[type="text"]').setValue('New Name')
    expect(wrapper.emitted('update:name')?.[0]).toEqual(['New Name'])
  })

  it('emits update:language when language changes', async () => {
    const wrapper = mount(CalendarSettingsForm, {
      props: { name: '', language: 'english', loading: false, canSubmit: true },
      ...mountOpts
    })
    const radios = wrapper.findAll('input[type="radio"]')
    await radios[1].trigger('change')
    expect(wrapper.emitted('update:language')).toBeTruthy()
  })

  it('emits submit when submit button is clicked', async () => {
    const wrapper = mount(CalendarSettingsForm, {
      props: { name: 'Test', language: 'english', loading: false, canSubmit: true },
      ...mountOpts
    })
    await wrapper.find('[data-testid="submit-btn"]').trigger('click')
    expect(wrapper.emitted('submit')).toBeTruthy()
  })

  it('disables submit when canSubmit is false', () => {
    const wrapper = mount(CalendarSettingsForm, {
      props: { name: '', language: 'english', loading: false, canSubmit: false },
      ...mountOpts
    })
    expect(wrapper.find('[data-testid="submit-btn"]').attributes('disabled')).toBeDefined()
  })
})
