---
name: Grep one example of each layer before drafting an implementation plan
description: Plans drafted from a spec without inspecting the actual codebase produce code stubs that don't match real type signatures, naming conventions, or module layout — costing real time at execution
type: feedback
originSessionId: be1596ff-ca01-4f16-80bf-1a49c8d8e222
---
When drafting a multi-task implementation plan that introduces new entities / mappers / services / controllers, **read at least one existing example of each layer** before writing the per-task code stubs. Skipping this produces stubs that compile in your head but don't match reality, costing rework on every dispatch.

**Real example (2026-05-05 co-editor sharing plan):** the spec → plan handoff was drafted from the spec alone. Discovered at Task 0.4 dispatch time:

| Plan said | Reality | Cost if not caught |
|---|---|---|
| `i64` IDs | `i32` everywhere on Calendar | every mapper signature wrong |
| `cal.owner_id` | `cal.user_id` | every authorization check wrong |
| `entity/mod.rs` | `entity.rs` (flat module file) | first edit lands in the wrong place |
| `query_as!` + `FromRow` | `query!` + manual struct construction | violates established convention |
| `mappers own a PgPool` | mappers wrap a `Database` struct | mapper constructors won't compile |
| `_in_tx(conn, ...)` everywhere | `_with(conn, ...)` on some, `_in_tx` on others — mixed | stylistic friction, not breakage |
| `SubscriptionMapper::is_paid` | `EntitlementService::effective_tier(uid) -> Tier` | service-level call wrong abstraction |
| `seed_user(pool, ...)` test helper | `UserMapper::create_user_with(&mut *tx, ...)` | every test setup wrong |

Each line is 30 seconds of grep / file-read at plan-write time, hours of subagent rework time at execution.

**Why:** A spec captures intent and architecture. A plan needs both that AND fluency in the project's local idiom — type aliases, naming, module layout, test helpers, where existing services live. The plan author has to be the bridge.

**How to apply:** Before writing the per-task code stubs section of any plan:

1. **Grep one entity** in the target layer: ID types, derives, common enums, naming. (`server/src/entity/calendar.rs`)
2. **Grep one mapper** in the target layer: constructor pattern, query macro choice, transaction-helper naming, how `Database` / `PgPool` is held. (`server/src/mappers/calendar.rs`)
3. **Grep one service** if your plan introduces a new one: signature shape, what it composes, error types. (`server/src/services/entitlement.rs`)
4. **Grep `test_helpers.rs`** for what seed/fixture helpers actually exist. Don't invent `seed_user` if the convention is `Mapper::create_x_with(&mut *tx, ...)`.
5. **Find the module aggregator file** (`entity.rs` vs `entity/mod.rs` vs `lib.rs` re-exports) — flat-file aggregators are common in Rust 2018+ projects but easy to miss.
6. **Confirm any "existing service this plan depends on" exists** with the method name you're calling. `is_paid` was a phantom; `effective_tier` was the real method.

A 5-minute grep pass at plan-write time saves 30+ minutes of mid-flight correction across the dispatched subagents.

This generalises the lessons in `feedback_verify_before_assuming_code_shape`, `feedback_implementer_brief_must_verify_types`, and `feedback_grep_config_shape_before_planning` to **the plan-drafting step specifically**, with a concrete checklist.
