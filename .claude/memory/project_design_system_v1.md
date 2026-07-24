---
name: Design System v1 (Track 1 Foundations)
description: OKLCH-token + Ui* primitives + theme/accent runtime landed 2026-04-30; Tracks 2-4 still pending
type: project
originSessionId: 7a8d378a-90cb-41a0-b043-addec4ba2a44
---
Track 1 of the design redesign shipped on 2026-04-30. The design system foundation is in place; existing screens are NOT yet re-skinned (Track 2).

**Why:** Source = `design_handoff_anime_calendar/` (Claude Design hifi handoff). Master breakdown at `docs/superpowers/specs/2026-04-30-design-redesign-master-breakdown.md` splits the work into 4 tracks; sequencing is 1 → 2 → 4 → 3.

**How to apply:** When working on UI, prefer `frontend/src/components/ui/Ui*` primitives over DaisyUI classes. Use the OKLCH tokens (`bg-bg-1`, `text-fg-1`, `bg-accent-1`, etc.) instead of DaisyUI semantic colors. DaisyUI is still loaded for legacy screens — don't remove it until Track 2's final commit.

## What's in place

- **Tokens** at `frontend/src/assets/tokens.css` — OKLCH colors, type scale, radii, spacing, shadows, motion. Theme via `[data-theme="light|dark"]` (default dark). Accent via `[data-accent="coral|iris|matcha|sakura|citron"]` (default coral) — only swaps `--accent-h1` / `--accent-h2`.
- **Tailwind v4 `@theme` block** in `frontend/src/assets/main.css` re-exports tokens as utilities (`bg-bg-1`, `text-fg-2`, `bg-accent-1-soft`, etc.).
- **Pre-paint inline script** in `frontend/index.html` reads `localStorage.theme` / `localStorage.accent`, validates against allow-lists, sets `<html data-*>` synchronously before bundle parse → no flash.
- **`useTheme` composable** at `frontend/src/composables/useTheme.ts` — module-scoped refs (singleton), `init()` / `setTheme` / `setAccent` / `reconcileFromServer`. PATCH debounced 400ms. Cross-tab sync via `storage` event.
- **`userSettingsStore`** extended with `accent_preference: Accent` (snake_case from Rust serde).
- **Self-hosted fonts** at `frontend/public/fonts/{Geist,GeistMono,InstrumentSerif}/` with `font-display: swap`.
- **UI primitives** at `frontend/src/components/ui/`: `UiButton`, `UiInput`, `UiSegmented`, `UiChip`, `UiModal`, `UiToast`, `UiAvatar`, `UiBannerFade`, `UiCheckbox`, `UiMenu`, `UiBottomSheet`, `UiBottomTabBar`, `UiEmptyState`, `UiToastHost`. **`UiCard` does NOT exist** — plans and specs that mention it are aspirational; use raw div wrappers with OKLCH token classes (`max-w-md w-full bg-bg-1 border border-line rounded-xl p-8`) instead.
- **22 icon SFCs** at `frontend/src/components/ui/icons/Icon*.vue`, ported verbatim from `design_handoff_anime_calendar/foundations.jsx` `window.Icon`. **Do not substitute Lucide / Heroicons.** Default stroke 1.6, functional set 1.8 — preserve per icon.

## Backend changes

- New Postgres ENUM `accent` (`coral|iris|matcha|sakura|citron`).
- New column `user_settings.accent_preference accent NOT NULL DEFAULT 'coral'` (migration `20260430000000_add_accent_preference.sql`).
- `Accent` Rust enum mirrors `Theme` shape (sqlx::Type, Default = Coral, FromStr, ToSchema).
- Mapper SELECT/INSERT queries updated; `.sqlx/` regenerated and committed.

## Signature contract — banner fade-in mask string

`UiBannerFade.vue` owns the verbatim mask string from the handoff README:
```
linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0) 50%, rgba(0,0,0,0.2) 65%, rgba(0,0,0,1) 95%)
```
A vitest **lock test** (`__tests__/UiBannerFade.spec.ts`) asserts the rendered HTML contains this exact string. Track 2/3 must consume `<UiBannerFade>` rather than reimplement the gradient.

## Tooling additions

- `tailwind-variants` (with `tailwind-merge` peer dep) — variant→class mapping at the top of each component file via `tv()`.

## Tracks pending

- **Track 2**: re-skin Login, Register, MyCalendars, CalendarPage (+ new Weekly schedule view), Account (sidebar tabs), state pages. Final commit removes DaisyUI plugin.
- **Track 4**: Upgrade flow (Interrupt → Paywall → Checkout → Success). Pro tier model, Stripe.
- **Track 3**: Mobile companion (responsive pass; Tabbed editor variant A is recommended default).

Specs at `docs/superpowers/specs/2026-04-30-*-design.md`. Plan for Track 1 at `docs/superpowers/plans/2026-04-30-track-1-foundations.md`.
