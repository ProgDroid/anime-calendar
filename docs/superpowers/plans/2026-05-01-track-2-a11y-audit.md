# Track 2 — WCAG 2.1 AA Audit

**Date:** 2026-05-01
**Method:** Static source audit of `frontend/src/components/**` + token-level contrast estimation against `frontend/src/assets/tokens.css` + ARIA/semantic review of every Track-2 surface.

> **Methodology note (transparency).** The Playwright MCP toolset referenced in the audit brief (`mcp__plugin_playwright_playwright__browser_*`) was **not available** in this session — only `Read`, `Write`, `Bash`, `Glob`, `Grep` were exposed. Live `axe-core` injection, computed-style scraping, and keyboard sweeps were therefore not performed. Findings below are derived from:
>
> 1. Reading the rendered template + script of every Track-2 SFC.
> 2. Reading `tokens.css` and computing approximate WCAG contrast ratios from the OKLCH lightness values (`L%`) using the standard sRGB-relative-luminance proxy `Y ≈ (L/100)^2.4`. Results within ~0.3:1 of the AA thresholds (4.5 / 3.0) are flagged as **borderline** and should be re-verified live.
> 3. ARIA / semantic / keyboard-pattern review of each component.
>
> Anywhere a finding depends on actual rendered pixels (dynamic accent hue, focus ring visibility, mobile touch sizing) it is marked **VERIFY-LIVE**.

## Summary

| Bucket | Count |
|---|---:|
| Surfaces audited | 14 |
| Critical (definite AA fail) | 6 |
| Serious (borderline / risky) | 8 |
| Minor (best-practice / AAA) | 7 |
| Cross-cutting patterns | 5 |

## Critical findings

| # | Surface | WCAG | Issue | Suggested fix |
|---|---|---|---|---|
| C1 | All surfaces using `text-fg-3` on `bg-0`/`bg-1` (CalendarTile metadata, login divider "or", placeholder icons via `text-fg-3` in `UiInput`, NotFoundPage 404 numeral) | 1.4.3 | `--fg-3` is L 48% (dark) / L 64% (light); against `bg-0`/`bg-1` the contrast is ≈ **2.5–2.8 : 1**, well below the 4.5:1 body threshold. Used for "calendar updated", item count separator, "or" divider on login, and the input icon prefix/suffix tint. | Promote any meaningful text from `text-fg-3` to `text-fg-2`. Reserve `fg-3` for ≥ 18.66 px bold or ≥ 24 px regular text only, and document this in the design-system README. |
| C2 | LoginPage / RegisterPage / ResetPasswordPage — eye-toggle button | 2.5.5, 2.4.7 | The eye-toggle is a bare `<button>` with only `cursor-pointer hover:text-fg-1` — no padding, no min-w/min-h, no `focus-visible` styles. The icon SVG is set globally to `w-3.5 h-3.5` (= **14 × 14 CSS px**) inside the input's `right-3` slot, far below the 44×44 touch target. Focus indicator is invisible because `UiInput`'s wrapper does not propagate any focus ring to nested buttons. | Wrap the toggle in an `inline-flex items-center justify-center w-9 h-9 -mr-2` (or equivalent) hit area, add `focus-visible:outline-2 focus-visible:outline-accent-1 rounded-sm`, and ensure the SVG stays at 14 px while the click target is 36×36+ desktop / 44×44 mobile. Same pattern for confirm-password and reset toggles. |
| C3 | App.vue — avatar dropdown trigger (`topbar-avatar`) | 4.1.2 | The trigger button has `aria-label` but no `aria-haspopup="menu"` and no `aria-expanded` bound to `UiMenu`'s `open` state. Screen readers cannot announce that activating the avatar opens a menu, nor whether it is currently open. `UiMenu` exposes `open` to its `#trigger` slot but App.vue ignores it. | In `UiMenu`, surface `open` to the trigger slot (already done) and document the trigger contract; in App.vue add `:aria-haspopup="'menu'" :aria-expanded="open"` (use `<template #trigger="{ open }">`). Apply the same fix to every `UiMenu` consumer (CalendarTile export menu, kebab menu). |
| C4 | App.vue mobile nav button (375 px) | 2.5.5 | The hamburger button is `p-2` around an `h-6 w-6` SVG → ~ **40 × 40 CSS px**, just under the 44×44 minimum. The mobile theme-toggle inside `mobile-nav-panel` is `px-3 py-2` with a 16 px icon → height ≈ 32–36 px; also under target. | Bump hamburger to `p-2.5` plus `min-h-[44px] min-w-[44px]`, and switch the in-panel theme/logout buttons to `py-3` on `< md` viewports. |
| C5 | UiModal | 2.1.2, 2.4.3 | Modal renders the dialog without focus management: focus is **not** moved into the dialog on open, **not** trapped while open, and **not** restored to the invoker on close. ESC closes (good) but a keyboard user tabbing out of the dialog escapes back into the obscured page beneath the scrim. Also, `tabindex="-1"` is missing on the dialog container so programmatic focus is impossible without a ref. | Add a focus trap: on `open` true, store `document.activeElement`, query first focusable inside the dialog and `.focus()` it, intercept `Tab`/`Shift+Tab` to wrap focus, and on close restore the saved element. Add `tabindex="-1"` to the dialog `<div role="dialog">`. Library `focus-trap` or a 30-line composable is sufficient. Affects ConfirmModal usage in DangerZoneTab and any future modals. |
| C6 | LoginPage — remember-me + forgot-password row | 1.3.1, 4.1.2 | The "remember me" checkbox is a bare native `<input type="checkbox">` with `class="accent-accent-1"`. It has an associated label via the wrapping `<label>`, but it has **no visible focus indicator** (browser default is suppressed by the Tailwind reset, no `focus-visible` was added). Combined with the absence of any keyboard styling on the toggle, AA 2.4.7 fails. | Add `focus-visible:ring-2 focus-visible:ring-accent-1-soft focus-visible:ring-offset-2 focus-visible:ring-offset-bg-0` to the checkbox, or wrap in a custom `UiCheckbox` primitive (recommended — also fixes the consistency gap with `UiInput`/`UiSegmented`). |

