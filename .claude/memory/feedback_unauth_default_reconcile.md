---
name: Unauth defaults are not server truth — guard reconcile call sites
description: Fetchers that return defaults when unauth must not feed those defaults into "server-wins" reconcile paths
type: feedback
originSessionId: 7a8d378a-90cb-41a0-b043-addec4ba2a44
---
When a settings/preferences fetcher returns hardcoded defaults for unauthenticated users (instead of `null` or throwing), downstream reconcile-from-server logic will treat those defaults as authoritative and silently overwrite the user's locally-stored preferences.

**Why:** Found at Track 1 smoke check (2026-04-30). `userSettingsStore.fetchSettings()` returns `getDefaultSettings()` (theme=dark, accent=coral) when `!authStore.isAuthenticated()`, without populating `store.settings`. `main.ts` was unconditionally calling `useTheme().reconcileFromServer({...defaults})`, which saw `coral !== local-matcha` and wrote `coral` back to `localStorage` — clobbering the unauth user's choice on every page load. Every unit test passed because the contract of `reconcileFromServer` is correct ("server wins"); the bug was at the *call site*. Fix: guard with `if (authStore.isAuthenticated())` before calling reconcile.

**How to apply:** When wiring a "server-wins" reconcile, audit the fetcher: does it return a sentinel (`null` / `undefined` / a tagged variant) for the unauth path, or fabricated defaults? If fabricated, the reconcile call site MUST guard on auth state — or the fetcher must change to return a sentinel. Unit tests of the reconcile function alone won't catch this; only smoke testing the unauth flow (or a wired integration test in `main.ts`) will.

This pattern generalizes beyond theme/accent: any preference, feature flag, or capability flag fetcher that has an unauth fallback shape can bite the same way.
