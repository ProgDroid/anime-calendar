# Track 1 — Foundations Design

**Source**: `design_handoff_anime_calendar/` (Claude Design hifi handoff)
**Master breakdown**: `2026-04-30-design-redesign-master-breakdown.md`
**Date**: 2026-04-30
**Status**: Approved design. Ready for implementation plan.

## 1. Goals & Non-Goals

### Goals
1. Land the canonical design tokens (OKLCH accents, type, spacing, radii, motion, shadows) as runtime CSS variables on `:root`.
2. Ship a complete `components/ui/` library: `UiButton`, `UiInput`, `UiSegmented`, `UiChip`, `UiModal`, `UiToast`, `UiAvatar`, `UiBannerFade`, plus per-icon SFCs under `components/ui/icons/`.
3. Wire theme (`dark | light`) and accent (`coral | iris | matcha | sakura | citron`) state — `localStorage` for instant pre-paint apply, `userSettingsStore` as source of truth, server-wins reconcile.
4. Self-host Geist + Geist Mono + Instrument Serif via `@font-face` in `frontend/public/fonts/`, declared in `main.css`.
5. Lock the `<UiBannerFade>` gradient mask string as a unit-test-enforced contract.

### Non-goals (punted to other tracks)
- Re-skinning any existing screen (Track 2).
- Removing DaisyUI from screens (final commit of Track 2).
- Mobile-specific component variants (Track 3) — base components must be responsive but mobile editor patterns wait.
- Storybook / visual review tooling.

## 2. Architecture

```
frontend/
├── public/fonts/
│   ├── Geist/                    # 400, 500, 600, 700 woff2
│   ├── GeistMono/                # 400, 500 woff2
│   └── InstrumentSerif/          # 400, 400-italic woff2
├── src/
│   ├── assets/
│   │   ├── main.css              # @theme block + @font-face + DaisyUI plugin (still loaded for Track 2)
│   │   └── tokens.css            # token CSS vars on :root + theme/accent overrides
│   ├── composables/
│   │   └── useTheme.ts           # theme + accent state, localStorage sync, store reconcile
│   ├── components/
│   │   └── ui/
│   │       ├── UiButton.vue
│   │       ├── UiInput.vue
│   │       ├── UiSegmented.vue
│   │       ├── UiChip.vue
│   │       ├── UiModal.vue
│   │       ├── UiToast.vue
│   │       ├── UiAvatar.vue
│   │       ├── UiBannerFade.vue
│   │       └── icons/
│   │           ├── IconChevronRight.vue
│   │           ├── IconCheck.vue
│   │           └── ... (one SFC per icon from foundations.jsx)
│   ├── stores/
│   │   └── userSettingsStore.ts  # extended with accent_preference; theme_preference already exists
│   └── main.ts                   # imports tokens.css, calls useTheme().init() on app boot
└── index.html                    # pre-paint inline script reads localStorage and sets :root data attrs
```

### Token layering (bottom wins)
1. `tokens.css` defines every token at `:root` for the **dark + coral** default.
2. `[data-theme="light"]` overrides theme-dependent tokens (`--bg-*`, `--fg-*`, `--line-*`).
3. `[data-accent="iris|matcha|sakura|citron"]` overrides `--accent-h1` and `--accent-h2`.
4. Pre-paint inline script sets `data-theme` and `data-accent` on `<html>` from `localStorage`.

### Runtime flow
1. Browser parses `index.html`. Inline script runs synchronously — reads `localStorage.theme` / `localStorage.accent` (or defaults), sets `<html data-theme="..." data-accent="...">`. **No flash.**
2. Bundle loads, Vue mounts. `main.ts` calls `useTheme().init()`.
3. On non-public route navigation, the existing auth guard fetches user settings. If server values differ from localStorage, `useTheme.reconcileFromServer()` writes through to localStorage and updates the `data-*` attributes.
4. User changes accent in Account → Preferences → `useTheme.setAccent('matcha')` writes localStorage, updates `<html data-accent>`, and patches the server via the settings store.

### Backend changes

The existing `user_settings` table already has `theme_preference theme NOT NULL DEFAULT 'dark'` (Postgres ENUM `light | dark`) and the `Theme` Rust enum in `server/src/entity/user_settings.rs`. **No theme work needed end-to-end.** Only accent is net-new.

1. **New migration** — `migrations/<timestamp>_add_accent_preference.sql`:
   ```sql
   CREATE TYPE accent AS ENUM ('coral', 'iris', 'matcha', 'sakura', 'citron');
   ALTER TABLE user_settings
     ADD COLUMN accent_preference accent NOT NULL DEFAULT 'coral';
   ```