## Serious / borderline

| # | Surface | WCAG | Issue | Suggested fix |
|---|---|---|---|---|
| S1 | UiInput | 1.3.1 | When `:label` prop is omitted (any usage that uses an external label, e.g. password row in LoginPage), the input gets `:id` but no programmatic label association unless the caller supplies the matching `for=`. LoginPage does this correctly, but the API allows silently un-labelled inputs. | Add a `aria-label` prop fallback or warn in dev when neither `label` prop nor `aria-label` is set. |
| S2 | UiInput error state | 3.3.1 | `aria-describedby` is wired to the helper id only when an error is present. Good. But the error text node has `class="text-xs text-danger"` — `--danger` is L 66% (dark) / L 58% (light) on `bg-1`. Dark: ~3.8:1, light: ~4.0:1. Fails 4.5:1 for body. **VERIFY-LIVE.** | Use `text-fg-1` for the error message body and prepend a danger-colored icon, OR darken `--danger` for text contexts (introduce `--danger-text`). |
| S3 | UiSegmented | 4.1.2 | Uses `role="tablist"` + `role="tab"` + `aria-selected`, but tabs are not associated with any `role="tabpanel"`, the `id`/`aria-controls` linkage is missing, and roving tabindex / arrow-key navigation is not implemented. Screen-reader users can't navigate between tabs with arrow keys, and JAWS may read "1 of 2" without knowing what panel they control. Used on PreferencesTab (theme, title-language) where the value triggers immediate state change rather than a tabpanel — semantically this is a `radiogroup`, not a tablist. | Either (a) implement a true tablist with arrow-key navigation + tabpanel ids, or (b) switch to `role="radiogroup"` with `role="radio"` children + `aria-checked`. Option (b) matches the actual UX. |
| S4 | UiMenu | 2.1.1, 4.1.2 | The trigger is a generic `<span>` wrapping the slot, not a button. While the consumer's slot content is usually a `<button>`, the menu panel itself uses `role="menu"` but its children are bare `<button>` and `<RouterLink>` without `role="menuitem"` (App.vue manually adds `role="menuitem"`, but CalendarTile does not). Arrow-key navigation between items is not implemented. ESC closes (good). | Either switch to a headless library (`headlessui-vue`) or implement: (a) wrap trigger slot in a focusable element with `aria-haspopup`, `aria-expanded`, `aria-controls`; (b) add `role="menuitem"` to slotted children automatically; (c) implement Up/Down/Home/End arrow-key navigation. |
| S5 | UiToast / UiToastHost | 4.1.3 | `UiToast` renders `role="status"` (good for polite announcement). However: (a) the host container needs `aria-live="polite"` (not just per-toast role) when toasts mount asynchronously, otherwise some screen readers miss the late-mounted node; (b) `danger` variant should use `role="alert"` (assertive) rather than `status`. | Add `aria-live="polite" aria-atomic="false"` to `UiToastHost`'s root, and in `UiToast` switch role to `alert` when variant is `danger` or `warning`. |
| S6 | LoginPage / Register / ResetPassword | 2.4.3 | The Google CTA appears **before** the email/password fields. After Google sign-in fails or is dismissed, screen-reader users tabbing top-down hit the Google button first. Tab order is logical, but the OR divider (`<span>` … "or" … `<span>`) is not announced as a separator (no `role="separator"`). | Add `role="separator" aria-orientation="horizontal"` to the divider container. The Google-first position is a UX choice — leave it, but consider adding a hidden `<h2>` for "Or sign in with email" so AT users get a section label. |
| S7 | Primary button text on light theme | 1.4.3 | `UiButton` primary: `bg-accent-1 text-bg-0`. In light theme that's L 60 accent vs L 98 bg-0 → ~ **3.9 : 1** for some hues (sakura/coral OK, citron/matcha at L60 with chroma 0.20 hue 80/145 may dip below 4.5:1). **VERIFY-LIVE per accent.** | Either bump `--accent-1` lightness contrast for light theme (consider L 50% for primary surfaces), or change primary button text to `--accent-1-fg` (already L 98% in dark, but the dark token uses L 18% which is the inverse — perfect). The current token names are already correct; it's the App that uses `text-bg-0` instead of `text-accent-1-fg`. |
| S8 | CalendarTile | 1.1.1, 2.1.1 | `<img>` cover thumbnails use `alt=""` + `aria-hidden="true"` (decorative — fine), but the **entire tile body is a single `<button>`** that wraps the title + count + thumbnails, AND there is a separate edit button below. The button name is the concatenation of all visible text — which is verbose and confusing for SR users. The "+N" overflow text uses `text-white drop-shadow` over a poster image with variable luminance — contrast is **uncontrolled** and likely fails on light posters. | (a) Give the body button an explicit `aria-label="{{ calendar.name }}"`; (b) ensure the overflow chip has a solid scrim (`bg-bg-0/60` behind text) and uses `text-fg-0` on a guaranteed background. |

