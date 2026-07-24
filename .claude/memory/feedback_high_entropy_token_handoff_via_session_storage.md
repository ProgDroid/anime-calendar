---
name: High-entropy possession-grant tokens must NOT ride in URL queries during auth handoff
description: Tokens that grant access on possession alone (invite, calendar subscribe) leak via history/bookmarks/sync/extensions if put in ?redirect= queries; use sessionStorage stash + consume-once
type: feedback
originSessionId: d141fb2f-b707-4ff0-8ba3-228ad77d407f
---
When an unauthenticated landing page (e.g. `/invite/:token`) needs to send the user through a login/register flow and then return to itself with the secret intact, do NOT use a `?redirect=/path/with/{token}` query parameter.

**Why:** URL queries land in:
- Browser history (Ctrl+H, Ctrl+Shift+T reopens tabs by URL)
- Bookmarks (user bookmarks the login page mid-flow)
- Browser sync (signed-in browsers replicate URL bars cross-device)
- Browser extensions with "read all data" / "tabs" permission
- Screenshots, screenshares, OBS recordings (URL bar visible)
- Server-side access logs (handled separately by `safe_url` redaction in this codebase, but defense-in-depth still wants the URL secret-free at the source)

`Referrer-Policy: strict-origin-when-cross-origin` (already set in nginx) prevents the path from being sent in `Referer` headers cross-origin, but does not address any of the above leak vectors.

**How to apply:**
- Stash the secret in `sessionStorage` (tab-scoped, never URL-serialized) before navigation. Consume-once: read + immediately remove.
- Routing changes from `<router-link :to="/login?redirect=/invite/${token}">` → `<button @click="goToAuth('login')">` where the click handler calls a `stashInviteToken(token); router.push('/login')` helper.
- LoginPage's post-success router.push reads the stash via `consumeInviteRedirect()` and prefers it over any legacy `?redirect=` query (which can stay as a same-origin-validated fallback).
- Keep the helper module narrow: `composables/inviteRedirect.ts` exports two functions (`stashInviteToken`, `consumeInviteRedirect`) and a private storage key. Wrap `sessionStorage.setItem` in try/catch — private-browsing modes can throw QuotaExceededError.
- Test the consume-clears-on-read behavior explicitly so a stale stash from a prior flow can't redirect a fresh login.

This pattern was implemented for AUDIT H-10 (Phase 3 T6). Same principle applies to any future token of this shape (one-time email-link confirmations, calendar subscribe tokens, magic-link auth, etc.).
