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
- **Styling**: Tailwind CSS v4 (`@tailwindcss/vite` plugin) + DaisyUI v5
- **State**: Pinia 3
- **Routing**: vue-router 4 (all routes lazy-loaded)
- **i18n**: vue-i18n 11 — English and Portuguese (`frontend/src/locales/{en,pt}.json`)
- **HTTP**: axios
- **Testing**: Vitest + @vue/test-utils + jsdom
- **Linting**: oxlint + eslint + prettier

## Project Structure

Cargo workspace: `server` (HTTP + business logic), `anilist` (API client), `common` (shared types). Backend follows a controllers → services → mappers (repository) layering. Frontend is a Vue SPA under `frontend/src/` with `components/`, `stores/`, `services/`, `router/`, and `locales/`.

Non-obvious layout: `server/src/entity/` holds domain structs; `server/src/mappers/` owns DB pool clones and acts as the repository layer; `components/calendar/` holds CalendarPage sub-components; `components/shared/` holds reusable UI.

## Build & Run

### Backend (Windows)
```bash
# Requires nasm.exe installed for aws-lc-sys (JWT backend)
AWS_LC_SYS_PREBUILT_NASM=1 cargo build
```

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
- sqlx: regenerate and commit `.sqlx/` after any query change (`DATABASE_URL=... cargo sqlx prepare --workspace`)

### CI/CD

Pin all third-party GitHub Actions to commit hashes, not tags:
```yaml
- uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd
```

### Frontend
- All user-facing strings use `$t()` / `t()` — never hardcode text in components
- When adding translatable text, add the key to **both** `en.json` and `pt.json`
- Translation key naming: hierarchical, e.g. `auth.login.title` — top-level namespaces: `app`, `auth`, `calendar`, `calendars`, `userDetails`, `userSettings`, `errors`
- Auth guard lives in `router/index.ts` `beforeEach` — settings fetched on non-public navigations only
- Public routes must declare `meta: { public: true }`; `fetchSettings` is skipped for them
- Pinia stores: `auth.ts` for auth state, `userSettingsStore.ts` for user preferences
- Use `axios.isAxiosError(err)` when you need `err.response.status`; use bare `catch` (no binding) when the error value is never read
- Deduplicate concurrent Pinia async actions with `ref<Promise<T> | null>` — return the in-flight promise if one exists
- Debounce watchers that write to `sessionStorage`/`localStorage` — at least 1s timeout, cleared in `onBeforeUnmount`
