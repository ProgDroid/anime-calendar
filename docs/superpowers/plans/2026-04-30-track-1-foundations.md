# Track 1 — Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the design system from `design_handoff_anime_calendar/` — tokens, base components, theme/accent runtime, self-hosted fonts, and the signature banner-fade-in primitive — without altering any existing screen.

**Architecture:** CSS variables on `:root` define every token. A pre-paint inline script in `index.html` reads `localStorage` and sets `<html data-theme>` / `<html data-accent>` before bundle parse to avoid flash. A `useTheme` composable owns the runtime API; `userSettingsStore` is the source of truth and reconciles via the existing settings-fetch flow. `components/ui/*` houses primitives built with `tailwind-variants`. DaisyUI stays loaded; existing screens are untouched.

**Tech Stack:** Vue 3.5, TypeScript 5.9, Tailwind CSS v4 (`@tailwindcss/vite`), DaisyUI v5 (kept loaded), Pinia 3, Vitest + @vue/test-utils + jsdom, `tailwind-variants` (new), self-hosted Geist + Geist Mono + Instrument Serif. Backend: Rust, sqlx 0.8, PostgreSQL ENUM types.

**Spec:** `docs/superpowers/specs/2026-04-30-track-1-foundations-design.md`
**Master breakdown:** `docs/superpowers/specs/2026-04-30-design-redesign-master-breakdown.md`

---

## Phase A — Backend: accent column

### Task A1: Migration + Accent enum

**Files:**
- Create: `migrations/20260430000000_add_accent_preference.sql`
- Modify: `server/src/entity/user_settings.rs`

- [ ] **Step 1: Write the migration**

Create `migrations/20260430000000_add_accent_preference.sql`:

```sql
CREATE TYPE accent AS ENUM ('coral', 'iris', 'matcha', 'sakura', 'citron');

ALTER TABLE user_settings
    ADD COLUMN accent_preference accent NOT NULL DEFAULT 'coral';
```

- [ ] **Step 2: Add `Accent` enum to entity**

In `server/src/entity/user_settings.rs`, after the `SiteLanguage` enum, add:

```rust
#[derive(Clone, Debug, Deserialize, Serialize, Default, sqlx::Type, Copy, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "accent", rename_all = "lowercase")]
pub enum Accent {
    #[default]
    Coral,
    Iris,
    Matcha,
    Sakura,
    Citron,
}

impl FromStr for Accent {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "iris" => Self::Iris,
            "matcha" => Self::Matcha,
            "sakura" => Self::Sakura,
            "citron" => Self::Citron,
            _ => Self::Coral,
        })
    }
}
```

- [ ] **Step 3: Add `accent_preference` field to `UserSettings`**

Edit the `UserSettings` struct to insert the field between `title_language_preference` and `timezone`:

```rust
pub struct UserSettings {
    pub user_id: i32,
    pub theme_preference: Theme,
    pub language_preference: SiteLanguage,
    pub title_language_preference: Language,
    pub accent_preference: Accent,
    pub timezone: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
```

- [ ] **Step 4: Apply migration to dev DB**

Run: `DATABASE_URL=<your dev url> sqlx migrate run`
Expected: migration applies, `accent_preference` column visible.

- [ ] **Step 5: Commit**

```bash
git add migrations/20260430000000_add_accent_preference.sql server/src/entity/user_settings.rs
git commit -m "feat(db): add accent_preference enum + column"
```

### Task A2: Update mapper queries

**Files:**
- Modify: `server/src/mappers/user_settings.rs`

- [ ] **Step 1: Update SELECT query**

In `get_user_settings_with`, change the SELECT to include the new column. Replace the existing `sqlx::query_as!` block with:

```rust
let settings: Option<UserSettings> = sqlx::query_as!(
    UserSettings,
    "SELECT user_id, theme_preference as \"theme_preference: Theme\", language_preference as \"language_preference: SiteLanguage\", title_language_preference as \"title_language_preference: Language\", accent_preference as \"accent_preference: Accent\", timezone, created_at, updated_at FROM user_settings WHERE user_id = $1",
    user_id
)
.fetch_optional(conn)
.await?;
```

- [ ] **Step 2: Update INSERT/UPDATE query**

In `update_user_settings_with`, replace the `sqlx::query!` block with:

```rust
sqlx::query!(
    "INSERT INTO user_settings (user_id, theme_preference, language_preference, title_language_preference, accent_preference, timezone) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (user_id) DO UPDATE SET theme_preference = EXCLUDED.theme_preference, language_preference = EXCLUDED.language_preference, title_language_preference = EXCLUDED.title_language_preference, accent_preference = EXCLUDED.accent_preference, timezone = EXCLUDED.timezone",
    user_id,
    settings.theme_preference as Theme,
    settings.language_preference as SiteLanguage,
    settings.title_language_preference as Language,
    settings.accent_preference as Accent,
    settings.timezone,
)
.execute(conn)
.await?;
```

- [ ] **Step 3: Add `Accent` to imports at top of file**

```rust
use crate::entity::user_settings::{Accent, SiteLanguage, Theme, UserSettings};
```

- [ ] **Step 4: Update `custom_settings` test helper**

In the `tests` module, edit `custom_settings` to include accent:

```rust
fn custom_settings(user_id: i32) -> UserSettings {
    UserSettings {
        user_id,
        theme_preference: Theme::Light,
        language_preference: SiteLanguage::Pt,
        title_language_preference: Language::Native,
        accent_preference: Accent::Matcha,
        timezone: "Europe/Lisbon".to_string(),
        created_at: chrono::NaiveDateTime::default(),
        updated_at: chrono::NaiveDateTime::default(),
    }
}
```

- [ ] **Step 5: Regenerate sqlx offline cache**

Run: `DATABASE_URL=<your dev url> cargo sqlx prepare --workspace`
Expected: `.sqlx/` updated.

- [ ] **Step 6: Run backend tests**

Run: `AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server user_settings`
Expected: all user_settings tests pass; existing tests round-trip the new field as `Coral` by default.

- [ ] **Step 7: Commit**

```bash
git add server/src/mappers/user_settings.rs .sqlx/
git commit -m "feat(db): persist accent_preference in user_settings mapper"
```

---

## Phase B — Frontend: fonts and tokens

### Task B1: Self-host fonts

**Files:**
- Create: `frontend/public/fonts/Geist/Geist-{400,500,600,700}.woff2`
- Create: `frontend/public/fonts/GeistMono/GeistMono-{400,500}.woff2`
- Create: `frontend/public/fonts/InstrumentSerif/InstrumentSerif-{400,400i}.woff2`

