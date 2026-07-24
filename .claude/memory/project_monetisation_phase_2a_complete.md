---
name: Phase 2a (backend) of monetisation refinement complete (2026-05-04)
description: Tier-aware VALARM emission, event_style branching, settings/event_style validation all shipped. Phase 2b (frontend) and Phase 3 remain pending. Supersedes the Phase 1 status memory's "what's next" note.
type: project
originSessionId: 75332392-b9dc-4e06-b274-05f71e4842d6
---
**State as of 2026-05-04 evening:** Phase 2a (backend slice) is on `main` in commits `5f48997..f6a21d5`. Phase 1 was completed earlier the same day.

**What's now on main (cumulative through Phase 2a):**

- All Phase 0 + Phase 1 plumbing (see `project_monetisation_phase_1_complete.md`).
- `IcsExportService::resolve_valarm_offsets(user_id)` — tier-aware. Free → hardcoded `[30]` (load-bearing anti-bypass; never consults settings). Pro → `settings.reminder_offsets_minutes` deduped+sorted+truncated to 5.
- `render_common_calendar(&CommonCalendar, event_style: &str, valarm_offsets: &[i32])` — emits one VALARM per offset (all in `-PT<seconds>S` form per `feedback_icalendar_duration_seconds_format`), and switches DTSTART format on `event_style == "timed"` vs `"all_day"`.
- `MAX_VALARMS_PER_EVENT = 5` enforced in TWO layers: validation rejects writes >5 in `update_user_settings`, and the renderer truncates defensively for any older rows that slipped through pre-validation.
- `validate_reminder_offsets` in `controllers::user.rs` — tier-agnostic structural validation. A Free user PUTting `[60, 1440]` succeeds (200) — values are stored for tier-transition continuity, ignored at render time.
- `event_style: String` field on `CalendarRequest` with default `"timed"`, validated against `&["timed", "all_day"]` before any DB write.

**Phase 2b (frontend, pending) entry points:**
- `frontend/src/components/account/PreferencesTab.vue` — add a "Reminders" section with 10 canonical chips (`[15, 30, 60, 120, 360, 720, 1440, 2880, 4320, 10080]` minutes), 5-cap, pro-lock overlay for Free users, info icon.
- `frontend/src/components/calendar/CalendarSettingsForm.vue` — add `UiSegmented` toggle for `event_style` with `timed`/`all_day` options + a fallback note.
- `frontend/src/stores/userSettingsStore.ts` — add `reminderOffsetsMinutes` field + `updateReminderOffsets(offsets)` action.
- Locales: `userSettings.reminders.*` and `calendar.settings.eventStyle.*` keys in **both** `en.json` and `pt.json`. The plan's spec (line 1275-1310) has full copy.

**Phase 3 (pending) entry points:**
- `frontend/src/components/UpgradePage.vue` rewrite — drop AniList sync / sharing / etc. claims; advertise only what's built.
- New pricing: $2.99/mo, $24.99/yr. Stripe price IDs in `config.toml` need updating (companion checklist at `docs/checklists/2026-05-monetisation-stripe-price-update.md`).
- Locale cleanup — sweep dead marketing strings across `en.json` and `pt.json`.

**Known surprises:**
- `feedback_icalendar_duration_seconds_format` — VALARM TRIGGER output is `-PT<seconds>S`, not `-PT30M`/`-P1D`.
- `feedback_audit_lock_in_tx_consistency` — when extending PUT /calendar's advisory-lock arm in future phases, every operation inside the lock MUST use the locked connection.
- `feedback_no_sqlx_test_use_test_pool` — never `#[sqlx::test]`; use `test_pool()` with random seeds + cleanup.
- `feedback_stale_lsp_after_rust_structural_changes` — fired 5 times this session. Trust `cargo check` over IDE.

**Test count baseline at end of session:** 240 server lib + 3 anilist + 1 metrics integration + 392 frontend unit. Use this as the diff target on Phase 2b — frontend should grow by ~10-15 tests, server unchanged.
