# anime-calendar — Claude Code Project Guide

## Project Overview

Anime Calendar is a web application for tracking anime series and episodes across personal calendars. Users register, create calendars, add anime from Anilist, and export schedules as iCalendar files.

## Tech Stack

### Backend
- **Language**: Rust (edition 2024), Cargo workspace (`server`, `anilist`, `common` crates)
- **Framework**: Actix-Web 4 + actix-cors
- **Rate limiting**: `actix-governor` 0.7 — token bucket per IP (60 burst, 1 req/s steady)
- **Database**: PostgreSQL via sqlx 0.8 (`runtime-tokio`, `tls-native-tls`, `postgres`, `chrono` features)
- **Cache**: Redis via `redis` crate 1.0 (`aio`, `tokio-comp`) — custom `Cache` struct in `server/src/cache.rs`
- **Auth**: JWT (`jsonwebtoken` 10 with `aws_lc_rs` backend) + Google OAuth (`google-oauth` crate)
- **Passwords**: argon2
- **Calendar export**: `icalendar` 0.17
- **Config**: `config` crate reading `config.toml` (server/Redis/JWT/OAuth) and `database.toml` (PostgreSQL)
- **Metrics**: `metrics` 0.24.3 façade + `metrics-exporter-prometheus` 0.18.1 — `/metrics` endpoint scraped by Prometheus

### Frontend
- **Framework**: Vue 3.5 + TypeScript 5.9
- **Build**: rolldown-vite (Rust-based Vite fork) with `@vitejs/plugin-vue`
- **Styling**: Tailwind CSS v4 (`@tailwindcss/vite` plugin) + DaisyUI v5 (legacy, removal planned in Track 2 of redesign)
- **Design tokens**: OKLCH-based, defined in `frontend/src/assets/tokens.css`, re-exported as Tailwind utilities via the `@theme` block in `main.css`. Theme via `[data-theme="light|dark"]` on `<html>`; accent via `[data-accent="coral|iris|matcha|sakura|citron"]` (only swaps `--accent-h1`/`--accent-h2`). A pre-paint inline script in `index.html` reads `localStorage` and sets these attrs synchronously to avoid flash. Self-hosted fonts (Geist, Geist Mono, Instrument Serif) under `frontend/public/fonts/`.
- **UI primitives**: `frontend/src/components/ui/` houses `Ui*` primitives (Button, Input, Segmented, Chip, Modal, Toast, Avatar, BannerFade) built with `tailwind-variants`. `frontend/src/components/ui/icons/` houses one SFC per icon, ported verbatim from `design_handoff_anime_calendar/foundations.jsx` — do not substitute Lucide / Heroicons / Material.
- **State**: Pinia 3
- **Routing**: vue-router 4 (all routes lazy-loaded)
- **i18n**: vue-i18n 11 — English and Portuguese (`frontend/src/locales/{en,pt}.json`)
- **HTTP**: axios
- **Testing**: Vitest + @vue/test-utils + jsdom
- **Linting**: oxlint + eslint + prettier

## Project Structure

Cargo workspace: `server` (HTTP + business logic), `anilist` (API client), `common` (shared types). Backend follows a controllers → services → mappers (repository) layering. Frontend is a Vue SPA under `frontend/src/` with `components/`, `stores/`, `services/`, `router/`, `composables/`, and `locales/`.

Non-obvious layout:
- `server/src/entity/` holds domain structs; `server/src/mappers/` owns DB pool clones and acts as the repository layer.
- `frontend/src/components/calendar/` holds CalendarPage sub-components; `components/shared/` holds reusable UI.
- `frontend/src/components/ui/` holds the design-system primitives (Track 1 of the redesign); `components/ui/icons/` holds icon SFCs. **Prefer these over DaisyUI classes when writing new UI.**
- `frontend/src/composables/useTheme.ts` owns theme + accent runtime state (singleton refs); `userSettingsStore` is the source of truth and reconciles via the existing settings-fetch flow on auth.

Design redesign: a 4-track plan from `design_handoff_anime_calendar/` is in flight. Track 1 (Foundations) is done; Tracks 2 (existing surfaces re-skin), 4 (upgrade flow), 3 (mobile companion) are pending. Sequencing: 2 → 4 → 3. Master breakdown at `docs/superpowers/specs/2026-04-30-design-redesign-master-breakdown.md`. Per-track specs and plans live alongside.

## Build & Run

### Backend (Windows)
```bash
# Requires nasm.exe installed for aws-lc-sys (JWT backend)
AWS_LC_SYS_PREBUILT_NASM=1 cargo build
```

### Backend in sandboxes without github.com egress
`utoipa-swagger-ui` fetches the Swagger UI archive from github.com **in its build
script**, so the `server` crate cannot build at all where that host is blocked —
Claude Code cloud sessions included. The failure is opaque: the proxy's 403 JSON
body gets saved as `v5.17.14.zip` and the build panics with
`InvalidArchive("Could not find EOCD")`. Build and test there with:
```bash
SQLX_OFFLINE=true cargo test -p server --no-default-features
```
That drops only the `/swagger-ui/` route, which is dev-only and already gated at
runtime behind `enable_docs` (a build without the feature logs a warning if
`enable_docs = true`). CI and local dev use default features, so the Swagger path
stays fully covered — don't "simplify" the feature away.

### Frontend
```bash
cd frontend
npm install
npm run dev        # dev server
npm run build      # type-check + build
npm run test:unit  # vitest
npm run lint       # oxlint + eslint
```

### Config
Copy `config.toml.dist` → `config.toml` and `database.toml.dist` → `database.toml`, then fill in values.

## Coding Conventions

