import api from '@/config/api'
import type { Calendar } from '@/types/calendar'
import type { Item, Title } from '@/types/item'
import type { ScheduleEntry, ScheduleByDay } from '@/types/schedule'
import { parseIsoWeek } from '@/composables/useWeekRange'

export type { ScheduleEntry, ScheduleByDay } from '@/types/schedule'

function pickTitle(title: Title, lang: 'english' | 'romaji' | 'native'): string {
  return title[lang] || title.english || title.romaji || title.native || ''
}

function pad2(n: number): string {
  return String(n).padStart(2, '0')
}

function isoDayUtc(d: Date): string {
  return `${d.getUTCFullYear()}-${pad2(d.getUTCMonth() + 1)}-${pad2(d.getUTCDate())}`
}

function timeUtc(d: Date): string {
  return `${pad2(d.getUTCHours())}:${pad2(d.getUTCMinutes())}`
}

/**
 * Derive a per-day schedule for the given ISO week from the calendar's items
 * and their airing_schedule. Backend has no /schedule endpoint in Track 2 scope,
 * so we group client-side. Times are emitted in UTC.
 */
export async function fetchScheduleForCalendar(
  calendarId: number,
  isoWeek: string,
): Promise<ScheduleByDay> {
  const response = await api.get(`/calendars/${calendarId}`)
  const calendar = response.data as Calendar

  const monday = parseIsoWeek(isoWeek)
  if (!monday) return {}
  const startMs = monday.getTime()
  const endMs = startMs + 7 * 24 * 60 * 60 * 1000

  const lang = calendar.language ?? 'english'
  const out: ScheduleByDay = {}

  for (const item of calendar.items as Item[]) {
    const schedule = item.airing_schedule ?? []
    for (const entry of schedule) {
      const ms = entry.airingAt * 1000
      if (ms < startMs || ms >= endMs) continue
      const date = new Date(ms)
      const dayKey = isoDayUtc(date)
      const payload: ScheduleEntry = {
        id: item.id * 10000 + entry.episode,
        title: pickTitle(item.title, lang),
        episode: entry.episode,
        time: timeUtc(date),
        coverUrl: item.cover_image?.medium,
      }
      if (!out[dayKey]) out[dayKey] = []
      out[dayKey].push(payload)
    }
  }

  for (const key of Object.keys(out)) {
    out[key]!.sort((a, b) => (a.time ?? '').localeCompare(b.time ?? ''))
  }

  return out
}
