---
name: Project Conventions
description: Coding conventions and patterns observed in the codebase
type: project
originSessionId: f6df5aa5-37d3-4dba-9061-556e785f1dad
---
Patterns to follow when writing code for this project.

**Why:** Observed from the existing codebase; deviating causes inconsistency.

**How to apply:** Match these patterns when adding or modifying code.

## Backend (Rust)
- All fallible public functions annotated with `/// # Errors` doc comment
- `ServerResult<T>` = `Result<T, Error>` type alias used throughout server crate
- Dependency injection via Actix `web::Data<T>` - mappers, cache, anilist, config all injected
- Mapper structs (UserMapper, CalendarMapper, etc.) own DB connection pool; passed as `web::Data`
- Tests go in the same file as the code being tested
- `#[must_use]` on pure functions; prefer `const fn` where possible
- Use `log::error!` for error logging — never `eprintln!` (it bypasses the logging infra)
- All DB mutations that touch multiple tables must be wrapped in a transaction with explicit rollback
- Redis key scanning: always use `SCAN` cursor loop, never `KEYS` (blocks Redis on large keyspaces)
- All `Error` variants return `{"error":"..."}` JSON via `error_response()` — never plain text
- Return uniform `Error::Unauthorised` for any auth failure (no leaking "user not found" vs "wrong password")
- Rate limiting: `actix-governor` is wired in `server.rs`; config built BEFORE `HttpServer::new` due to lifetime constraints
- CORS: configured via `allowed_origins: Vec<String>` in `ServerConfig`; empty list falls back to `allow_any_origin()`
- sqlx compile-time queries require `DATABASE_URL` env var or offline `.sqlx` cache — set it before `cargo check`

## Frontend (Vue / TypeScript)
- All routes lazy-loaded via dynamic `import()`
- Auth guard in `router/index.ts` `beforeEach` hook
- Public routes (login) must have `meta: { public: true }`; `fetchSettings` is skipped for them
- Settings fetched on navigation (non-public routes only), applied immediately via `applySettings`
- All user-facing strings use `$t()` / `t()` — never hardcode strings
- Translation keys must be added to BOTH `en.json` and `pt.json`
- i18n namespaces: `app`, `auth` (login/register/google), `calendar`, `calendars`, `userDetails`, `userSettings`, `errors` — do NOT use `components.*` prefix
- Error/success key suffix convention: `*Failed` for errors, `*Success` for success
- Never use native `confirm()` — use `shared/ConfirmModal.vue` with `v-model` + `@confirm`
- CalendarPage sub-components live in `components/calendar/`; reusable shared components in `components/shared/`
- Pinia stores: `auth.ts` (auth state), `userSettingsStore.ts` (settings)
- Deduplicate concurrent async calls in Pinia stores with `ref<Promise<T> | null>` — return in-flight promise instead of firing a second request
- Catch bindings: use `axios.isAxiosError(err)` when you need `err.response.status`; use bare `catch` when the error value is never read
- Debounce any watcher that writes to `sessionStorage`/`localStorage` — at least 1s; store timer in a module-level `let` and clear it in `onBeforeUnmount`

## Config
- `config.toml` for server/Redis/JWT/OAuth (copy from `config.toml.dist`)
- `database.toml` for PostgreSQL connection (copy from `database.toml.dist`)
- On Windows, build with `AWS_LC_SYS_PREBUILT_NASM=1 cargo build` (requires nasm.exe)
