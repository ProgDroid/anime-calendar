---
name: Project clippy command (use this, not the generic one)
description: The actual clippy invocation this codebase is graded against — pedantic + nursery with a curated allow-list. Use instead of bare `cargo clippy --workspace -- -D warnings`.
type: reference
originSessionId: 75332392-b9dc-4e06-b274-05f71e4842d6
---
The project uses a custom clippy ruleset that's stricter than the default. **Always use this command instead of `cargo clippy --workspace -- -D warnings`** when verifying or briefing implementers:

```bash
cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::pedantic -W clippy::nursery -A clippy::missing_docs_in_private_items -A clippy::separated_literal_suffix -A clippy::implicit_return -A clippy::print_stderr -A clippy::exhaustive_enums -A clippy::exhaustive_structs -A clippy::single_char_lifetime_names -A clippy::missing_inline_in_public_items -A clippy::self_named_module_files -A clippy::wildcard_enum_match_arm -A clippy::pattern_type_mismatch -A clippy::std-instead-of-core
```

To auto-fix what's auto-fixable:

```bash
cargo clippy --fix --allow-dirty --workspace --all-targets -- -W clippy::all -W clippy::pedantic -W clippy::nursery -A clippy::missing_docs_in_private_items -A clippy::separated_literal_suffix -A clippy::implicit_return -A clippy::print_stderr -A clippy::exhaustive_enums -A clippy::exhaustive_structs -A clippy::single_char_lifetime_names -A clippy::missing_inline_in_public_items -A clippy::self_named_module_files -A clippy::wildcard_enum_match_arm -A clippy::pattern_type_mismatch -A clippy::std-instead-of-core
```

**On Windows, prefix with `AWS_LC_SYS_PREBUILT_NASM=1`** so the JWT crate's NASM dependency resolves.

**When dispatching implementer subagents:** include the full clippy command in their acceptance criteria. The bare `-D warnings` version passes silently while pedantic+nursery would flag dozens of issues — different bar entirely.

## Recurring gotchas this command surfaces in test code

These are the lints I keep tripping over in mapper / service tests; pre-empting them in new test code saves a clippy round-trip:

- **`cast_possible_wrap` on `len as i64`** — `Vec::len()` returns `usize`, casting to `i64` is flagged. In tests, write `i64::try_from(len).unwrap()`. (Or, if comparing to a `count: i64` from `SELECT COUNT(*)`, cast that down: `assert_eq!(usize::try_from(count).unwrap(), len)`.)
- **`cast_sign_loss` / `cast_possible_truncation`** when binding `usize` page params — the `CalendarMapper::get_calendars_by_user_paginated_with` block uses an explicit `#[allow(...)]` for these. Mirror that approach (fn-level allow with the three lints) rather than trying to cast clean.
