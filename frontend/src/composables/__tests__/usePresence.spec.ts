import { describe, it, expect, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { MockEventSource } from '../../__tests__/setup'
import { usePresence } from '../usePresence'

// Reset mock state between tests
beforeEach(() => {
  MockEventSource.instances = []
})

const TestHost = defineComponent({
  setup() {
    return usePresence(42)
  },
  template: '<div />',
})

describe('usePresence', () => {
  it('opens EventSource on mount with correct URL and withCredentials', () => {
    mount(TestHost)
    expect(MockEventSource.lastInstance.url).toBe('/api/calendars/42/events')
    expect(MockEventSource.lastInstance.withCredentials).toBe(true)
  })

  it('closes EventSource on unmount', () => {
    const wrapper = mount(TestHost)
    wrapper.unmount()
    expect(MockEventSource.lastInstance.closed).toBe(true)
  })

  it('updates metaVersion on meta_snapshot frame', async () => {
    const wrapper = mount(TestHost)
    MockEventSource.lastInstance.emit('message', JSON.stringify({ type: 'meta_snapshot', v: 7 }))
    await flushPromises()
    expect(wrapper.vm.metaVersion).toBe(7)
  })

  it('updates viewers on presence frame', async () => {
    const wrapper = mount(TestHost)
    MockEventSource.lastInstance.emit(
      'message',
      JSON.stringify({ type: 'presence', viewers: [{ user_id: 1, display: 'Alice' }] }),
    )
    await flushPromises()
    expect(wrapper.vm.viewers).toEqual([{ user_id: 1, display: 'Alice' }])
  })

  it('sets lastEvent on any frame', async () => {
    const wrapper = mount(TestHost)
    MockEventSource.lastInstance.emit(
      'message',
      JSON.stringify({ type: 'kick', reason: 'owner_downgrade' }),
    )
    await flushPromises()
    expect(wrapper.vm.lastEvent).toMatchObject({ type: 'kick', reason: 'owner_downgrade' })
  })

  it('updates metaVersion on item_added frame', async () => {
    const wrapper = mount(TestHost)
    MockEventSource.lastInstance.emit(
      'message',
      JSON.stringify({ type: 'item_added', media_id: 99, actor: '1', v: 3, at: '2026-01-01' }),
    )
    await flushPromises()
    expect(wrapper.vm.metaVersion).toBe(3)
  })
})
