---
name: Tier-gated cosmetic apply — preserve preference, resolve at apply-time
description: For tier-gated cosmetics (accents, themes, etc.), keep the user's stored preference intact on downgrade and resolve to default only at apply-time. Re-upgrade restores instantly without a server roundtrip.
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
For tier-gated cosmetic settings (Pro accents are the canonical example, but the same applies to themes, layouts, etc.), don't overwrite the stored preference when the user downgrades. Instead, separate the two concerns:

- **Stored value** (Pinia ref + localStorage + server settings row) — user's *preference*. Never touched by tier changes.
- **Rendered value** (DOM attribute, applied class) — `resolveValue(stored, isPaid)` → falls back to default if a Pro value is stored on a free user.

Pattern in this repo (`frontend/src/composables/useTheme.ts`):
```ts
function resolveAccent(a: Accent): Accent {
  if (!isPaid.value && PRO_ACCENTS.has(a)) return DEFAULT_ACCENT
  return a
}
function setAccent(a: Accent) {
  accent.value = a                  // stored
  localStorage.setItem('accent', a)
  document.documentElement.setAttribute('data-accent', resolveAccent(a))  // rendered
}
function setIsPaid(paid: boolean) {
  isPaid.value = paid
  document.documentElement.setAttribute('data-accent', resolveAccent(accent.value))  // re-resolve only
}
```

**Why:** A user who upgrades, picks Matcha, then lapses, should not lose their Matcha preference. When they re-upgrade, the DOM should flip back to Matcha *without* a settings refetch — it's a 1-line attribute write because the stored value never moved.

**How to apply:**
- Whenever you add a tier-gated cosmetic, ask: "if the user lapses and resubscribes, do I need a server roundtrip to restore their preference?" If yes, you've coupled storage to the gate. Decouple them.
- Default `isPaid` to `true` in the composable so unauth/pre-fetch states don't blank out a stored Pro value before the entitlement read resolves. The server gate (402) is the source of truth for actual writes.
- Pair with a frontend gate at the picker level (emit `interrupt` instead of `update`) so free users never get to call the apply function with a Pro value.
- Pair with a backend 402 (defense in depth) — UI gate stops the click; API gate stops a tampered client.