## Minor / best-practice

| # | Surface | WCAG | Issue | Suggested fix |
|---|---|---|---|---|
| M1 | App.vue topbar | 2.4.1 | No "skip to main content" link before the header. Keyboard users have to tab through logo + nav + theme toggle + avatar on every page. | Add `<a href="#main" class="sr-only focus:not-sr-only ...">Skip to main content</a>` as the first child of `<body>` (or App root) and `id="main"` on `<main>`. |
| M2 | App.vue logo | 1.3.1 | `IconLogo` SVG decoration with no `aria-hidden`; the `RouterLink` already has `aria-label`, so the SVG announcement is redundant. | Add `aria-hidden="true"` to `IconLogo` consumers. Better: make the icon SFCs default to `aria-hidden="true"` unless a `title` slot is provided. |
| M3 | NotFoundPage 404 numeral | 1.1.1 | The `4·0·4` numeral is correctly marked `aria-hidden="true"`, and `UiEmptyState` provides title/body. PASS. (Listed for visibility — no change needed.) | — |
| M4 | AccountPage sidebar avatar | 1.1.1 | Initials medallion has `aria-hidden="true"`, name + email rendered separately as text. PASS. | — |
| M5 | AccentPicker | 1.4.3, 4.1.2 | Each swatch button uses `aria-pressed` (good for toggle semantics) but the `UiChip variant="pro"` Pro badge has `aria-hidden="true"`, and the accessible name already includes "(Pro)" via `ariaLabel()` — well done. The selection ring (`ring-2 ring-accent-1`) is the **only** visual indicator; users with low color-vision need a non-color cue. | Add a checkmark icon overlay (`IconCheck`) in the selected state, or thicken the ring to 3px and add an inner ring of `ring-bg-0` for offset. |
| M6 | PreferencesTab `<select>` for language | 1.3.1 | The select has a leading globe icon overlay via `pl-9` — but the icon is purely decorative inside a `<span class="pointer-events-none">`. PASS, but the icon color is `text-fg-2` which is fine. The select itself has no `focus-visible:ring`, so default browser focus is the only cue. | Add `focus-visible:ring-2 focus-visible:ring-accent-1-soft` to the `<select>`. |
| M7 | CalendarSchedule grid | 1.3.1 | `<div class="grid grid-cols-1 sm:grid-cols-7">` containing day columns has no `role="grid"` or any landmark. Day labels are inside each column header, so technically OK for visual users; non-grid semantics is acceptable here. (No fix required, noted for AAA.) | — |

