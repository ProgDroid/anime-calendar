---
name: How to add a new 402 cap gate end-to-end
description: Reference for wiring a new tier-gated feature through the existing 402 reason-routing infrastructure shipped in Phase 1 (server Error::PaymentRequired + axios interceptor + global UpgradeInterruptModal + reason-localised copy).
type: reference
originSessionId: 75332392-b9dc-4e06-b274-05f71e4842d6
---
The 402 reason-routing pipeline is fully built — new tier gates plug into it; do not invent parallel paths.

**Server side (when adding a new cap):**
1. Throw `Error::PaymentRequired { required_tier: "paid", reason: Some("your_new_reason") }` from the relevant handler/service.
2. The error type Display-renders `"upgrade_required"`; the response body shape is `{ "error": "upgrade_required", "required_tier": "paid", "reason": "your_new_reason" }`. No additional handler code needed.
3. Existing reason codes: `cap_calendars`, `cap_shows`, `pro_accent`. Pick a new snake_case string — keep it stable, the frontend keys off it directly.

**Frontend side (when adding the new reason):**
1. Add to the `UpgradeReason` union in `frontend/src/composables/useUpgradeInterrupt.ts`.
2. Add localised `interrupt.heading.<reason>` and `interrupt.description.<reason>` keys to **both** `frontend/src/locales/en.json` and `pt.json`.
3. `UpgradeInterruptModalBody.vue` already routes `t(\`interrupt.heading.${reason}\`)` — no body changes needed if you add the keys.
4. The axios interceptor in `frontend/src/config/api.ts` already calls `openUpgradeModal(reason ?? 'pro_accent')` for any 402 with `required_tier === 'paid'` — no interceptor changes needed.
5. For client-side "open the modal before the request" gates (e.g., disabled-button click handlers), import `openUpgradeModal` from the composable and call directly.

**Architectural rules to preserve:**
- Server errors are the source of truth; the frontend interceptor must always re-throw so call sites can handle the original error inline.
- Default to `pro_accent` when `reason` is absent (covers older error sites that don't yet pass a reason).
- The `UpgradeInterruptModal` is mounted ONCE globally in `App.vue`. Don't re-mount it per page — multiple instances fight over the singleton ref.
- Cap-gated UI buttons should mirror server-side idempotency: `assert_can_add_show` lets already-tracked items pass at cap, so the matching client-side click handler must short-circuit on already-tracked **before** the cap check.

**Reason-naming convention:** `cap_<resource>` for hard caps, `<feature>` for tier-gated features. Examples: `cap_calendars`, `cap_shows`, `pro_accent`. Future likely: `cap_reminders`, `pro_event_style`.
