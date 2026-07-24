---
name: Frontend public config — runtime endpoint, not Vite env vars
description: Public bootstrap config is served at runtime by `GET /api/public-config`, not baked into the bundle via VITE_* env vars
type: feedback
originSessionId: 6affb378-d634-41e7-873e-fb9eed26e218
---

**Current pattern (since 2026-05-03):** Public, non-secret config the SPA needs at bootstrap is served by the backend at `GET /api/public-config` and fetched once during bootstrap in `frontend/src/main.ts` before `app.mount`. The bundle has zero environment-specific values — the same Docker image deploys to staging and production.

**Why the migration:**
- **Identical staging/prod images.** With `VITE_*` baked in, every environment needed its own bundle/image, defeating "test the same artefact you ship". Now `frontend:{sha}` is built once and promoted via tag.
- **Single source of truth.** `google_client_id` was duplicated between `config.toml` (backend, used to *verify* tokens) and `VITE_GOOGLE_CLIENT_ID` (frontend, used to *mint* tokens). Drift silently broke OAuth. Now both consume the same `config.toml` value.
- **Type safety.** A typo in `import.meta.env.VITE_FOO` is `undefined` at runtime; a typo in the runtime endpoint surfaces at compile time on the backend (`PublicConfig` struct) and at fetch time on the frontend (`PublicConfigResponse` interface).
- **Future CSP compatibility.** Inline `<script>__APP_CONFIG__ = ...</script>` blocks via nginx `sub_filter` would conflict with `script-src 'self'` (no `unsafe-inline`). Backend endpoint avoids this entirely.

**How to apply (adding new public bootstrap config):**
1. Add the field to `PublicConfig` in `server/src/controllers/public_config.rs`. Construction site is `server/src/server.rs` (`let public_config = PublicConfig { ... }`).
2. Mirror the field on the TypeScript side: extend the `PublicConfigResponse` and `PublicConfig` interfaces in `frontend/src/services/publicConfig.ts`.
3. Read it via `getPublicConfig().yourField` in components — it's synchronous because bootstrap awaited it.
4. **Failure mode is "hard fail."** A missing `/api/public-config` response renders the static error shell in `main.ts` instead of mounting the SPA — misconfigured deploys fail loudly rather than shipping silently-broken features.
5. **Never put secrets here.** Anything in the response is visible in DevTools. Stripe publishable keys, OAuth client IDs, feature flags = OK. JWT secret, DB password, Stripe secret key = absolutely not.

**Don't reintroduce `VITE_*`** for environment-specific values. The legacy `VITE_API_BASE_URL` was already dead (axios uses relative `/api`); `VITE_GOOGLE_CLIENT_ID` was the last one and was retired 2026-05-03. The `frontend/Dockerfile` and `docker-compose.yml` no longer have any `ARG`/`build.args` for Vite vars and shouldn't gain new ones.

**Historical note:** Earlier guidance recommended `ARG VITE_FOO` + `ENV VITE_FOO=$VITE_FOO` in the Dockerfile because Vite inlines env vars at build time. That mechanic is still true — but you no longer need it, because we deliver public config at runtime instead.