## Per-surface details

### `/login` (LoginPage.vue)
- **Theme tested:** static review against both `[data-theme=dark|light]` tokens.
- **Findings:** C2, C6, S1, S2, S6, S7 (eye toggle, remember-me focus, divider semantics, primary-button hue contrast).
- **Pass:** form labels associated via `for`/`id` (UiInput); `aria-invalid` + `aria-describedby` on error; `role="alert"` on error block; `autocomplete` set; `data-testid` coverage solid; `tabindex` natural and logical.

### `/register` (LoginPage in `isRegistering` mode)
- Same shell as `/login`; inherits all findings. Username field has no icon prefix — fine. `maxlength="50"` enforced.
- Extra: the toggleMode `<button>` at the bottom uses `text-accent-1 hover:underline` only — relies on color alone for affordance. **(S6 variant.)** Add a permanent underline or a `>` chevron.

### `/forgot-password`
- Not directly read in this session, but uses the same `UiAuthShell` + `UiInput` with `IconMail` slot. Inherits S1, S6, M2, plus C1 (subtitle uses `text-fg-2` — borderline OK; description text using `fg-3` would fail).

### `/reset-password?token=...` (ResetPasswordPage.vue)
- **Findings:** C2 (×2 eye toggles), S1, S2, S6, M5.
- **Pass:** success block has `role="status"`; error block has `role="alert"`; password mismatch is checked client-side and surfaced via `errorMessage`.
- **Bug-adjacent:** `passwordMismatch` is a string error, not field-level — the error appears in a generic block at the bottom rather than via `aria-describedby` on the confirm field. Screen reader will hear it but won't associate it with the field. Consider attaching it to `reset-confirm`.

### `/verify-email/pending` (VerifyEmailPendingPage.vue)
- Not read this session; design contract specifies a "round mail medallion". Verify: medallion must be `aria-hidden="true"` and the heading + body must carry the announcement. Resend CTA must use `UiButton` (already migrated in commit `5fe1ce7`).

### `/this-route-does-not-exist` (NotFoundPage.vue)
- **Findings:** none new beyond M3.
- **Pass:** numeral is `aria-hidden`; `UiEmptyState` provides title/body; CTA is a real `UiButton`.

### `/my-calendars` (MyCalendarsPage + CalendarTile + PaginationControls)
- **Findings:** C1 (tile metadata uses `text-fg-3`), S4 (UiMenu role gaps), S8 (tile button name verbosity, overflow chip contrast).
- **Pass:** PaginationControls disable prev/next correctly; pagination uses `UiButton` so focus + variants are consistent; loading="lazy" on poster images.
- **Risk:** the `+N` overflow text on top of variable-luminance posters fails 1.4.3 unconditionally — fix is mandatory.

