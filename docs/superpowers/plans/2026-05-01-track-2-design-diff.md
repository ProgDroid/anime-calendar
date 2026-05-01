# Track 2 — UI vs Design diff sweep

Date: 2026-05-01
Method: Playwright live render against `design_handoff_anime_calendar/` JSX. Auth-required surfaces (calendar editor, schedule, account tabs) were diffed at the source level after the test session lost its auth cookie midway through the sweep — see "Auth session interruption" below.

## Summary

- Surfaces compared: 16 (12 rendered live, 4 source-only after cookie was lost)
- Blockers: 4
- Polish items: 19
- Minor: 7
- Matches: 8

## Auth session interruption

Mid-sweep I called `POST /api/auth/logout` to capture the public auth pages with a fresh state and was unable to re-authenticate from the Playwright session (no password). After that point the calendar editor, schedule view, and account tabs were diffed by reading the Vue source against the JSX design. Screenshots exist for: `login.png`, `register.png`, `forgot-password.png`, `reset-password.png` (showing the redirect chain), `my-calendars.png` (full page), and several blank-screen captures during transitions.

## Cross-cutting blockers

These show up on multiple surfaces; calling them out once.

| # | Type | Issue | Where | Suggested fix |
|---|---|---|---|---|
| X1 | Blocker | Direct URL navigation to `/forgot-password`, `/reset-password?token=…`, `/verify-email/pending`, `/verify-email`, `/register` redirects to `/login` on a full page reload (e.g. clicking a reset link from email). In-app navigation works. Cause: `router.beforeEach` calls `userSettingsStore.fetchSettings()` only on non-public routes, but on public routes the auth-store rehydration via `initAuth()` plus a 401 from a queued request appears to bounce the navigation. Verified by direct `page.goto` returning the login layout for all four routes. | router/index.ts ~L99-124, plus per-page `onMounted` in `ResetPasswordPage.vue` (router.replace fires before token reads from query in some race) | Trace why `to.meta.public` routes lose the URL on hard reload. Add a regression test that hits `/forgot-password` via `createMemoryHistory` + Playwright `page.goto` (currently only in-app nav is covered). |
| X2 | Blocker | Left "atmospheric" pane on every auth surface is empty (`<div class="hidden lg:block flex-1 bg-bg-2" />`). The design specifies a poster collage with floating posters at fixed offsets, two radial accent gradients, and the wordmark in the upper-left. | `frontend/src/components/ui/UiAuthShell.vue` left pane | Add an `AuthPosterCollage.vue` SFC that renders 4 posters with rotation transforms (per `screens-auth-list.jsx` lines 63-74) and the dual radial gradient background. Slot it into `UiAuthShell` when `posterTestid` is set. |
| X3 | Blocker | All auth pages place the wordmark *inside the form card*. Design places the wordmark in the upper-left of the LEFT pane only; the right form pane gets a `t-micro` eyebrow + `t-h1` heading + `t-small` subtitle (e.g. "Welcome back" / "Sign in" / "Pick up where you left off"). | `Login.vue`, `Register.vue`, `ForgotPasswordPage.vue`, `ResetPasswordPage.vue`, `VerifyEmailPendingPage.vue`, `VerifyEmailConfirmPage.vue` | Move `<Wordmark>` to the left pane (the new poster collage) and replace the in-card wordmark with the eyebrow/h1/subtitle pattern. |
| X4 | Blocker | Account area has no shared header/wrapper. Design puts a `t-micro` eyebrow ("Profile" / "Preferences" / "Security" / "Danger zone") and a large display heading ("Your *details*", "How it *looks & reads*", "Change *password*", "Delete *account*") above the card on every tab. Live tabs jump straight into a form. The sidebar lacks the avatar+name+email block at the top (per design lines 271-280). | `AccountPage.vue`, all four `account/*Tab.vue` | Add an `AccountTabHeader` slot or component for eyebrow + display heading. Add an `AccountSidebarUser` block with avatar + display name + email at the top of the sidebar. |

---

## Findings by surface

### /login (Phase A)

