---
name: Track 4 Phase 5 complete (2026-05-02)
description: Pro accent enforcement landed — frontend interrupt modal, downgrade-safe theme apply, backend 402 PaymentRequired. Phase 6 (reconcile loop + e2e) is next.
type: project
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
Phase 5 of Track 4 (Pro accent enforcement) shipped 2026-05-02 in 4 commits:

1. `feat(theme): downgrade-safe accent apply via setIsPaid` — `useTheme.ts` adds `isPaid` ref + `resolveAccent()` + `setIsPaid()`. Stored accent (ref + localStorage) is preserved on downgrade; only the DOM attribute is mapped to the default ('coral'). Re-upgrading flips the DOM back without a server roundtrip.
2. `feat(accent): pro-accent interrupt event in AccentPicker` — new `isPaid` prop; clicking a Pro accent as a free user emits `interrupt` instead of `update:modelValue`.
3. `feat(upgrade): UpgradeInterruptModal wired into PreferencesTab` — modal uses `UiModal` (inherits Track 2 focus trap). PreferencesTab fetches subscription tier on mount, calls `setIsPaid`, and opens modal on interrupt. New top-level `interrupt.*` i18n namespace (en + pt).
4. `feat(api): 402 PaymentRequired on Pro accent for free users` — `Error::PaymentRequired` variant with custom body `{"error":"upgrade_required","required_tier":"paid"}`. `Accent::is_pro()` helper. PUT /user/settings now blocks Pro accents for free users.

**Why:** Pro accents (Matcha, Sakura, Citron) had to become real entitlement-gated features end-to-end — UI gate, API gate, and downgrade safety.

**How to apply:**
- Phase 6 is next: hourly reconcile loop (`tokio::spawn`), final UI polish, manual e2e against Stripe test mode (cards 4242, 4000 0000 0000 9995, 4000 0027 6000 3184).
- Deferred: axios 402 interceptor for `/user/settings`. UI gate already prevents this path; revisit if telemetry shows API-only attempts.
- Tests: 293 frontend (+4 from Phase 5: 2 useTheme + 2 AccentPicker), 172 server (+2: free-402, paid-200 with seeded paid subscription).

**Files of interest** (verify before recommending):
- `frontend/src/composables/useTheme.ts`
- `frontend/src/components/UpgradeInterruptModal.vue`
- `frontend/src/components/account/AccentPicker.vue`
- `frontend/src/components/account/PreferencesTab.vue`
- `server/src/error.rs` (`PaymentRequired` variant)
- `server/src/entity/user_settings.rs` (`Accent::is_pro`)
- `server/src/controllers/user.rs` (gate + tests)
