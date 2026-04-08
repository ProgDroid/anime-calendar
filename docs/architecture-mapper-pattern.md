# Architecture Note: Mapper Pattern as Repository Layer

## Pattern

Each data access concern is encapsulated in a dedicated Mapper struct (e.g., `UserMapper`, `CalendarMapper`, `UserSettingsMapper`). Each mapper owns a clone of the PostgreSQL connection pool and is injected into Actix handlers via `web::Data<T>`.

```rust
// Registered at startup in main.rs
let user_mapper = UserMapper::new(db_config.clone()).await?;

// Injected into handlers automatically by Actix
pub async fn get_user(
    user_mapper: web::Data<UserMapper>,
    ...
)
```

## Why This Is Unusual for Actix

The conventional Actix pattern is to inject `web::Data<PgPool>` directly into handlers and write queries inline or in free functions. Using mapper structs is closer to the Repository pattern from DDD — it groups related queries behind a typed interface and keeps handlers thin.

## Trade-offs

**Benefits:**
- Handlers don't need to know about SQL at all
- Each mapper is independently testable by constructing it with a test pool
- Adding a new data operation means adding a method to the relevant mapper, not scattering SQL across controllers

**Watch out for:**
- Each mapper holds its own pool clone — pool clones are cheap (they share the underlying connection pool), but if mappers proliferate it's worth confirming they all point to the same underlying pool
- The pattern can become awkward if a service needs to coordinate writes across multiple mappers transactionally — transactions would need to be passed across mapper boundaries or handled at a higher level