2. **`server/src/entity/user_settings.rs`** — add `Accent` enum mirroring `Theme` (sqlx `Type`, `Default = Coral`, `FromStr`, `ToSchema`). Add `accent_preference: Accent` field to `UserSettings`.
3. **`server/src/mappers/user_settings.rs`** — extend the existing UPDATE query to include `accent_preference`. Regenerate and commit `.sqlx/` after the change (project rule).
4. **Validation** — handled by Postgres ENUM at the DB layer and sqlx typed deserialization at the Rust layer. No application-level validation needed.

## 3. Tokens (canonical reference)

Defined in `frontend/src/assets/tokens.css`, applied to `:root`. All values ported verbatim from `design_handoff_anime_calendar/colors_and_type.css` and `app.css`.

### Color tokens (theme-dependent, dark default)

```css
:root {
  /* Surfaces */
  --bg-0: oklch(...);   /* deepest background */
  --bg-1: oklch(...);   /* card / surface */
  --bg-2: oklch(...);   /* raised surface */
  --fg-1: oklch(...);   /* primary text */
  --fg-2: oklch(...);   /* secondary text */
  --fg-3: oklch(...);   /* tertiary / muted */
  --line: oklch(...);
  --line-soft: oklch(...);

  /* Status */
  --success: oklch(...);
  --warning: oklch(...);
  --danger:  oklch(...);

  /* Accent system — h1/h2 swapped per accent */
  --accent-h1: 28;      /* Coral default */
  --accent-h2: 285;
  --accent-1:       oklch(... var(--accent-h1) ...);
  --accent-1-soft:  oklch(... var(--accent-h1) ... / 0.18);
  --accent-1-glow:  oklch(... var(--accent-h1) ... / 0.38);
  --accent-2:       oklch(... var(--accent-h2) ...);
}

[data-theme="light"] { /* override --bg-*, --fg-*, --line-* */ }

[data-accent="iris"]   { --accent-h1: 285; --accent-h2: 28;  }
[data-accent="matcha"] { --accent-h1: 145; --accent-h2: 80;  }
[data-accent="sakura"] { --accent-h1: 350; --accent-h2: 285; }
[data-accent="citron"] { --accent-h1: 80;  --accent-h2: 200; }
```

Exact OKLCH lightness/chroma/alpha values come from `colors_and_type.css` at implementation time — not pinned in this spec to avoid copy drift.

### Type

```css
--font-sans:    'Geist', system-ui, sans-serif;
--font-mono:    'Geist Mono', ui-monospace, monospace;
--font-display: 'Instrument Serif', Georgia, serif;

/* Scale: 11 / 12.5 / 14 / 16 / 18 / 24 / 34 / 56 */
--text-micro:   0.6875rem;
--text-xs:      0.78125rem;
--text-sm:      0.875rem;
--text-base:    1rem;
--text-md:      1.125rem;
--text-lg:      1.5rem;
--text-xl:      2.125rem;
--text-display: 3.5rem;
```

### Radii / Spacing / Shadows / Motion

- Radii: `--r-xs 4 / --r-sm 6 / --r-md 10 / --r-lg 14 / --r-xl 20 / --r-2xl 28 / --r-full 9999px`
- Spacing: `--s-1 4 / --s-2 8 / --s-3 12 / --s-4 16 / --s-5 20 / --s-6 24 / --s-8 32 / --s-10 40 / --s-12 48 / --s-16 64`
- Shadows: `--shadow-xs / --shadow-sm / --shadow-md / --shadow-lg / --shadow-glow` (last uses `var(--accent-1-glow)`)
- Motion: `--d-1: 120ms / --d-2: 220ms / --d-3: 360ms / --d-4: 600ms`; `--ease-out: cubic-bezier(0.22, 1, 0.36, 1)`

### Tailwind v4 integration

`main.css` `@theme` block re-exports tokens as utilities:

```css
@theme {
  --color-bg-1: var(--bg-1);
  --color-fg-1: var(--fg-1);
  --color-accent-1: var(--accent-1);
  --radius-lg: var(--r-lg);
  --spacing-5: var(--s-5);
  /* ... */
}
```

This makes `bg-bg-1 text-fg-1 rounded-lg p-5` work directly. Single source of truth (`tokens.css`); Tailwind utilities are a thin alias layer.

## 4. Component contracts

Each primitive lives in `frontend/src/components/ui/`. Variant matrices live at the top of each component file via `tailwind-variants` (`tv()`).

### `UiButton.vue`
- **Props**: `variant: 'primary' | 'secondary' | 'ghost' | 'danger'` (default `primary`), `size: 'sm' | 'md' | 'lg'` (default `md`), `loading?: boolean`, `disabled?: boolean`, `type?: 'button' | 'submit'` (default `button`).
- **Slots**: default (label), `icon-left`, `icon-right`.
- **Behavior**: hover `translateY(-0.5px)` over `--d-1`; primary gains `box-shadow: 0 6px 18px var(--accent-1-glow)`. Press resets to `translateY(0)`. `loading` disables the button and shows a spinner in place of `icon-left`.
- **Tests**: variant→class mapping; `disabled` blocks click; `loading` renders spinner and prevents emit.

