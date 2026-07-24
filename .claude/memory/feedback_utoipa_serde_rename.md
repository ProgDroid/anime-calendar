---
name: utoipa rename does NOT rename serde
description: Pair every utoipa #[param(rename = ...)] with #[serde(rename = ...)] or the API contract diverges from the OpenAPI doc
type: feedback
originSessionId: 7a8d378a-90cb-41a0-b043-addec4ba2a44
---
`#[param(rename = "x")]` is a utoipa-only attribute that updates the OpenAPI schema. It does NOT change how serde deserializes the field — that still uses the field's Rust name.

**Why:** The `/items` endpoint had `#[param(rename = "id")] ids: Vec<u64>` — utoipa documented the param as `id`, but actix-web-lab's `Query<T>` deserialized via serde under the field name `ids`. So `?id=X` returned 422 while `?ids=X` worked. Caught on 2026-05-01 during Track 2 Phase B browser smoke when `CalendarTile`'s cover-fetcher hit the documented contract and got 422s on every tile.

**How to apply:** Whenever adding `#[param(rename = "X")]` to a utoipa Params struct, also add `#[serde(rename = "X")]`. They configure two independent layers (docs vs deserializer) and must agree.