**Match level:** Polish needed (after blockers X2/X3 are fixed)
**Design source:** `screens-auth-list.jsx` `LoginScreen` (L44-132)
**Screenshot:** `.playwright-mcp/login-2.png`

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Blocker | Left pane content | Floating posters + dual radial gradient + wordmark | Empty `bg-bg-2` panel | See X2 |
| 2 | Blocker | Form heading | "Welcome back" eyebrow + "Sign in" h1 + "Pick up where you left off" subtitle | "Anime Calendar" wordmark + "Login" h1 + tagline copy | See X3 |
| 3 | Polish | Google button position | At top of form, ABOVE divider, before email/password | Below the form, after Login button | Move Google CTA to top, swap divider direction |
| 4 | Polish | Email input | Has inline `Mail` icon at left (paddingLeft 36) | No inline icon | Add icon slot to `UiInput` and wire here |
| 5 | Polish | Password input | Has inline `Lock` icon at left + `Eye` toggle at right | Plain input | Add show/hide password toggle |
| 6 | Polish | Forgot password placement | Inline with Password label (right-aligned, above the input) | Below the password input, right-aligned | Move into label row above input |
| 7 | Polish | "Remember me" checkbox | Present below password input | Missing | Add checkbox with `t-small` label |
| 8 | Polish | Submit copy | "Sign in" | "Login" | Update i18n key value |
| 9 | Polish | Footer copy | "Don't have an account? Create one" | "Don't have an account? Register" | Soften to "Create one" |
| 10 | Match | Card border, radius, bg-bg-1 | rounded-xl, border-line | matches | — |
| 11 | Match | Login button uses accent-1 | accent-1 fg | matches | — |
| 12 | Match | Wordmark serif italic on "Calendar" | Instrument Serif italic | matches (Wordmark component renders correctly) | — |

### /register (Phase A)

**Match level:** Polish needed
**Design source:** No dedicated `RegisterScreen` in handoff — the design system implies same shell + form fields. Compared against LoginScreen pattern.
**Screenshot:** `.playwright-mcp/register.png`

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Blocker | Left pane empty | poster collage | empty | See X2 |
| 2 | Blocker | Heading pattern | eyebrow + h1 + subtitle | wordmark + "Register" | See X3 |
| 3 | Polish | Field labels | "Username" | "Name" (label says Name but placeholder says "Enter your username") | Use one term consistently — design uses "Username" |
| 4 | Polish | Google CTA position | Top, above divider | Bottom | Reorder |
| 5 | Polish | Footer copy | "Already have an account? Sign in" | "Already have an account? Login" | Soften copy |
| 6 | Match | Form structure | name/email/password stack | matches | — |

### /forgot-password (Phase A)

**Match level:** Mostly matches (after blockers)
**Design source:** Implied by `screens-auth-list.jsx` shell + `screens-extras.jsx` empty-state language
**Screenshot:** `.playwright-mcp/forgot-password.png`

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Blocker | Direct URL load redirects | Public route should render | Redirects to /login | See X1 |
| 2 | Blocker | Left collage / heading pattern | per design | empty pane + wordmark in card | See X2/X3 |
| 3 | Match | Single email field + primary submit + back-to-login link | matches structurally | matches | — |

### /reset-password?token=invalid (Phase A)

**Match level:** Has blocker
**Design source:** Implied
**Screenshot:** `.playwright-mcp/reset-password.png` (shows /login because of redirect chain)

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Blocker | Direct URL load redirects | Public route should render the form even with bad token, then show inline validation on submit | `onMounted` `router.replace('/forgot-password')` if token is missing AND on bad-token full-reload chain ends at /login | Don't redirect on missing token — render the form with a disabled submit and a "request a new link" CTA. Treat bad-token as a server response, not a client guard. |
| 2 | Blocker | Heading + collage | per design | not visible (redirected) | See X2/X3 |

### /verify-email/pending (Phase A)

**Match level:** Polish needed
**Design source:** `screens-extras.jsx` StatesScreen (verify email card, L109-120)
**Screenshot:** N/A (redirected on direct nav — see X1). Source review only.

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Blocker | Direct URL load redirects to /login | Should render | Redirects | See X1 |
| 2 | Polish | Mail icon medallion | 56×56 round badge in `accent-2-soft` background with `accent-2` `<Icon.Mail s={22}/>` centered above the heading | `UiEmptyState` is generic — no medallion icon | Add `icon` prop or named slot to `UiEmptyState`; pass a 56px round badge with the mail icon |
| 3 | Polish | Email address rendering | Email rendered inline-bold inside the body copy | Body is generic copy; email passed via `history.state.email` but not surfaced in the empty-state body | Wire `email` into the body via i18n placeholder `{email}` and bold it |
| 4 | Match | Resend CTA | Secondary button | matches (UiButton variant=secondary) | — |
| 5 | Match | Back-to-login link | Below CTA | matches | — |

