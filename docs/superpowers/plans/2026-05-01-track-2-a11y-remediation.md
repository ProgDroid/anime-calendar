# Track 2 — A11y Remediation Plan

**Date filed:** 2026-05-01
**Source audit:** `docs/superpowers/plans/2026-05-01-track-2-a11y-audit.md`
**Status:** Done (2026-05-01) — All 6 batches landed; merged to `main` as commit `504516a`. Test count 267 / 49 files passing. Live axe verified 0 contrast / link-in-text-block violations on `/login` across 5 accents × 2 themes. Authenticated-route sweep deferred to `2026-05-01-track-2-a11y-authenticated-sweep.md`.

This plan turns the WCAG 2.1 AA audit findings into actionable batches. Address in batch order; each batch is independently shippable.

## Pre-flight (do this first)

The audit was static-source only — Playwright/axe-core tooling wasn't reachable from the sub-agent's environment. Borderline contrast findings (S2, S7) and all touch-target measurements (C4) are flagged **VERIFY-LIVE** in the audit and need pixel verification before fixes land.

Tasks:
- [ ] Run a fresh Playwright pass with axe-core injected (CDN: `https://cdnjs.cloudflare.com/ajax/libs/axe-core/4.10.0/axe.min.js`) against each Track 2 surface. Record actual `axe.run()` violations into a delta file.
- [ ] Use `getComputedStyle()` to capture real RGB values for: `text-fg-3` on `bg-bg-0`/`bg-bg-1`, `text-danger` on `bg-bg-1`, primary button text per accent hue (coral/iris/matcha/sakura/citron) in BOTH `[data-theme]` modes. Compute WCAG contrast ratios with `tinycolor` or hand-rolled formula.
- [ ] Measure rendered hit areas of: hamburger toggle, mobile theme toggle, eye toggles, accent swatches, AccentPicker chips on a 375×812 viewport. Confirm/refute C2 + C4.
- [ ] Open the Delete-Account ConfirmModal and tab around — confirm/refute C5 (no focus trap).

If the live audit retires any findings as false positives, edit this plan accordingly before starting Batch 1.

---

## Batch 1 — Quick wins (blocks AA, ~1 day)

**Goal:** Close the most user-impactful WCAG failures with minimal code surgery.

### Tasks

- [ ] **C1 — Replace `text-fg-3` body text with `text-fg-2`** across:
  - `frontend/src/components/shared/CalendarTile.vue` (item count metadata, updated date)
  - `frontend/src/components/LoginPage.vue` (OR divider — change `text-fg-3` → `text-fg-2`)
  - Any other meaningful body text using `text-fg-3` (grep `text-fg-3` to find all)
  - Acceptable to keep `text-fg-3` on: large hero numerals (NotFoundPage 4·0·4 already `aria-hidden`), placeholder icon prefixes inside UiInput, large display headings ≥ 18.66px bold or ≥ 24px regular.
