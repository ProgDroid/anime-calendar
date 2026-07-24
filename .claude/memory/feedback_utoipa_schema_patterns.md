---
name: utoipa v5 schema annotation patterns
description: Critical rules for annotating Rust types with utoipa v5 — newtype value_type, IntoParams vs ToSchema, invalid field attrs
type: feedback
originSessionId: 6affb378-d634-41e7-873e-fb9eed26e218
---
Three non-obvious constraints discovered when implementing OpenAPI annotations (2026-04-11):

**Rule 1: `value_type` is field-level only — cannot use it on the newtype struct itself.**

**Why:** utoipa v5's `#[schema(value_type = ...)]` is only valid on individual FIELDS, not at the struct level. Attempting `#[cfg_attr(feature = "utoipa", derive(ToSchema))] #[schema(value_type = i64)]` on a newtype like `pub struct Id(pub i64)` fails with "value_type is not a valid struct-level schema attribute".

**How to apply:** For `#[serde(transparent)]` newtypes like `Id` and `Timestamp`, do NOT annotate the newtype itself. Instead, annotate each field that uses it in containing structs:
```rust
// In Schedule, Item, Calendar, etc.:
#[cfg_attr(feature = "utoipa", schema(value_type = i64))]
pub id: Id,
#[cfg_attr(feature = "utoipa", schema(value_type = Option<i64>))]
pub media_id: Option<Id>,
```

---

**Rule 2: Query/path parameter structs need `IntoParams`, not `ToSchema`.**

**Why:** `params(MyStruct)` inside `#[utoipa::path]` requires `IntoParams` trait. Using `ToSchema` on the struct and referencing it in `params(...)` fails with a trait bound error.

**How to apply:** For structs used as query parameters (e.g. `PaginationParams`), derive `IntoParams` and add `#[into_params(parameter_in = Query)]` at struct level. These can also derive `ToSchema` if referenced in schemas(). Both are compatible.

---

**Rule 3: `description` is not a valid field-level `#[schema]` attribute.**

**Why:** utoipa v5 field-level `#[schema]` accepts: `value_type, format, example, nullable, rename, required, write_only, read_only, xml, default, deprecated, min_length, max_length, pattern, minimum, maximum` — but NOT `description`.

**How to apply:** Remove `description = "..."` from field-level `#[schema(...)]` annotations. Struct-level doc comments (`///`) are picked up automatically as the description.
