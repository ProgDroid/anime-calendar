import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { AxiosError } from 'axios'
import { MockEventSource } from '../../__tests__/setup'
import { usePresence } from '../usePresence'
import api from '@/config/api'

vi.mock('@/config/api', () => ({
  default: { get: vi.fn().mockResolvedValue({ data: {} }), post: vi.fn().mockResolvedValue({}) },
}))
const mockedApi = vi.mocked(api)

// Reset mock state between tests
beforeEach(() => {
  MockEventSource.instances = []
  mockedApi.get.mockReset().mockResolvedValue({ data: {} })
  mockedApi.post.mockReset().mockResolvedValue({})
})

afterEach(() => {
  vi.useRealTimers()
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

  it('sends heartbeats through the api instance (401-refresh interceptor)', async () => {
    vi.useFakeTimers()
    mount(TestHost)
    await vi.advanceTimersByTimeAsync(30_000)
    expect(mockedApi.post).toHaveBeenCalledWith('/calendars/42/presence/heartbeat')
  })

  it('reconnects after a fatal connection failure once the auth ping succeeds', async () => {
    vi.useFakeTimers()
    mount(TestHost)
    expect(MockEventSource.instances).toHaveLength(1)

    MockEventSource.lastInstance.failFatally()
    // First backoff step is 1s; the auth ping resolves, then a fresh
    // EventSource must be opened.
    await vi.advanceTimersByTimeAsync(1_000)
    await flushPromises()

    expect(mockedApi.get).toHaveBeenCalledWith('/user')
    expect(MockEventSource.instances).toHaveLength(2)
  })

  it('stops reconnecting when the auth ping returns 401 (session is dead)', async () => {
    vi.useFakeTimers()
    const unauth = new AxiosError('Unauthorized')
    Object.assign(unauth, { response: { status: 401 } })
    mockedApi.get.mockRejectedValue(unauth)

    mount(TestHost)
    MockEventSource.lastInstance.failFatally()
    await vi.advanceTimersByTimeAsync(120_000)
    await flushPromises()

    // One ping, no further retries, no new EventSource.
    expect(mockedApi.get).toHaveBeenCalledTimes(1)
    expect(MockEventSource.instances).toHaveLength(1)
  })

  it('keeps backing off on transient ping failures', async () => {
    vi.useFakeTimers()
    mockedApi.get.mockRejectedValue(new Error('network down'))

    mount(TestHost)
    MockEventSource.lastInstance.failFatally()
    // 1s + 2s + 4s of backoff → three pings, still no new EventSource.
    await vi.advanceTimersByTimeAsync(7_500)
    await flushPromises()

    expect(mockedApi.get.mock.calls.length).toBeGreaterThanOrEqual(3)
    expect(MockEventSource.instances).toHaveLength(1)
  })

  it('does not reconnect after unmount', async () => {
    vi.useFakeTimers()
    const wrapper = mount(TestHost)
    MockEventSource.lastInstance.failFatally()
    wrapper.unmount()

    await vi.advanceTimersByTimeAsync(120_000)
    await flushPromises()

    expect(MockEventSource.instances).toHaveLength(1)
  })
})
