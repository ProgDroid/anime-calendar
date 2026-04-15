# httpOnly Cookie JWT Migration — Design Spec

**Date:** 2026-04-15
**Status:** Approved, pending implementation

---

## Context

JWT tokens are currently stored in `localStorage` and injected into every request via an axios `Authorization: Bearer` interceptor. `localStorage` is readable by any JavaScript on the page, making the token vulnerable to XSS. This migration replaces localStorage with an httpOnly cookie that the browser manages automatically and JS cannot read.

---

## Architecture

### Deployment topology

```
Browser
  └─ GET /            → nginx → frontend static files
  └─ GET /api/*       → nginx → backend (strips /api prefix)
```

The cookie is scoped to `Path=/api/` so it is only sent on API requests — not on every asset load.

### Dev topology

```
Browser (localhost:5173)
  └─ GET /api/*  → Vite dev server proxy → backend (localhost:8080)
```

Vite's proxy makes the backend appear same-origin. The cookie lands on `localhost:5173`, satisfying `SameSite=Strict` without HTTPS in local dev.

### Cookie attributes

```
Set-Cookie: auth_token=<jwt>;
  HttpOnly;           -- JS cannot read it
  SameSite=Strict;    -- not sent on cross-site requests (CSRF mitigation)
  Path=/api/;         -- only sent to API routes
  Secure;             -- HTTPS only; toggled off in dev via cookie_secure config flag
  Max-Age=86400;      -- 24h, matches current JWT expiry
```

### CSRF posture

`SameSite=Strict` + same-domain nginx reverse proxy provides sufficient CSRF protection for this use case. No additional CSRF token is required. If the API is ever exposed cross-origin (e.g. mobile client), this must be revisited.

### axios base URL

`baseURL` changes from `import.meta.env.VITE_API_BASE_URL` to the relative path `/api`. This eliminates the need to bake a URL into the frontend bundle — the relative path resolves correctly in both dev (via Vite proxy) and production (via nginx).

---

## Backend changes

### 1. `server/src/config/server.rs`

Add `cookie_secure: bool` to the `Server` config struct.

### 2. `config.toml.dist`

```toml
cookie_secure = false   # set true in production
```

Set `cookie_secure = true` in `config.docker.toml.dist`.

### 3. `server/src/controllers/auth.rs` — login + register

- Build an `actix_web::cookie::Cookie` with the JWT value + all attributes.
- Add it to the `HttpResponse` via `.cookie(cookie)`.
- Change the JSON body from `{ token, username }` to `{ username }` only. The token never leaves the server boundary.

### 4. `server/src/controllers/oauth.rs`

Same cookie treatment as login.

### 5. `server/src/controllers/auth.rs` — new `logout` handler

```
POST /auth/logout   (auth-gated)
```

Responds with a cookie of the same name but `Max-Age=0` to instruct the browser to delete it. Returns `200 {}`.

### 6. `server/src/middleware/auth.rs`

Change from reading `req.headers().get("Authorization")` to `req.cookie("auth_token")`. Same JWT validation logic, different source. Error response stays `401 {"error":"Unauthorized"}`.

### 7. `server/src/server.rs`

- Add `.allow_credentials(true)` to the Cors builder (required for cookies with cross-origin credentials; safe because `allowed_origins` is already an explicit list, not a wildcard).
- Register `POST /auth/logout`.

No new crate dependencies — `actix-web` includes cookie support in `actix_web::cookie`.

---

## Frontend changes

### 1. `frontend/vite.config.ts`

Add a `server.proxy` block:

```ts
server: {
  proxy: {
    '/api': {
      target: 'http://localhost:8080',
      rewrite: (path) => path.replace(/^\/api/, ''),
    },
  },
},
```

### 2. `frontend/src/config/api.ts`

- Remove the `Authorization: Bearer` request interceptor entirely.
- Change `baseURL` from `import.meta.env.VITE_API_BASE_URL` to `/api`.
- Add `withCredentials: true` to the axios instance so cookies are included in proxied requests.

`VITE_API_BASE_URL` is no longer needed. `VITE_GOOGLE_CLIENT_ID` is unaffected.

### 3. `frontend/src/stores/auth.ts`

- Remove all `localStorage.getItem/setItem/removeItem('authToken', ...)` calls.
- `initAuth()` — call `GET /auth/me` instead of reading localStorage. On success, populate `user.value`. On 401, set `user.value = null`. Use an `initialized: Ref<boolean>` flag (idempotent — skip if already run). Use the existing in-flight Promise guard pattern to avoid concurrent calls.
- `isAuthenticated()` — check `user.value !== null`. No localStorage involved.
- `logout()` — call `POST /auth/logout`, then clear `user.value`.
- `login()` / `register()` / `oauthLogin()` — response body no longer contains `token`; populate `user.value` from `{ username }` and any other returned fields.

### 4. `frontend/src/router/index.ts`

`initAuth()` must complete before the first navigation so the guard has auth state. Call it in the `beforeEach` guard on first entry (the `initialized` flag prevents re-running). The rest of the guard logic (redirect unauthenticated, skip `fetchSettings` on public routes) is unchanged.

### 5. `frontend/nginx.conf`

Add a proxy location block:

```nginx
location /api/ {
    proxy_pass http://server:8080/;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
}
```

The trailing slash on `proxy_pass` strips the `/api/` prefix before forwarding to the backend.

---

## Testing

### Backend tests to update

- `middleware/auth.rs` tests — change `Authorization: Bearer <token>` header injection to `Cookie: auth_token=<token>`.
- `controllers/auth.rs` tests:
  - Drop assertion for `token` field in JSON body.
  - Add assertion that `Set-Cookie` response header contains `auth_token` and `HttpOnly`.

### Frontend tests to update

- `LoginPage.spec.ts` — mock response no longer includes `token`; assert token is NOT stored in localStorage.
- `routerGuard.spec.ts` — mock `initAuth` to set `user.value` directly rather than making a network call; guard logic itself is unchanged.

---

## Edge cases

| Scenario | Handling |
|----------|----------|
| Cookie expires mid-session | Middleware returns 401; existing axios 401 interceptor redirects to login and clears Pinia state |
| `initAuth()` network failure on page load | Catch → `user.value = null` → router guard redirects to login |
| Concurrent `initAuth()` calls | Already handled by existing in-flight Promise guard pattern |
| iCal subscription endpoint (`GET /calendars/subscription/:token`) | Public endpoint using subscription token in URL, not a cookie — no change |
| `GET /calendars/:id/export` | Auth-gated via middleware; will now read cookie automatically |

---

## File change summary

| File | Change |
|------|--------|
| `server/src/config/server.rs` | Add `cookie_secure: bool` |
| `config.toml.dist` | Add `cookie_secure = false` |
| `config.docker.toml.dist` | Add `cookie_secure = true` |
| `server/src/controllers/auth.rs` | Set cookie on login/register; add logout handler |
| `server/src/controllers/oauth.rs` | Set cookie on OAuth login |
| `server/src/middleware/auth.rs` | Read `auth_token` cookie instead of Authorization header |
| `server/src/server.rs` | `allow_credentials(true)`; register logout route |
| `frontend/vite.config.ts` | Add `server.proxy` for `/api/` |
| `frontend/src/config/api.ts` | `baseURL = '/api'`; `withCredentials = true`; remove Bearer interceptor |
| `frontend/src/stores/auth.ts` | Replace localStorage token with `initAuth()` → `/auth/me`; update logout |
| `frontend/src/router/index.ts` | Call `initAuth()` before first navigation |
| `frontend/nginx.conf` | Add `/api/` proxy location block |
