import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'
import { makeSmokeRouter } from '@/__tests__/test-utils/router'

import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

describe('NotFoundPage — mobile smoke test', () => {
  beforeEach(async () => {
    await mockViewport(390)
  })

  afterEach(() => {
    resetViewportMock()
  })

  it('renders at 390 px without throwing and shows the page-not-found heading', async () => {
    const { default: NotFoundPage } = await import('../NotFoundPage.vue')
    const wrapper = mount(NotFoundPage, {
      global: { plugins: [i18n, makeSmokeRouter('/')] },
    })
    expect(wrapper.find('[data-testid="not-found"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.errors.notFound.title)
  })
})