- [ ] **Step 1: Download Geist + Geist Mono**

Download from `https://github.com/vercel/geist-font/tree/main/fonts` the woff2 files for weights 400, 500, 600, 700 (Geist) and 400, 500 (Geist Mono).

Place in `frontend/public/fonts/Geist/` and `frontend/public/fonts/GeistMono/` respectively.

- [ ] **Step 2: Download Instrument Serif**

Download Instrument Serif Regular and Italic woff2 from Google Fonts (`https://fonts.google.com/specimen/Instrument+Serif`) → "Download family" → extract → convert TTF→woff2 if needed (use `https://www.fontsquirrel.com/tools/webfont-generator` or `pyftsubset`).

Place in `frontend/public/fonts/InstrumentSerif/`.

- [ ] **Step 3: Verify file presence**

Run: `ls frontend/public/fonts/*/`
Expected: 4 + 2 + 2 woff2 files present.

- [ ] **Step 4: Commit**

```bash
git add frontend/public/fonts/
git commit -m "feat(frontend): self-host Geist, Geist Mono, Instrument Serif"
```

### Task B2: Tokens CSS file

**Files:**
- Create: `frontend/src/assets/tokens.css`
- Modify: `frontend/src/assets/main.css`

- [ ] **Step 1: Port token values from handoff**

Open `design_handoff_anime_calendar/colors_and_type.css` and `design_handoff_anime_calendar/app.css`. Copy every `:root` declaration (color OKLCH values, type sizes, radii, spacing, shadows, motion) into `frontend/src/assets/tokens.css`.

Preserve every variable name verbatim (`--bg-1`, `--fg-2`, `--accent-h1`, `--d-3`, `--ease-out`, etc.). The handoff file is canonical — do not paraphrase values.

Add at the bottom of the file:

```css
[data-theme="light"] {
  /* Override --bg-*, --fg-*, --line-* with light-theme OKLCH values
     copied from the [data-theme="light"] block in design_handoff_anime_calendar/app.css */
}

[data-accent="iris"]   { --accent-h1: 285; --accent-h2: 28;  }
[data-accent="matcha"] { --accent-h1: 145; --accent-h2: 80;  }
[data-accent="sakura"] { --accent-h1: 350; --accent-h2: 285; }
[data-accent="citron"] { --accent-h1: 80;  --accent-h2: 200; }

@font-face {
  font-family: 'Geist';
  font-weight: 400;
  font-style: normal;
  font-display: swap;
  src: url('/fonts/Geist/Geist-400.woff2') format('woff2');
}
@font-face {
  font-family: 'Geist';
  font-weight: 500;
  font-style: normal;
  font-display: swap;
  src: url('/fonts/Geist/Geist-500.woff2') format('woff2');
}
@font-face {
  font-family: 'Geist';
  font-weight: 600;
  font-style: normal;
  font-display: swap;
  src: url('/fonts/Geist/Geist-600.woff2') format('woff2');
}
@font-face {
  font-family: 'Geist';
  font-weight: 700;
  font-style: normal;
  font-display: swap;
  src: url('/fonts/Geist/Geist-700.woff2') format('woff2');
}
@font-face {
  font-family: 'Geist Mono';
  font-weight: 400;
  font-style: normal;
  font-display: swap;
  src: url('/fonts/GeistMono/GeistMono-400.woff2') format('woff2');
}
@font-face {
  font-family: 'Geist Mono';
  font-weight: 500;
  font-style: normal;
  font-display: swap;
  src: url('/fonts/GeistMono/GeistMono-500.woff2') format('woff2');
}
@font-face {
  font-family: 'Instrument Serif';
  font-weight: 400;
  font-style: normal;
  font-display: swap;
  src: url('/fonts/InstrumentSerif/InstrumentSerif-400.woff2') format('woff2');
}
@font-face {
  font-family: 'Instrument Serif';
  font-weight: 400;
  font-style: italic;
  font-display: swap;
  src: url('/fonts/InstrumentSerif/InstrumentSerif-400i.woff2') format('woff2');
}

@media (prefers-reduced-motion: reduce) {
  :root {
    --d-1: 0ms;
    --d-2: 0ms;
    --d-3: 0ms;
    --d-4: 0ms;
  }
}
```

- [ ] **Step 2: Wire `tokens.css` into the app**

Edit `frontend/src/assets/main.css`. Insert at the very top, before `@import "tailwindcss";`:

```css
@import './tokens.css';
```

After the existing `@import "tailwindcss"` and `@plugin "daisyui"` lines, add a `@theme` block that re-exports tokens as Tailwind utilities:

```css
@theme {
  --color-bg-0: var(--bg-0);
  --color-bg-1: var(--bg-1);
  --color-bg-2: var(--bg-2);
  --color-fg-1: var(--fg-1);
  --color-fg-2: var(--fg-2);
  --color-fg-3: var(--fg-3);
  --color-line: var(--line);
  --color-line-soft: var(--line-soft);
  --color-success: var(--success);
  --color-warning: var(--warning);
  --color-danger: var(--danger);
  --color-accent-1: var(--accent-1);
  --color-accent-1-soft: var(--accent-1-soft);
  --color-accent-1-glow: var(--accent-1-glow);
  --color-accent-2: var(--accent-2);

  --font-sans: 'Geist', system-ui, sans-serif;
  --font-mono: 'Geist Mono', ui-monospace, monospace;
  --font-display: 'Instrument Serif', Georgia, serif;

  --radius-xs: 4px;
  --radius-sm: 6px;
  --radius-md: 10px;
  --radius-lg: 14px;
  --radius-xl: 20px;
  --radius-2xl: 28px;
}
```

- [ ] **Step 3: Verify dev build still works**

Run: `cd frontend && npm run dev`
Open browser, view computed style of `<html>` in DevTools. Confirm `--bg-1` and `--accent-h1` appear with non-empty values.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/assets/tokens.css frontend/src/assets/main.css
git commit -m "feat(frontend): add design tokens + @font-face + theme block"
```

### Task B3: Pre-paint inline script

**Files:**
- Modify: `frontend/index.html`

- [ ] **Step 1: Add inline script + default attributes**

Edit `frontend/index.html`. Replace the `<head>` block with:

```html
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Anime Calendar</title>
    <script>
      (function() {
        try {
          var t = localStorage.getItem('theme');
          var a = localStorage.getItem('accent');
          if (t === 'light' || t === 'dark') {
            document.documentElement.setAttribute('data-theme', t);
          } else {
            document.documentElement.setAttribute('data-theme', 'dark');
          }
          var accents = ['coral','iris','matcha','sakura','citron'];
          if (accents.indexOf(a) !== -1) {
            document.documentElement.setAttribute('data-accent', a);
          } else {
            document.documentElement.setAttribute('data-accent', 'coral');
          }
        } catch (e) {
          document.documentElement.setAttribute('data-theme', 'dark');
          document.documentElement.setAttribute('data-accent', 'coral');
        }
      })();
    </script>
