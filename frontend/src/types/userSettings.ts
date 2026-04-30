export type Accent = 'coral' | 'iris' | 'matcha' | 'sakura' | 'citron'

export interface UserSettings {
  theme_preference: 'light' | 'dark'
  language_preference: 'en' | 'pt'
  title_language_preference: 'English' | 'Romaji' | 'Native'
  accent_preference: Accent
  timezone: string
}
