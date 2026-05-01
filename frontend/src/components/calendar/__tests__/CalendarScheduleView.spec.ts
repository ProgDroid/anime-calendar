import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import en from '@/locales/en.json'

vi.mock('@/services/calendars', () => ({
  fetchScheduleForCalendar: vi.fn(),
}))

import { fetchScheduleForCalendar } from '@/services/calendars'
import CalendarScheduleView from '@/components/calendar/CalendarScheduleView.vue'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/calendars/:id/schedule', name: 'schedule', component: CalendarScheduleView },
    ],
  })
}

async function mountView(week = '2026-W18') {
  const router = makeRouter()
  await router.push(`/calendars/42/schedule?week=${week}`)
  await router.isReady()
  const wrapper = mount(CalendarScheduleView, {
    global: { plugins: [i18n, router] },
  })
  await flushPromises()
  return { wrapper, router }
}

describe('CalendarScheduleView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // ISO week 2026-W18 starts Mon 2026-04-27 UTC
    vi.mocked(fetchScheduleForCalendar).mockResolvedValue({
      '2026-04-27': [
        { id: 11, title: 'Anime A', episode: 3, time: '14:00', coverUrl: 'a.png' },
      ],
      '2026-04-30': [
        { id: 21, title: 'Anime B', episode: 7, time: '09:30' },
        { id: 22, title: 'Anime C', episode: 1, time: '21:00' },
      ],
    })
  })

  it('renders 7 day columns', async () => {
    const { wrapper } = await mountView()
    const cols = wrapper.findAll('[data-testid="day-label"]')
    expect(cols).toHaveLength(7)
  })

  it('shows entries on populated days and empty placeholder on others', async () => {
    const { wrapper } = await mountView()
    const entries = wrapper.findAll('[data-testid="schedule-entry"]')
    expect(entries).toHaveLength(3)
    const empties = wrapper.findAll('[data-testid="day-empty"]')
    // 7 days, 2 populated => 5 empty
    expect(empties).toHaveLength(5)
  })

  it('displays the active ISO week', async () => {
    const { wrapper } = await mountView('2026-W18')
    expect(wrapper.find('[data-testid="schedule-week"]').text()).toBe('2026-W18')
  })

  it('shifts week forward when next is clicked', async () => {
    const { wrapper, router } = await mountView('2026-W18')
    await wrapper.find('[data-testid="schedule-next"]').trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.query.week).toBe('2026-W19')
  })

  it('shifts week backward when prev is clicked', async () => {
    const { wrapper, router } = await mountView('2026-W18')
    await wrapper.find('[data-testid="schedule-prev"]').trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.query.week).toBe('2026-W17')
  })

  it('calls fetcher with calendar id and current week', async () => {
    await mountView('2026-W18')
    expect(fetchScheduleForCalendar).toHaveBeenCalledWith(42, '2026-W18')
  })
})
