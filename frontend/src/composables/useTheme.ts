import { ref, type Ref } from 'vue'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { useAuthStore } from '@/stores/auth'
import { updateUserSettings } from '@/services/userSettingsService'
import type { Accent, UserSettings } from '@/types/userSettings'

type Theme = 'dark' | 'light'

const THEMES: readonly Theme[] = ['dark', 'light'] as const
const ACCENTS: readonly Accent[] = ['coral', 'iris', 'matcha', 'sakura', 'citron'] as const

const theme: Ref<Theme> = ref('dark')
const accent: Ref<Accent> = ref('coral')
let patchTimer: ReturnType<typeof setTimeout> | null = null

function isTheme(v: unknown): v is Theme {
  return typeof v === 'string' && (THEMES as readonly string[]).includes(v)
}

function isAccent(v: unknown): v is Accent {
  return typeof v === 'string' && (ACCENTS as readonly string[]).includes(v)
}

function schedulePatch() {
  if (patchTimer) clearTimeout(patchTimer)
  patchTimer = setTimeout(() => {
    patchTimer = null
    void persistToServer()
  }, 400)
}

async function persistToServer() {
  const auth = useAuthStore()
  if (!auth.isAuthenticated()) return
  const store = useUserSettingsStore()
  const current: UserSettings = store.settings ?? store.getDefaultSettings()
  const next: UserSettings = {
    ...current,
    theme_preference: theme.value,
    accent_preference: accent.value,
  }
  try {
    await updateUserSettings(next)
    store.settings = next
  } catch (err) {
    console.error('Failed to persist theme/accent', err)
  }
}

export function useTheme() {
  function init() {
    const t = document.documentElement.getAttribute('data-theme')
    const a = document.documentElement.getAttribute('data-accent')
    if (isTheme(t)) theme.value = t
    if (isAccent(a)) accent.value = a

    window.addEventListener('storage', (e) => {
      if (e.key === 'theme' && isTheme(e.newValue)) {
        theme.value = e.newValue
        document.documentElement.setAttribute('data-theme', e.newValue)
      } else if (e.key === 'accent' && isAccent(e.newValue)) {
        accent.value = e.newValue
        document.documentElement.setAttribute('data-accent', e.newValue)
      }
    })
  }

  function setTheme(t: Theme) {
    if (!isTheme(t)) return
    theme.value = t
    document.documentElement.setAttribute('data-theme', t)
    localStorage.setItem('theme', t)
    schedulePatch()
  }

  function setAccent(a: Accent) {
    if (!isAccent(a)) return
    accent.value = a
    document.documentElement.setAttribute('data-accent', a)
    localStorage.setItem('accent', a)
    schedulePatch()
  }

  function reconcileFromServer(s: { theme_preference: Theme; accent_preference: Accent }) {
    if (isTheme(s.theme_preference) && s.theme_preference !== theme.value) {
      theme.value = s.theme_preference
      document.documentElement.setAttribute('data-theme', s.theme_preference)
      localStorage.setItem('theme', s.theme_preference)
    }
    if (isAccent(s.accent_preference) && s.accent_preference !== accent.value) {
      accent.value = s.accent_preference
      document.documentElement.setAttribute('data-accent', s.accent_preference)
      localStorage.setItem('accent', s.accent_preference)
    }
  }

  return { theme, accent, init, setTheme, setAccent, reconcileFromServer }
}
