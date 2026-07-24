---
name: Track 2 redesign complete (2026-05-01)
description: All Phase A-E work + 7 design-diff follow-ups + form polish landed; 4 pending plans for deferred work
type: project
originSessionId: 7a8d378a-90cb-41a0-b043-addec4ba2a44
---
Track 2 ("Existing surfaces redesign") shipped 2026-05-01. Snapshot of state:

## Completed phases

- **Phase A** — auth pages + states + shared shells reskinned (UiAuthShell)
- **Phase B** — MyCalendarsPage + tiles + `recent_item_ids` backend field
- **Phase C** — CalendarPage split into shell + editor view + schedule sub-route; weekly schedule grid; UiBannerFade adoption
- **Phase D** — AccountPage restructured into shell + 4 tabs (profile, preferences, password, danger); AccentPicker + PRO_ACCENTS; legacy UserDetails/UserSettings deleted
- **Phase E** — DaisyUI fully evicted: toast → UiToast + reactive host; ConfirmModal → UiModal + UiButton; App navbar + PaginationControls re-skinned; daisyui dependency removed; full i18n compliance sweep

## Follow-ups landed post-Track-2 gate

Filed by the design-diff sweep (`docs/superpowers/plans/2026-05-01-track-2-design-diff.md`):
- **FU-1** (5e1ee79) — axios interceptor public-route guard (was breaking email reset links)
- **FU-2** (40be04f) — auth shell visual fidelity: AuthPosterCollage SFC, UiAuthShell eyebrow/h1/subtitle slots, all 6 auth pages updated
- **FU-2b** (c07af20) — auth form polish: UiInput iconLeft/iconRight slots, password eye toggles, Google CTA at top, Remember me checkbox
- **FU-3** (2aac793) — account sidebar avatar block, 260px column, per-tab eyebrow + display heading
- **FU-4** (a0cc8bb) — MyCalendars tile poster header strip with mini-thumbs + "+N" overflow (airing chip deferred — needs backend)
- **FU-5** (cf751e0) — preferences pickers as UiSegmented, icon prefixes, card surfaces, danger-tinted card border
- **FU-6** (8b9e255) — topbar logo glyph, theme toggle, 30px avatar with dropdown housing Logout
- **FU-7** (b501994) — UiEmptyState illustration slot used in NotFoundPage (4·0·4 numeral) + VerifyEmailPendingPage (round mail medallion)

## Test/build state at Track 2 close

- 257 tests / 49 files passing
- oxlint + eslint clean
- `npm run build` ~900ms, no errors
- DaisyUI: zero classes anywhere in `frontend/src`; package removed from `package.json`

## A11y remediation (also 2026-05-01)

Merged on `main` as `504516a` (3 commits: docs/plans + token tuning + the rest). All 6 audit batches landed in one session:

- New tokens: `--accent-1-text` (accent text on neutral bg) and `--danger-text` (error microcopy) split from surface tokens. `--accent-1-l` per-theme + per-hue lightness tuning so light-theme buttons hit 4.5:1 across all 5 accents.
- New primitive: `UiCheckbox` with attr-forwarded `data-testid` (uses `inheritAttrs: false` + `v-bind="$attrs"` on the input).
- `UiModal` gains focus trap + initial focus + restore on close.
- `UiMenu` gains `aria-haspopup`/`aria-expanded`/`aria-controls` (via `panelId` slot prop), `role="menuitem"` on items, arrow/Home/End keyboard nav.
- `UiSegmented` gains `variant: 'radio' | 'tab'` (radio is default, with `role="radiogroup"`/`role="radio"`/`aria-checked`).
- CalendarPage Editor/Schedule + AccountPage sidebar tabs converted to `<RouterLink>` pairs with `aria-current="page"`.
- Skip-to-main link as first focusable on every page; landmark labels on all `<nav>`s.
- Live axe verification: 0 contrast violations on `/login` across 5 accents × 2 themes at 375×812.

Test count: **267 passing / 49 files** (was 261 / 49).

## Pending work (deferred plans filed)

See `reference_track_2_followup_plans.md` for paths.

## Sequencing for the rest of the redesign

Per `reference_design_redesign_tracks.md`: Track 4 (upgrade flow) before Track 3 (mobile companion). Track 2 is fully done; pick up with brainstorm → spec → plan → implement on whichever is next.
