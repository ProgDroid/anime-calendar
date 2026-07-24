---
name: Reference: Next features task list
description: Standalone backlog extracted from the deep-dive audit plan; includes cache endpoint lockdown note for the admin dashboard task
type: reference
originSessionId: a3c52327-2f59-44c2-b68f-a63d69222034
---
Backlog file: `~/.claude/plans/anime-calendar-next-features.md`

Contains all next-feature items from the deep-dive audit plan (`deep-wondering-liskov.md`), plus:
- Outstanding audit item: httpOnly cookie migration (still ❌)
- Admin/cache dashboard note: cache endpoints (`/cache/*`) must be locked to owner-only access before any UI is built
