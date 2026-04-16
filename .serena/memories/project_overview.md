# anime-calendar Project Overview

Anime Calendar is a web app for tracking anime series across personal calendars with iCalendar export.

## Tech Stack
- Backend: Rust 2024 edition, Cargo workspace (server, anilist, common crates), Actix-Web 4, PostgreSQL via sqlx 0.8, Redis cache, JWT auth + Google OAuth, lettre for SMTP email
- Frontend: Vue 3.5 + TypeScript, rolldown-vite, Tailwind CSS v4 + DaisyUI v5, Pinia, vue-router 4, vue-i18n 11, axios, Vitest

## Key Commands
- Backend build: `AWS_LC_SYS_PREBUILT_NASM=1 cargo build` (Windows requires nasm for aws-lc-sys)
- Backend test: `cargo test` or `cargo test --package server`
- Backend code quality checks: `cargo clippy --package server --all-targets -- -W clippy::all -W clippy::pedantic -W clippy::nursery -A clippy::missing_docs_in_private_items -A clippy::separated_literal_suffix -A clippy::implicit_return -A clippy::print_stderr -A clippy::exhaustive_enums -A clippy::exhaustive_structs -A clippy::single_char_lifetime_names -A clippy::missing_inline_in_public_items -A clippy::self_named_module_files -A clippy::wildcard_enum_match_arm -A clippy::pattern_type_mismatch -A clippy::std-instead-of-core`
- Automatic clippy issue fixes: `cargo clippy --fix --allow-dirty`
- Frontend dev: `cd frontend && npm run dev`
- Frontend test: `cd frontend && npm run test:unit`
- sqlx offline cache: `DATABASE_URL=... cargo sqlx prepare --workspace` (commit .sqlx/ after)
- sqlx env var: set `SQLX_OFFLINE=true` to build without DB

## Architecture
- Dependency injection via actix-web `web::Data<T>` — all mappers, services wired at startup in server.rs/main.rs
- Mapper pattern: each mapper owns a DB pool clone, acts as repository layer
- All errors return JSON `{"error":"..."}`, unified via `error.rs` `ResponseError` impl
- httpOnly cookies for auth (SameSite=Strict, path=/api/)
- Anti-enumeration: unknown user returns same response as wrong password
