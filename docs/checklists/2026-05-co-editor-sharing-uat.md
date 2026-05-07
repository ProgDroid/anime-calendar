# Co-Editor Sharing — Manual UAT

**Date created:** 2026-05-07
**Owner:** _assign on first run_
**Source spec:** ../superpowers/specs/2026-05-05-co-editor-sharing-design.md

## 0. Prerequisites
- [ ] All five phases merged + deployed to staging.
- [ ] Two test accounts: `owner@test` (Pro) and `invitee@test` (Free).
- [ ] Real email inboxes for both (or Mailhog).

## 1. Invite happy path
- [ ] Pro owner opens calendar → Members tab → invites `invitee@test` → email arrives.
- [ ] Invitee opens link → `/invite/:token` landing page → preview card shown (calendar name, owner, item count, masked email).
- [ ] Invitee clicks Accept → lands on the calendar editor.
- [ ] Owner's tab sees `member.joined` toast + presence chip "You + 1".

## 2. Invite expiry
- [ ] Set `expires_at = NOW() - INTERVAL '1 day'` on a pending row in psql.
- [ ] Hit `GET /api/invitations/:token` → `400 invalid_invite_token`.
- [ ] Frontend shows the generic invalid page: "This invitation link is no longer valid."

## 3. Cap enforcement
- [ ] With 5 active+pending editors, attempt 6th invite → server returns 409 + frontend shows inline error: "This calendar already has 5 editors."

## 4. Anti-enumeration
```bash
for status in not_found expired accepted revoked; do
  curl -s "http://staging/api/invitations/SCENARIO_$status" -o /tmp/$status.json
done
# All four bodies must be byte-identical.
diff /tmp/{not_found,expired}.json && diff /tmp/{accepted,revoked}.json
```
Expected: no diff output (all return identical `{"error":"Invalid request data"}`).

## 5. Self-invite rejected
- [ ] Owner attempts to invite their own email → 400 with self_invite error.

## 6. Cross-instance fan-out (skip if single-instance staging)
- [ ] Open calendar editor in two browsers connected to different staging pods.
- [ ] Add an item in one browser → see it appear in the other within 5 s.

## 7. Downgrade → kick → upgrade → restore
- [ ] Editor connected to calendar editor.
- [ ] Trigger Stripe downgrade webhook (or use psql to flip `subscriptions.tier = 'free'` + call reconcile).
- [ ] Editor: kick toast appears + redirect to `/my-calendars`; calendar gone from "Shared with you".
- [ ] Re-upgrade owner (flip tier back + trigger webhook).
- [ ] Editor: receives "access restored" email; calendar reappears in "Shared with you".

## 8. Mobile editor
- [ ] In mobile viewport (≤ 1023px), presence chip is visible in the header.
- [ ] Members tab opens correctly in the settings drawer; invite modal renders and submits correctly.

## 9. Email rendering
- [ ] Real Gmail (light + dark theme) — calendar name renders without escaped HTML sequences.
- [ ] Real Outlook web — invitation link is clickable.

## 10. Token scrubbing in logs
```bash
gcloud logging read "resource.type=cloud_run_revision AND textPayload:invite" --limit 100 --format json \
  | jq -r '.[].textPayload' | grep -E '\?token=[^&]+' | wc -l
```
Expected: `0`.
