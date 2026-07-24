---
name: project-followup-audit-2026-06-11
description: "2026-06-11 follow-up audit in AUDIT.md — steps 1+2 shipped; step 3 (shared editor composable, H-3, F2-10) is next"
metadata: 
  node_type: memory
  type: project
  originSessionId: 02001aee-2b27-44bb-90dc-b0292787dc7a
---

The 2026-06-11 follow-up audit is recorded in `AUDIT.md` (section "Follow-up audit — 2026-06-11") with findings F2-1..F2-30 plus refreshed line refs for the still-open 2026-05-07 docket. **Step 1 shipped** (RUSTSEC cargo-update batch — deny.toml ignores only RUSTSEC-2023-0071 now; SSE Closed→break; two hot-path indexes; axios/vitest bumps; e2e suite repaired 33/39→39/39). **Step 2 shipped same day** (F2-8 OAuth email_verified gate, F2-1 trust_proxy_header rate-limit keying, F2-3/6/20 AniList chunking+status handling+page clamps, F2-9 settings-cache invalidation on login, F2-4/13 SSE reconnect + heartbeat via api).

⚠️ Deploy note: the live production config (not in git) must add `trust_proxy_header = true` or the rate limiter keeps keying on nginx's IP.

**Next (step 3):** extract shared Desktop/Mobile editor composable (fixes F2-11 draft-persistence drift + F2-12 raw-actor-id toasts surface), docket H-3 (advisory-lock bypass on add_item, `controllers/calendar.rs:~1100`), F2-10 (router-guard settings reconcile clobbers theme/locale). Then step 4 backlog (H-16/17/18, M-*, CR-1 ICS SUMMARY sanitize, F2 LOWs). Check AUDIT.md before starting any improvement work (supersedes [[reference_deep_dive_plan]] — the old plan file no longer exists).
