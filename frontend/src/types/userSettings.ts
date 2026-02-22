export interface UserSettings {
  theme_preference: 'light' | 'dark'
  language_preference: 'en' | 'pt'
  title_language_preference: 'English' | 'Romaji' | 'Native'
  date_display_preference: 'yyyymmdd' | 'ddmmyy' | 'mmddyyyy'
  date_separator_preference: 'slash' | 'dash'
  timezone: string
}
