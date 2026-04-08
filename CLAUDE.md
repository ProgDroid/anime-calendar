# anime-calendar — Claude Code Project Guide

## Project Overview

Anime Calendar is a web application for tracking anime series and episodes across personal calendars. Users register, create calendars, add anime from Anilist, and export schedules as iCalendar files.

## Tech Stack

### Backend
- **Language**: Rust (edition 2024), Cargo workspace (`server`, `anilist`, `common` crates)
- **Framework**: Actix-Web 4 + actix-cors
- **Database**: PostgreSQL via sqlx 0.8 (`runtime-tokio`, `tls-native-tls`, `postgres`, `chrono` features)
- **Cache**: Redis via `redis` crate 1.0 (`aio`, `tokio-comp`) — custom `Cache` struct in `server/src/cache.rs`
- **Auth**: JWT (`jsonwebtoken` 10 with `aws_lc_rs` backend) + Google OAuth (`google-oauth` crate)
- **Passwords**: argon2
- **Calendar export**: `icalendar` 0.17
- **Config**: `config` crate reading `config.toml` (server/Redis/JWT/OAuth) and `database.toml` (PostgreSQL)

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

```
server/src/
  cache.rs          — Redis wrapper with metrics + key generators
  config/           — Server + DB config structs
  controllers/      — HTTP handlers (auth, calendar, item, items, oauth, user, cache_metrics)
  entity/           — Domain structs (Calendar, User, UserSettings, Language enum)
  error.rs          — Unified error type
  mappers/          — DB <-> domain layer (UserMapper, CalendarMapper, UserSettingsMapper, etc.)
  middleware/       — JWT auth middleware
  services/         — Business logic (auth, calendar_export, google_oauth)
  server.rs         — App factory + route registration
  main.rs           — Entry point, wires dependencies

frontend/src/
  components/       — Page-level Vue components (LoginPage, MyCalendarsPage, CalendarPage, etc.)
  locales/          — en.json, pt.json
  router/index.ts   — Route definitions + auth guard + settings fetch on navigation
  services/         — applySettings.ts, toastService.ts, userSettingsService.ts
  stores/           — auth.ts, userSettingsStore.ts
  types/            — TypeScript type definitions

anilist/            — Anilist API client crate
common/             — Shared types (calendar, item, id, language, schedule, etc.)
docs/               — Architecture and implementation notes
```

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

### Frontend
- All user-facing strings use `$t()` / `t()` — never hardcode text in components
- When adding translatable text, add the key to **both** `en.json` and `pt.json`
- Translation key naming: hierarchical, e.g. `components.login.title`, `userSettings.language`
- Auth guard lives in `router/index.ts` `beforeEach` — settings fetched on every navigation
- Pinia stores: `auth.ts` for auth state, `userSettingsStore.ts` for user preferences

## API Routes

```
POST   /auth/login
POST   /auth/register
GET    /auth/me
GET    /auth/verify
POST   /auth/google

GET    /calendars          (paginated)
GET    /calendars/:id
PUT    /calendars
DELETE /calendars/:id
GET    /calendars/:id/export

GET    /items/:id
GET    /items              (by IDs)
GET    /items/search

GET    /user/details
PUT    /user
DELETE /user
PUT    /user/password
GET    /user/settings
PUT    /user/settings

GET    /cache/metrics
GET    /cache/performance
GET    /cache/health
GET    /cache/stats
POST   /cache/reset
POST   /cache/flush
```

## Implementation Status

Core features are complete:
- User auth (JWT + Google OAuth)
- Calendar CRUD + iCalendar export
- Anilist item search and fetch
- Redis caching (calendars, items, search, user settings, paginated lists) with TTL + invalidation
- Cache metrics/monitoring endpoints
- User profile management + settings
- i18n (English + Portuguese)
- Responsive frontend (DaisyUI + Tailwind)

## Known TODOs (from source)
- Rate limiting for API endpoints
- More backend tests (controllers + services)
- Frontend component refactoring
- Episode-specific times (not just all-day calendar entries)
- CORS config tightening (currently allow_any_*)
- Load testing
