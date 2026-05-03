import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import en from '@/locales/en.json'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'

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

const scheduleData = {
  '2026-04-27': [
    { id: 11, title: 'Anime A', episode: 3, time: '14:00', coverUrl: 'a.png' },
  ],
  '2026-04-30': [
    { id: 21, title: 'Anime B', episode: 7, time: '09:30' },
    { id: 22, title: 'Anime C', episode: 1, time: '21:00' },
  ],
}

describe('CalendarScheduleView — desktop', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // ISO week 2026-W18 starts Mon 2026-04-27 UTC
    vi.mocked(fetchScheduleForCalendar).mockResolvedValue(scheduleData)
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

describe('CalendarScheduleView — mobile', () => {
  beforeEach(async () => {
    vi.clearAllMocks()
    vi.mocked(fetchScheduleForCalendar).mockResolvedValue(scheduleData)
    await mockViewport(390)
  })

  afterEach(() => {
    resetViewportMock()
  })

  it('renders day pills row at 390 px', async () => {
    const { wrapper } = await mountView()
    expect(wrapper.find('[data-testid="schedule-mobile"]').exists()).toBe(true)
    const pills = wrapper.findAll('[data-testid^="schedule-mobile-pill-"]')
    expect(pills).toHaveLength(7)
  })

  it('renders desktop layout above 1024 px', async () => {
    resetViewportMock()
    await mockViewport(1280)
    const { wrapper } = await mountView()
    // Mobile root should not exist
    expect(wrapper.find('[data-testid="schedule-mobile"]').exists()).toBe(false)
    // Desktop columns should exist
    const cols = wrapper.findAll('[data-testid="day-label"]')
    expect(cols).toHaveLength(7)
    resetViewportMock()
  })

  it('first pill has aria-current true by default', async () => {
    const { wrapper } = await mountView()
    const pill0 = wrapper.find('[data-testid="schedule-mobile-pill-0"]')
    expect(pill0.attributes('aria-current')).toBe('true')
  })

  it('active pill uses bg-accent-1 class', async () => {
    const { wrapper } = await mountView()
    const pill0 = wrapper.find('[data-testid="schedule-mobile-pill-0"]')
    expect(pill0.classes()).toContain('bg-accent-1')
  })

  it('clicking a pill activates that day and shows its episodes', async () => {
    // 2026-W18 Mon=2026-04-27 (index 0), Thu=2026-04-30 (index 3)
    const { wrapper } = await mountView()

    // Click Thursday pill (index 3)
    await wrapper.find('[data-testid="schedule-mobile-pill-3"]').trigger('click')
    await flushPromises()

    // Pill 3 now active
    const pill3 = wrapper.find('[data-testid="schedule-mobile-pill-3"]')
    expect(pill3.attributes('aria-current')).toBe('true')
    expect(pill3.classes()).toContain('bg-accent-1')

    // Pill 0 no longer active
    const pill0 = wrapper.find('[data-testid="schedule-mobile-pill-0"]')
    expect(pill0.attributes('aria-current')).toBeUndefined()
    expect(pill0.classes()).not.toContain('bg-accent-1')

    // Episode list shows Thursday's entries (Anime B and Anime C)
    const episodes = wrapper.findAll('[data-testid="schedule-mobile-episode"]')
    expect(episodes).toHaveLength(2)
    expect(episodes[0]!.text()).toContain('Anime B')
    expect(episodes[1]!.text()).toContain('Anime C')
  })

  it('shows empty state for a day with no episodes', async () => {
    // Monday has episodes, Tuesday (index 1) has none
    const { wrapper } = await mountView()
    await wrapper.find('[data-testid="schedule-mobile-pill-1"]').trigger('click')
    await flushPromises()

    expect(wrapper.find('[data-testid="schedule-mobile-empty"]').exists()).toBe(true)
    const episodes = wrapper.findAll('[data-testid="schedule-mobile-episode"]')
    expect(episodes).toHaveLength(0)
  })

  it('renders mobile header with i18n keys', async () => {
    const { wrapper } = await mountView()
    expect(wrapper.text()).toContain(en.mobile.schedule.title)
    expect(wrapper.text()).toContain(en.mobile.schedule.sub)
  })
})