### `UiInput.vue`
- **Props**: `modelValue: string`, `type?: 'text' | 'email' | 'password'` (default `text`), `placeholder?: string`, `label?: string`, `error?: string`, `disabled?: boolean`, `autocomplete?: string`.
- **Emits**: `update:modelValue`. v-model compatible.
- **Behavior**: focus → border `--accent-1`, box-shadow `0 0 0 3px var(--accent-1-soft)`, transition `--d-1`. `error` switches border to `--danger` and renders helper text below.
- **Tests**: v-model round-trip; focus class application; error state class.

### `UiSegmented.vue`
- **Props**: `modelValue: string`, `options: { value: string; label: string }[]`.
- **Emits**: `update:modelValue`.
- **Behavior**: underline indicator translates between segments over `--d-2 var(--ease-out)`. Click switches selection.
- **Tests**: click emits correct value; underline class targets the selected option.

### `UiChip.vue`
- **Props**: `variant: 'default' | 'anime' | 'manga' | 'success' | 'warning' | 'pro'` (default `default`), `size?: 'sm' | 'md'`.
- **Slots**: default (label).
- **Behavior**: static, presentational. `pro` variant uses `--accent-1` background.
- **Tests**: variant→class mapping.

### `UiModal.vue`
- **Props**: `open: boolean`, `closeOnScrim?: boolean` (default `true`), `ariaLabel: string` (required for a11y).
- **Emits**: `update:open`, `close`.
- **Slots**: default (body), `header`, `footer`.
- **Behavior**: scrim fades over `--d-2`; card scales from 0.96 + opacity 0 to 1/1 over `--d-2 var(--ease-out)`. Esc closes. Focus trap inside the card. Body scroll locked while open.
- **Tests**: open/close lifecycle; Esc handler; scrim click respects `closeOnScrim`; aria-modal + role attrs present.

### `UiToast.vue`
- **Props**: `variant: 'info' | 'success' | 'warning' | 'danger'` (default `info`), `message: string`, `duration?: number` (default `4000`, `0` = sticky).
- **Emits**: `dismiss`.
- **Behavior**: enters from top-right, auto-dismisses after `duration`ms. Track 1 ships the primitive only; the existing `Toast.vue` wiring is rewired in Track 2 to avoid scope creep.
- **Tests**: variant→class; auto-dismiss timing; sticky when `duration === 0`.

### `UiAvatar.vue`
- **Props**: `src?: string`, `alt: string`, `size?: 'sm' | 'md' | 'lg'` (default `md`), `fallback?: string` (initials).
- **Behavior**: renders `img` if `src` resolves; otherwise renders a circle with `fallback` initials on `--bg-2`. Stack composition (overlapping avatars on My Calendars tile) is the consumer's job — UiAvatar stays atomic.
- **Tests**: img vs fallback render path; alt always present.

### `UiBannerFade.vue` (signature primitive)
- **Props**: `selected: boolean`, `posterUrl: string | null`.
- **Behavior**:
  - When `selected` flips `false → true`, opacity transitions `0 → 1` over `--d-3 var(--ease-out)`.
  - Background = a duotone gradient derived from `posterUrl` (poster blurred + tinted with `--accent-1`). If `posterUrl` is null, fall back to a flat `--accent-1-soft` fill.
  - Fade-out mask is **fixed and verbatim from the README**:
    ```
    mask-image: linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0) 50%, rgba(0,0,0,0.2) 65%, rgba(0,0,0,1) 95%);
    ```
  - Honors `prefers-reduced-motion: reduce` → no transition, instant final state.
- **Slots**: default — content rendered on top of the banner.
- **Tests**:
  - **Lock test**: rendered HTML/CSS contains the verbatim mask-image string. Highest-stakes contract test in Track 1.
  - `selected` toggle drives opacity class change.
  - Reduced-motion branch removes transition.

### Icons (`components/ui/icons/IconX.vue`)
- **Props**: `class?: string`, `aria-label?: string`.
- **Default behavior**: `aria-hidden="true"` if no `aria-label` is provided. SVG inline, 24×24, stroke from spec (1.6 default, 1.8 functional set), `currentColor`.
- **One SFC per icon**, ported verbatim from `foundations.jsx` `window.Icon`.

## 5. Theme + Accent runtime

`composables/useTheme.ts` is the single touchpoint. Everything reads/writes through it.

```ts
type Theme = 'dark' | 'light';
type Accent = 'coral' | 'iris' | 'matcha' | 'sakura' | 'citron';

interface UseTheme {
  theme: Ref<Theme>;
  accent: Ref<Accent>;
  setTheme(t: Theme): void;
  setAccent(a: Accent): void;
  init(): void;
  reconcileFromServer(s: { theme_preference: Theme; accent_preference: Accent }): void;
}
```

