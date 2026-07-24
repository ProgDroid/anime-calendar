---
name: Verify type shapes (impls, derives, field names) before sketching code in implementer briefs
description: When briefing implementer subagents with code stubs, every type/method referenced in the stub must be verified against the source first. Sketching against assumed shapes wastes time when the implementer hits compile errors.
type: feedback
originSessionId: 75332392-b9dc-4e06-b274-05f71e4842d6
---
When dispatching an implementer subagent with embedded code stubs (e.g. test helpers, struct definitions), every type, method, derive, and field name in the stub MUST be verified against the actual source first. Sketches that assume shapes lead to compile errors the implementer has to track down — at which point the brief becomes noise rather than help.

**Why:** During Phase 0.4 (CachedAnilist adapter) on 2026-05-04, the brief sketched a `stub_item()` test helper using:
- `assert_eq!(items[0].id, id_a)` — but `common::id::Id` lacks `PartialEq`, so this didn't compile
- `id.to_int() as i64` — but `Id::to_int()` already returns `u64`, requiring a different cast
- `generate_search_key(&name, media_type.as_ref())` — but the function takes `Option<&str>`, not `Option<&Type>`
- `Type::Anime` — variant naming wasn't verified

The implementer adapted in all four cases (good reporting), but each adaptation cost back-and-forth and made the brief partially noise. A 60-second symbol check on each type would have produced an accurate stub.

**How to apply:**
- For every type referenced in a code stub: run `mcp__serena__find_symbol` with `include_body=true` to see the actual derives, fields, and methods. Cost: seconds.
- For every method called on a type: confirm it exists with the expected signature (especially return type — `to_int() -> u64` vs `-> i64` matters in cache keys).
- For helper functions referenced in stubs (e.g. `generate_search_key`): grep for the signature line, don't assume.
- Pay particular attention to:
  - `PartialEq` / `Eq` / `Debug` derives — assertions assume them
  - Numeric type widths (i32 vs i64 vs u64) — casts cluster at boundaries
  - Enum variant naming — `Anime` vs `ANIME` vs whatever
  - Visibility (pub vs pub(crate)) — `use crate::x::y` only works if `y` is reachable
- Faster than verifying everything: only sketch the structural shape (steps + file paths + acceptance criteria) and let the implementer fill in concrete syntax. Reserve embedded code stubs for cases where the syntax is non-obvious (proc macros, async-fn-in-trait edge cases).