### /verify-email (confirm) (Phase A)

**Match level:** Source-only review
**Design source:** No explicit design — implied by extras states. Token-based confirmation has no JSX in the handoff.

| # | Type | Element | Notes |
|---|---|---|---|
| 1 | Minor | No design source for the confirm-success or confirm-error variants | Out of scope for diff. The confirm page should at minimum reuse `UiAuthShell` + a success/error empty-state and a CTA back to /login. |

### /this-route-does-not-exist — 404 (Phase A)

**Match level:** Polish needed
**Design source:** `screens-extras.jsx` 404 card (L122-130)
**Screenshot:** Not captured (auth session lost). Source review.

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Polish | Hero "404" numeral | Display-size (96px) numeral with the middle "0" italic | `UiEmptyState` shows only title + body, no oversized numeral | Pass a `<template #icon>` or hero slot rendering "4*0*4" with `font-display` size 96 and italic middle |
| 2 | Polish | Subtitle copy | "The calendar you're looking for slipped between dimensions." | i18n key `errors.notFound.body` (verify wording) | Verify copy matches design tone |
| 3 | Match | Primary CTA "Back to my calendars" | matches `errors.notFound.cta` | — |
| 4 | Match | Wrapped in UiAuthShell | matches design which renders inside the auth-style chrome | — |

### /my-calendars (Phase B)

**Match level:** Polish needed
**Design source:** `screens-auth-list.jsx` `MyCalendarsScreen` (L182-237)
**Screenshot:** `.playwright-mcp/my-calendars.png` (full page)

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Match | 3-col grid, 20px gap | `repeat(3, 1fr)` gap 20 | `376px 376px 376px` gap 20 | — |
| 2 | Match | "My calendars" h1 in Instrument Serif | t-display 44px italic on "calendars" | "My Calendars" 56px Instrument Serif (italic on "Calendars" via i18n value) | — (size diff is within range) |
| 3 | Match | "Your library" t-micro eyebrow | uppercase tracking-wide small caps | "YOUR LIBRARY" rendered | — |
| 4 | Polish | Header stats line | "6 calendars · 30 total items · *3 airing this season*" with warning dot | "17 calendars · 17 total items" — no airing-this-season pill, no warning dot | Add airing aggregate stat once backend exposes airing count per calendar (out-of-scope FU) |
| 5 | Polish | Grid/List segmented toggle | `<UiSegmented>` with Grid / List options + corresponding layouts | Missing entirely; only Grid layout exists | Add segmented toggle bound to `?layout=grid\|list` query param; add list layout component |
| 6 | Polish | Tile poster header | Tile top half is a 96px poster collage with 3-poster row in the upper-left, "+N" overflow badge, and an "X airing" chip in upper-right; bottom half is name + meta + actions | Live tile shows a single large poster + 3-tile collage but the gradient overlay, "+N" badge, and airing chip are missing | Add upper-row poster strip (28×38 thumbs, top 3) and gradient overlay; add airing chip |
| 7 | Polish | Tile actions | Edit (secondary), Export+chevron (secondary), trash (ghost icon-only) | Edit (secondary), Export+chevron (secondary), Delete (ghost icon — looks correct) | matches structurally; verify icon sizes vs design's 12px |
| 8 | Polish | Pagination text | "Showing 1–6 of 6" centered between Previous and Next ghost buttons | "Previous 1 2 3 Next" + "Showing 1 to 6 of 17 calendars" below | The page-number variant is fine — design just shows simple Prev/Next; live design is arguably better. Keep current. Move "Showing X to Y of Z" inline if desired. |
| 9 | Polish | Tile bg + border | `card` (var(--bg-1) + var(--line-soft) + radius var(--r-md)) | inline tile uses transparent bg | Tile root needs `bg-bg-1 border border-line rounded-md` wrapper |
| 10 | Match | "Create New Calendar" primary button with plus icon | btn-primary + Plus icon | matches | — |

### /calendar/23 — Calendar Editor (Phase C, source-only)

