# Refresh Tokens — Implementation Plan

**Branch:** `feature/refresh-tokens`
**Worktree:** `.worktrees/refresh-tokens`
**Date:** 2026-04-16

## Design

- **Access token** (`auth_token` cookie): 30 min JWT, httpOnly, SameSite=Strict, path `/api/`
- **Refresh token** (`refresh_token` cookie): 30 day opaque token, httpOnly, SameSite=Strict, path `/api/auth/refresh`
- **Rotation**: every use of `/auth/refresh` marks old token `used_at = NOW()` and issues a new one atomically
- **Token format**: 32 random bytes → hex string (raw, sent to client); SHA-256 hash stored in DB
- **Frontend**: axios interceptor — on 401 call `POST /auth/refresh`, retry once; on refresh failure redirect to `/login`

## Token generation pattern (from `password_reset.rs`)
```rust
fn generate_raw_token() -> String { /* rand::rng().fill_bytes + hex fold */ }
fn hash_token(raw: &str) -> String { /* Sha256::digest + hex fold */ }
```

## Files

### New
- `migrations/20260416000001_add_refresh_tokens.sql`
- `server/src/mappers/refresh_token.rs`
- `server/src/controllers/refresh.rs`

### Modified
- `server/src/mappers.rs` — add `pub mod refresh_token`
- `server/src/controllers.rs` — add `pub mod refresh`
- `server/src/services/auth.rs` — `generate_token` TTL 24h → 30 min
- `server/src/controllers/auth.rs` — `build_auth_cookie` 24h → 30 min; add `build_refresh_cookie`; update login/register/logout
- `server/src/controllers/oauth.rs` — issue refresh token on OAuth login
- `server/src/server.rs` — inject `RefreshTokenMapper`, register `/auth/refresh`
- `server/src/main.rs` — construct `RefreshTokenMapper`
- `server/src/openapi.rs` — add refresh endpoint
- `frontend/src/config/api.ts` — add 401 interceptor

## Tasks

- [ ] **Task 1**: DB migration — `refresh_tokens` table
- [ ] **Task 2**: `RefreshTokenMapper` + module registration
- [ ] **Task 3**: Reduce access token TTL to 30 min (`services/auth.rs` + `build_auth_cookie`)
- [ ] **Task 4**: Add `build_refresh_cookie` helper and update login/register/logout to issue refresh token
- [ ] **Task 5**: Update OAuth controller to issue refresh token
- [ ] **Task 6**: New `POST /auth/refresh` endpoint (`controllers/refresh.rs`)
- [ ] **Task 7**: Wire up in `server.rs` + `main.rs`
- [ ] **Task 8**: Frontend axios interceptor (`api.ts`)
- [ ] **Task 9**: Regenerate `.sqlx/` offline cache
- [ ] **Task 10**: Update OpenAPI spec
- [ ] **Task 11**: Full test suite — verify all pass

## Notes

- `TIMESTAMP` not `TIMESTAMPTZ` for new columns (matches rest of `users` table pattern)
- Refresh token mapper follows exact same pattern as `PasswordResetMapper`
- `replace_token` must be transactional (atomic invalidate + create)
- After frontend refresh, re-run `initAuth` or store state is stale
