---
name: Implementation Status
description: Current project state as of 2026-04-17; refresh tokens complete — auth stack fully hardened
type: project
originSessionId: 09480f2a-20ee-482c-9ba8-4bfed05009a3
---
The project is production-ready. All planned auth/security features are complete.

**Why:** Completed httpOnly cookie migration (2026-04-15) + password reset (2026-04-16) + email verification (2026-04-16) + refresh tokens (2026-04-17).

**How to apply:** Don't treat this as a greenfield project. Next work is load testing, DB indexes, or Docker/CI polish.

## Infrastructure ✅
- CI: `.github/workflows/ci.yml` — 3 parallel jobs (backend clippy/test/fmt, frontend build/test/lint, OpenAPI validation)
- Docker: `Dockerfile` + `frontend/Dockerfile` (nginx, non-root users) + `docker-compose.yml`
- `schema.sql`: baseline PostgreSQL schema at repo root
- `.sqlx/`: offline query cache committed; CI uses `SQLX_OFFLINE=true`

## Auth: httpOnly Cookies ✅ (completed 2026-04-15)
JWT tokens stored in httpOnly + SameSite=Strict cookies — no longer in localStorage.
- Same-domain nginx reverse proxy routes `/api/*` → backend (strips `/api` prefix)
- Vite dev proxy: `/api/` → `http://localhost:8080`
- axios: `withCredentials: true`; no `Authorization` header interceptor
- Backend middleware: reads `auth_token` cookie
- `CookieSettings` newtype injected via `web::Data`

## Password Reset ✅ (completed 2026-04-16)
Full forgot-password / reset-password flow on branch `feat/password-reset`.

Architecture:
- `password_reset_tokens` table: `id`, `user_id`, `token_hash` (SHA-256 hex), `expires_at` (1h), `used_at`
- `PasswordResetMapper`: `invalidate_previous_tokens`, `create_token`, `replace_token` (atomic tx), `find_valid_token`, `complete_reset` (tx)
- `EmailService` (`server/src/services/email.rs`): lettre 0.11 STARTTLS; dev fallback logs URL when `smtp.host` is empty
- `POST /auth/forgot-password`: always 200, anti-enumeration; silently skips OAuth-only users
- `POST /auth/reset-password`: SHA-256 hash lookup, password strength check, atomic complete_reset
- Raw token: 32 CSPRNG bytes hex-encoded in email; only hash stored in DB
- Frontend: `ForgotPasswordPage.vue`, `ResetPasswordPage.vue`, "Forgot password?" link in `LoginPage.vue`
- Routes: `/forgot-password`, `/reset-password` — both `meta: { public: true }`
- i18n: `auth.forgotPassword.*`, `auth.resetPassword.*` in both `en.json` and `pt.json`

## Email Verification on Register ✅ (completed 2026-04-16)
Unverified accounts cannot log in with password. Google OAuth users auto-verified.

Architecture:
- Migration `20260416000000`: `email_verified_at TIMESTAMP` on `users` (backfilled for existing rows); `email_verification_tokens` table with `token_hash UNIQUE`, 24h `expires_at`
- `EmailVerificationMapper`: `replace_token` (atomic DELETE+INSERT), `find_valid_token` (checks expiry), `consume_and_verify` (atomic DELETE token + UPDATE user)
- Token utilities in `services/auth.rs`: `generate_random_token()`, `hash_token()` (shared with password reset)
- `POST /auth/verify-email`: validates token, calls `consume_and_verify`, issues auth cookie + `AuthResponse`
- `POST /auth/resend-verification`: always 200 (anti-enumeration); skips unknown/already-verified; re-generates token + resends email
- `register` handler: no longer issues cookie; sends verification email; compensating `delete_user` if `replace_token` fails
- `login` handler: returns `Error::EmailNotVerified` (403) for unverified password accounts
- OAuth handler: calls `mark_email_verified` immediately after `create_user`
- Frontend: `VerifyEmailPendingPage.vue` (check-your-inbox + resend button), `VerifyEmailConfirmPage.vue` (token → cookie → redirect)
- Auth store: `register` no longer sets user state; `verifyEmail(token)` action added
- Routes: `/verify-email/pending` (VerifyEmailPending), `/verify-email` (VerifyEmailConfirm) — both `meta: { public: true }`
- i18n: `auth.verifyEmail.pending.*`, `auth.verifyEmail.confirm.*` in both `en.json` and `pt.json`
- `MessageResponse` moved to `controllers::auth` (shared by auth + password_reset + email_verification)

