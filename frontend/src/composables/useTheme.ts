import { ref, type Ref } from 'vue'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { useAuthStore } from '@/stores/auth'
import { updateUserSettings } from '@/services/userSettingsService'
import type { Accent, UserSettings } from '@/types/userSettings'
import { PRO_ACCENTS } from '@/constants/proAccents'

const DEFAULT_ACCENT: Accent = 'coral'

type Theme = 'dark' | 'light'

const THEMES: readonly Theme[] = ['dark', 'light'] as const
const ACCENTS: readonly Accent[] = ['coral', 'iris', 'matcha', 'sakura', 'citron'] as const

const theme: Ref<Theme> = ref('dark')
const accent: Ref<Accent> = ref('coral')

// Whether the current user is entitled to Pro accents. Defaults to `true`
// so unauth and pre-fetch states don't blank out a stored Pro accent before
// we know the answer (the backend gate is the source of truth — see
// `feedback_billing_endpoint_no_request_body.md`). PreferencesTab + the
// app's auth flow call `setIsPaid` when the entitlement read resolves.
const isPaid: Ref<boolean> = ref(true)

let patchTimer: ReturnType<typeof setTimeout> | null = null
let storageListenerAttached = false

// Cross-tab sync: mirror theme/accent changes made in another tab. Defined at
// module scope (a stable reference) and attached at most once (F2-28) — the
// singleton composable would otherwise leak a fresh, un-removable listener on
// every init() call (e.g. across test re-instantiations).
function handleStorageEvent(e: StorageEvent) {
  if (e.key === 'theme' && isTheme(e.newValue)) {
    theme.value = e.newValue
    document.documentElement.setAttribute('data-theme', e.newValue)
  } else if (e.key === 'accent' && isAccent(e.newValue)) {
    accent.value = e.newValue
    document.documentElement.setAttribute('data-accent', e.newValue)
  }
}

/// Map an accent value to what should actually be rendered. Pro accents on
/// a free user fall back to the default — preserves `accent` (the stored
/// preference) so re-upgrading immediately restores it without needing a
/// server roundtrip.
function resolveAccent(a: Accent): Accent {
  if (!isPaid.value && PRO_ACCENTS.has(a)) return DEFAULT_ACCENT
  return a
}

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

    if (!storageListenerAttached) {
      storageListenerAttached = true
      window.addEventListener('storage', handleStorageEvent)
    }
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
    // Stored value: whatever the caller picked. localStorage holds the
    // user's preference so a later re-upgrade restores it without a
    // server roundtrip.
    accent.value = a
    localStorage.setItem('accent', a)
    // Rendered value: gated through `resolveAccent` so a free user with a
    // stale Pro accent (post-downgrade) sees the default, not their
    // preference.
    document.documentElement.setAttribute('data-accent', resolveAccent(a))
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
      localStorage.setItem('accent', s.accent_preference)
      document.documentElement.setAttribute('data-accent', resolveAccent(s.accent_preference))
    }
  }

  /// Update the tier flag and re-resolve the rendered accent. Called by the
  /// auth/entitlement bootstrapping flow once the tier is known. The stored
  /// `accent` ref + localStorage are NOT touched — only the DOM attribute
  /// changes if the resolved render-value differs.
  function setIsPaid(paid: boolean) {
    if (isPaid.value === paid) return
    isPaid.value = paid
    document.documentElement.setAttribute('data-accent', resolveAccent(accent.value))
  }

  return { theme, accent, isPaid, init, setTheme, setAccent, reconcileFromServer, setIsPaid }
}
