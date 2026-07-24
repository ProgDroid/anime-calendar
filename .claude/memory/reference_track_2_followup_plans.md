---
name: Track 2 follow-up plans (4 deferred items)
description: Paths to the four plan files filed during Track 2 wrap-up that are pending implementation
type: reference
originSessionId: 7a8d378a-90cb-41a0-b043-addec4ba2a44
---
Track 2 shipped 2026-05-01. Four plan files were filed for deferred work — pick these up in later sessions when prioritized.

## Pending plans

1. **`docs/superpowers/plans/2026-05-01-track-2-design-diff.md`** — original sweep of UI vs design fidelity. Most findings already addressed via FU-1 through FU-7. AccentPicker checkmark + schedule view week label landed via the a11y remediation. Remaining: calendar editor sub-header layout. Reference doc — most action lives in the other plan files now.

2. **`docs/superpowers/plans/2026-05-01-track-2-a11y-authenticated-sweep.md`** — pending verification: run live axe-core on the authenticated routes (`/my-calendars`, `/calendar/:id`, `/calendar/:id/schedule`, `/account/*`) once the backend is up. Public surfaces (`/login` etc.) were verified live and clean on 2026-05-01.

## Done

3. ~~**`docs/superpowers/plans/2026-05-01-track-2-a11y-audit.md`**~~ — **Done 2026-05-01** — addressed via the remediation merge.

4. ~~**`docs/superpowers/plans/2026-05-01-track-2-a11y-remediation.md`**~~ — **Done 2026-05-01** (merge `504516a` on main, 3 squashed commits). All 6 batches landed. Live axe sweep on `/login` shows zero color-contrast / link-in-text-block violations across 5 accents × 2 themes.

5. ~~**`docs/superpowers/plans/2026-05-01-airing-count-aggregate.md`**~~ — **Done 2026-05-01** (commits eefd5a5 + 02041f6). Path B chosen: derived `airing_count` from existing `Item.airing_schedule` (any future-dated entry counts) — no `MediaStatus` plumbing or cache version bump needed.

## Picking up

- The authenticated-sweep plan is a 30-min verification once backend is running; very likely clean since same primitives apply.
- For the remaining design-diff polish (calendar editor sub-header), file as a small standalone task.

## Status tracking

The TaskCreate tasks #25–#33 (FU-1 through FU-7 + form polish + a11y audit) all completed during the 2026-05-01 session. New work should be filed under fresh task IDs.
