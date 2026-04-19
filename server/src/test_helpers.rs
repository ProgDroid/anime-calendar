/// Test infrastructure for transaction-based isolation.
///
/// Rather than using `#[sqlx::test]` (which creates a new Postgres database per
/// test and is limited by a 20-connection setup pool), mapper tests use a shared
/// `DATABASE_URL` database and wrap each test in a transaction that is always
/// rolled back.  This keeps tests fully isolated with zero per-test database
/// creation overhead.
///
/// # Usage
/// ```ignore
/// #[tokio::test]
/// async fn my_test() {
///     let mut tx = test_tx().await;
///     let user = UserMapper::create_user_with(&mut *tx, "alice", "a@b.com", None).await.unwrap();
///     // assertions ...
///     tx.rollback().await.unwrap(); // always roll back — no data persists
/// }
/// ```

/// Shared pool for controller integration tests.
///
/// Returns a connection pool backed by the same `DATABASE_URL` database used
/// for mapper tests.  Unlike `test_tx`, controller tests need a real pool
/// because the Actix test app injects mapper instances that acquire connections
/// internally.  Use random usernames/emails in each test to avoid unique-key
/// collisions when tests run in parallel.
#[cfg(test)]
pub(crate) async fn test_pool() -> sqlx::PgPool {
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set to run controller tests");
    sqlx::PgPool::connect(&url)
        .await
        .expect("Failed to connect to test database")
}

#[cfg(test)]
pub(crate) async fn test_tx() -> sqlx::Transaction<'static, sqlx::Postgres> {
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set to run mapper tests");
    let pool = sqlx::PgPool::connect(&url)
        .await
        .expect("Failed to connect to test database");
    pool.begin()
        .await
        .expect("Failed to begin test transaction")
}