</head>
```

The inline script is static text controlled by us — it reads only known string keys from `localStorage`, validates against an allow-list, and writes whitelisted values to `data-*` attributes (not into the DOM tree). No untrusted HTML is ever inserted.

- [ ] **Step 2: Verify in browser**

Run: `cd frontend && npm run dev`
Open DevTools, check `<html>` element — should have `data-theme="dark" data-accent="coral"` even before main.ts loads.

In the console, run `localStorage.setItem('accent','matcha'); location.reload();` — after reload, `<html data-accent>` should be `matcha`.

Reset with `localStorage.removeItem('accent'); location.reload();`.

- [ ] **Step 3: Commit**

```bash
git add frontend/index.html
git commit -m "feat(frontend): pre-paint theme/accent script in index.html"
```

---

## Phase C — Frontend: theme runtime

### Task C1: Extend UserSettings type

**Files:**
- Modify: `frontend/src/types/userSettings.ts`
- Modify: `frontend/src/stores/userSettingsStore.ts`

- [ ] **Step 1: Add accent_preference to type**

Edit `frontend/src/types/userSettings.ts`:

```typescript
export type Accent = 'coral' | 'iris' | 'matcha' | 'sakura' | 'citron'

export interface UserSettings {
  theme_preference: 'light' | 'dark'
  language_preference: 'en' | 'pt'
  title_language_preference: 'English' | 'Romaji' | 'Native'
  accent_preference: Accent
  timezone: string
}
```

- [ ] **Step 2: Update getDefaultSettings**

In `frontend/src/stores/userSettingsStore.ts`, edit `getDefaultSettings`:

```typescript
const getDefaultSettings = (): UserSettings => {
  return {
    theme_preference: 'dark',
    language_preference: 'en',
    title_language_preference: 'English',
    accent_preference: 'coral',
    timezone: 'UTC'
  }
}
```

- [ ] **Step 3: Run frontend type-check**

Run: `cd frontend && npm run type-check`
Expected: passes; if a test fixture is missing the new field, fix it inline (add `accent_preference: 'coral'`).

- [ ] **Step 4: Run frontend tests**

Run: `cd frontend && npm run test:unit`
Expected: passes.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/types/userSettings.ts frontend/src/stores/userSettingsStore.ts frontend/src/__tests__/
git commit -m "feat(frontend): add accent_preference to UserSettings type"
```

### Task C2: useTheme composable + tests

**Files:**
- Create: `frontend/src/composables/useTheme.ts`
- Test: `frontend/src/__tests__/useTheme.spec.ts`

- [ ] **Step 1: Write failing tests**

Create `frontend/src/__tests__/useTheme.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useTheme } from '@/composables/useTheme'

vi.mock('@/services/userSettingsService', () => ({
  updateUserSettings: vi.fn().mockResolvedValue(undefined),
  getUserSettings: vi.fn(),
  invalidateSettingsCache: vi.fn(),
}))

vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({ isAuthenticated: () => true }),
}))

describe('useTheme', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    document.documentElement.setAttribute('data-theme', 'dark')
    document.documentElement.setAttribute('data-accent', 'coral')
    vi.useFakeTimers()
  })

  it('init() reads data attributes from <html>', () => {
    document.documentElement.setAttribute('data-theme', 'light')
    document.documentElement.setAttribute('data-accent', 'matcha')
    const t = useTheme()
    t.init()
    expect(t.theme.value).toBe('light')
    expect(t.accent.value).toBe('matcha')
  })

  it('setTheme writes data-theme attribute and localStorage', () => {
    const t = useTheme()
    t.init()
    t.setTheme('light')
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
    expect(localStorage.getItem('theme')).toBe('light')
  })

  it('setAccent writes data-accent and localStorage', () => {
    const t = useTheme()
    t.init()
    t.setAccent('iris')
    expect(document.documentElement.getAttribute('data-accent')).toBe('iris')
    expect(localStorage.getItem('accent')).toBe('iris')
  })

  it('setTheme rejects unknown values', () => {
    const t = useTheme()
    t.init()
    // @ts-expect-error invalid value test
    t.setTheme('purple')
    expect(localStorage.getItem('theme')).toBeNull()
  })

  it('setAccent rejects unknown values', () => {
    const t = useTheme()
    t.init()
    // @ts-expect-error invalid value test
    t.setAccent('rainbow')
    expect(localStorage.getItem('accent')).toBeNull()
  })

  it('reconcileFromServer overrides local state when server differs', () => {
    const t = useTheme()
    t.init()
    t.setTheme('dark')
    t.reconcileFromServer({
      theme_preference: 'light',
      accent_preference: 'sakura',
    })
    expect(t.theme.value).toBe('light')
    expect(t.accent.value).toBe('sakura')
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
    expect(localStorage.getItem('accent')).toBe('sakura')
  })

  it('debounces PATCH calls on rapid setAccent toggles', async () => {
    const { updateUserSettings } = await import('@/services/userSettingsService')
    const t = useTheme()
    t.init()
    t.setAccent('iris')
    t.setAccent('matcha')
    t.setAccent('sakura')
    expect(updateUserSettings).not.toHaveBeenCalled()
    await vi.advanceTimersByTimeAsync(500)
    expect(updateUserSettings).toHaveBeenCalledTimes(1)
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd frontend && npm run test:unit -- useTheme`
Expected: FAIL — module `@/composables/useTheme` not found.

- [ ] **Step 3: Implement useTheme**

Create `frontend/src/composables/useTheme.ts`:

