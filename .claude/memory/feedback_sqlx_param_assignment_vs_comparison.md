---
name: sqlx::query! refuses same-param in SET + comparison contexts
description: Postgres PREPARE can't deduce a single type for a param used in both an UPDATE SET and a CASE WHEN comparison. sqlx::query! reports "inconsistent types deduced for parameter $N" — bind the comparison values as distinct extra parameters.
type: feedback
originSessionId: 76bbcb64-35b1-478f-a642-0b655f10cd65
---
**Rule:** When writing an UPDATE that assigns columns AND compares the new values against the stored row in the same statement (e.g. conditional `meta_version` bump), do **NOT** reuse the same `$N` placeholders in both contexts. Bind the comparison values as distinct extra params (`$6`, `$7`, `$8`) and pass the Rust values twice (`.clone()` for `String`).

**Why:** PostgreSQL's prepared-statement parameter type inference is global per param. When the same `$1` appears in:
- `SET name = $1` (assignment context — type pinned to the column's TEXT)
- `name <> $1` or `$1 IS DISTINCT FROM name` (comparison context — type only inferred from the operand pairing)

…the planner cannot reconcile the two and the macro errors with `error returned from database: inconsistent types deduced for parameter $1`. This holds even with explicit casts (`$1::TEXT`), even with `IS DISTINCT FROM`, even with table aliases (`UPDATE calendars c SET ... WHEN c.name <> $1`). None of those workarounds let the planner choose a single type.

**How to apply:** Switch to the duplicated-param shape:

```sql
UPDATE calendars SET
    name = $1,
    language = $2,
    event_style = $3,
    updated_at = CURRENT_TIMESTAMP,
    meta_version = meta_version + (CASE
        WHEN name <> $6 OR language <> $7 OR event_style <> $8
        THEN 1 ELSE 0
    END)
WHERE id = $4 AND user_id = $5 AND deleted_at IS NULL
RETURNING ...
```

```rust
sqlx::query!(SQL,
    calendar.name.clone(),       // $1 SET
    calendar.language as Language, // $2 SET (Copy, no clone)
    calendar.event_style.clone(),  // $3 SET
    calendar.id,                 // $4 WHERE
    calendar.user_id,            // $5 WHERE
    calendar.name,               // $6 comparison
    calendar.language as Language, // $7 comparison
    calendar.event_style,        // $8 comparison
)
```

`String` needs `.clone()` (not `Copy`); custom enums tagged `#[derive(Copy)]` like `Language` can be re-cast for free.

**Things that DO NOT fix it (don't waste time retrying):**
- `name <> $1::TEXT` → "the cast tells the planner $1 is unspecified"; conflicts with the SET context.
- `$1 IS DISTINCT FROM name` → operand reorder doesn't anchor the type any better than `<>`.
- `UPDATE calendars c ... WHEN c.name <> $1` → table alias is irrelevant; param scope is the whole statement.
- Wrapping the comparison in a CTE / sub-SELECT — same param, same problem.

**First seen:** Task 0.10 of co-editor-sharing (commit `bcdc5e4`), `mappers/calendar.rs::update_calendar` + `update_calendar_with`. Cost ~3 cargo-check cycles before settling on duplicated params.
