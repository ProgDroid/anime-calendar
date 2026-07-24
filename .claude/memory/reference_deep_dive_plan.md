---
name: codebase-audit-tracker
description: Authoritative audit findings + remediation status live in AUDIT.md at the repo root
metadata: 
  node_type: memory
  type: reference
  originSessionId: 7b78438c-a9c3-4505-b520-c441c9853c9c
---

**The audit tracker is `AUDIT.md` at the repo root** (`G:/rustDev/anime-calendar/AUDIT.md`) — NOT the old `~/.claude/plans/deep-wondering-liskov.md` path (that file does not exist; chasing it wastes tool calls).

`AUDIT.md` holds: the original 2026-05-07 parallel-subagent audit (C-*/H-*/M-*/L-* findings), the 2026-06-11 follow-up audit (F2-* findings on the originally-skipped surfaces), and a `## Step N status` / `## Phase N status` section appended each time a remediation batch ships. When the user says "continue with the audit tasks," read `AUDIT.md` first and look at the latest status section + the "Recommended sequence" to find what's next.

Convention: each remediation step ships **direct to `main`** (user-confirmed), one focused commit per finding-cluster, then a status section appended to `AUDIT.md`. Per-finding design/plan docs live under `docs/superpowers/specs/` and `docs/superpowers/plans/`. Newly-surfaced findings during remediation get an `F2-NN` id and a backlog note in the relevant status section.