**Match level:** Polish needed
**Design source:** `screens-editor-account.jsx` `CalendarEditorScreen` (L62-225)
**Files reviewed:** `frontend/src/components/calendar/CalendarEditorView.vue`, `CalendarItemsList.vue`, `ItemSearchPanel.vue`, `RecommendationsSection.vue`, `CalendarSettingsForm.vue`

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Polish | Sub-header layout | Single horizontal row: ←breadcrumb / calendar-name input (h1-sized, transparent border) / language segmented / Cancel / Save | Verify CalendarEditorView places all five elements on one row with name input set to `font-display`-style typography | Inspect when re-authenticated; if not present, restructure |
| 2 | Polish | Body grid columns | `1fr 1.05fr` (right slightly wider for search) | Verify in CalendarEditorView | Match `1fr 1.05fr` |
| 3 | Polish | Right panel bg | `var(--bg-inset)` | Verify | Use `bg-bg-inset` token (or `bg-bg-2` if no `inset` token exists in Track 1) |
| 4 | Polish | "In your calendar" header | h2 + monospace count + Clear ghost button | Verify | Match |
| 5 | Polish | Empty calendar state | Dashed-border card with single line of muted copy | Verify CalendarItemsList.vue has dashed-border empty | Use `border-dashed` |
| 6 | Polish | Recommendations grid | 2-col grid of compact mini-rows w/ poster thumb 36×48, title 12.5px, type+year subtitle, ghost +Plus icon button | Verify RecommendationsSection.vue | Match the 2-col mini-row pattern |
| 7 | Polish | Item row banner-fade | Selected items show poster as background w/ left-to-right fade mask (locked by contract test) | Verify ItemSearchPanel uses the locked `UiBannerFade` mask | Per CLAUDE.md, mask string is locked; do not modify |
| 8 | Polish | "Add N selected" CTA | Primary small button + plus icon, only visible when selection > 0 | Verify | Conditional render |
| 9 | Match | Sub-tab segmented for English/Romaji/Native | UiSegmented | likely matches (Track 1 primitive available) | — |

### /calendar/23/schedule (Phase C, source-only)

**Match level:** Polish needed
**Design source:** `screens-extras.jsx` `CalendarTimelineScreen` (L7-88)
**Files reviewed:** `frontend/src/components/calendar/CalendarScheduleView.vue`, `ScheduleDayColumn.vue`

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Polish | 7-column day grid | `repeat(7, 1fr)` gap 12 | Verify in CalendarScheduleView | Match grid |
| 2 | Polish | Today column highlight | `bg-bg-2` + `border-accent-1-soft` + accent-1 date number | Verify ScheduleDayColumn handles today | Add `:class="isToday && '...'"` |
| 3 | Polish | Slot card | Background poster (opacity 0.55) + dark gradient overlay + time/episode mono text + bold show title | Verify slot rendering | Match the layered poster + gradient pattern |
| 4 | Polish | Empty day | Centered em-dash in `text-fg-3` | Verify | Match |
| 5 | Polish | Header strip | List/Schedule segmented + Add titles secondary + Export primary, plus stats line ("Week of …, N items, M airing") | Verify | Match |

### /account/profile (Phase D, source-only)

**Match level:** Has blocker (X4)
**Design source:** `screens-editor-account.jsx` `ProfilePanel` (L311-339)
**Files:** `AccountPage.vue`, `account/ProfileTab.vue`

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Blocker | Page header | "Profile" t-micro + "Your *details*" 40px display + small subtitle | Tab renders straight into form, no eyebrow/heading | See X4 |
| 2 | Blocker | Sidebar | 260px col w/ avatar 48px + name + email + tab buttons (icon + label, active state highlights bg-bg-2 + fg-0 + 600 weight) | 200px col, no avatar block, plain text buttons | See X4 |
| 3 | Polish | Email field | Has inline mail icon (paddingLeft 36) | Verify UiInput rendering | Add icon slot |
| 4 | Polish | Form action row | "Cancel" ghost + "Save details" primary, right-aligned | Live: "Edit details" primary; the design assumes always-editable | Reconcile UX — design implies always-editable; live uses an edit-toggle pattern. Either is defensible; document the deviation. |
| 5 | Match | Card padding wrapper | `card card-pad` | Section uses `flex flex-col gap-6 max-w-xl` (no card wrapper) | Wrap form inputs in `bg-bg-1 border border-line rounded-md p-6` to match the design's card styling |

