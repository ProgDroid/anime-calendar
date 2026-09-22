# Memory History - anime-calendar (shipped milestones)

Archived from `MEMORY.md` to keep the always-loaded index lean. These are "work shipped" markers — the design-redesign tracks, monetisation phases, and co-editor sharing phases, all complete. **Not auto-loaded each session.** Read on demand when you need context on past work; the full detail lives in each linked topic file and in git history.

## Project status (historical point-in-time)

- [Implementation Status (2026-04-19)](project_implementation_status.md) — Refresh tokens done 2026-04-17; CI green 2026-04-19; full auth stack; ~258 tests. (Long superseded.)

## Design redesign (Tracks 1–4, sequence 1→2→4→3 — all complete)

- [Design System v1 (Track 1)](project_design_system_v1.md) — OKLCH tokens + Ui* primitives + theme/accent runtime + UiBannerFade lock test landed 2026-04-30.
- [Track 2 complete (2026-05-01)](project_track_2_complete.md) — Phases A-E + 7 follow-ups + form polish shipped. 257 tests / 49 files. DaisyUI fully evicted.
- [Track 2 a11y authenticated sweep (2026-05-01)](project_track_2_a11y_auth_sweep_complete.md) — Live axe across 7 authed routes; 3 fixes (h1 promotion, AccountPage main→section, UiMenu focus restore).
- [Track 4 Phase 2 (2026-05-02)](project_track_4_phase_2_complete.md) — Stripe Checkout happy path shipped.
- [Track 4 Phase 3 (2026-05-02)](project_track_4_phase_3_complete.md) — Stripe webhook (sig verify + idempotency + atomic upsert); ?session_id stopgap removed.
- [Track 4 Phase 4 (2026-05-02)](project_track_4_phase_4_complete.md) — POST /api/stripe/portal + SubscriptionTab (5th account tab). 11 new FE tests.
- [Track 4 Phase 5 (2026-05-02)](project_track_4_phase_5_complete.md) — Pro accent enforcement: interrupt modal + downgrade-safe apply + 402. 293 FE / 172 server tests.
- [Track 4 Phase 6 + deferred closed (2026-05-03)](project_track_4_phase_6_complete.md) — Hourly reconcile + drift counter + CLI mode; 2-tier pricing card + e2e checklist. Track 4 done.
- [Track 3 Mobile Companion (2026-05-03)](project_track_3_complete.md) — All 16 tasks; 358 unit / 24 e2e green. 1024px breakpoint, editor desktop/mobile split. Redesign fully complete.
- [Runtime public config endpoint (2026-05-03)](project_runtime_public_config.md) — `GET /api/public-config` + bootstrap split; FE Docker image identical across envs; `VITE_*` retired.

## Monetisation refinement (all complete)

- [Phase 1 (2026-05-04)](project_monetisation_phase_1_complete.md) — Tier enforcement, frozen-blob subscribe, advisory-lock caps, 402 reason routing. (Superseded by v1.)
- [Phase 2a backend (2026-05-04)](project_monetisation_phase_2a_complete.md) — Tier-aware VALARM + event_style branching + validation. (Superseded by v1.)
- [v1 complete (2026-05-05)](project_monetisation_v1_complete.md) — Phase 2b+3: reminders chip-list, event_style toggle, $2.99/$24.99 pricing, UpgradePage rewrite. 406 FE / 240 server tests.

## Co-editor sharing (Spec 2 — all phases complete)

- [Phase 0 (2026-05-05)](project_co_editor_phase_0_complete.md) — 12 tasks + gate green (281 server / 412 FE tests, redocly lint, build).
- [Phase 1 (2026-05-06)](project_co_editor_phase_1_complete.md) — Invitation lifecycle (`d5c5b17`); 9 routes, SHA-256 tokens, service-layer rate limit, anti-enum. 288 server / 412 FE.
- [Phase 2 (2026-05-06)](project_co_editor_phase_2_complete.md) — Per-item save, Members tab, leave-calendar. 433 FE tests.
- [Phase 3 (2026-05-07)](project_co_editor_phase_3_complete.md) — Live sync (SSE + Pub/Sub). 297 server / 450 FE tests.
- [Phase 4 (2026-05-07)](project_co_editor_phase_4_complete.md) — Tier transitions (suspend/restore, 402 modal, UpgradePage swap). 298 server / 451 FE.
- [Phase 5 (2026-05-07)](project_co_editor_phase_5_complete.md) — /invite/:token landing, E2E specs, UAT checklist, token-scrubbing. 302 server / 456 FE.

## Codebase audit (tracker is `AUDIT.md` at repo root — see reference_deep_dive_plan)

- [Audit Phase 3 remediation (2026-05-07)](project_phase_3_complete.md) — 6 findings (CD-LICENSE-1, C-1, H-4, H-6, H-7, H-8, H-10) on `audit/phase-3-critical`. In-house MIT rate-limit middleware at `server/src/middleware/rate_limit.rs`.
- [Follow-up audit kicked off (2026-06-11)](project_followup_audit_2026_06_11.md) — F2-1..30 logged in AUDIT.md. (Live status lives in AUDIT.md; Steps 1–6 shipped since.)
- [actix-governor path exemption (superseded 2026-05-07)](feedback_actix_governor_path_exempt.md) — custom KeyExtractor mapping exempt paths to a sentinel IP. Replaced by the in-house `server/src/middleware/rate_limit.rs` from the same Phase 3 work, so the technique no longer applies to this codebase.
