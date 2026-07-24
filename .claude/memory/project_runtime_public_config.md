---
name: Runtime public config endpoint shipped (2026-05-03)
description: GET /api/public-config now serves SPA bootstrap config; frontend image is environment-agnostic
type: project
originSessionId: 68ef861d-4b45-4a3c-8efa-28e7a87de409
---
Shipped 2026-05-03 (commit `930739f` on `main`).

**What changed:**
- New backend endpoint `GET /api/public-config` returns `{ google_client_id }`, populated at startup from `config.toml` (`server/src/controllers/public_config.rs`).
- New frontend service `frontend/src/services/publicConfig.ts` with `loadPublicConfig()` (async, called once in `main.ts`) and `getPublicConfig()` (sync, called by components).
- Bootstrap split: config-load is a hard-fail (renders static error shell), auth/settings remain soft-fail (mount with defaults).
- `VITE_GOOGLE_CLIENT_ID` and `VITE_API_BASE_URL` removed from `frontend/Dockerfile`, `docker-compose.yml`, `frontend/.env.example` (deleted), `frontend/src/env.d.ts`.
- Playwright fixture `frontend/e2e/fixtures.ts` now stubs `/api/public-config` → 200 *before* the catch-all `route.abort()`.

**Why:** The frontend Docker image was environment-coupled because `VITE_*` values were inlined at `npm run build` time. Same git SHA → different bundles per env → "image promoted from staging to prod" was a lie. Goal was a single image deployed to both environments. Runtime endpoint chosen over nginx `sub_filter` for type safety + future CSP compatibility.

**How to apply:** Adding new public bootstrap values: extend `PublicConfig` struct (Rust) + matching `PublicConfigResponse`/`PublicConfig` interfaces (TS), then `getPublicConfig().yourField` in consumers. See `feedback_vite_docker_env_vars.md` for full pattern.

**Verification at ship:** cargo check/test/clippy clean; frontend build/test:unit/lint clean (359/359 unit tests). E2E not run on Windows; relies on next CI run.
