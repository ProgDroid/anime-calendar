# Code Style and Conventions

## Rust (backend)
- All fallible public functions have `/// # Errors` doc comment
- `ServerResult<T>` = `Result<T, Error>` — use throughout server crate
- `#[must_use]` on pure functions; `const fn` where possible
- Tests in same file as code under test
- Use `log::error!` for errors, never `eprintln!`
- Multi-table DB mutations use explicit transactions with rollback
- All errors return `{"error":"..."}` JSON — never plain text
- Auth failures: always `Error::Unauthorised` regardless of user existence
- Dependency injection via `web::Data<T>` — mappers, services wired at startup

## Frontend
- All user-facing strings use `$t()` / `t()` — never hardcode text
- Add keys to BOTH en.json and pt.json
- Key naming: hierarchical, e.g. `auth.login.title`, top-level namespaces: `app`, `auth`, `calendar`, `calendars`, `userDetails`, `userSettings`, `errors`
- Public routes must declare `meta: { public: true }`
- Use `data-testid` for test selectors (not class names)
- Deduplicate concurrent Pinia async actions with `ref<Promise<T> | null>`
- Use `axios.isAxiosError(err)` for response status checks; bare `catch` when error value is never read
