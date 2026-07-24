---
name: Phase 1 of monetisation refinement complete (2026-05-04)
description: Tier enforcement, frozen-blob subscribe path, advisory-lock cap protection, and 402 reason routing all shipped. Phase 2 (composable reminders + event style) and Phase 3 (pricing reset + UpgradePage rewrite) remain pending.
type: project
originSessionId: 75332392-b9dc-4e06-b274-05f71e4842d6
---
**State as of 2026-05-04:** Phase 1 of `docs/superpowers/plans/2026-05-04-monetisation-refinement.md` is fully on `main`.

**Why:** Caching/trait foundation (Phase 0) and tier-enforcement work (Phase 1) had to land before Phase 2 could layer composable reminders + per-calendar event style on a working renderer.

**How to apply:**
- **Don't re-do Phase 1 work.** Before drafting any new plan that touches: ICS rendering, calendar caps, the subscribe URL, the upgrade modal, frozen blobs, or the Stripe webhook tier transitions — read `git log --oneline a6a5173..HEAD` for the precise wire-up.
- **Phase 2 entry points (when starting):**
  - Renderer to extend: `server/src/services/ics_export.rs::render_common_calendar` (currently emits a hardcoded 30-min VALARM; needs per-tier per-user offset emission, capped at 5 for Pro, single-30 for Free).
  - Settings storage: `user_settings.reminder_offsets_minutes INTEGER[]` (already migrated in P1.1).
  - Calendar event-style storage: `calendars.event_style TEXT` with `'timed' | 'all_day'` CHECK (already migrated).
  - UI surfaces: `frontend/src/components/account/PreferencesTab.vue` (reminders chip-list with 5-cap UX); `frontend/src/components/calendar/CalendarSettingsForm.vue` (event_style toggle).

**Key plumbing now in place** (don't reinvent these):
- `Error::PaymentRequired { required_tier: &'static str, reason: Option<&'static str> }` is the only 402 path; reason codes used so far: `"cap_calendars"`, `"cap_shows"`, `"pro_accent"`.
- `EntitlementService::assert_can_create_calendar`, `assert_can_add_show`, `effective_tier` (and their `_in_tx` variants).
- `IcsExportService` (rendering) and `FrozenIcsService` (regenerate / regenerate_for_user / clear_for_user) — both DI-wired.
- Stripe webhook detects tier transitions inside the txn and dispatches `FrozenIcsService` actions **post-commit** via a `FrozenAction` enum — see `server/src/controllers/stripe_webhook.rs::dispatch_frozen_action`.
- `GET /api/account/usage` returns `{ shows: i64, calendars: i64 }`.
- Frontend: `useUpgradeInterrupt` composable + global `<UpgradeInterruptModal>` mount in `App.vue`; `usageStore` Pinia store; axios 402 interceptor in `frontend/src/config/api.ts`.

**Phase 2 + 3 plan sections** are still untouched in `docs/superpowers/plans/2026-05-04-monetisation-refinement.md` (sections at line 1000+ and 1339+).