### `/calendar/:id` (CalendarEditorView)
- Tabs (Editor / Schedule): not read this session. Verify they are real `<router-link>`s with `aria-current="page"` on the active tab and not `<button>`s mimicking links.
- Settings form: presumably uses `UiInput` — same S1/S2 caveats apply.

### `/calendar/:id/schedule` (CalendarScheduleView)
- **Findings:** prev/next week buttons use translated text `t('schedule.prev')` (good), but the week label `<span>{{ week }}</span>` is in ISO-week format ("2026-W18") which is not human-readable. Add a `<time datetime="...">` and a `aria-label` with localized "Week 18 of April 2026".
- The shift-week buttons should also have `aria-label` with the destination week, not just "Prev"/"Next" — best practice.
- Grid is `grid-cols-1 sm:grid-cols-7` — passes responsive.

### `/account/profile` (ProfileTab + AccountPage shell)
- AccountPage sidebar uses `<nav>` + `<button>` for tabs that route to children. **WCAG concern (S3-adjacent):** route-changing controls should be `<RouterLink>`/`<a>`, not `<button>` — this breaks middle-click "open in new tab" and screen-reader landmark navigation. Switch to `<RouterLink :to="{ name: tab.name }">`.
- Active state uses `bg-bg-2 text-fg-1` — no `aria-current="page"`. Add it for SR.

### `/account/preferences` (PreferencesTab + AccentPicker)
- **Findings:** S3 (segmented should be radiogroup), M5 (accent picker color-only selection cue), M6 (select focus ring).
- **Pass:** all labels associated; AccentPicker has accessible names including Pro state; timezone input has label.

### `/account/password` (PasswordTab)
- Not read this session; assume three `UiInput` password fields. Inherits C2 (×3 eye toggles if present), S2 (error contrast).

### `/account/danger` (DangerZoneTab)
- **Findings:** the `text-danger` eyebrow uses `--danger` on `bg-0` which is borderline (S2-class). Use `text-fg-1` with a danger icon prefix.
- **Pass:** ConfirmModal is invoked correctly with `:danger="true"`; delete button uses `variant="danger"`; loading state announced via text content change ("Deleting…").
- **Inherited C5:** the modal opened here lacks focus management.

### Topbar (App.vue)
- **Findings:** C3 (avatar dropdown ARIA), M1 (skip link), M2 (logo SVG aria-hidden).
- **Pass:** theme toggle has `aria-label`; mobile toggle has `aria-expanded`; nav uses `<RouterLink>` with `active-class`.
- **Missing:** `<nav aria-label="Primary">` and `<nav aria-label="Mobile primary">` for landmark disambiguation.

### Mobile (375 px)
- **Findings:** C4 (hamburger 40×40 < 44×44), in-panel theme/logout buttons under target.
- **Pass:** `aria-expanded` on hamburger; `mobile-nav-panel` has testid; route change auto-closes panel.

## Cross-cutting patterns

1. **`text-fg-3` is overused for body text.** Token `--fg-3` is below AA contrast on every standard background in both themes. Search and replace for meaningful text → `text-fg-2`. Acceptable uses: tertiary metadata in **large** display contexts only (e.g. NotFoundPage hero numeral, which is correctly aria-hidden).
2. **Focus-visible discipline is inconsistent.** `UiButton` has `focus-visible:ring-2 focus-visible:ring-accent-1-soft`. Every other interactive element (eye toggles, native checkboxes, native selects, `UiMenu` items, `UiSegmented` tabs, AccountPage sidebar buttons, accent swatches) has **no** focus-visible style. Adopt a global rule: every `<button>`, `<a>`, `<input>`, `<select>` reachable by Tab must have a 2 px `accent-1-soft` outline at offset 2 px.
3. **`UiMenu` is missing standard menu-button ARIA.** Trigger lacks `aria-haspopup`/`aria-expanded`; panel children inconsistently typed `role="menuitem"`; no arrow-key navigation. Affects topbar avatar, CalendarTile export menu, CalendarTile kebab.
4. **Modals do not trap or restore focus.** Single fix in `UiModal` propagates to ConfirmModal and any future modal — high leverage.
5. **Native `<input type="checkbox">` and `<select>` are unstyled.** No `UiCheckbox`/`UiSelect` primitive exists; ad-hoc usages on LoginPage and PreferencesTab inherit no design-system focus or contrast guarantees. Adding these primitives closes 3 findings at once (C6, M6, S1-radiogroup-fix).