Key files:
- `server/src/mappers/email_verification.rs`
- `server/src/controllers/email_verification.rs`
- `frontend/src/components/VerifyEmailPendingPage.vue`
- `frontend/src/components/VerifyEmailConfirmPage.vue`

## OpenAPI ✅
All handlers annotated; Swagger UI at `/swagger-ui/`, JSON at `/api-docs/openapi.json`

## Soft deletes ✅
`deleted_at TIMESTAMP NULL` on `users` and `calendars`

## Refresh Tokens ✅ (completed 2026-04-17, branch `feature/refresh-tokens`)
Short-lived JWT (30 min) + long-lived opaque refresh token (30 days, rotation on every use).

Architecture:
- `refresh_tokens` table: `id`, `user_id`, `token_hash` (SHA-256 hex), `expires_at`, `created_at`, `used_at`
- `RefreshTokenMapper`: `replace_token` (atomic DELETE+INSERT, used at login/register/OAuth), `find_valid_token`, `rotate_token` (mark-used+INSERT, used at refresh endpoint), `invalidate_all_for_user` (logout)
- `POST /auth/refresh`: validates refresh cookie → rotates token → issues new JWT cookie + new refresh cookie
- `build_refresh_cookie`: `path("/api/auth/refresh")` — browser only sends it to the refresh endpoint
- Token helpers `generate_raw_token()` / `hash_refresh_token()` live in `controllers/auth.rs` (pub, reused by oauth.rs and refresh.rs)
- JWT TTL changed from 24h → 30 min; `build_auth_cookie` max_age updated
- Frontend: axios 401 interceptor in `api.ts` — silent refresh → retry; on failure `window.location.href = '/login'`

✅ **Merge resolved** (verified 2026-04-18): `feature/refresh-tokens` merged cleanly into main. `register` correctly sends a verification email with no cookies; `login` enforces `email_verified_at.is_none()` before issuing the refresh cookie; `google_oauth` calls `mark_email_verified` + `replace_token` (refresh) for new OAuth users. All three handlers are correctly integrated.

## Test coverage ✅ (~258 total)
**Backend:** All mapper + controller integration tests use `#[sqlx::test]` with live migrations.
- RefreshTokenMapper: 5 integration tests
- refresh controller: 3 integration tests
- EmailVerificationMapper: 4 integration tests
- email_verification controller: 4 integration tests
- (plus all pre-existing 118 backend tests)

**Frontend — 117 tests** across 20 spec files (all passing).

**Known non-issue**: 4 tests (`mappers/calendar`, `mappers/password_reset`) fail with `PoolTimedOut` when the full suite runs concurrently against `aegyptvault.local`. Connection-limit problem on the remote test DB, not a code bug. They pass individually.

## CI pipeline: all commands pass ✅ (fixed 2026-04-19)
`npm run build`, `npm run test:unit -- --run`, `npm run lint` all green.
Fixes: TS strict array-index `!` assertions, `HTMLInputElement` casts, `no-explicit-any` in tests,
`vue/multi-word-component-names` via `defineOptions`, unused `props`/`catch(err)` vars cleaned up.

## Outstanding items (non-blocking)
1. DB indexes: `calendars.user_id`, `calendars.subscription_token`, `calendar_items.calendar_id`
2. Load testing
3. Episode-specific times (not just all-day entries)

## Design redesign — Track 1 ✅ (completed 2026-04-30)
Foundations layer of the Claude Design hifi handoff (`design_handoff_anime_calendar/`) shipped on 2026-04-30. See `project_design_system_v1.md` for the complete picture. Tracks 2 (existing surfaces re-skin), 4 (upgrade flow), 3 (mobile companion) still pending — sequencing 2→4→3. Master breakdown at `docs/superpowers/specs/2026-04-30-design-redesign-master-breakdown.md`.

Headline numbers post-Track 1:
- Frontend tests: 161 passing across 30 spec files (was 117).
- Backend tests: 154 passing (added accent_preference round-trip).
- 20 commits on main covering migration, tokens, fonts, theme runtime, 8 UI primitives, 22 icons, 1 smoke fix.