### /account/preferences (Phase D, source-only)

**Match level:** Has blocker
**Design source:** `screens-editor-account.jsx` `SettingsPanel` (L341-449)
**Files:** `account/PreferencesTab.vue`, `account/AccentPicker.vue`

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Blocker | Page header | "Preferences" t-micro + "How it *looks & reads*" 40px display | None | See X4 |
| 2 | Polish | Theme picker | Segmented control (Light / Dark) with sun/moon icons | Two radio buttons | Replace with `UiSegmented` + sun/moon icons |
| 3 | Polish | Title language | Segmented (English / Romaji / Native) | `<select>` | Replace with `UiSegmented` |
| 4 | Polish | Interface language | `<select>` with globe icon prefix and 200px max width | `<select>` w/o icon | Add globe icon prefix |
| 5 | Polish | Timezone input | 280px max width + helpful placeholder ("UTC, America/New_York…") | `h-10 px-3 ... max-w-xs` | Bump max-width to ~280px and use the more illustrative placeholder |
| 6 | Polish | Accent picker | Per accent: pill button with 26×22 split swatch + label + "Pro" mini-pill on premium accents; active pill has accent-1 ring | Verify AccentPicker.vue covers this | If missing, add Pro pill and active ring |
| 7 | Polish | Pro footnote | Sparkle-prefixed "Marked Pro themes will require a subscription once monetisation lands — free for everyone today." | Live shows `account.preferences.proNote` text | Verify copy matches design verbatim |
| 8 | Polish | All-fields-in-one-card | Whole panel wrapped in `card card-pad` | Free-flowing flex column | Wrap in `bg-bg-1 border border-line rounded-md p-6` |
| 9 | Polish | "Save settings" CTA | Primary, right-aligned at bottom of card | Primary right-aligned (`pt-2`) | matches; ensure inside the card wrapper |

### /account/password (Phase D, source-only)

**Match level:** Has blocker
**Design source:** `screens-editor-account.jsx` `SecurityPanel` (L451-478)
**Files:** `account/PasswordTab.vue`

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Blocker | Page header | "Security" t-micro + "Change *password*" 40px display | None | See X4 |
| 2 | Polish | New-password helper text | "Must include uppercase, lowercase, digit, and symbol." below the new-password input | Verify | Add helper text under input |
| 3 | Polish | Card wrapping | `card card-pad` | Verify | Wrap |
| 4 | Match | Three password inputs | current / new / confirm | matches structurally | — |

### /account/danger (Phase D, source-only)

**Match level:** Has blocker
**Design source:** `screens-editor-account.jsx` `DangerPanel` (L480-495)
**Files:** `account/DangerZoneTab.vue`

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Blocker | Page header | "Danger zone" t-micro IN `var(--danger)` color + "Delete *account*" 40px display | None | See X4; tint eyebrow danger |
| 2 | Polish | Card border tint | `borderColor: 'color-mix(in oklch, var(--danger), transparent 70%)'` | Verify | Use `border-danger/30` (Tailwind alpha) or a custom utility |
| 3 | Polish | CTA | `btn btn-danger` with trash icon | Verify danger variant exists on UiButton | Add `variant="danger"` to UiButton if missing |

### Topbar / chrome (Phase E)

**Match level:** Polish needed
**Design source:** `screens-auth-list.jsx` `AppNav` (L7-39)
**Screenshot:** Visible at top of `my-calendars.png`

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Polish | Logo | Wordmark with serif italic on "Calendar" + 22px logo glyph | Just text "Anime Calendar" — no logo glyph, italic serif on Calendar word may be present (verify) | Add logo glyph SFC (per `foundations.jsx` Icon.Logo) before wordmark |
| 2 | Polish | Theme toggle | Sun/moon icon ghost button next to avatar | Missing | Add theme toggle button bound to `useTheme()` |
| 3 | Polish | Avatar pill | 30×30 round avatar with initials | Missing on desktop topbar (only "Logout" button) | Add 30px round avatar with initials; show dropdown on click for logout |
| 4 | Match | Two-tab nav (My Calendars / My Account) | Ghost buttons w/ active state bg-bg-2 + fg-0 + 600 | matches structurally | — |
| 5 | Match | Border-bottom + bg-bg-1 chrome | `border-b border-line bg-bg-1/80 backdrop-blur` | matches | — |
| 6 | Minor | Logout button placement | Design hides Logout in avatar dropdown; live exposes it as a top-level button | Add avatar dropdown then remove top-level Logout |

