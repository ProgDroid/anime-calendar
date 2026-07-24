---
name: utoipa derives operationId from bare fn name — collisions go undetected until redocly lint
description: Same-named handlers across controllers collide silently; set operation_id explicitly when adding a new short-named handler
type: feedback
originSessionId: b675f889-963b-4b54-b10b-a6019774130c
---
utoipa's `#[utoipa::path]` macro derives the OpenAPI `operationId` from the **bare function name**, not the module-qualified path. So `controllers::item::get` and `controllers::public_config::get` both serialize as `"operationId": "get"` and trip redocly's `operation-operationId-unique` rule at CI time.

Local Rust compilation, `cargo test`, and the in-file `openapi_spec_generates_successfully` test all pass — only `npx @redocly/cli lint openapi.json` fails. The CI job is `.github/workflows/ci.yml` → `openapi:` → "Validate OpenAPI spec".

**Why:** `controllers::item::get` (preexisting) and `controllers::public_config::get` (added 2026-05-03 in commit `930739f`) collided. Looked obvious in hindsight; took ~5 minutes of redocly trace reading to find because the error message points at the path, not the duplicate operationId, and the duplicate is across files.

**How to apply:**
- When adding a new `#[utoipa::path]` handler with a generic short name (`get`, `list`, `create`, `delete`), set `operation_id = "get_<resource>"` explicitly in the macro. One-line fix; doesn't disturb the `service(module::get)` registration in `server.rs` or the `crate::controllers::module::get` reference in `openapi.rs`.
- Existing short-named handlers in `controllers::item`, `controllers::items`, `controllers::calendar` etc. are fine as long as no other module reuses the same fn name. If you ever need to add another `fn get` somewhere, set `operation_id` on whichever one is new — don't churn the existing ones.
- Reproduce locally before pushing: `cargo run --quiet --bin openapi-export > openapi.json && npx @redocly/cli@latest lint openapi.json`.