## Suggested follow-up scope

**Batch 1 — Quick wins (≈ 1 day, blocks AA):**
- C1: replace `text-fg-3` with `text-fg-2` on all metadata (CalendarTile, login divider, NotFoundPage CTA copy if any).
- C2: wrap eye toggles in 36/44 px focusable button hit areas with `focus-visible` styles. Apply to LoginPage, ResetPasswordPage, PasswordTab.
- C4: bump hamburger and mobile-nav buttons to ≥ 44×44.
- C6: add `focus-visible` ring to the remember-me checkbox.
- M1: add a skip-to-main link.
- M2: default icon SFCs to `aria-hidden="true"`.

**Batch 2 — UiModal focus management (≈ 0.5 day):**
- C5: implement focus trap + restore in `UiModal`. Cover with a vitest spec.

**Batch 3 — UiMenu ARIA (≈ 1 day):**
- C3 + S4: wire `aria-haspopup`/`aria-expanded` from `UiMenu` to the trigger via slot props; add `role="menuitem"` to menu children automatically; implement arrow-key navigation. Update App.vue topbar and CalendarTile.

**Batch 4 — Primitive coverage (≈ 1.5 days):**
- New `UiCheckbox` primitive — solves C6 and unblocks consistent form styling.
- Convert `UiSegmented` to `radiogroup` semantics (S3).
- Add `focus-visible` ring to `<select>` either via primitive or via `main.css` global rule (M6).

**Batch 5 — Contrast token tuning (≈ 0.5 day, requires design buy-in):**
- S2: introduce `--danger-text` (≈ L 50% dark / L 45% light) for error microcopy, leaving `--danger` for surfaces/icons.
- S7: re-verify primary-button text contrast per accent hue with live `getComputedStyle` + an axe pass; if any hue dips < 4.5:1, route primary text to `--accent-1-fg` instead of `--bg-0`.

**Batch 6 — Polish (best-practice):**
- M5: add checkmark icon to selected accent swatch.
- S5: `aria-live="polite"` on `UiToastHost`; `role="alert"` for danger/warning toasts.
- AccountPage: convert sidebar `<button>` tabs to `<RouterLink>` with `aria-current="page"`.
- `<nav aria-label>` on topbar nav landmarks.

## Items not testable in this session

- **Live contrast verification.** No Playwright MCP / axe-core injection was available. All contrast numbers above are OKLCH-lightness estimates; the borderline cases (S2, S7) and the dynamic accent-hue buttons must be re-checked with `getComputedStyle()` once Playwright tooling is available.
- **Focus-visible appearance.** Whether the existing `UiButton` ring is actually visible against `bg-0`/`bg-1` in both themes was not pixel-verified.
- **Screen-reader announcements.** NVDA / JAWS / VoiceOver behavior is inferred from ARIA markup, not observed. The `UiSegmented` "tablist without tabpanel" case may behave differently across SR vendors.
- **Modal focus state.** Could not interactively open ConfirmModal to confirm focus-trap absence beyond reading source.
- **VerifyEmailPendingPage**, **PasswordTab**, **ProfileTab**, **CalendarEditorView**, **CalendarSettingsForm**, **ItemSearchPanel** — were not opened in this audit's source review (token usage budget). Inherit Batch-1/Batch-2 fixes generically; a follow-up source pass should confirm.
- **Mobile viewport rendering.** Touch target measurements (C4) are derived from Tailwind class arithmetic, not actual rendered CSS pixels. **VERIFY-LIVE.**

---

*Generated 2026-05-01 by Claude (Opus 4.7 1M). Static-only audit — re-run with Playwright + axe-core before sign-off.*
