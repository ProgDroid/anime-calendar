---
name: Default shape for a brand-new mapper — instance + `_in_tx` delegation, every method
description: When writing a NEW mapper from scratch, structure every public method as a thin instance wrapper around a static `_in_tx(conn, ...)` helper, and use the `_in_tx` suffix (not `_with`). Pays for itself the moment you write your first test or the first cross-mapper composition.
type: feedback
originSessionId: 76e4f2f6-0be5-408d-b084-29d1ed2b8eab
---
When authoring a new mapper file, give every public method this shape by default — even the ones you think will only ever be called from a single controller:

```rust
pub async fn do_thing(&self, calendar_id: i32) -> ServerResult<u64> {
    crate::metrics::db::timed("calendar_editor.do_thing", async {
        Self::do_thing_in_tx(&mut *self.db.pool.acquire().await?, calendar_id).await
    })
    .await
}

pub(crate) async fn do_thing_in_tx(
    conn: &mut sqlx::PgConnection,
    calendar_id: i32,
) -> ServerResult<u64> {
    /* the actual SQL */
}
```

**Why:**
1. **Tests get free isolation.** Every test calls the `_in_tx` form against a `test_tx()` rollback-tx. Without the `_in_tx` form you're stuck either using `test_pool()` (which leaks rows across tests on the shared dev DB and silently breaks parallel runs), or writing per-test cleanup, or building a mapper from a pool that owns a transaction (you can't — `Database` owns the pool, not a tx). The cost is one delegation line; the alternative is hours of debugging cross-test pollution. Confirmed in Spec 2 / Phase 0.6 (CalendarEditorMapper) — 7 tests, all using `test_tx()`, no test setup boilerplate beyond two seed helpers.
2. **Cross-mapper composition is free when the next phase needs it.** The moment a controller needs to write to your mapper + another mapper inside one outer transaction, the `_in_tx` form is already there. No "oh I have to refactor my mapper now" detour mid-feature. Spec 2's invitation-accept flow needs `cap_check` + `INSERT calendar_editors` + `UPDATE calendar_invitations` all inside one `pg_advisory_xact_lock` transaction; having all three mappers expose `_in_tx` from day one means the controller is a 15-line composition, not a refactor.
3. **`metrics::db::timed` is centralised.** The instance method is the single place metrics get emitted, and you don't accidentally emit a duplicate metric when a test bypasses the instance.

**Suffix convention: `_in_tx` everywhere.** As of 2026-06-11 (audit H-17, commit `cc64ad6`) the `_with` suffix is **extinct** — all 37 former `*_with(conn, ...)` helpers across the 7 holdout mappers (`calendar`, `email_verification`, `password_reset`, `refresh_token`, `subscription`, `user`, `user_settings`) were renamed to `*_in_tx`. The codebase is now uniform. **Always use `_in_tx` for new conn-bound helpers; never `_with`.** If a recalled snippet references a `*_with` method name (e.g. `create_user_with`, `insert_calendar_with`, `get_calendar_by_id_with`), it is stale — the live name is the `_in_tx` form. (The only surviving `*_with` identifiers in the crate are Rust's std `starts_with`/`ends_with` in `services/ics_export.rs` — don't touch those.)

**Pool-bound caller with no mapper instance → associated `*_from_pool` fn.** When a call site holds a raw `web::Data<PgPool>` (e.g. the sharing / SSE controllers) rather than a mapper instance, don't inject a new `web::Data<Mapper>` just to reach an `&self` pool method — that trips the `feedback_actix_web_data_extractor_ordering` hazard (every test app, including 401 tests, must then register it). Instead add an **associated** fn taking the pool that delegates to the `_in_tx` form:
```rust
pub(crate) async fn get_by_id_any_owner_from_pool(
    pool: &sqlx::PgPool, calendar_id: i32,
) -> ServerResult<Calendar> {
    Self::get_by_id_any_owner_in_tx(&mut *pool.acquire().await?, calendar_id).await
}
```
Worked example: `CalendarMapper::get_by_id_any_owner_from_pool` (audit M-30, commit `cc64ad6`) replaced a raw-SQL `load_calendar_any_owner` helper that had been duplicated **in a controller** — keep calendar-fetch SQL in the mapper layer, expose a `*_from_pool` adapter for pool-only callers.

**How to apply:**
- New mapper file: every public method gets the two-block shape above. No exceptions for "this is just a getter, surely it's overkill."
- Any mapper getting a brand-new conn-bound method: name it `*_in_tx` (the suffix is now uniform crate-wide — see the suffix-convention note above).
- `count_*` and `restore_*` style helpers that are ONLY ever called inside a transaction (cap-counting, reconcile loops): expose ONLY the `_in_tx` form (no instance wrapper). Mark `#[allow(dead_code)]` with a `// First production caller lands in Phase X` comment until the controller appears — the dead-code warning is real (rustc doesn't count `#[cfg(test)]` callers as live), and silencing it explicitly with a phase pointer beats letting warnings rot during scaffolding phases.
- Tests use the `_in_tx` form directly with `test_tx()` (per `feedback_no_sqlx_test_use_test_pool`). Seeding uses existing mapper helpers (`UserMapper::create_user_in_tx`, `CalendarMapper::insert_calendar_in_tx` — post-H-17 names) on the same `&mut tx`. There is no project `seed_user`/`seed_calendar` helper.

**Concrete worked example:** `server/src/mappers/calendar_editor.rs` (commit `6216338`, 2026-05-05) — 8 methods, every public one delegates, two pure `_in_tx`-only helpers (`restore_for_calendar_in_tx`, `count_active_in_tx`) marked `#[allow(dead_code)]` until Phase 1/4 wires them.
