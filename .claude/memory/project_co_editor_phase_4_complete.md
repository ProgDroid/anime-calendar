---
name: Project: Co-editor sharing — Phase 4 complete (2026-05-07)
description: Tier transitions (suspend-on-downgrade, restore-on-upgrade, 402 modal, UpgradePage swap) shipped. 298 server / 451 frontend tests green. Phase 5 (public landing + E2E) is next.
type: project
originSessionId: 5f88ca2f-c4a5-4cba-99c3-92cedfbba71c
---
Phase 4 (Tier Transitions) of the co-editor sharing spec fully shipped on 2026-05-07.

**Commits:** `c665313` → `009c5b4`. 5 tasks (4.1–4.4 + verification gate).

**What shipped:**
- `server/src/services/sharing.rs` (new) — `KickRecord`, `suspend_owner_sharing_in_tx`, `RestoredEditor`, `restore_owner_sharing_in_tx`. Cross-cutting helpers shared by webhook + reconcile.
- `server/src/controllers/stripe_webhook.rs` — wired suspend (Paid→Free) and restore (Free→Paid) inside the tx; publishes `MemberLeft` + `kick` events post-commit; sends `send_editor_restored` emails on upgrade.
- `server/src/services/reconcile.rs` — `SharingDeps { editor_mapper, invitation_mapper, publisher, pool, email_service, user_mapper }` injected; `apply_sharing_side_effects` and `apply_restore_sharing_side_effects` mirror webhook paths.
- `server/src/mappers/subscription.rs` — `ReconcileRow` gained `user_id: i32`; `list_for_reconcile` updated.
- Frontend: `UpgradeInterruptModalBody.spec.ts` test for `share_calendar` reason; `UpgradePage.vue` `earlyAccess` → `shareCalendars`; locale files updated + old key retired.

**Tests:** 298 server / 451 frontend.

**Why:** Phase 5 (public /invite/:token landing page + E2E specs + ops notes) is the final phase before co-editor shipping is feature-complete.

**How to apply:** Phase 5 plan is at `docs/superpowers/plans/2026-05-05-co-editor-sharing.md` Phase 5 section. Tasks 5.1–5.3 cover the landing page, E2E specs, and ops checklist.
