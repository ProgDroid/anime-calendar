import { describe, it, expect, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import { defineComponent, h } from 'vue'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'

import en from '@/locales/en.json'
import UpgradeInterruptModal from '@/components/UpgradeInterruptModal.vue'
import UiBottomSheet from '@/components/ui/UiBottomSheet.vue'
import UiModal from '@/components/ui/UiModal.vue'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

// Note: do NOT call router.isReady() after mockViewport — vi.doMock inside
// mockViewport blocks the microtask queue, causing isReady() to hang.
// createMemoryHistory + no push means the router is synchronously ready.
function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: defineComponent({ render: () => h('div') }) },
      { path: '/upgrade', component: defineComponent({ render: () => h('div') }) },
    ],
  })
}

async function mountModal(viewportWidth: number) {
  await mockViewport(viewportWidth)
  return mount(UpgradeInterruptModal, {
    props: { open: true },
    global: { plugins: [i18n, makeRouter()] },
    attachTo: document.body,
  })
}

describe('UpgradeInterruptModal — mobile (bottom sheet)', () => {
  afterEach(() => {
    resetViewportMock()
  })

  it('renders UiBottomSheet on mobile and not UiModal', async () => {
    const wrapper = await mountModal(390)

    expect(wrapper.findComponent(UiBottomSheet).exists()).toBe(true)
    expect(wrapper.findComponent(UiModal).exists()).toBe(false)
    // Body content is teleported — query document.body directly
    expect(document.body.querySelector('[data-testid="upgrade-interrupt-later"]')).not.toBeNull()

    wrapper.unmount()
  })

  it('emits update:open false and close when maybe-later is clicked on mobile', async () => {
    const wrapper = await mountModal(390)

    const laterBtn = document.body.querySelector<HTMLElement>(
      '[data-testid="upgrade-interrupt-later"]',
    )
    expect(laterBtn).not.toBeNull()
    laterBtn!.click()
    await wrapper.vm.$nextTick()

    expect(wrapper.emitted('update:open')?.[0]).toEqual([false])
    expect(wrapper.emitted('close')).toBeDefined()

    wrapper.unmount()
  })
})

describe('UpgradeInterruptModal — desktop (modal)', () => {
  afterEach(() => {
    resetViewportMock()
  })

  it('renders UiModal on desktop and not UiBottomSheet', async () => {
    const wrapper = await mountModal(1280)

    expect(wrapper.findComponent(UiModal).exists()).toBe(true)
    expect(wrapper.findComponent(UiBottomSheet).exists()).toBe(false)

    wrapper.unmount()
  })

  it('emits update:open false and close when maybe-later is clicked on desktop', async () => {
    const wrapper = await mountModal(1280)

    const laterBtn = document.body.querySelector<HTMLElement>(
      '[data-testid="upgrade-interrupt-later"]',
    )
    expect(laterBtn).not.toBeNull()
    laterBtn!.click()
    await wrapper.vm.$nextTick()

    expect(wrapper.emitted('update:open')?.[0]).toEqual([false])
    expect(wrapper.emitted('close')).toBeDefined()

    wrapper.unmount()
  })
})