```typescript
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
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cd frontend && npm run test:unit -- useTheme`
Expected: all 7 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/composables/useTheme.ts frontend/src/__tests__/useTheme.spec.ts
git commit -m "feat(frontend): useTheme composable with localStorage + server reconcile"
```

### Task C3: Wire useTheme into main.ts

**Files:**
- Modify: `frontend/src/main.ts`

- [ ] **Step 1: Call init + reconcile in bootstrap**

Edit `frontend/src/main.ts`:

```typescript
import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import './assets/main.css'
import { useAuthStore } from './stores/auth'
import { i18n, initI18n } from './plugins/i18n'
import { useUserSettingsStore } from './stores/userSettingsStore'
import { useTheme } from './composables/useTheme'

const app = createApp(App)

app.use(createPinia())
app.use(router)
app.use(i18n)

const theme = useTheme()
theme.init()

const authStore = useAuthStore()
authStore.initAuth().then(() => {
    const userSettingsStore = useUserSettingsStore()
    userSettingsStore.fetchSettings().then(settings => {
        theme.reconcileFromServer({
            theme_preference: settings.theme_preference,
            accent_preference: settings.accent_preference,
        })
        initI18n(settings.language_preference)
        app.mount('#app')
    }).catch(() => {
        initI18n('en')
        app.mount('#app')
    })
})
```

- [ ] **Step 2: Verify dev build**

Run: `cd frontend && npm run type-check && npm run test:unit`
Expected: passes.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/main.ts
git commit -m "feat(frontend): init useTheme and reconcile from server settings"
```

---

## Phase D — UI primitives

### Task D1: Add tailwind-variants dependency

**Files:**
- Modify: `frontend/package.json`

- [ ] **Step 1: Install**

Run: `cd frontend && npm install tailwind-variants`

- [ ] **Step 2: Verify installed**

Run: `cd frontend && grep tailwind-variants package.json`
Expected: appears in `dependencies`.

- [ ] **Step 3: Commit**

```bash
git add frontend/package.json frontend/package-lock.json
git commit -m "chore(frontend): add tailwind-variants"
```

### Task D2: UiButton

**Files:**
- Create: `frontend/src/components/ui/UiButton.vue`
- Test: `frontend/src/__tests__/UiButton.spec.ts`

- [ ] **Step 1: Write failing tests**

Create `frontend/src/__tests__/UiButton.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiButton from '@/components/ui/UiButton.vue'

describe('UiButton', () => {
  it('renders default variant primary, size md', () => {
    const w = mount(UiButton, { slots: { default: 'Click' } })
    expect(w.text()).toBe('Click')
    expect(w.attributes('type')).toBe('button')
    expect(w.classes().some(c => c.includes('accent'))).toBe(true)
  })

  it('disabled blocks click emission', async () => {
    const w = mount(UiButton, { props: { disabled: true } })
    await w.trigger('click')
    expect(w.emitted('click')).toBeUndefined()
  })

  it('loading shows spinner and prevents click', async () => {
    const w = mount(UiButton, { props: { loading: true } })
    expect(w.find('[data-testid="spinner"]').exists()).toBe(true)
    await w.trigger('click')
    expect(w.emitted('click')).toBeUndefined()
  })

  it('danger variant applies danger color class', () => {
    const w = mount(UiButton, { props: { variant: 'danger' } })
    expect(w.classes().some(c => c.includes('danger'))).toBe(true)
  })

  it('size sm applies smaller padding class', () => {
    const w = mount(UiButton, { props: { size: 'sm' } })
    const small = mount(UiButton, { props: { size: 'lg' } })
    expect(w.classes().join(' ')).not.toBe(small.classes().join(' '))
  })
})
```

- [ ] **Step 2: Run tests to verify fail**

Run: `cd frontend && npm run test:unit -- UiButton`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement UiButton**

Create `frontend/src/components/ui/UiButton.vue`:

```vue
<script setup lang="ts">
import { computed } from 'vue'
import { tv } from 'tailwind-variants'

interface Props {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger'
  size?: 'sm' | 'md' | 'lg'
  loading?: boolean
  disabled?: boolean
  type?: 'button' | 'submit'
}
const props = withDefaults(defineProps<Props>(), {
  variant: 'primary',
  size: 'md',
  loading: false,
  disabled: false,
  type: 'button',
})

defineOptions({ name: 'UiButton' })

const emit = defineEmits<{ (e: 'click', ev: MouseEvent): void }>()

const button = tv({
  base: 'inline-flex items-center justify-center gap-2 font-medium transition-all rounded-md select-none disabled:opacity-50 disabled:pointer-events-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-1-soft hover:-translate-y-[0.5px] active:translate-y-0',
  variants: {
    variant: {
      primary: 'bg-accent-1 text-bg-0 hover:shadow-[0_6px_18px_var(--accent-1-glow)]',
      secondary: 'bg-bg-2 text-fg-1 border border-line',
      ghost: 'bg-transparent text-fg-1 hover:bg-bg-2',
      danger: 'bg-danger text-bg-0',
    },
    size: {
      sm: 'h-8 px-3 text-sm',
      md: 'h-10 px-4 text-base',
      lg: 'h-12 px-6 text-md',
    },
  },
})

const classes = computed(() => button({ variant: props.variant, size: props.size }))

function onClick(ev: MouseEvent) {
  if (props.disabled || props.loading) return
  emit('click', ev)
}
</script>

<template>
  <button :class="classes" :type="type" :disabled="disabled || loading" @click="onClick">
    <span v-if="loading" data-testid="spinner" class="inline-block w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin" />
    <slot v-else name="icon-left" />
    <slot />
    <slot name="icon-right" />
  </button>
</template>
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cd frontend && npm run test:unit -- UiButton`
Expected: all 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/ui/UiButton.vue frontend/src/__tests__/UiButton.spec.ts
git commit -m "feat(ui): UiButton with variant/size/loading"
```

### Task D3: UiInput

**Files:**
- Create: `frontend/src/components/ui/UiInput.vue`
- Test: `frontend/src/__tests__/UiInput.spec.ts`

- [ ] **Step 1: Write failing tests**

Create `frontend/src/__tests__/UiInput.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiInput from '@/components/ui/UiInput.vue'

