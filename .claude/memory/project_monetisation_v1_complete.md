---
name: Monetisation refinement v1 complete (2026-05-05)
description: Phase 2b + Phase 3 shipped — all 4 phases of the monetisation refinement plan are done. 406 frontend / 240 server tests green.
type: project
originSessionId: be1596ff-ca01-4f16-80bf-1a49c8d8e222
---
Phase 2b (frontend) and Phase 3 of `docs/superpowers/plans/2026-05-04-monetisation-refinement.md` shipped 2026-05-05. All four phases (0/1/2a/2b/3) are now complete.

**What landed in this session (Phase 2b + 3):**

Phase 2b — frontend:
- Reminders chip-list in `account/PreferencesTab.vue` — 10 canonical offsets, 5-cap visible-from-start, Pro-locked overlay with `/upgrade` CTA for Free users, info-icon explaining the cap.
- `event_style` segmented toggle (timed/all_day) in `calendar/CalendarSettingsForm.vue` with fallback note copy.
- Wired `event_style` through `CalendarEditorViewDesktop` + `CalendarEditorViewMobile` (load on edit, persist in sessionStorage, include in PUT payload).
- Added `IconInfo` SFC + `CANONICAL_REMINDER_OFFSETS` const + `EventStyle` type + `reminder_offsets_minutes` field on `UserSettings`.
- Locale keys (en + pt): `userSettings.reminders.*` and `calendar.settings.eventStyle.*`.
- Tests: 4 event_style toggle tests on `CalendarSettingsForm.spec.ts`, 7 reminder UX tests on new `PreferencesTab.spec.ts`.

Phase 3 — pricing reset + UpgradePage rewrite:
- `UpgradePage.vue` PRO_FEATURE_KEYS now lists 6 v1-enforced features (unlimited cals, unlimited shows, live subscribe URL, customisable reminders, all accents, early access). FREE_FEATURES expanded to 8 rows with cap callouts.
- `data-testid="pro-feature"` added to each Pro `<li>`.
- Locales: `pricing.price.{monthly,annual,savings}` reset to `$2.99 / $24.99 / Save ~30%`. Retired keys removed (`priorityRefresh`, `exportFlexibility`, `support`, `proAccents`, `noPrioritySupport`). New keys added (`unlimitedShows`, `liveSubscribeUrl`, `customisableReminders`, `allAccents`, `earlyAccess`, `limitedCalendars`, `limitedShows`, `noLiveSubscribe`, `noCustomReminders`).
- Tests: 3 new UpgradePage.spec.ts tests (price strings, 6 features, no-unbuilt-features grep).
- `config.toml.dist` annotated with $2.99/$24.99 context — actual price IDs are operator work via `docs/checklists/2026-05-monetisation-stripe-price-update.md`.

**Test baseline at end of session:**
- 240 server lib + 3 anilist + 1 metrics integration
- 406 frontend unit (was 392 — +14 for Phase 2b/3)

**Why:** Plan called for completing Phase 2b + Phase 3 to close out spec 1. All work was independent of backend (Phase 2a already shipped the .ics emission + validation that the frontend now drives).

**How to apply:** When picking up follow-up specs (Spec 2 co-editor sharing, Spec 3 AniList OAuth, Spec 4 MAL OAuth), the v1 monetisation infrastructure is the gate they must respect — pro accents already live, cap enforcement live, frozen-blob subscribe URL live, tier-aware VALARM live. New Pro features added in those specs should append to `PRO_FEATURE_KEYS` only after they ship (no pre-advertising).

**Gotcha discovered:** `vi.mock('vue-router', ...)` in a spec file leaks across spec files in the same vitest worker, breaking sibling specs that use `createRouter` from real vue-router. Use `createMemoryHistory` + override `router.push` directly, never `vi.mock('vue-router', ...)`. Adds to memory `feedback_test_history_pollution`.
