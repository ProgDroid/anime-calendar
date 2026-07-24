---
name: icalendar 0.17 emits VALARM TRIGGER durations in seconds, not minutes/hours/days
description: `chrono::Duration::minutes(30)` rendered through icalendar 0.17 produces `-PT1800S`, NOT the more natural `-PT30M`. Test assertions and external consumers must match this format.
type: feedback
originSessionId: 75332392-b9dc-4e06-b274-05f71e4842d6
---
icalendar 0.17 (the version pinned in this codebase) renders VALARM `TRIGGER` values via the underlying `chrono::Duration` in **seconds form**, regardless of whether the input was constructed with `Duration::minutes(...)`, `Duration::hours(...)`, or `Duration::days(...)`. The output:

| Input | RFC-5545 minimal form | What icalendar 0.17 emits |
|---|---|---|
| `Duration::minutes(30)` | `-PT30M` | `-PT1800S` |
| `Duration::minutes(60)` | `-PT1H` | `-PT3600S` |
| `Duration::minutes(1440)` | `-P1D` | `-PT86400S` |
| `Duration::minutes(10080)` | `-P1W` | `-PT604800S` |

This is RFC-5545-valid (seconds notation is allowed) and Apple Calendar / Google Calendar / Outlook all interpret it correctly. The plan stub for Phase 2 expected `-PT30M` / `-P1D` style output; the actual output is in seconds.

**Why:** During Phase 2a (2026-05-04), the implementer's tests asserted on the seconds form because that's what the renderer actually emits. The plan's docstring said `TRIGGER:-PT30M`, `TRIGGER:-PT1H`, `TRIGGER:-P1D`. Aligning tests to plan would have made them red.

**How to apply:**
- Test assertions on VALARM blobs must match `-PT<seconds>S` form, not the mixed-unit forms in RFC-5545 examples.
- If we add an external consumer (e.g., a third-party calendar plugin) that string-matches on `PT30M`, that will break. Convert the duration to minimal form server-side first, or upgrade icalendar to a version that emits minimal form.
- If a future plan stub references the mixed-unit form, override it with the seconds form in the implementer brief.
- A clean fix would be a helper that converts `Duration` to minimal-form `String` and uses `Alarm` directly with the formatted trigger — but until a real consumer needs it, the seconds form is fine.
