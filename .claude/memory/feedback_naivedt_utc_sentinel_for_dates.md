---
name: Append 'Z' to NaiveDateTime strings before constructing Date() in the frontend
description: Backend serialises NaiveDateTime without timezone — frontend appending 'Z' anchors to UTC so cross-timezone users see a consistent rendered date
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
**Rule:** When the frontend renders a `NaiveDateTime` value from the backend (the codebase's chosen sqlx type — see `feedback_sqlx_timestamp_types.md`), append `'Z'` before constructing `new Date()`:

```ts
const d = new Date(`${iso}Z`)
return d.toLocaleDateString(locale.value, { year: 'numeric', month: 'long', day: 'numeric' })
```

**Why:** `NaiveDateTime` serialises as `"2030-01-15T00:00:00"` — no timezone marker. JavaScript's `Date` constructor treats unmarked strings as *local* time, so a backend value of "2030-01-15T00:00:00" reads as midnight Pacific for a user in Los Angeles but midnight Tokyo for a user in Japan. For billing dates and renewal dates this means two users with the *same* subscription see different "Renews on" copy.

The backend's underlying source is always UTC (`naive_from_timestamp(stripe_ts)` does `DateTime::from_timestamp(ts, 0).naive_utc()`). Appending `'Z'` tells the JS Date constructor "this is UTC" so `toLocaleDateString` formats it correctly in the user's locale. This matches what Stripe itself does in its renewal-date emails.

**How to apply:**
- Use this pattern any time you render a `current_period_end`, `trial_end`, `created_at`, etc. coming from the backend.
- The `Z` suffix is safe even if the value is already ISO with offset — the Date constructor accepts both shapes.
- Don't try to convert via `new Date(iso).toISOString()` first; that already silently treats the input as local.
- If you ever need the user's *local* date (e.g. "today's calendar entries"), the backend should send a TZ-marked timestamp instead — never assume the timezone client-side.

**When NOT to use:** Calendar event start/end times that the user picked in their local zone — those are inherently local. The `'Z'` trick is for *server-anchored* moments (renewals, audit logs, billing).
