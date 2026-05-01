export interface ScheduleEntry {
  id: number
  title: string
  episode?: number
  time?: string
  coverUrl?: string
}

export type ScheduleByDay = Record<string, ScheduleEntry[]>
