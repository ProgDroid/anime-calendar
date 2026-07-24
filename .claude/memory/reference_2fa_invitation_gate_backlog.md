---
name: 2FA gate for invitation actions — backlog item
description: When 2FA ships as its own auth-hardening spec, layer it onto co-editor invitation send + accept as an additional gate. Surfaced 2026-05-05 during Spec 2 brainstorm.
type: reference
originSessionId: be1596ff-ca01-4f16-80bf-1a49c8d8e222
---
User flagged 2FA as a future hardening that should layer onto co-editor sharing (Spec 2) once 2FA itself is built.

**Why:** Invitation send (owner action) and invitation accept (invitee action) are both account-level mutations that grant cross-account write access. Stolen-credential attackers can use either side to extract data or pivot. Standard password+session is the floor; 2FA confirmation is the natural ceiling.

**How to apply when the time comes:**
- Don't bake 2FA into Spec 2's invitation flow. Spec 2 ships with email + password auth as the only gate (consistent with the rest of the app).
- When 2FA lands as its own spec, layer it onto two existing endpoints (no schema change needed in `calendar_invitations`):
  - `POST /calendars/:id/invitations` — owner sending an invite. Require fresh 2FA challenge (≤ 5 min since last confirmation) if the owner has 2FA enabled.
  - `POST /invitations/:token/accept` — invitee accepting. Same fresh-challenge requirement if invitee has 2FA enabled.
- Decline / revoke / leave-calendar don't need 2FA — they reduce access, not grant it.
- Preview (`GET /invitations/:token`) doesn't need 2FA — read-only of public-ish data.
- Implementation hint: add a `requires_2fa_confirmation: bool` flag to the per-route auth extractor based on user's 2FA status; reject with `403 {"error":"two_factor_required"}` on stale challenges. Frontend prompts and retries.

Not in scope for Spec 2 — captured here so it doesn't get lost when 2FA work begins.
