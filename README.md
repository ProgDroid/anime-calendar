# Anime Calendar

Track anime series and episodes across personal calendars and export schedules as iCalendar feeds.

## Overview

Anime Calendar is a self-hosted web application that lets users build calendars of anime (and manga) series sourced from [AniList](https://anilist.co). Once a calendar is created, it can be exported as an `.ics` file or subscribed to via a persistent iCal feed URL — making it easy to import into Google Calendar, Apple Calendar, or any standards-compliant calendar client.

Users register with email/password or sign in with Google OAuth. Each user can manage multiple calendars, search AniList for titles, and configure display preferences (title language, etc.).

## Prerequisites

- **Rust** 1.85+ (edition 2024)
- **Node.js** 20+ and npm
- **PostgreSQL** 14+
- **Redis** 6+
- **NASM** (required on Windows for the `aws-lc-sys` JWT backend)

## Quick Start

```bash
# Clone
git clone <repo-url>
cd anime-calendar

# --- Backend ---
# Copy and fill in config files
cp config.toml.dist config.toml        # server, Redis, JWT, Google OAuth
cp database.toml.dist database.toml    # PostgreSQL connection

# Build (Windows — requires NASM installed)
AWS_LC_SYS_PREBUILT_NASM=1 cargo build

# Run migrations, then start
cargo run

# --- Frontend ---
cd frontend
npm install
npm run dev   # Dev server at http://localhost:5173
```

## Configuration

### `config.toml`

| Key | Description | Default |
|-----|-------------|---------|
| `server.host` | Bind address | `127.0.0.1` |
| `server.port` | HTTP port | `8080` |
| `server.log_level` | Log verbosity (`debug`, `info`, `warn`, `error`) | `debug` |
| `server.jwt_secret` | Secret used to sign JWT tokens | — |
| `server.google_client_id` | Google OAuth client ID | — |
| `server.compress` | Enable gzip compression | `true` |
| `server.allowed_origins` | CORS allowed origins list | `["http://localhost:5173"]` |
| `server.redis.host` | Redis host | `127.0.0.1` |
| `server.redis.port` | Redis port | `6379` |
| `server.redis.password` | Redis password | — |
| `server.redis.db` | Redis database index | `0` |

### `database.toml`

PostgreSQL connection settings (host, port, user, password, database name).

### Frontend (`frontend/config.toml`)

| Key | Description |
|-----|-------------|
| `api.host` | Backend host | 
| `api.port` | Backend port |
| `api.protocol` | `http` or `https` |
| `google_client_id` | Google OAuth client ID (same as backend) |

## Building for Production

```bash
# Backend
AWS_LC_SYS_PREBUILT_NASM=1 cargo build --release

# Frontend
cd frontend
npm run build   # Output in frontend/dist/
```

The frontend build output (`dist/`) should be served by a reverse proxy (e.g., nginx) in front of the Actix-Web backend.

## Project Structure

```
server/src/
  cache.rs          — Redis wrapper with metrics and TTL constants
  config/           — Server + DB config structs
  controllers/      — HTTP handlers (auth, calendar, item, items, oauth, user, cache_metrics)
  entity/           — Domain structs (Calendar, User, UserSettings, Language)
  error.rs          — Unified error type (all variants return {"error":"..."} JSON)
  mappers/          — DB ↔ domain layer (repository pattern)
  middleware/       — JWT auth middleware (extracts Claims from Bearer token)
  services/         — Business logic (auth, calendar export, Google OAuth validation)
  server.rs         — App factory + route registration
  main.rs           — Entry point, dependency wiring

frontend/src/
  components/       — Page-level Vue components
    calendar/       — CalendarPage sub-components
    shared/         — ConfirmModal, MediaItemCard, PaginationControls
  composables/      — useCalendarSearch, useRecommendations, useWindowSize
  locales/          — en.json, pt.json (full i18n)
  router/           — Route definitions + auth guard
  services/         — applySettings, toastService, userSettingsService
  stores/           — auth.ts (Pinia), userSettingsStore.ts (Pinia)
  types/            — TypeScript type definitions

anilist/            — AniList GraphQL API client crate
common/             — Shared types (Calendar, Item, Language, etc.)
```

## API

All endpoints return `{"error": "..."}` JSON on failure. Protected endpoints require `Authorization: Bearer <token>`.

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/auth/register` | — | Register with email + password |
| `POST` | `/auth/login` | — | Login, returns JWT |
| `GET` | `/auth/me` | ✓ | Current user info |
| `GET` | `/auth/verify` | ✓ | Validate token |
| `POST` | `/auth/google` | — | Google ID token → JWT |
| `GET` | `/calendars` | ✓ | List calendars (paginated) |
| `GET` | `/calendars/:id` | ✓ | Get calendar with items |
| `PUT` | `/calendar` | ✓ | Create or update calendar |
| `DELETE` | `/calendars/:id` | ✓ | Delete calendar |
| `GET` | `/calendars/:id/export` | ✓ | Download `.ics` file |
| `GET` | `/calendars/subscription/:token` | — | Public iCal feed URL |
| `GET` | `/items/:id` | ✓ | Fetch single item from AniList |
| `GET` | `/items` | ✓ | Fetch multiple items by IDs |
| `GET` | `/items/search` | ✓ | Search AniList by name + media type |
| `GET` | `/user/details` | ✓ | User profile |
| `PUT` | `/user` | ✓ | Update profile |
| `PUT` | `/user/password` | ✓ | Change password |
| `DELETE` | `/user` | ✓ | Delete account |
| `GET` | `/user/settings` | ✓ | User preferences |
| `PUT` | `/user/settings` | ✓ | Update preferences |

## Development

### Running Tests

```bash
# Backend (Rust)
cargo test --workspace

# Frontend (Vitest)
cd frontend
npm run test:unit
```

### sqlx Offline Query Cache

sqlx verifies SQL queries at compile time. After changing any query, regenerate the cache so CI and teammates can build without a live database:

```bash
DATABASE_URL=postgresql://user:pass@host:port/dbname cargo sqlx prepare --workspace
# Commit the generated .sqlx/ directory
```

### Code Quality

```bash
# Backend
cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::pedantic -W clippy::nursery -A clippy::missing_docs_in_private_items -A clippy::separated_literal_suffix -A clippy::implicit_return -A clippy::print_stderr -A clippy::exhaustive_enums -A clippy::exhaustive_structs -A clippy::single_char_lifetime_names -A clippy::missing_inline_in_public_items -A clippy::self_named_module_files -A clippy::wildcard_enum_match_arm -A clippy::pattern_type_mismatch -A clippy::std-instead-of-core
cargo fmt --check

# Frontend
cd frontend
npm run lint    # oxlint + eslint
npm run build   # includes type-check via vue-tsc
```

## Rate Limiting

The API is rate-limited to **60 requests burst, 1 req/s steady** per IP (via `actix-governor`). This applies to all endpoints.

## License

MIT