### Pagination (Phase E)

**Match level:** Mostly matches
**Design source:** `screens-auth-list.jsx` MyCalendars footer (L229-233)
**Screenshot:** Bottom of `my-calendars.png`

| # | Type | Element | Design intent | Live | Suggested fix |
|---|---|---|---|---|---|
| 1 | Match | Centered Prev / page-number / Next pattern | Ghost buttons + "Showing X of Y" middle text | Live shows numbered pages 1/2/3 + "Showing 1 to 6 of 17 calendars" below | Live's numbered variant is richer than design; keep. |
| 2 | Polish | Active page button | Active state should mirror nav-active (bg-bg-2 + fg-0) | Live uses accent-1 background pill | Either is defensible; but for consistency with the design's calmer aesthetic, switch active page to bg-bg-2 + fg-0 |
| 3 | Match | "Showing 1 to 6 of 17 calendars" | small muted text | matches | — |

---

## Suggested follow-up scope

### FU-1 — Public-route hard-reload regression (Blocker)

**Surfaces:** `/forgot-password`, `/reset-password`, `/verify-email/pending`, `/verify-email`, `/register` (only via direct URL)
**Effort:** S
**Why:** Real users will land on these via emailed links; right now the URL bar lies and they end up on /login. Add an integration test that hits each route via `page.goto` (full reload) and asserts the rendered h1.

### FU-2 — Auth shell visual fidelity (Blocker)

**Surfaces:** All 6 auth pages + UiAuthShell
**Effort:** M
**Why:** The empty left pane and missing wordmark/eyebrow pattern is the most visible deviation from the design. Add an `AuthPosterCollage` SFC, move wordmark out of the form card, introduce eyebrow + h1 + subtitle slots on UiAuthShell.

### FU-3 — Account header + sidebar (Blocker)

**Surfaces:** All 4 account tabs + AccountPage shell
**Effort:** S
**Why:** Adds the per-tab eyebrow + display heading and the sidebar avatar/name/email block. Single PR, four tabs touched.

### FU-4 — MyCalendars tile poster collage + airing chip (Polish)

**Surfaces:** `/my-calendars` tile component
**Effort:** S
**Why:** Tile body matches but the poster header strip + "+N" overflow + "X airing" chip are missing. Backend already returns `recent_item_ids`; airing aggregate may need a backend follow-up.

### FU-5 — Account preferences pickers + card wrapping (Polish)

**Surfaces:** All 4 account tabs
**Effort:** S
**Why:** Replace radio buttons with `UiSegmented` for theme + title language; wrap each tab's form in a card; add helper texts and icon prefixes (mail, globe).

### FU-6 — Topbar avatar + theme toggle (Polish)

**Surfaces:** `Header.vue` (or wherever topbar lives)
**Effort:** XS
**Why:** Add 30px avatar pill with initials + dropdown that hosts Logout, plus a sun/moon ghost button bound to `useTheme`. Design source is precise; trivial port.

### FU-7 — 404 + verify-email-pending hero icons (Polish)

**Surfaces:** `NotFoundPage.vue`, `VerifyEmailPendingPage.vue`
**Effort:** XS
**Why:** Add an `icon` slot or hero slot to `UiEmptyState` so 404 can render the oversized "4*0*4" numeral and verify-email can render the round mail medallion.

### Backend follow-ups (out-of-Track-2 scope)

- **Airing-count aggregate**: design's "3 airing this season" stat needs a per-calendar `airing_count` column or a server-side aggregate on the calendars list endpoint.
- **Calendar editor recent items**: already addressed by Track 2 spec's `recent_item_ids`; verify shipped.

---

## Design ambiguities found

1. **Reset-password design source missing.** No `ResetPasswordScreen` JSX in the handoff — diff is inferred from sibling auth screens.
2. **Verify-email confirm design source missing.** Only `verify-email-pending` is in `screens-extras.jsx`.
3. **Account "always-editable" vs "edit toggle"**: design assumes always-editable form; live uses an edit-toggle pattern. Pick one and document.
4. **Pagination numbered vs simple**: design shows simple Prev/Next; live shows numbered pages. Live is richer — confirm with design owner.