describe('UiInput', () => {
  it('renders label linked via for/id', () => {
    const w = mount(UiInput, { props: { modelValue: '', label: 'Email' } })
    const label = w.find('label')
    const input = w.find('input')
    expect(label.attributes('for')).toBe(input.attributes('id'))
  })

  it('v-model round-trips', async () => {
    const w = mount(UiInput, { props: { modelValue: 'hi' } })
    const input = w.find('input').element as HTMLInputElement
    expect(input.value).toBe('hi')
    await w.find('input').setValue('hello')
    expect(w.emitted('update:modelValue')?.[0]).toEqual(['hello'])
  })

  it('error prop renders helper and danger class', () => {
    const w = mount(UiInput, { props: { modelValue: '', error: 'Required' } })
    expect(w.text()).toContain('Required')
    expect(w.find('input').classes().some(c => c.includes('danger'))).toBe(true)
  })
})
```

- [ ] **Step 2: Run tests to fail**

Run: `cd frontend && npm run test:unit -- UiInput`
Expected: FAIL.

- [ ] **Step 3: Implement UiInput**

Create `frontend/src/components/ui/UiInput.vue`:

```vue
<script setup lang="ts">
import { computed, useId } from 'vue'
import { tv } from 'tailwind-variants'

interface Props {
  modelValue: string
  type?: 'text' | 'email' | 'password'
  placeholder?: string
  label?: string
  error?: string
  disabled?: boolean
  autocomplete?: string
}
const props = withDefaults(defineProps<Props>(), {
  type: 'text',
  placeholder: '',
  label: '',
  error: '',
  disabled: false,
  autocomplete: '',
})

defineOptions({ name: 'UiInput' })

const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>()
const id = useId()
const helperId = `${id}-helper`

const input = tv({
  base: 'w-full h-10 px-3 rounded-md bg-bg-1 text-fg-1 border outline-none transition-shadow duration-[var(--d-1)] focus:shadow-[0_0_0_3px_var(--accent-1-soft)]',
  variants: {
    state: {
      default: 'border-line focus:border-accent-1',
      error: 'border-danger focus:border-danger',
    },
  },
})

const classes = computed(() => input({ state: props.error ? 'error' : 'default' }))

function onInput(ev: Event) {
  emit('update:modelValue', (ev.target as HTMLInputElement).value)
}
</script>

<template>
  <div class="flex flex-col gap-1">
    <label v-if="label" :for="id" class="text-sm text-fg-2">{{ label }}</label>
    <input
      :id="id"
      :class="classes"
      :type="type"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      :autocomplete="autocomplete"
      :aria-invalid="!!error"
      :aria-describedby="error ? helperId : undefined"
      @input="onInput"
    >
    <span v-if="error" :id="helperId" class="text-xs text-danger">{{ error }}</span>
  </div>
</template>
```

- [ ] **Step 4: Tests pass**

Run: `cd frontend && npm run test:unit -- UiInput`
Expected: all 3 PASS.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/ui/UiInput.vue frontend/src/__tests__/UiInput.spec.ts
git commit -m "feat(ui): UiInput with label/error a11y"
```

### Task D4: UiChip

**Files:**
- Create: `frontend/src/components/ui/UiChip.vue`
- Test: `frontend/src/__tests__/UiChip.spec.ts`

- [ ] **Step 1: Write failing tests**

Create `frontend/src/__tests__/UiChip.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiChip from '@/components/ui/UiChip.vue'

describe('UiChip', () => {
  it('renders default variant', () => {
    const w = mount(UiChip, { slots: { default: 'tag' } })
    expect(w.text()).toBe('tag')
  })
  it('pro variant uses accent class', () => {
    const w = mount(UiChip, { props: { variant: 'pro' } })
    expect(w.classes().some(c => c.includes('accent'))).toBe(true)
  })
  it('success variant uses success class', () => {
    const w = mount(UiChip, { props: { variant: 'success' } })
    expect(w.classes().some(c => c.includes('success'))).toBe(true)
  })
})
```

- [ ] **Step 2: Run, fail, then implement**

Create `frontend/src/components/ui/UiChip.vue`:

```vue
<script setup lang="ts">
import { computed } from 'vue'
import { tv } from 'tailwind-variants'

interface Props {
  variant?: 'default' | 'anime' | 'manga' | 'success' | 'warning' | 'pro'
  size?: 'sm' | 'md'
}
const props = withDefaults(defineProps<Props>(), { variant: 'default', size: 'sm' })

defineOptions({ name: 'UiChip' })

const chip = tv({
  base: 'inline-flex items-center rounded-full font-medium',
  variants: {
    variant: {
      default: 'bg-bg-2 text-fg-2',
      anime: 'bg-accent-1-soft text-accent-1',
      manga: 'bg-accent-2/20 text-accent-2',
      success: 'bg-success/20 text-success',
      warning: 'bg-warning/20 text-warning',
      pro: 'bg-accent-1 text-bg-0',
    },
    size: { sm: 'h-5 px-2 text-xs', md: 'h-6 px-3 text-sm' },
  },
})
const classes = computed(() => chip({ variant: props.variant, size: props.size }))
</script>

<template>
  <span :class="classes"><slot /></span>
</template>
```

- [ ] **Step 3: Tests pass + commit**

Run: `cd frontend && npm run test:unit -- UiChip`
Expected: PASS.

```bash
git add frontend/src/components/ui/UiChip.vue frontend/src/__tests__/UiChip.spec.ts
git commit -m "feat(ui): UiChip variants"
```

### Task D5: UiSegmented

**Files:**
- Create: `frontend/src/components/ui/UiSegmented.vue`
- Test: `frontend/src/__tests__/UiSegmented.spec.ts`

- [ ] **Step 1: Write failing tests**

Create `frontend/src/__tests__/UiSegmented.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiSegmented from '@/components/ui/UiSegmented.vue'

const opts = [{ value: 'a', label: 'A' }, { value: 'b', label: 'B' }]

describe('UiSegmented', () => {
  it('renders all options', () => {
    const w = mount(UiSegmented, { props: { modelValue: 'a', options: opts } })
    expect(w.findAll('button')).toHaveLength(2)
  })
  it('clicking emits update:modelValue', async () => {
    const w = mount(UiSegmented, { props: { modelValue: 'a', options: opts } })
    await w.findAll('button')[1]!.trigger('click')
    expect(w.emitted('update:modelValue')?.[0]).toEqual(['b'])
  })
  it('selected option marked aria-selected', () => {
    const w = mount(UiSegmented, { props: { modelValue: 'b', options: opts } })
    expect(w.findAll('button')[1]!.attributes('aria-selected')).toBe('true')
    expect(w.findAll('button')[0]!.attributes('aria-selected')).toBe('false')
  })
})
```

- [ ] **Step 2: Run, fail, implement**

Create `frontend/src/components/ui/UiSegmented.vue`:

```vue
<script setup lang="ts">
interface Option { value: string; label: string }
interface Props { modelValue: string; options: Option[] }
defineProps<Props>()
defineOptions({ name: 'UiSegmented' })
const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>()
</script>

<template>
  <div role="tablist" class="inline-flex rounded-md bg-bg-2 p-1 gap-1">
    <button
      v-for="o in options"
      :key="o.value"
      role="tab"
      type="button"
      :aria-selected="modelValue === o.value"
      :class="[
        'h-8 px-3 text-sm rounded-sm transition-all duration-[var(--d-2)]',
        modelValue === o.value ? 'bg-bg-1 text-fg-1' : 'text-fg-2 hover:text-fg-1'
      ]"
      @click="emit('update:modelValue', o.value)"
    >
      {{ o.label }}
    </button>
  </div>
</template>
```

- [ ] **Step 3: Tests pass + commit**

Run: `cd frontend && npm run test:unit -- UiSegmented`
Expected: PASS.

```bash
git add frontend/src/components/ui/UiSegmented.vue frontend/src/__tests__/UiSegmented.spec.ts
git commit -m "feat(ui): UiSegmented control"
```

### Task D6: UiAvatar

**Files:**
- Create: `frontend/src/components/ui/UiAvatar.vue`
- Test: `frontend/src/__tests__/UiAvatar.spec.ts`

- [ ] **Step 1: Failing tests**

Create `frontend/src/__tests__/UiAvatar.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiAvatar from '@/components/ui/UiAvatar.vue'

describe('UiAvatar', () => {
  it('renders img when src provided', () => {
    const w = mount(UiAvatar, { props: { src: '/p.png', alt: 'Pat' } })
    expect(w.find('img').exists()).toBe(true)
    expect(w.find('img').attributes('alt')).toBe('Pat')
  })
  it('renders fallback initials when no src', () => {
    const w = mount(UiAvatar, { props: { alt: 'Pat', fallback: 'PA' } })
    expect(w.find('img').exists()).toBe(false)
    expect(w.text()).toContain('PA')
  })
})
```

- [ ] **Step 2: Implement**

Create `frontend/src/components/ui/UiAvatar.vue`:

```vue
<script setup lang="ts">
import { computed } from 'vue'
import { tv } from 'tailwind-variants'

interface Props {
  src?: string
  alt: string
  size?: 'sm' | 'md' | 'lg'
  fallback?: string
}
const props = withDefaults(defineProps<Props>(), { src: '', size: 'md', fallback: '' })

defineOptions({ name: 'UiAvatar' })

const avatar = tv({
  base: 'inline-flex items-center justify-center rounded-full overflow-hidden bg-bg-2 text-fg-2 font-medium select-none',
  variants: {
    size: { sm: 'w-6 h-6 text-xs', md: 'w-8 h-8 text-sm', lg: 'w-12 h-12 text-base' },
  },
})
const classes = computed(() => avatar({ size: props.size }))
</script>

<template>
  <span :class="classes" :aria-label="alt">
    <img v-if="src" :src="src" :alt="alt" class="w-full h-full object-cover">
    <span v-else>{{ fallback }}</span>
  </span>
</template>
```

- [ ] **Step 3: Tests pass + commit**

Run: `cd frontend && npm run test:unit -- UiAvatar`
Expected: PASS.

```bash
git add frontend/src/components/ui/UiAvatar.vue frontend/src/__tests__/UiAvatar.spec.ts
git commit -m "feat(ui): UiAvatar img + fallback"
```

### Task D7: UiModal

**Files:**
- Create: `frontend/src/components/ui/UiModal.vue`
- Test: `frontend/src/__tests__/UiModal.spec.ts`

- [ ] **Step 1: Failing tests**

Create `frontend/src/__tests__/UiModal.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiModal from '@/components/ui/UiModal.vue'
import { nextTick } from 'vue'

describe('UiModal', () => {
  it('does not render dialog when open=false', () => {
    const w = mount(UiModal, { props: { open: false, ariaLabel: 'm' }, attachTo: document.body })
    expect(document.querySelector('[role="dialog"]')).toBeNull()
    w.unmount()
  })

  it('renders dialog with aria-modal when open=true', () => {
    const w = mount(UiModal, { props: { open: true, ariaLabel: 'My modal' }, attachTo: document.body })
    const d = document.querySelector('[role="dialog"]')
    expect(d).not.toBeNull()
    expect(d!.getAttribute('aria-modal')).toBe('true')
    expect(d!.getAttribute('aria-label')).toBe('My modal')
    w.unmount()
  })

  it('Esc emits close', async () => {
    const w = mount(UiModal, { props: { open: true, ariaLabel: 'm' }, attachTo: document.body })
    await nextTick()
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(w.emitted('close')).toBeTruthy()
    w.unmount()
  })

  it('scrim click respects closeOnScrim=false', async () => {
    const w = mount(UiModal, { props: { open: true, ariaLabel: 'm', closeOnScrim: false }, attachTo: document.body })
    await document.querySelector('[data-testid="scrim"]')!.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    expect(w.emitted('close')).toBeUndefined()
    w.unmount()
  })
})
```

- [ ] **Step 2: Implement**

Create `frontend/src/components/ui/UiModal.vue`:

```vue
<script setup lang="ts">
import { onMounted, onBeforeUnmount, watch } from 'vue'

interface Props { open: boolean; closeOnScrim?: boolean; ariaLabel: string }
const props = withDefaults(defineProps<Props>(), { closeOnScrim: true })

defineOptions({ name: 'UiModal' })

const emit = defineEmits<{ (e: 'update:open', v: boolean): void; (e: 'close'): void }>()

function close() {
  emit('update:open', false)
  emit('close')
}

function onKey(ev: KeyboardEvent) {
  if (ev.key === 'Escape' && props.open) close()
}

function onScrimClick() {
  if (props.closeOnScrim) close()
}

onMounted(() => document.addEventListener('keydown', onKey))
onBeforeUnmount(() => document.removeEventListener('keydown', onKey))

watch(() => props.open, (v) => {
  document.body.style.overflow = v ? 'hidden' : ''
})
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center">
      <div
        data-testid="scrim"
        class="absolute inset-0 bg-bg-0/70 backdrop-blur-sm transition-opacity duration-[var(--d-2)]"
        @click="onScrimClick"
      />
      <div
        role="dialog"
        aria-modal="true"
        :aria-label="ariaLabel"
        class="relative bg-bg-1 border border-line rounded-lg shadow-lg max-w-md w-full mx-4 transition-all duration-[var(--d-2)]"
      >
        <header v-if="$slots.header" class="p-4 border-b border-line-soft"><slot name="header" /></header>
        <div class="p-4"><slot /></div>
        <footer v-if="$slots.footer" class="p-4 border-t border-line-soft"><slot name="footer" /></footer>
      </div>
    </div>
  </Teleport>
</template>
```