- [ ] **C2 — Eye-toggle hit areas + focus-visible**
  - Wrap the eye-toggle button in `inline-flex items-center justify-center w-9 h-9` (36×36 desktop). On mobile breakpoint, ensure ≥ 44×44.
  - Add `focus-visible:outline-2 focus-visible:outline-accent-1 rounded-sm` to the button itself.
  - Apply in: `LoginPage.vue` (password field), `Register.vue` (password + confirm), `ResetPasswordPage.vue` (both fields), `account/PasswordTab.vue` (3 fields if that pattern is used there too — currently it isn't).
- [ ] **C4 — Bump touch targets ≥ 44×44 on mobile**
  - Hamburger toggle in `App.vue`: `p-2` → `p-2.5 min-h-[44px] min-w-[44px]`
  - Mobile-nav-panel theme toggle + logout: switch from `px-3 py-2` to `py-3` on `< md` viewports
- [ ] **C6 — Remember-me checkbox focus-visible**
  - Add `focus-visible:ring-2 focus-visible:ring-accent-1-soft focus-visible:ring-offset-2 focus-visible:ring-offset-bg-0` to the checkbox in `LoginPage.vue`.
- [ ] **M1 — Skip-to-main link**
  - Add `<a href="#main" class="sr-only focus:not-sr-only fixed top-2 left-2 z-50 px-3 py-2 rounded-md bg-bg-1 border border-line">Skip to main content</a>` as the first child inside the App.vue root (or `<body>`-equivalent).
  - Add `id="main"` and `tabindex="-1"` to the `<main>` element so the link can land on it.
  - Add i18n key `app.skipToMain` (en "Skip to main content" / pt "Saltar para o conteúdo principal").
- [ ] **M2 — Default icon SFCs to `aria-hidden="true"`**
  - In each `frontend/src/components/ui/icons/Icon*.vue`, default the SVG to `aria-hidden="true"` unless a `title`/`ariaLabel` slot/prop is provided.
  - Verify no icon SFC currently relies on the parent passing `aria-label` to the wrapper instead of the SVG (most do, so this should be safe).

### Acceptance

- `npm run lint && npm run test:unit && npm run build` clean.
- Re-run axe on `/login` and `/my-calendars` — C1, C2, C4, C6, M1, M2 violations gone.
- Skip link works: Tab once on a fresh page load → focus lands on the link, Enter jumps to `<main>`.

### Suggested commits

1. `fix(a11y): bump body text from fg-3 to fg-2 for AA contrast`
2. `fix(a11y): give password eye-toggles a hit target and focus ring`
3. `fix(a11y): bump mobile touch targets to ≥44px`
4. `fix(a11y): add focus-visible to remember-me checkbox`
5. `feat(a11y): add skip-to-main link`
6. `chore(icons): default icon SVGs to aria-hidden`

---

## Batch 2 — UiModal focus management (~0.5 day)

**Goal:** Close C5 — no focus trap, no restore, no initial focus into the dialog.

### Tasks

- [ ] In `frontend/src/components/ui/UiModal.vue`:
  - On `open` → `true`: store `document.activeElement` as the invoker. Query first focusable inside the dialog (`button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])`) and `.focus()` it. If nothing focusable found, focus the dialog container itself (add `tabindex="-1"` to `[role="dialog"]`).
  - Intercept `Tab` and `Shift+Tab` while open to wrap focus inside the dialog (focus trap).
  - On `open` → `false`: restore focus to the saved invoker.
  - Make sure ESC still closes (already works).
- [ ] Add a vitest spec that opens `UiModal`, asserts focus moves into the dialog, asserts Tab wraps, and asserts close restores focus.
- [ ] Test live by triggering ConfirmModal on `/account/danger` (Delete account) and `/my-calendars` (Delete calendar tile).

### Acceptance

- Tab inside the open Delete-Account modal cycles only between Cancel and Delete buttons.
- ESC closes; focus returns to the original "Delete Account" button.
- New vitest case passes.

### Commit

`fix(a11y): trap and restore focus in UiModal`

### Library note

A 30-line composable handles this fine. If you want a battle-tested option, `focus-trap` (npm) is small and proven. Either is acceptable.

---

## Batch 3 — UiMenu ARIA + arrow keys (~1 day)

**Goal:** Close C3 + S4 — avatar dropdown lacks `aria-haspopup`/`aria-expanded`; menu items inconsistently typed; no arrow-key navigation.

### Tasks

- [ ] **UiMenu trigger contract**
  - Already exposes `open` to the `#trigger` slot. Document this in a JSDoc on the component.
  - Add `aria-controls` that points at the panel's id (generate via `useId()`).
- [ ] **UiMenu consumers — wire ARIA**
  - In `App.vue` topbar avatar trigger: use `<template #trigger="{ open }">`, bind `:aria-haspopup="'menu'"` and `:aria-expanded="open"` on the button.
  - In `CalendarTile.vue` export menu + kebab menu: same treatment.
- [ ] **UiMenu items — `role="menuitem"`**
  - Add `role="menuitem"` to every direct child slot element in App.vue's UiMenu (already partial), CalendarTile's export and kebab menus.
  - OR (cleaner): wrap the default slot in a `<template>` that auto-applies `role="menuitem"` to children. Trade-off: more magic, harder to debug. Per-consumer is fine.
- [ ] **Arrow-key navigation**
  - Inside `UiMenu`'s panel: on `ArrowDown` move focus to next focusable, `ArrowUp` to previous, `Home` to first, `End` to last. Implement as a keydown listener on the panel.
  - Add a vitest spec covering arrow-key navigation.

### Acceptance

- NVDA/VoiceOver announces "Avatar menu, button, collapsed" → activate → "expanded, menu" → arrow-down navigates items.
- Existing click + ESC behavior unchanged.

### Commit

`feat(a11y): add menu-button ARIA and arrow-key navigation to UiMenu`

---

## Batch 4 — Primitive coverage (~1.5 days)

**Goal:** Close C6, M6, S3 with new/upgraded primitives. Highest leverage for future-proofing.

### Tasks

- [ ] **`UiCheckbox` primitive** (new file: `frontend/src/components/ui/UiCheckbox.vue`)
  - Props: `modelValue: boolean`, `label?: string`, `disabled?: boolean`, `id?: string`, `error?: string`.
  - Renders a real `<input type="checkbox">` styled with the design system; built-in `focus-visible:ring-2 focus-visible:ring-accent-1-soft`.
  - Replace the bare `<input type="checkbox">` in `LoginPage.vue` (remember-me) with `<UiCheckbox>`.
- [ ] **Convert `UiSegmented` to `radiogroup` semantics** (S3)
  - Replace `role="tablist"` / `role="tab"` / `aria-selected` with `role="radiogroup"` / `role="radio"` / `aria-checked`. Add a `aria-label` prop required for the radiogroup wrapper.
  - Update consumers' aria-label: `PreferencesTab` (theme + title-language), `CalendarPage` (Editor/Schedule tabs).
  - Wait — Calendar Editor/Schedule IS actually a tablist (changes which view is shown). For that surface, keep tablist semantics or convert it back to a real `<RouterLink>` pair with `aria-current="page"` (cleaner). Decide.
  - Keep a `variant` prop if both semantics are needed: `variant: 'radio' | 'tab'`.
- [ ] **Add focus-visible ring to `<select>`** (M6)
  - Either: scope a global rule in `frontend/src/assets/main.css`: `select:focus-visible { box-shadow: 0 0 0 2px var(--accent-1-soft); border-color: var(--accent-1); }`.
  - Or: build a `UiSelect` primitive (out of scope unless we have multiple uses — currently only PreferencesTab language picker). Global rule is sufficient.

### Acceptance

- Remember-me, theme toggle, title-language, language `<select>` all have visible focus indicators.
- VoiceOver/NVDA announce theme + title-language as "radio button group, [N] of [Total]".

### Commit

1. `feat(ui): add UiCheckbox primitive`
2. `refactor(ui): UiSegmented uses radiogroup semantics`
3. `chore(a11y): add global focus-visible to select elements`

---

## Batch 5 — Contrast token tuning (~0.5 day, requires design buy-in)

**Goal:** Close S2 + S7 by introducing dedicated text contrast tokens.

### Tasks

- [ ] **`--danger-text` token** (S2)
  - Add `--danger-text` to `frontend/src/assets/tokens.css`: dark theme `oklch(50% 0.18 28)`, light theme `oklch(45% 0.20 28)` (verify with live contrast pass first).
  - Expose as Tailwind utility `text-danger-text` via the `@theme` block in `main.css`.
  - Replace `text-danger` with `text-danger-text` in: `UiInput` error helper, all form-error blocks (`login-error`, `password-error`, `preferences-error`, etc.), DangerZoneTab eyebrow.
  - Keep `text-danger` for icon/accent surfaces (ConfirmModal danger button, danger card border).
- [ ] **Primary button text contrast per accent** (S7)
  - Live-measure primary button text contrast for each of the 5 accents in both themes.
  - If any hue dips below 4.5:1, either:
    - (a) bump `--accent-1` lightness for that hue in the `[data-accent]` rule, or
    - (b) change `UiButton` primary variant from `text-bg-0` to `text-accent-1-fg` (per-accent foreground token) — preferred, cleaner.
  - This is a UiButton primitive change, ripples to every primary button.

### Acceptance

- Re-run axe on every form error path → no contrast violations.
- All 5 accents × 2 themes = 10 button screenshots captured with contrast values ≥ 4.5:1.

### Commits

1. `feat(tokens): add --danger-text for AA-compliant error microcopy`
2. `fix(a11y): use accent-1-fg for primary button text contrast per accent`

---

## Batch 6 — Polish (~0.5 day)

**Goal:** Close M5, S5, S6, plus AccountPage tab semantics + landmark labels.

### Tasks

- [ ] **M5 — AccentPicker checkmark for selected**
  - Add a small `IconCheck` overlay on the active swatch (color-vision-safe selection cue beyond the ring).
  - Position absolutely inside the swatch button.
- [ ] **S5 — UiToastHost `aria-live`**
  - Add `aria-live="polite" aria-atomic="false"` to the host container.
  - In `UiToast.vue`, conditional role: `role="alert"` when variant is `danger` or `warning`, else `role="status"`.
- [ ] **S6 — OR divider semantics**
  - Add `role="separator" aria-orientation="horizontal"` to the divider container in LoginPage + Register.
- [ ] **AccountPage sidebar tabs**
  - Convert the sidebar `<button @click="router.push(...)">` tabs to `<RouterLink :to="{ name: tab.name }">`.
  - Add `aria-current="page"` on the active tab via `:aria-current="route.name === tab.name ? 'page' : undefined"`.
  - Preserves middle-click "open in new tab", proper SR landmark navigation.
- [ ] **Landmark labels**
  - Add `aria-label="Primary"` to desktop topbar `<nav>`.
  - Add `aria-label="Mobile primary"` to mobile-nav-panel.
  - Add `aria-label="Account"` to the AccountPage sidebar `<nav>`.
- [ ] **Schedule view week label**
  - In `CalendarScheduleView.vue`, wrap the ISO-week token in `<time :datetime="weekIso" :aria-label="localizedWeekLabel">`. Add a localized week-of-month string for SR users.

### Commits

1. `feat(a11y): add checkmark to selected accent swatch`
2. `fix(a11y): UiToastHost aria-live and danger toast role`
3. `fix(a11y): mark auth OR divider as a separator`
4. `refactor(account): sidebar tabs use RouterLink with aria-current`
5. `chore(a11y): add aria-label to topbar and sidebar nav landmarks`
6. `fix(a11y): add datetime and localized week label to schedule view`

---

## Out of scope for this plan

- AAA-tier improvements not flagged as AA failures.
- Brand redesign / token palette overhaul (only narrow tuning of `--danger-text` and `--accent-1` per Batch 5).
- New translation strings beyond what each task explicitly adds.
- Audit of Track 3 (mobile companion) and Track 4 (upgrade flow) — those tracks aren't built.

## Tracking

Optionally split this into one TaskCreate per batch when picking it up. The audit doc has the full per-finding detail; this plan is the action surface.

## Done definition

- All 6 critical findings resolved.
- All 8 serious findings resolved or explicitly waived (with rationale in the commit body).
- Live axe-core run on every Track 2 surface returns zero violations at AA level.
- Manual NVDA + VoiceOver pass on Login + AccountPage + ConfirmModal confirms expected announcements.
