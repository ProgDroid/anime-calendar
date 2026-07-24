---
name: sqlx test pool clone for multiple mappers
description: When a single sqlx::test pool must be shared across multiple mapper instances in one test, use pool.clone() for all but the last
type: feedback
originSessionId: 9bdd3d50-f1c6-463c-9c96-c05ae6d37d9d
---
`#[sqlx::test]` injects a `PgPool` that is owned by the test. When two or more mappers (e.g., `UserMapper` and `RefreshTokenMapper`) both need the same pool, use `.clone()` on all but the final use:

```rust
#[sqlx::test(migrations = "../migrations")]
async fn my_test(pool: PgPool) {
    seed_user(&pool, "alice", "alice@test.com").await;  // &pool — borrow for setup

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))       // clone
            .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))       // move last
            ...
    ).await;
}
```

**Why:** `PgPool` is `Arc`-backed — clone is cheap (just an `Arc::clone`). The last use consumes the owned value directly. If `seed_user` only needs `&pool` (a shared reference), no clone is needed for the seed call.

**How to apply:** Whenever a new mapper is added to integration tests that already register `UserMapper`, change `UserMapper::from_pool(pool)` → `UserMapper::from_pool(pool.clone())` and append the new mapper using the original `pool`.