- [ ] **Step 3: Tests pass + commit**

Run: `cd frontend && npm run test:unit -- UiModal`
Expected: PASS.

```bash
git add frontend/src/components/ui/UiModal.vue frontend/src/__tests__/UiModal.spec.ts
git commit -m "feat(ui): UiModal with focus trap basics + Esc"
```

### Task D8: UiToast

**Files:**
- Create: `frontend/src/components/ui/UiToast.vue`
- Test: `frontend/src/__tests__/UiToast.spec.ts`

- [ ] **Step 1: Failing tests**

Create `frontend/src/__tests__/UiToast.spec.ts`:

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import UiToast from '@/components/ui/UiToast.vue'

describe('UiToast', () => {
  beforeEach(() => vi.useFakeTimers())

  it('renders message', () => {
    const w = mount(UiToast, { props: { message: 'hi' } })
    expect(w.text()).toContain('hi')
  })

  it('auto-dismisses after duration', async () => {
    const w = mount(UiToast, { props: { message: 'x', duration: 1000 } })
    await vi.advanceTimersByTimeAsync(1000)
    expect(w.emitted('dismiss')).toBeTruthy()
  })

  it('duration=0 stays sticky', async () => {
    const w = mount(UiToast, { props: { message: 'x', duration: 0 } })
    await vi.advanceTimersByTimeAsync(10000)
    expect(w.emitted('dismiss')).toBeUndefined()
  })

  it('danger variant uses danger class', () => {
    const w = mount(UiToast, { props: { message: 'x', variant: 'danger' } })
    expect(w.classes().some(c => c.includes('danger'))).toBe(true)
  })
})
```

- [ ] **Step 2: Implement**

Create `frontend/src/components/ui/UiToast.vue`:

```vue
<script setup lang="ts">
import { onMounted, onBeforeUnmount, computed } from 'vue'
import { tv } from 'tailwind-variants'

interface Props {
  message: string
  variant?: 'info' | 'success' | 'warning' | 'danger'
  duration?: number
}
const props = withDefaults(defineProps<Props>(), { variant: 'info', duration: 4000 })
defineOptions({ name: 'UiToast' })
const emit = defineEmits<{ (e: 'dismiss'): void }>()

let timer: ReturnType<typeof setTimeout> | null = null

const toast = tv({
  base: 'px-4 py-3 rounded-md shadow-md border',
  variants: {
    variant: {
      info: 'bg-bg-1 text-fg-1 border-line',
      success: 'bg-success/15 text-success border-success/30',
      warning: 'bg-warning/15 text-warning border-warning/30',
      danger: 'bg-danger/15 text-danger border-danger/30',
    },
  },
})
const classes = computed(() => toast({ variant: props.variant }))

onMounted(() => {
  if (props.duration > 0) {
    timer = setTimeout(() => emit('dismiss'), props.duration)
  }
})

onBeforeUnmount(() => { if (timer) clearTimeout(timer) })
</script>

<template>
  <div role="status" :class="classes">{{ message }}</div>
</template>
```

- [ ] **Step 3: Tests pass + commit**

Run: `cd frontend && npm run test:unit -- UiToast`
Expected: PASS.

```bash
git add frontend/src/components/ui/UiToast.vue frontend/src/__tests__/UiToast.spec.ts
git commit -m "feat(ui): UiToast primitive"
```

### Task D9: UiBannerFade (signature primitive)

**Files:**
- Create: `frontend/src/components/ui/UiBannerFade.vue`
- Test: `frontend/src/__tests__/UiBannerFade.spec.ts`

- [ ] **Step 1: Failing tests (mask string lock)**

Create `frontend/src/__tests__/UiBannerFade.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiBannerFade from '@/components/ui/UiBannerFade.vue'

const VERBATIM_MASK = 'linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0) 50%, rgba(0,0,0,0.2) 65%, rgba(0,0,0,1) 95%)'

describe('UiBannerFade', () => {
  it('rendered HTML contains the verbatim mask string', () => {
    const w = mount(UiBannerFade, { props: { selected: true, posterUrl: '/p.png' } })
    expect(w.html()).toContain(VERBATIM_MASK)
  })

  it('selected=false sets opacity-0 class', () => {
    const w = mount(UiBannerFade, { props: { selected: false, posterUrl: '/p.png' } })
    expect(w.find('[data-testid="banner"]').classes()).toContain('opacity-0')
  })

  it('selected=true sets opacity-100 class', () => {
    const w = mount(UiBannerFade, { props: { selected: true, posterUrl: '/p.png' } })
    expect(w.find('[data-testid="banner"]').classes()).toContain('opacity-100')
  })

  it('posterUrl=null falls back to flat accent-soft fill', () => {
    const w = mount(UiBannerFade, { props: { selected: true, posterUrl: null } })
    const style = w.find('[data-testid="banner"]').attributes('style') ?? ''
    expect(style).toContain('var(--accent-1-soft)')
  })
})
```

- [ ] **Step 2: Run tests, verify fail**

Run: `cd frontend && npm run test:unit -- UiBannerFade`
Expected: FAIL.

- [ ] **Step 3: Implement UiBannerFade**

Create `frontend/src/components/ui/UiBannerFade.vue`:

```vue
<script setup lang="ts">
import { computed } from 'vue'

interface Props { selected: boolean; posterUrl: string | null }
const props = defineProps<Props>()
defineOptions({ name: 'UiBannerFade' })

const MASK = 'linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0) 50%, rgba(0,0,0,0.2) 65%, rgba(0,0,0,1) 95%)'

const bgStyle = computed(() => {
  if (!props.posterUrl) {
    return { background: 'var(--accent-1-soft)' }
  }
  return {
    backgroundImage: `linear-gradient(135deg, var(--accent-1-soft), transparent), url(${props.posterUrl})`,
    backgroundSize: 'cover',
    backgroundPosition: 'center',
  }
})

const maskStyle = computed(() => ({
  maskImage: MASK,
  WebkitMaskImage: MASK,
}))
</script>

<template>
  <div class="relative w-full h-full overflow-hidden">
    <div
      data-testid="banner"
      :class="[
        'absolute inset-0 transition-opacity duration-[var(--d-3)] ease-[var(--ease-out)]',
        selected ? 'opacity-100' : 'opacity-0'
      ]"
      :style="{ ...bgStyle, ...maskStyle }"
    />
    <div class="relative z-10"><slot /></div>
  </div>
