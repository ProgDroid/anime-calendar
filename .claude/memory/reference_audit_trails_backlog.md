---
name: Audit trails — backlog item
description: Look deeper into a unified audit trail subsystem at a later date; surfaced during 2026-05-05 Spec 2 (co-editor sharing) brainstorm
type: reference
originSessionId: be1596ff-ca01-4f16-80bf-1a49c8d8e222
---
User flagged audit trails as a topic to explore in its own spec at a later date. Surfaced during the Spec 2 (co-editor sharing) brainstorm (2026-05-05) where invitation lifecycle ("I invited X on date Y, they accepted/declined on Z, I revoked them on W") is the obvious first audit-trail use case.

**Why:** Co-editor sharing introduces multi-actor mutations. When two people can edit the same calendar, "who added this show?" and "who removed it?" become real questions. A scoped audit log gives the user (and customer support) a way to answer them without running ad-hoc DB queries.

**How to apply when the time comes:**
- Don't bake an ad-hoc invitation log into Spec 2's `calendar_invitations` table beyond minimum lifecycle states. Treat it as a feature for the eventual unified audit subsystem to consume.
- Likely candidates for the audit log when designed: invitations (created, accepted, declined, revoked, expired), calendar mutations (item add/remove/clear, rename, delete), subscription transitions (already covered by Stripe events, but app-side mirror would help support), security events (password resets, OAuth link/unlink, refresh-token rotation).
- Storage shape worth considering: append-only `audit_events` table with `actor_user_id`, `subject_type`, `subject_id`, `action`, `metadata jsonb`, `occurred_at`. Don't promise immutability beyond a soft "no UPDATE" convention; durable immutability needs separate infra.
- Privacy: GDPR right-to-erasure may force redaction (not deletion) of audit rows. Plan for that by separating PII from event metadata.

Not in scope for Spec 2 — that spec captures the minimum invitation lifecycle it needs. Promote to its own brainstorm/spec when prioritised.
