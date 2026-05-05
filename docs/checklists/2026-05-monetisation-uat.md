# Monetisation Refinement — Phase 2/3 Manual UAT

**Date created:** 2026-05-05
**Owner:** _assign on first run_
**Source spec:** [`docs/superpowers/specs/2026-05-04-monetisation-refinement-design.md`](../superpowers/specs/2026-05-04-monetisation-refinement-design.md)
**Companion to:** [`2026-05-monetisation-stripe-price-update.md`](./2026-05-monetisation-stripe-price-update.md), [`2026-05-track-4-release-readiness.md`](./2026-05-track-4-release-readiness.md)

This checklist covers user-visible surfaces that the automated test suite cannot fully exercise: cap-reach modals, the frozen-blob subscribe URL across tier transitions, reminder customisation rendering in real calendar apps, and the per-calendar event-style toggle. Run after the spec's automated gates pass and before the public release.

---

## 0. Prerequisites

- [ ] All four phases of spec 1 are merged and deployed to a staging-like env (or local dev with PostgreSQL + Redis up).
- [ ] `cargo test --workspace` and `cd frontend && npm run test:unit` both green on the deployed commit.
- [ ] Stripe webhook listener forwarding to your dev server (`stripe listen --forward-to localhost:8080/api/stripe/webhook`).
- [ ] At least two test user accounts available — one Free, one capable of going Pro via Stripe test card.
- [ ] A real calendar app on hand for the subscribe-URL tests: Google Calendar (web), Apple Calendar (macOS or iOS), or Outlook. Multiple is better.

---

## 1. Cap reach — calendars (Free tier)

- [ ] Sign in as the Free user. Hit `/my-calendars`.
- [ ] Confirm the calendar counter chip reads `0 / 3 calendars` (or whatever the existing count is).
- [ ] Create three calendars in succession. After each, the counter increments.
- [ ] At `3 / 3`, the **New calendar** button is **disabled** in the header.
- [ ] Click the disabled button. The **upgrade modal** opens with the `cap_calendars` heading + description copy.
- [ ] Click **Maybe later** — modal closes, no nav. Click **See plans** — lands on `/upgrade`.
- [ ] Delete one calendar; counter drops to `2 / 3`; **New calendar** button is enabled again.

## 2. Cap reach — shows (Free tier)

- [ ] Open one of the Free user's calendars in the editor.
- [ ] Confirm the **show counter chip** reads in the editor header (e.g. `0 / 25 shows` initially).
- [ ] Add 19 distinct shows (across one or more calendars). At 19, no banner.
- [ ] Add the 20th. The **80% warning banner** appears: *"Approaching your tracking limit (20 of 25). Upgrade to Pro for unlimited."*
- [ ] Dismiss the banner (`×`); confirm it stays dismissed for the session.
- [ ] Reach 25/25. Try to add a brand-new show via search. The add button is **disabled**.
- [ ] Click the disabled add button. Upgrade modal opens with `cap_shows` reason copy.
- [ ] In a *second* calendar, add a show that's *already tracked* in another calendar — **succeeds** (idempotent at-cap behaviour).
- [ ] Add a brand-new show — **fails** with 402 + modal.

## 3. Subscribe URL — tier transitions

This is the headline behaviour change in Phase 1. Verify with a real calendar consumer, not just curl.

- [ ] As the Pro user, create a calendar with at least one show that has a known airing schedule (e.g. a currently-airing seasonal anime).
- [ ] Copy the calendar's subscribe URL (`/calendar/<token>` or whatever the public route is).
- [ ] In Google Calendar (or your calendar of choice), **Add by URL**. Confirm events appear within a few minutes (Google polls hourly).
- [ ] In a separate session/incognito, hit the subscribe URL via `curl`. Body is a `text/calendar` response. Note an episode datetime — should be the **live** AniList airing time.
- [ ] Trigger a downgrade via the CLI (`cargo run --bin set_subscription -- --user-id <N> --tier free`) **or** by cancelling in the Customer Portal and waiting for the period to end. Wait for the webhook to fire.
- [ ] Hit the subscribe URL again. Body is now the **frozen blob** — same content as before (the snapshot taken at downgrade time).
- [ ] Edit the calendar (add a new show) as the Free user. Hit the subscribe URL. The frozen blob has been **regenerated** to include the new show (per Phase 1 PUT-side regen on Free).
- [ ] Re-upgrade via Stripe test card. Hit the subscribe URL. Body is **live** again (`frozen_subscribe_ics` should be NULL in the DB).
- [ ] Capture the subscribe token at each step — it must be **identical** across the Pro→Free→Pro round trip.
- [ ] Brand-new Free user (never been Pro): query their `subscribe_token` directly from the DB, hit the URL → **404** (not part of Free tier).

## 4. Reminders — Pro customisation

