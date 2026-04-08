# Tech Debt: Unnecessary Mutex on MultiplexedConnection

## Location

`server/src/cache.rs` — `Cache` struct

```rust
pub struct Cache {
    connection: Arc<Mutex<MultiplexedConnection>>,
    ...
}
```

## The Issue

`redis::aio::MultiplexedConnection` is designed for concurrent use — it multiplexes multiple commands over a single TCP connection internally and is safe to clone and use from many tasks simultaneously. Wrapping it in `Arc<Mutex<...>>` serialises all Redis commands behind a lock, negating the concurrency benefit.

Every `cache.get()`, `cache.set()`, `cache.delete()` etc. acquires this lock, even though the underlying connection doesn't need it.

## Impact

Low to moderate under light load. Under higher concurrency (many simultaneous requests hitting cached endpoints) this becomes a bottleneck: tasks queue up waiting for the mutex rather than issuing commands in parallel.

## Fix (when ready to address)

`MultiplexedConnection` implements `Clone`. The idiomatic approach is:

```rust
pub struct Cache {
    connection: MultiplexedConnection,  // Clone it per use, or store as-is
    ...
}
```

Or store the `Client` and call `get_multiplexed_async_connection()` once, keeping the connection directly (it's already `Send + Sync`). Each clone shares the same underlying multiplexed pipeline.

The `metrics` field (`Arc<Mutex<CacheMetrics>>`) is fine — metrics need mutual exclusion since they're mutable shared state.

## Do Not Address Yet

Noted here for future reference. The current implementation is correct and safe; the mutex just limits throughput under load. Address when load testing reveals it as a bottleneck.
