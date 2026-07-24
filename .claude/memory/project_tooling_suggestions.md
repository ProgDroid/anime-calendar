---
name: Tooling suggestions from audit sessions
description: Skills, hooks, commands and agents recommended after the 2026-04-11 audit sessions, with implementation order
type: project
originSessionId: 6affb378-d634-41e7-873e-fb9eed26e218
---
Captured 2026-04-11. Pick up in next session.

## Suggested Implementation Order

### 1. i18n guard hook (project-local, PostToolUse:Edit on *.vue)
Warn when a hardcoded user-visible string is introduced in a Vue file without going through `$t()`. Simple regex: look for `"[A-Z]` or `'[A-Z]` patterns in the diff that aren't inside `$t(...)`. Prevents the "15 console.log" situation but for i18n violations.
- **Scope**: project hook in `.claude/settings.local.json` or equivalent
- **Effort**: low (shell one-liner or small script)

### 2. `/translate` command (project-local)
Takes a dot-notation i18n key + English string + Portuguese string and inserts them in the correct place in both `frontend/src/locales/en.json` and `frontend/src/locales/pt.json`. Eliminates the manual step and the risk of forgetting one locale file on every new feature.
- **Scope**: project command (`.claude/commands/translate.md` or similar)
- **Effort**: low-medium (needs JSON path insertion logic)

### 3. `console.log` warning hook (global, PostToolUse:Edit on *.ts, *.vue)
After editing any TypeScript or Vue file, check the written content for `console.log` and emit a warning. We removed 15 occurrences manually this session — better to catch them incrementally.
- **Scope**: global hook
- **Effort**: low (one-liner grep on file content)

### 4. `/docker-stack` skill (global)
Encodes the proven multi-stage Docker patterns from this project:
- Non-root user (`appuser` for Rust, `nginx` user for frontend)
- cargo-chef layer caching pattern for Rust builds
- `ARG` + `ENV` before `npm run build` for Vite `VITE_*` vars
- nginx SPA routing config (`try_files $uri /index.html`)
- docker-compose with `condition: service_healthy` dependency ordering
- postgres auto-init via `docker-entrypoint-initdb.d/` + `schema.sql`
- Config template `.dist` file pattern
- **Scope**: global skill (`~/.claude/skills/docker-stack/`)
- **Effort**: medium (mostly documentation of proven patterns)

### 5. `/rust-openapi` skill (global)
Step-by-step guide for adding utoipa OpenAPI annotations to an Actix-Web project:
1. Add `utoipa` + `utoipa-swagger-ui` deps to `server/Cargo.toml`
2. Add optional `utoipa` feature to shared crates (`common`)
3. Annotate shared types with `cfg_attr(feature = "utoipa", derive(ToSchema))` + field-level `#[schema(value_type = ...)]` for newtypes
4. Annotate controller types (`ToSchema` for bodies, `IntoParams` for query/path params)
5. Add `#[utoipa::path]` to each handler
6. Create `openapi.rs` with `ApiDoc` + `BearerAuth` modifier + smoke test
7. Register `SwaggerUi` in `server.rs`
8. Includes the three gotchas from `~/.claude/learnings/utoipa-v5-schema-gotchas.md`
- **Scope**: global skill (`~/.claude/skills/rust-openapi/`)
- **Effort**: medium-high (thorough write-up needed to be useful)

---

## Additional Suggestions (not in priority order above)

### sqlx prepare reminder hook (project-local, PostToolUse:Edit on *.rs)
Detect when a `sqlx::query!` macro is present in a file that was just edited, and print a reminder to regenerate `.sqlx/` with `cargo sqlx prepare --workspace`. The feedback memory covers the rule but a hook makes it automatic.
- **Scope**: project hook
- **Effort**: low

### `/rust-security-audit` skill (global)
Checklist skill that walks through Rust/Actix-specific security items:
- `unwrap()`/`expect()` in non-test code
- `eprintln!` instead of `log::error!`
- Auth enumeration leakage (uniform error responses)
- Redis `KEYS` vs `SCAN`
- Multi-table mutations without transactions
- JWT secret as plain `String` vs `SecretString`
- CORS `allow_any_origin()` in production
- Rate limiting presence check
Overlaps with `/cr --focus=security` but more Rust-specific.
- **Scope**: global skill
- **Effort**: low (mostly a checklist)

### Vue i18n compliance agent (global subagent)
A subagent that does a full scan of all `.vue` files looking for hardcoded user-visible strings not wrapped in `$t()`. More thorough than the hook (which is incremental). Useful as a one-off audit before a release.
- **Scope**: global agent definition
- **Effort**: medium (needs good heuristics to avoid false positives on class names, URLs, etc.)
