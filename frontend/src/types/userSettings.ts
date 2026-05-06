export type Accent = 'coral' | 'iris' | 'matcha' | 'sakura' | 'citron'

/**
 * Canonical reminder offsets (minutes before episode air time). Server-side
 * validation in `server/src/controllers/user.rs` enforces this exact set.
 */
export const CANONICAL_REMINDER_OFFSETS = [
  15, 30, 60, 120, 360, 720, 1440, 2880, 4320, 10080,
] as const

export interface UserSettings {
  user_id?: number
  theme_preference: 'light' | 'dark'
  language_preference: 'en' | 'pt'
  title_language_preference: 'English' | 'Romaji' | 'Native'
  accent_preference: Accent
  timezone: string
  /**
   * Reminder offsets in minutes. Free users have a single 30-min default;
   * Pro users can store up to 5 entries from `CANONICAL_REMINDER_OFFSETS`.
   * Stored values for Free users are preserved across tier transitions but
   * ignored at .ics emission time (server-enforced).
   */
  reminder_offsets_minutes?: number[]
}
