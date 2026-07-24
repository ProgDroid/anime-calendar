---
name: Error variant for authz failure — Forbidden vs Unauthorised vs NotFound
description: Project convention for picking the right Error variant when an authorization check fails. Forbidden (403) for "wrong role"; Unauthorised (401) for "not logged in"; NotFound (404) only when anti-enumeration matters.
type: feedback
originSessionId: c52b64cd-a699-4ba2-b082-946c535a03c2
---
Pick the Error variant that matches the *signal* you want to send the client, not the one that matches the closest existing handler.

| Situation | Variant | HTTP | Reasoning |
|---|---|---|---|
| Caller has no valid auth token | `Error::Unauthorised` | 401 | Frontend axios refresh interceptor uses 401 to trigger token refresh. |
| Caller is authenticated but lacks the role needed | `Error::Forbidden` | 403 | Don't lie about existence when the caller can legitimately see the resource (e.g. an editor who can read the calendar but can't rename it). |
| Caller has no business knowing the resource exists | `Error::NotFound` | 404 | Anti-enumeration. Used by `CalendarMapper::get_calendar_by_id` (filters on user_id and surfaces RowNotFound as NotFound). |
| Caller's tier doesn't allow the action | `Error::PaymentRequired { required_tier, reason }` | 402 | Frontend `UpgradeInterruptModal` keys off `reason` to show the right copy. |

**Why:** Co-editor sharing exposes the calendar to non-owners (editors). An editor PATCHing the calendar name should get a clear 403 ("wrong role") — `NotFound` would be wrong UX (the resource is right there in their list). `Unauthorised` would be wrong because they ARE authenticated. `Forbidden` was added to the enum in commit `ac7f12c` specifically for this distinction.

**How to apply:**
- Adding a new authz check on a resource the caller *can see legitimately* → `Error::Forbidden`.
- Adding a new authz check on a resource that *should be hidden* from non-owners (e.g. a private calendar the user has no relationship with) → `Error::NotFound`. Mirror the `WHERE user_id = $1 AND deleted_at IS NULL` filter pattern from existing mappers.
- `Error::Forbidden` is also used for `EmailNotVerified` (legacy convention) — both map to 403. New 403 cases prefer `Forbidden`.
- Adding a new variant: don't forget to add it to the `match` arm in `impl ResponseError for Error::status_code()`.