- [ ] As Pro, hit `/account/preferences`. Locate the **Reminders** section under Accent.
- [ ] Confirm the chip count reads `1 of 5 active` (assuming the default 30-min is your only stored offset).
- [ ] Click an additional chip — `1 hour`. Counter reads `2 of 5`. The chip shows the active state.
- [ ] Add three more chips (e.g. 2h, 6h, 1d). At `5 of 5`, the remaining inactive chips are **disabled** (greyed out, no hover affordance).
- [ ] Click a disabled chip — nothing happens. Counter unchanged.
- [ ] Click an *active* chip to deactivate. Counter drops to `4 of 5`. Inactive chips re-enabled.
- [ ] Hover the info icon — tooltip explains the cap.
- [ ] Click **Save Settings**. Toast: success.
- [ ] Download or open the .ics for one of your calendars. **One `BEGIN:VALARM` block per active offset** with `TRIGGER:-PTNS` (icalendar 0.17 emits seconds, see memory `feedback_icalendar_duration_seconds_format`). All five reminders fire at the right times in the test calendar app.

## 5. Reminders — Free downgrade preserves stored values

- [ ] Continuation of §4 (Pro user with 5 stored offsets).
- [ ] Downgrade via CLI or Stripe. Webhook arrives.
- [ ] Hit `/account/preferences`. The Reminders section is **Pro-locked** (overlay with upgrade CTA visible).
- [ ] Open the .ics for one of the user's calendars (now Free). Confirm **exactly one** `VALARM` per event with `TRIGGER:-PT1800S` (30 min — the Free single-reminder enforcement).
- [ ] In `psql`, query `SELECT reminder_offsets_minutes FROM user_settings WHERE user_id = <N>;`. The stored array still contains the original 5 offsets (preservation across downgrade).
- [ ] Re-upgrade. Hit Preferences. The 5 chips are **active again** without any user action — server-stored values resume driving the .ics output.

## 6. Event style — per-calendar toggle

- [ ] Pick a calendar with a mix of items: some currently-airing (known `airing_at`) and some completed/upcoming with no air time.
- [ ] In the editor, locate the **Event style** segmented toggle. Default is `Timed`.
- [ ] With **Timed** selected, save. Open the .ics:
  - Items with known `airing_at` use `DTSTART:` followed by a UTC datetime.
  - Items without `airing_at` fall back to `DTSTART;VALUE=DATE:` (the "all-day even in Timed mode" fallback).
  - Confirm the `Episodes without a known air time will appear as all-day events even in Timed mode` note is visible in the form.
- [ ] Switch to **Daily** (all_day). Save. Open the .ics:
  - **Every** event uses `DTSTART;VALUE=DATE:` regardless of whether `airing_at` is known.
- [ ] In a calendar consumer, confirm the visual difference: Timed mode shows hourly slots; Daily shows full-day events at the top of the day.

## 7. UpgradePage rewrite

- [ ] As any user, hit `/upgrade`.
- [ ] Confirm the prices read **$2.99** (monthly) and **$24.99** (annual). Annual interval shows the **Save ~30%** chip.
- [ ] The Pro tier lists exactly **6** features:
  1. Unlimited calendars
  2. Unlimited tracked shows
  3. Live subscribe URL with hourly refresh
  4. Customisable reminders (up to 5)
  5. All accent themes — Coral, Iris, Matcha, Sakura, Citron
  6. Early access to new features
- [ ] Free tier lists 8 rows mixing ✓ caps and ✗ Pro-only items.
- [ ] No mention of: AniList sync, MyAnimeList, shared editors, "Studio", priority support, multiple export profiles, or priority refresh — those are retired or unbuilt.
- [ ] Switch language to Portuguese; same shape, translated copy.
- [ ] Click **Start free trial** → Stripe Checkout opens with the new test prices. (Defer to checklist §2 of the price-update doc.)

## 8. 402 reason routing — sanity

- [ ] Trigger a `cap_calendars` 402 (from §1). Modal heading + body match `interrupt.heading.cap_calendars` + `.description.cap_calendars` keys.
- [ ] Trigger a `cap_shows` 402 (from §2). Modal heading + body match `cap_shows` keys.
- [ ] Trigger a `pro_accent` 402 (try to apply Matcha/Sakura/Citron as Free). Modal heading + body match `pro_accent` keys.
- [ ] Translate UI to Portuguese. All three modals retranslate.

---

## Sign-off

- [ ] All sections above completed (or explicitly skipped with a reason).
- [ ] Any defects logged in the issue tracker, not in this checklist.
- [ ] No regression observed in Track 4 surfaces (cancel, manage, past_due banner) — re-run the relevant section of the Track 4 readiness checklist if uncertain.

**Run by:** ____________
**Date:** ____________
**Notes:** ____________