</template>
```

- [ ] **Step 4: Tests pass**

Run: `cd frontend && npm run test:unit -- UiBannerFade`
Expected: all 4 PASS, including the verbatim mask lock test.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/ui/UiBannerFade.vue frontend/src/__tests__/UiBannerFade.spec.ts
git commit -m "feat(ui): UiBannerFade with locked mask string contract"
```

---

## Phase E — Icons

### Task E1: Inventory and port icons

**Files:**
- Create: `frontend/src/components/ui/icons/IconX.vue` (one per icon in `foundations.jsx`)

- [ ] **Step 1: List icons defined in handoff**

Open `design_handoff_anime_calendar/foundations.jsx`. Find the `window.Icon = { ... }` block. List every key (e.g., `chevronRight`, `check`, `x`, `search`, `plus`, `trash`, `edit`, etc.).

- [ ] **Step 2: Port each icon to a SFC**

For each icon `<name>` in the list, create `frontend/src/components/ui/icons/Icon<PascalName>.vue`:

```vue
<script setup lang="ts">
interface Props { ariaLabel?: string }
const props = defineProps<Props>()
defineOptions({ name: 'Icon<PascalName>' })
</script>

<template>
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    width="24"
    height="24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.6"
    stroke-linecap="round"
    stroke-linejoin="round"
    :aria-hidden="!props.ariaLabel"
    :aria-label="props.ariaLabel"
    :role="props.ariaLabel ? 'img' : undefined"
  >
    <!-- paste exact SVG path/group content from window.Icon.<name> -->
  </svg>
</template>
```

For functional icons (those flagged with stroke 1.8 in `foundations.jsx`), set `stroke-width="1.8"` instead.

- [ ] **Step 3: Spot-check render**

Open `frontend/src/App.vue` temporarily in a scratch branch (or in dev mode) and render 3 icons of your choice to confirm they paint correctly with `currentColor`.

- [ ] **Step 4: Type-check**

Run: `cd frontend && npm run type-check`
Expected: passes.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/ui/icons/
git commit -m "feat(ui): port icon set verbatim from foundations.jsx"
```

### Task E2: Icon contract test

**Files:**
- Test: `frontend/src/__tests__/Icons.spec.ts`

- [ ] **Step 1: Write test**

Create `frontend/src/__tests__/Icons.spec.ts`. Pick 3 representative icons (e.g., `IconCheck`, `IconChevronRight`, `IconPlus`) and assert the contract holds:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import IconCheck from '@/components/ui/icons/IconCheck.vue'
import IconChevronRight from '@/components/ui/icons/IconChevronRight.vue'
import IconPlus from '@/components/ui/icons/IconPlus.vue'

describe.each([
  ['IconCheck', IconCheck],
  ['IconChevronRight', IconChevronRight],
  ['IconPlus', IconPlus],
])('%s', (_name, Cmp) => {
  it('uses 24x24 viewBox and currentColor stroke', () => {
    const w = mount(Cmp)
    const svg = w.find('svg')
    expect(svg.attributes('viewBox')).toBe('0 0 24 24')
    expect(svg.attributes('stroke')).toBe('currentColor')
  })

  it('aria-hidden when no aria-label', () => {
    const w = mount(Cmp)
    expect(w.find('svg').attributes('aria-hidden')).toBe('true')
  })

  it('exposes aria-label and role=img when provided', () => {
    const w = mount(Cmp, { props: { ariaLabel: 'foo' } })
    expect(w.find('svg').attributes('aria-label')).toBe('foo')
    expect(w.find('svg').attributes('role')).toBe('img')
  })
})
```

- [ ] **Step 2: Run + commit**

Run: `cd frontend && npm run test:unit -- Icons`
Expected: PASS.

```bash
git add frontend/src/__tests__/Icons.spec.ts
git commit -m "test(ui): icon contract assertions"
```

---

## Phase F — Final integration check

### Task F1: Smoke check all gates

- [ ] **Step 1: Type-check passes**

Run: `cd frontend && npm run type-check`
Expected: PASS.

- [ ] **Step 2: Lint passes**

Run: `cd frontend && npm run lint`
Expected: PASS (or only warnings; address any errors).

- [ ] **Step 3: Frontend tests pass**

Run: `cd frontend && npm run test:unit`
Expected: all tests PASS, including the new ~30+ added in this plan.

- [ ] **Step 4: Frontend build succeeds**

Run: `cd frontend && npm run build`
Expected: `dist/` produced without errors.

- [ ] **Step 5: Backend tests pass**

Run: `AWS_LC_SYS_PREBUILT_NASM=1 cargo test`
Expected: all tests PASS.

- [ ] **Step 6: Manual browser verification**

Run: `cd frontend && npm run dev`
Open browser. In DevTools console:

```js
// 1. Default state
document.documentElement.dataset.theme  // 'dark'
document.documentElement.dataset.accent // 'coral'

// 2. Apply iris accent
localStorage.setItem('accent', 'iris'); location.reload();
// confirm <html data-accent="iris"> with no flash

// 3. Reset
localStorage.clear(); location.reload();
```

Confirm the existing Login page still renders (DaisyUI still loaded — no visual regression in this track).

- [ ] **Step 7: Final commit (if anything tweaked)**

```bash
git status
# if any files changed during smoke check
git add -A
git commit -m "chore: post-integration smoke fixes"
```

---

## Done criteria

- All 30+ tests added in this plan PASS.
- `<UiBannerFade>` mask string lock test passes (verbatim string from README enforced).
- `<html data-theme>` / `<html data-accent>` set before paint via inline script; values come from localStorage.
- Server `accent_preference` round-trips through migration → entity → mapper → frontend store.
- All eight `Ui*` primitives + per-icon SFCs present under `components/ui/`.
- DaisyUI still loaded; no existing screen visually regressed.
- Type-check, lint, and full vitest suite all green.

## What this plan deliberately does NOT do

- Re-skin any existing screen (`LoginPage.vue`, `MyCalendarsPage.vue`, etc.) → Track 2.
- Remove DaisyUI plugin → final commit of Track 2.
- Wire `useTheme` into a UI control where the user picks accent/theme → Track 2 (Account page).
- Replace `Toast.vue` consumers with `UiToast` → Track 2.
- Storybook → not in this track.