### `init()` — called once from `main.ts` after Pinia is installed but before `app.mount`
1. Read `<html data-theme>` and `<html data-accent>` (already set by the inline script).
2. Initialize `theme.value` / `accent.value` refs from those attributes.
3. No DOM write — the inline script already did it.

### `setTheme(t)` / `setAccent(a)`
1. Update the ref.
2. Set `<html data-theme="...">` (or `data-accent`).
3. Write `localStorage.theme` / `localStorage.accent`.
4. If user is authenticated, fire-and-forget PATCH to `/users/settings` with the new value. Errors are logged but don't roll back local state — the UI stays optimistic, server reconcile on next settings fetch is the safety net.
5. Network call is debounced 400ms to coalesce rapid toggles.

### `reconcileFromServer(settings)`
- Called by the existing settings-fetch flow (the auth guard's `fetchSettings` step in `router/index.ts`).
- If `settings.theme_preference !== theme.value`, treat server as truth: update ref, `<html>` attr, and localStorage. Same for accent.

### Pre-paint inline script (in `index.html` `<head>`, before any other script)

```html
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
```

Whitelist guards (theme ∈ {dark, light}, accent ∈ five known values) handle malformed localStorage without throwing or applying unsafe attribute values.

### `userSettingsStore` integration
- Extend the existing TS settings type to include `accent_preference` (snake_case to match Rust serde). `theme_preference` already exists.
- After a successful `fetchSettings`, call `useTheme().reconcileFromServer(settings)`.
- Existing settings update PATCH path is reused — `setTheme`/`setAccent` calls the same endpoint.

### Cross-tab sync
- Listen for `storage` events on `localStorage.theme` / `localStorage.accent`. When fired in another tab, re-apply locally. ~5 lines, prevents desync between two open tabs.

### Tests
- `useTheme.spec.ts`: `setTheme('light')` writes `data-theme`, `localStorage`, fires PATCH (mocked); `reconcileFromServer({theme_preference: 'light', ...})` from a `dark` baseline updates everything; whitelist rejects unknown values.
- `index.html` script not unit-tested — 20 lines of vanilla JS guarded by try/catch, validated by hand.

## 6. Error handling, edge cases, accessibility

### Error handling
- `useTheme.setTheme/setAccent` PATCH failure → log via existing logging shim, keep local state. Next `fetchSettings` reconciles.
- Malformed `localStorage` values → caught by inline script's try/catch and the whitelist; defaults applied.
- Missing fonts (network blip) → `font-display: swap` shows fallback, swaps when ready.
- Banner fade-in with `posterUrl: null` → flat `--accent-1-soft` background, no transition jank.
- Backend migration failure (app run against old DB) → sqlx fails fast at startup querying for `accent_preference`. Expected.

### Edge cases
- Rapid theme/accent toggling → 400ms debounce on PATCH; local state and `<html>` attrs update immediately.
- Two tabs open, one changes accent → `storage` event listener re-applies in the other tab.
- Public routes (login, register, password reset) — `fetchSettings` is skipped per existing auth guard convention. `useTheme` works fine on public routes; values come from localStorage only.

### Accessibility
- `prefers-reduced-motion: reduce` honored. Add a `@media (prefers-reduced-motion: reduce)` block in `tokens.css` setting `--d-1`/`--d-2`/`--d-3` to `0ms` so every component using motion tokens degrades correctly with no per-component code.
- All icon-only buttons must pass `aria-label`. Documented as a convention; future eslint rule deferred.
- Color contrast — handoff is hifi and contrast-checked, but spot-check `--fg-3` on `--bg-1` and `--bg-0` against WCAG AA during implementation.
- `UiModal` — focus trap, Esc to close, `role="dialog"` + `aria-modal="true"` + required `aria-label`, focus restored to invoking element on close.
- `UiInput` — `label` prop renders a real `<label>` linked via `for`/`id`. Error message gets `aria-describedby`. Required fields use `aria-required`.

### Cross-cutting project rules respected
- Token names and component prop values are not user-visible — no i18n needed in Track 1.
- The Account page accent labels ("Coral", "Iris", etc.) **are** user-visible; their keys go in both `en.json` and `pt.json` when Track 2 wires the Account page.
- sqlx: regenerate `.sqlx/` after the new migration + query change.
- Accent enum follows the existing `Theme`/`SiteLanguage` pattern verbatim.

## 7. Out of scope / open questions

- Tablet breakpoints (430–1280px) — handoff defers; Track 3 will address.
- Light-theme polish — README says "spot-checked, dark is canonical." Surface any contrast issues during implementation; punt fixes if non-blocking.
- Eslint rule for icon-only buttons requiring `aria-label` — future improvement.
