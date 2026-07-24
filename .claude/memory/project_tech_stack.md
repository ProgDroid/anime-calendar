---
name: Tech Stack (Actual)
description: Actual technology stack - corrects outdated Cline docs which claimed SQLite and "no implementation"
type: project
---

Backend is Rust with Actix Web, PostgreSQL (not SQLite), and Redis for caching.
Frontend is Vue 3 + TypeScript + Tailwind CSS v4 + DaisyUI v5 + Pinia + vue-i18n v11.

**Why:** Cline docs originally said SQLite - that was wrong. sqlx in Cargo.toml uses the postgres feature, config has Redis settings, database.toml is the postgres config.

**How to apply:** Never suggest SQLite migrations or adapters. Caching invalidation goes through Redis. Use sqlx postgres query patterns.

## Backend
- Rust edition 2024, Cargo workspace (`server`, `anilist`, `common` crates)
- actix-web 4, actix-cors
- sqlx 0.8 with postgres + chrono features
- Redis 1.0 (aio, tokio-comp features) via custom `Cache` struct in `server/src/cache.rs`
- JWT auth (jsonwebtoken 10 with aws_lc_rs) + Google OAuth (google-oauth crate)
- argon2 for password hashing
- icalendar 0.17 for calendar export
- Config loaded from `config.toml` (see `config.toml.dist` for template), DB from `database.toml`

## Frontend
- Vue 3.5 + TypeScript 5.9
- Vite (rolldown-vite), @vitejs/plugin-vue
- Tailwind CSS v4 (@tailwindcss/vite plugin), DaisyUI v5
- Pinia 3 for state management
- vue-router 4 with lazy-loaded routes
- vue-i18n 11 (English + Portuguese locales in `frontend/src/locales/`)
- Vitest + @vue/test-utils for unit tests
- oxlint + eslint + prettier for linting/formatting

## Structure
- `server/src/controllers/` - HTTP handlers (auth, calendar, item, items, oauth, user, cache_metrics)
- `server/src/services/` - business logic
- `server/src/mappers/` - DB <-> domain transformations (UserMapper, CalendarMapper, UserSettingsMapper, etc.)
- `server/src/entity/` - domain structs
- `server/src/cache.rs` - Redis cache wrapper with metrics
- `frontend/src/components/` - page-level Vue components
- `frontend/src/stores/` - Pinia stores (auth, userSettingsStore)
- `frontend/src/services/` - API communication + settings application