### Backend
- All fallible public functions have `/// # Errors` doc comments
- `ServerResult<T>` = `Result<T, Error>` — use this alias throughout the server crate
- Dependency injection via `web::Data<T>` — mappers, cache, anilist, google_oauth, config all injected at startup
- Mapper structs own a DB connection pool clone; they act as repository layer
- `#[must_use]` on pure functions; `const fn` where possible
- Tests go in the same file as the code under test
- Use `log::error!` for all error logging — never `eprintln!` (bypasses logging infra)
- Multi-table DB mutations must use a transaction with explicit rollback
- Redis key scanning: use `SCAN` cursor loop, never blocking `KEYS`
- All `Error` variants must return `{"error":"..."}` JSON — never plain text
- Auth failures: always return `Error::Unauthorised` regardless of whether the user exists
- sqlx: regenerate and commit `.sqlx/` after any query change (`DATABASE_URL=... cargo sqlx prepare --workspace -- --all-targets`). The trailing `-- --all-targets` flag is required — `prepare` walks default targets only by default, and CI's `cargo test --workspace` step needs cached entries for test-module `sqlx::query!` calls or it fails with "no cached data for this query".
- Config is layered: `config.toml` / `database.toml` first, then environment variables override (both files are optional, so a container with no config files works). `Server` reads env **unprefixed** with `__` for nesting (`PORT`, `LOG_LEVEL`, `REDIS__URL`) so a platform-injected `PORT` is honoured; `Database` reads env **prefixed** (`DATABASE__URL`, `DATABASE__HOST`) because its field names (`user`, `host`, `pass`) would otherwise collide with ambient shell/CI variables. Don't unify them — `config/database.rs` has a test locking in why.
- Redis has three independent consumers (`Cache`, `RedisPubSub`, `PresenceService`). They all take a URL from `RedisConfig::connection_url()`, resolved once in `main.rs` — if you add a fourth, use the same helper or it will quietly dial localhost when `redis.url` is set.
- **There is exactly ONE Postgres pool per process.** `main.rs` calls `Database::new` once; every mapper and service takes an Arc-backed `Database`/`PgPool` clone. Do **not** call `Database::new` again to get "a dedicated pool" — it opens a real second pool, which is how this got to fifteen of them at sqlx's default of 10 connections each. The per-instance budget is `database.toml`'s `max_connections` (default 5); its doc comment carries the arithmetic a managed Postgres has to satisfy. Mapper constructors take `Database` and are neither `async` nor fallible.
- **`actix-web` is declared with `default-features = false` specifically to exclude `http2`.** The server binds with plain `HttpServer::bind()`, which serves HTTP/1.1 only (h2 needs `bind_auto_h2c()` or ALPN via `bind_rustls`/`bind_openssl`), and TLS terminates at nginx/Cloudflare. The feature pulled in `h2` 0.3.x, whose line ended at 0.3.27 with RUSTSEC-2026-0258 unpatched and no backport — so this is what keeps `cargo-deny` green. If you re-add `http2`, re-check that advisory; and if you add a feature to that list, keep the rest, since `default-features = false` drops `macros`, `cookies` and the `compress-*` set too.
- Migrations are **up-only** (sqlx simple style); there are no `down.sql` files by design. Rollback is via PostgreSQL backup / point-in-time recovery, not down migrations — several migrations are destructive to reverse. See `docs/rollback-strategy.md` (the inventory + reverse-safety table) before adding or reversing a migration. Do **not** rename migrations to `.up/.down` style: it invalidates every `_sqlx_migrations` checksum and deepens the known dev-DB drift.

### CI/CD

Pin all third-party GitHub Actions to commit hashes, not tags:
```yaml
- uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd
```

### Frontend
- All user-facing strings use `$t()` / `t()` — never hardcode text in components
- When adding translatable text, add the key to **both** `en.json` and `pt.json`
- Translation key naming: hierarchical, e.g. `auth.login.title` — top-level namespaces: `app`, `auth`, `calendar`, `calendars`, `userDetails`, `userSettings`, `errors`
- Design-system primitives in `components/ui/` use `tailwind-variants` (`tv()`) for variant→class mapping at the top of each component file. Use OKLCH tokens (`bg-bg-1`, `text-fg-2`, `bg-accent-1`, etc.) over DaisyUI semantic colors when writing new UI.
- **Text vs surface accent/danger tokens**: For accent/danger *text* on neutral backgrounds use `text-accent-1-text` and `text-danger-text` (AA 4.5:1 in both themes). Reserve bare `bg-accent-1`, `border-danger`, etc. for *surfaces* (button bg, badge bg, card border). Bare `text-accent-1` / `text-danger` fail contrast for body text in light theme.
- The `<UiBannerFade>` mask string is locked verbatim by a vitest contract test — do not edit it without updating the lock test and confirming the change is intentional.
- "Server-wins" reconcile call sites must guard on auth state when the upstream fetcher fabricates defaults for unauth users (see `useTheme` reconcile in `main.ts`). Otherwise the unauth defaults will silently clobber `localStorage`.
- Auth guard lives in `router/index.ts` `beforeEach` — settings fetched on non-public navigations only
- Public routes must declare `meta: { public: true }`; `fetchSettings` is skipped for them
- Pinia stores: `auth.ts` for auth state, `userSettingsStore.ts` for user preferences
- Use `axios.isAxiosError(err)` when you need `err.response.status`; use bare `catch` (no binding) when the error value is never read
- Deduplicate concurrent Pinia async actions with `ref<Promise<T> | null>` — return the in-flight promise if one exists
- Debounce watchers that write to `sessionStorage`/`localStorage` — at least 1s timeout, cleared in `onBeforeUnmount`. When clearing the slot on success/navigation (e.g. after a successful save before `router.push`), cancel the pending timer *before* `removeItem`, or a late fire re-persists stale/empty data over the cleared slot
