---
name: Feedback: httpOnly cookie JWT migration patterns
description: Key patterns for same-domain cookie-based auth in Actix-Web + Vue3 + nginx stack
type: feedback
originSessionId: c705d17f-baf7-4179-ac03-b43158e94b8f
---
For httpOnly cookie auth in this stack, the same-domain constraint is the linchpin — everything else follows from it.

**Why:** localStorage is JS-readable (XSS risk); httpOnly cookies are not. SameSite=Strict prevents CSRF. The same-domain nginx proxy makes cookies same-origin so they flow on every request automatically.

**How to apply:** When touching auth, cookie config, or proxy routing, verify these 5 things are in sync:

## The 5-part contract

1. **nginx proxy** strips `/api` prefix and forwards to backend:
   ```nginx
   location /api/ {
       proxy_pass http://server:8080/;
   }
   ```

2. **Vite dev proxy** mirrors nginx so dev and prod behave identically:
   ```ts
   proxy: { '/api': { target: 'http://localhost:8080', rewrite: (p) => p.replace(/^\/api/, '') } }
   ```

3. **axios** sets `withCredentials: true` globally — never send `Authorization` header:
   ```ts
   axios.defaults.withCredentials = true
   ```

4. **Actix-Web CORS** must call `.supports_credentials(true)` and list explicit origins (wildcard `*` is rejected by browsers when credentials are involved):
   ```rust
   Cors::default().allowed_origins(origins).supports_credentials()
   ```

5. **Cookie attributes** — always set these three:
   - `http_only(true)`
   - `same_site(SameSite::Strict)`
   - `secure(cookie_settings.secure)` — `false` in dev, `true` in prod

## Auth store pattern (Vue3 / Pinia)
- `initAuth()` calls `GET /api/user` — if 200, populate user state; if 401, clear state
- `initAuth()` is idempotent: guard against concurrent calls with an in-flight promise ref
- Router `beforeEach` calls `initAuth()` once (flag `authInitialized`) before any navigation
- No token stored anywhere in JS — user state is the source of truth

## Logout
- `POST /api/auth/logout` on backend sets `Max-Age=0` to clear the cookie
- Frontend clears Pinia state and redirects to `/login`
- No token to wipe from localStorage
