---
name: Axios 401 refresh interceptor pattern
description: How the silent-refresh interceptor in api.ts works, the _retried guard, and the public-route guard against breaking email links
type: feedback
originSessionId: 7a8d378a-90cb-41a0-b043-addec4ba2a44
---
The 401 interceptor in `frontend/src/config/api.ts` silently refreshes the JWT and retries the original request, with a public-route guard so direct-load auth pages don't get bounced.

Key invariants:

**`_retried` guard** — Without it, if `POST /auth/refresh` itself returns 401 (expired/missing refresh token), the interceptor would catch that 401, try to refresh again, get another 401, and loop forever. `_retried = true` breaks the cycle.

**`window.location.href` not `router.push`** — Using `router.push('/login')` from inside `api.ts` creates a circular dependency at module init: `router → stores/auth → @/config/api → router`. `window.location.href` is a clean redirect that avoids the import cycle.

**Public-route guard (added 2026-05-01 in commit 5e1ee79, fixing FU-1)** — The redirect MUST be skipped when the user is on a public route (`/login`, `/forgot-password`, `/reset-password`, `/register`, `/verify-email/pending`, `/verify-email`, `/404`). Otherwise emailed reset/verify links 401 on the initial session check and bounce to `/login` before the user ever sees the form.

The check is a path-prefix list in api.ts (Option A from the FU-1 plan), NOT a router import. Reason: importing `@/router` to read `currentRoute.value.meta.public` re-introduces the circular init dep — even though *reading* doesn't navigate, the import order at module-init time still cycles. The path-prefix list is hand-maintained and there's a code comment in `api.ts` pointing to this memory note explaining why no router import.

```typescript
const PUBLIC_PATH_PREFIXES = ['/login', '/forgot-password', '/reset-password',
                              '/register', '/verify-email', '/404']
function isPublicRoute(): boolean {
  return PUBLIC_PATH_PREFIXES.some(p => window.location.pathname.startsWith(p))
}
```

Tests at `frontend/src/__tests__/api.spec.ts` cover: 200 passthrough, 401 + refresh-fail on private (redirects), 401 + refresh-fail on public (no redirect), 401 + refresh-success (retries), 401 with `_retried` already set (no loop), `isPublicRoute()` direct cases. The test setup must `vi.unmock` both `'../config/api'` AND `'@/config/api'` because the global setup mocks both forms.

**No store import in interceptor** — The interceptor does not try to reset auth store state. The hard redirect to `/login` ensures the store is re-initialized naturally when the login page loads.

**How to apply:** This is the canonical pattern for this codebase. If a new public route is added to the router, the prefix list in `api.ts` MUST be updated alongside.
