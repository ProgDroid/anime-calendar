use redis::{Client, RedisResult, aio::MultiplexedConnection};
use serde::{Serialize, de::DeserializeOwned};

/// Cache TTL for item lookups and single-resource responses (1 hour).
pub const CACHE_TTL_ITEM: u64 = 3600;
/// Cache TTL for search results (30 minutes).
pub const CACHE_TTL_SEARCH: u64 = 1800;
/// Cache TTL for calendar export responses (2 hours).
pub const CACHE_TTL_CALENDAR: u64 = 7200;

#[derive(Clone)]
pub struct Cache {
    connection: MultiplexedConnection,
}

impl Cache {
    /// Connect to a test Redis instance.  Host/port are read from the
    /// `REDIS_HOST` / `REDIS_PORT` environment variables, falling back to the
    /// development defaults.  Panics if the connection fails.
    #[cfg(test)]
    /// # Panics
    /// If misconfigured
    pub async fn for_tests() -> Self {
        let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "aegyptvault.local".to_owned());
        let port = std::env::var("REDIS_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(2435_u16);
        Self::new(&host, port, "", 0)
            .await
            .expect("test Redis must be reachable")
    }

    /// # Errors
    /// Fails if Redis connection fails.
    pub async fn new(host: &str, port: u16, password: &str, _db: u8) -> RedisResult<Self> {
        let url = if password.is_empty() {
            format!("redis://{host}:{port}")
        } else {
            format!("redis://:{password}@{host}:{port}")
        };

        let client = Client::open(url)?;
        let connection = client.get_multiplexed_async_connection().await?;

        Ok(Self { connection })
    }

    /// # Errors
    /// Fails if Redis query fails.
    pub async fn get<T>(&self, key: &str) -> RedisResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        use crate::metrics::names::{
            CACHE_HITS_TOTAL, CACHE_MISSES_TOTAL, CACHE_OPERATION_DURATION_SECONDS, LABEL_OP,
        };
        let start = std::time::Instant::now();
        let mut conn = self.connection.clone();
        let result: RedisResult<Option<String>> =
            redis::cmd("GET").arg(key).query_async(&mut conn).await;

        metrics::histogram!(CACHE_OPERATION_DURATION_SECONDS, LABEL_OP => "get")
            .record(start.elapsed().as_secs_f64());

        result?.map_or_else(
            || {
                metrics::counter!(CACHE_MISSES_TOTAL).increment(1);
                Ok(None)
            },
            |v| {
                metrics::counter!(CACHE_HITS_TOTAL).increment(1);
                Ok(serde_json::from_str(&v).ok())
            },
        )
    }

    /// # Errors
    /// Fails if Redis query fails.
    pub async fn delete(&self, key: &str) -> RedisResult<()> {
        use crate::metrics::names::{CACHE_OPERATION_DURATION_SECONDS, LABEL_OP};
        let start = std::time::Instant::now();
        let result = async {
            let mut conn = self.connection.clone();
            redis::cmd("DEL").arg(key).exec_async(&mut conn).await
        }
        .await;
        metrics::histogram!(CACHE_OPERATION_DURATION_SECONDS, LABEL_OP => "delete")
            .record(start.elapsed().as_secs_f64());
        result
    }

    /// # Errors
    /// Fails if Redis query fails.
    pub async fn exists(&self, key: &str) -> RedisResult<bool> {
        let mut conn = self.connection.clone();
        let result: i64 = redis::cmd("EXISTS").arg(key).query_async(&mut conn).await?;
        Ok(result > 0)
    }

    /// # Errors
    /// Fails if Redis query fails.
    pub async fn get_keys(&self, pattern: &str) -> RedisResult<Vec<String>> {
        use crate::metrics::names::{CACHE_OPERATION_DURATION_SECONDS, LABEL_OP};
        let start = std::time::Instant::now();
        let result = async {
            let mut keys: Vec<String> = Vec::new();
            let mut cursor: u64 = 0;
            let mut conn = self.connection.clone();
            loop {
                let (next_cursor, batch): (u64, Vec<String>) = redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(pattern)
                    .arg("COUNT")
                    .arg(100)
                    .query_async(&mut conn)
                    .await?;
                keys.extend(batch);
                cursor = next_cursor;
                if cursor == 0 {
                    break;
                }
            }
            RedisResult::Ok(keys)
        }
        .await;
        metrics::histogram!(CACHE_OPERATION_DURATION_SECONDS, LABEL_OP => "scan")
            .record(start.elapsed().as_secs_f64());
        result
    }

    /// # Errors
    /// Fails if Redis query fails.
    pub async fn flush(&self) -> RedisResult<()> {
        use crate::metrics::names::{CACHE_OPERATION_DURATION_SECONDS, LABEL_OP};
        let start = std::time::Instant::now();
        let result = async {
            let mut conn = self.connection.clone();
            redis::cmd("FLUSHALL").exec_async(&mut conn).await
        }
        .await;
        metrics::histogram!(CACHE_OPERATION_DURATION_SECONDS, LABEL_OP => "flush")
            .record(start.elapsed().as_secs_f64());
        result
    }

    /// # Errors
    /// Fails if Redis query fails or value cannot be serialised.
    pub async fn set<T>(&self, key: &str, value: &T, ttl_seconds: u64) -> RedisResult<()>
    where
        T: Serialize + Sync,
    {
        use crate::metrics::names::{CACHE_OPERATION_DURATION_SECONDS, LABEL_OP};
        let start = std::time::Instant::now();
        let result = async {
            let serialized = serde_json::to_string(value).map_err(|e| {
                redis::RedisError::from((
                    redis::ErrorKind::Io,
                    "Serialization failed",
                    e.to_string(),
                ))
            })?;
            let mut conn = self.connection.clone();
            redis::cmd("SET")
                .arg(key)
                .arg(serialized)
                .arg("EX")
                .arg(ttl_seconds)
                .exec_async(&mut conn)
                .await
        }
        .await;
        metrics::histogram!(CACHE_OPERATION_DURATION_SECONDS, LABEL_OP => "set")
            .record(start.elapsed().as_secs_f64());
        result
    }

    /// # Errors
    /// Fails if Redis query fails.
    pub async fn cached_response<T, F, Fut>(
        &self,
        cache_key: &str,
        ttl_seconds: u64,
        fetch_fn: F,
    ) -> RedisResult<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = RedisResult<T>> + Send,
    {
        // Try to get from cache first
        if let Some(cached) = self.get::<T>(cache_key).await? {
            return Ok(cached);
        }

        // If not in cache, fetch fresh data
        let result = fetch_fn().await?;

        // Store in cache
        self.set(cache_key, &result, ttl_seconds).await?;

        Ok(result)
    }

    // Invalidate cache for a specific key
    /// # Errors
    /// Fails if Redis query fails.
    pub async fn invalidate(&self, key: &str) -> RedisResult<()> {
        self.delete(key).await
    }

    // Invalidate cache for a pattern (e.g., all calendar keys)
    /// # Errors
    /// Fails if Redis query fails.
    pub async fn invalidate_pattern(&self, pattern: &str) -> RedisResult<()> {
        let keys = self.get_keys(pattern).await?;
        for key in keys {
            self.delete(&key).await?;
        }
        Ok(())
    }

    // Invalidate cache for a specific calendar
    /// # Errors
    /// Fails if Redis query fails.
    pub async fn invalidate_calendar(&self, calendar_id: i32) -> RedisResult<()> {
        self.delete(&generate_calendar_key(calendar_id)).await?;
        self.delete(&generate_export_key(calendar_id)).await?;
        self.delete(&generate_calendar_items_key(calendar_id))
            .await?;
        Ok(())
    }

    /// # Errors
    /// Fails if Redis query fails.
    pub async fn invalidate_subscription(&self, token: &str) -> RedisResult<()> {
        self.delete(&format!("subscribe:{token}")).await
    }

    // Invalidate cache for a specific item
    /// # Errors
    /// Fails if Redis query fails.
    pub async fn invalidate_item(&self, item_id: i64) -> RedisResult<()> {
        let item_key = generate_item_key(item_id);
        self.delete(&item_key).await?;
        Ok(())
    }

    // Invalidate cache for search results
    /// # Errors
    /// Fails if Redis query fails.
    pub async fn invalidate_search(
        &self,
        query: &str,
        media_type: Option<&str>,
    ) -> RedisResult<()> {
        let search_key = generate_search_key(query, media_type);
        self.delete(&search_key).await?;
        Ok(())
    }

    // Invalidate cache for user settings
    /// # Errors
    /// Fails if Redis query fails
    pub async fn invalidate_user_settings(&self, user_id: i32) -> RedisResult<()> {
        let key = generate_user_settings_key(user_id);
        self.delete(&key).await?;
        Ok(())
    }

    // Invalidate cache for user details
    /// # Errors
    /// Fails if Redis query fails
    pub async fn invalidate_user_details(&self, user_id: i32) -> RedisResult<()> {
        let key = generate_user_details_key(user_id);
        self.delete(&key).await?;
        Ok(())
    }

    // Invalidate cache for user paged calendars
    /// # Errors
    /// Fails if Redis query fails
    pub async fn invalidate_user_paged_calendars(&self, user_id: i32) -> RedisResult<()> {
        let pattern = generate_user_paged_calendars_key(user_id);
        self.invalidate_pattern(&pattern).await
    }
}

#[must_use]
pub fn generate_calendar_key(id: i32) -> String {
    format!("calendar:{id}")
}

#[must_use]
pub fn generate_export_key(id: i32) -> String {
    format!("export:{id}")
}

#[must_use]
pub fn generate_calendar_items_key(id: i32) -> String {
    format!("calendar:items:{id}")
}

#[must_use]
pub fn generate_item_key(id: i64) -> String {
    format!("item:{id}")
}

#[must_use]
pub fn generate_search_key(query: &str, media_type: Option<&str>) -> String {
    media_type.map_or_else(
        || format!("search:{query}"),
        |type_str| format!("search:{query}:{type_str}"),
    )
}

#[must_use]
pub fn generate_items_key(ids: &[common::id::Id]) -> String {
    let id_string: Vec<String> = ids.iter().map(|id| id.to_int().to_string()).collect();
    format!("items:{}", id_string.join(","))
}

#[must_use]
pub fn generate_paginated_key(
    user_id: i32,
    endpoint: &str,
    page: usize,
    page_size: usize,
) -> String {
    format!("{user_id}:{endpoint}:page:{page}:size:{page_size}")
}

#[must_use]
pub fn generate_user_settings_key(user_id: i32) -> String {
    format!("user_settings:{user_id}")
}

#[must_use]
pub fn generate_user_details_key(user_id: i32) -> String {
    format!("user_details:{user_id}")
}

#[must_use]
pub fn generate_user_paged_calendars_key(user_id: i32) -> String {
    format!("{user_id}:calendars:page:*")
}

// Middleware trait for caching
pub trait Cacheable {
    fn cache_key(&self) -> String;
    fn cache_ttl(&self) -> u64;
}

// Cache middleware configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub ttl_seconds: u64,
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl_seconds: CACHE_TTL_ITEM,
            enabled: true,
        }
    }
}

impl CacheConfig {
    #[must_use]
    pub const fn new(ttl_seconds: u64) -> Self {
        Self {
            ttl_seconds,
            enabled: true,
        }
    }

    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            ttl_seconds: 3600,
            enabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a key unique to this test to avoid collisions when tests run in parallel.
    /// Each test passes its own descriptive tag as `test_id`.
    fn k(test_id: &str, suffix: &str) -> String {
        format!("test:{test_id}:{suffix}")
    }

    // ── Basic CRUD ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn set_and_get_round_trips_value() {
        let cache = Cache::for_tests().await;
        let key = k("set_get", "v");
        cache.set(&key, &42_u32, 300).await.unwrap();
        let result: Option<u32> = cache.get(&key).await.unwrap();
        cache.delete(&key).await.unwrap();
        assert_eq!(result, Some(42));
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let cache = Cache::for_tests().await;
        let key = k("missing", "v");
        cache.delete(&key).await.unwrap(); // ensure absent
        let result: Option<u32> = cache.get(&key).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn delete_removes_key() {
        let cache = Cache::for_tests().await;
        let key = k("delete", "v");
        cache.set(&key, &"hello", 300).await.unwrap();
        cache.delete(&key).await.unwrap();
        let result: Option<String> = cache.get(&key).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn exists_returns_true_when_key_present() {
        let cache = Cache::for_tests().await;
        let key = k("exists_true", "v");
        cache.set(&key, &1_u8, 300).await.unwrap();
        let present = cache.exists(&key).await.unwrap();
        cache.delete(&key).await.unwrap();
        assert!(present);
    }

    #[tokio::test]
    async fn exists_returns_false_when_key_absent() {
        let cache = Cache::for_tests().await;
        let key = k("exists_false", "v");
        cache.delete(&key).await.unwrap(); // ensure absent
        let present = cache.exists(&key).await.unwrap();
        assert!(!present);
    }

    #[tokio::test]
    async fn set_overwrites_existing_value() {
        let cache = Cache::for_tests().await;
        let key = k("overwrite", "v");
        cache.set(&key, &"first", 300).await.unwrap();
        cache.set(&key, &"second", 300).await.unwrap();
        let result: Option<String> = cache.get(&key).await.unwrap();
        cache.delete(&key).await.unwrap();
        assert_eq!(result.as_deref(), Some("second"));
    }

    // ── TTL ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn key_expires_after_ttl() {
        let cache = Cache::for_tests().await;
        let key = k("ttl", "v");
        cache.set(&key, &"ephemeral", 1).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        let result: Option<String> = cache.get(&key).await.unwrap();
        assert!(result.is_none(), "key should have expired after 1 s TTL");
    }

    // ── Metrics ────────────────────────────────────────────────────────────
    //
    // All three metric tests share ONE DebuggingRecorder installed via OnceLock.
    // Only one recorder can be global per process, so the recorder is installed
    // once and the snapshotter captures metrics emitted by any test in this binary.
    // Assertions only check for the *presence* of a metric name, not exact values,
    // so accumulated state from other tests does not cause false negatives.

    #[tokio::test]
    #[allow(clippy::mutable_key_type)]
    async fn get_on_hit_increments_hit_counter() {
        let _ = crate::test_helpers::shared_snapshotter(); // ensure recorder installed before ops
        let cache = Cache::for_tests().await;
        let key = k("metric_hit", "v");
        cache.set(&key, &1_u8, 300).await.unwrap();
        let _: Option<u8> = cache.get(&key).await.unwrap();
        cache.delete(&key).await.unwrap();

        let snapshot = crate::test_helpers::shared_snapshotter()
            .snapshot()
            .into_hashmap();
        assert!(
            snapshot
                .keys()
                .any(|k| k.key().name() == crate::metrics::names::CACHE_HITS_TOTAL),
            "cache_hits_total not recorded"
        );
    }

    #[tokio::test]
    #[allow(clippy::mutable_key_type)]
    async fn get_on_miss_increments_miss_counter() {
        let _ = crate::test_helpers::shared_snapshotter();
        let cache = Cache::for_tests().await;
        let key = k("metric_miss", "v");
        cache.delete(&key).await.unwrap();
        let _: Option<u8> = cache.get(&key).await.unwrap();

        let snapshot = crate::test_helpers::shared_snapshotter()
            .snapshot()
            .into_hashmap();
        assert!(
            snapshot
                .keys()
                .any(|k| k.key().name() == crate::metrics::names::CACHE_MISSES_TOTAL),
            "cache_misses_total not recorded"
        );
    }

    #[tokio::test]
    #[allow(clippy::mutable_key_type)]
    async fn set_emits_duration_histogram() {
        let _ = crate::test_helpers::shared_snapshotter();
        let cache = Cache::for_tests().await;
        let key = k("metric_set_dur", "v");
        cache.set(&key, &1_u8, 300).await.unwrap();
        cache.delete(&key).await.unwrap();

        let snapshot = crate::test_helpers::shared_snapshotter()
            .snapshot()
            .into_hashmap();
        assert!(
            snapshot
                .keys()
                .any(|k| k.key().name() == crate::metrics::names::CACHE_OPERATION_DURATION_SECONDS),
            "cache_operation_duration_seconds not recorded"
        );
    }

    // ── Invalidation ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn invalidate_calendar_removes_all_three_keys() {
        let cache = Cache::for_tests().await;
        // Use an ID unlikely to clash with real data in other tests
        let id = 99_991_i32;
        let cal_key = generate_calendar_key(id);
        let exp_key = generate_export_key(id);
        let items_key = generate_calendar_items_key(id);

        cache.set(&cal_key, &"cal", 300).await.unwrap();
        cache.set(&exp_key, &"exp", 300).await.unwrap();
        cache.set(&items_key, &"items", 300).await.unwrap();

        cache.invalidate_calendar(id).await.unwrap();

        assert!(
            !cache.exists(&cal_key).await.unwrap(),
            "calendar key should be gone"
        );
        assert!(
            !cache.exists(&exp_key).await.unwrap(),
            "export key should be gone"
        );
        assert!(
            !cache.exists(&items_key).await.unwrap(),
            "items key should be gone"
        );
    }

    #[tokio::test]
    async fn invalidate_subscription_removes_key() {
        let cache = Cache::for_tests().await;
        let token = "test-token-invalidate-sub";
        let key = format!("subscribe:{token}");
        cache.set(&key, &"cal", 300).await.unwrap();
        cache.invalidate_subscription(token).await.unwrap();
        assert!(!cache.exists(&key).await.unwrap());
    }

    #[tokio::test]
    async fn invalidate_item_removes_key() {
        let cache = Cache::for_tests().await;
        let id = 9_999_991_i64;
        let key = generate_item_key(id);
        cache.set(&key, &"item", 300).await.unwrap();
        cache.invalidate_item(id).await.unwrap();
        assert!(!cache.exists(&key).await.unwrap());
    }

    #[tokio::test]
    async fn invalidate_search_without_type_removes_key() {
        let cache = Cache::for_tests().await;
        let query = "naruto-test-notype";
        let key = generate_search_key(query, None);
        cache.set(&key, &"results", 300).await.unwrap();
        cache.invalidate_search(query, None).await.unwrap();
        assert!(!cache.exists(&key).await.unwrap());
    }

    #[tokio::test]
    async fn invalidate_search_with_type_removes_key() {
        let cache = Cache::for_tests().await;
        let query = "naruto-test-typed";
        let key = generate_search_key(query, Some("ANIME"));
        cache.set(&key, &"results", 300).await.unwrap();
        cache.invalidate_search(query, Some("ANIME")).await.unwrap();
        assert!(!cache.exists(&key).await.unwrap());
    }

    #[tokio::test]
    async fn invalidate_user_settings_removes_key() {
        let cache = Cache::for_tests().await;
        let user_id = 99_992_i32;
        let key = generate_user_settings_key(user_id);
        cache.set(&key, &"settings", 300).await.unwrap();
        cache.invalidate_user_settings(user_id).await.unwrap();
        assert!(!cache.exists(&key).await.unwrap());
    }

    #[tokio::test]
    async fn invalidate_user_details_removes_key() {
        let cache = Cache::for_tests().await;
        let user_id = 99_993_i32;
        let key = generate_user_details_key(user_id);
        cache.set(&key, &"details", 300).await.unwrap();
        cache.invalidate_user_details(user_id).await.unwrap();
        assert!(!cache.exists(&key).await.unwrap());
    }

    #[tokio::test]
    async fn invalidate_user_paged_calendars_removes_all_paginated_keys() {
        let cache = Cache::for_tests().await;
        let user_id = 99_994_i32;
        // Paginated keys follow the pattern "{user_id}:calendars:page:{n}:size:{m}"
        let k1 = generate_paginated_key(user_id, "calendars", 1, 10);
        let k2 = generate_paginated_key(user_id, "calendars", 2, 10);
        cache.set(&k1, &"p1", 300).await.unwrap();
        cache.set(&k2, &"p2", 300).await.unwrap();

        cache
            .invalidate_user_paged_calendars(user_id)
            .await
            .unwrap();

        assert!(
            !cache.exists(&k1).await.unwrap(),
            "page 1 key should be gone"
        );
        assert!(
            !cache.exists(&k2).await.unwrap(),
            "page 2 key should be gone"
        );
    }

    // ── SCAN loop correctness ───────────────────────────────────────────────

    #[tokio::test]
    async fn get_keys_returns_all_matching_keys() {
        let cache = Cache::for_tests().await;
        let prefix = "test:scan_all";
        let keys: Vec<String> = (1..=3).map(|i| format!("{prefix}:{i}")).collect();
        for k in &keys {
            cache.set(k, &1_u8, 300).await.unwrap();
        }

        let mut found = cache.get_keys(&format!("{prefix}:*")).await.unwrap();
        found.sort();

        for k in &keys {
            cache.delete(k).await.unwrap();
        }

        let mut expected = keys;
        expected.sort();
        assert_eq!(found, expected);
    }

    #[tokio::test]
    async fn get_keys_excludes_non_matching_keys() {
        let cache = Cache::for_tests().await;
        let prefix = "test:scan_filter";
        let yes1 = format!("{prefix}:yes1");
        let yes2 = format!("{prefix}:yes2");
        let no = "test:scan_other:no";

        cache.set(&yes1, &1_u8, 300).await.unwrap();
        cache.set(&yes2, &1_u8, 300).await.unwrap();
        cache.set(no, &1_u8, 300).await.unwrap();

        let found = cache.get_keys(&format!("{prefix}:*")).await.unwrap();

        cache.delete(&yes1).await.unwrap();
        cache.delete(&yes2).await.unwrap();
        cache.delete(no).await.unwrap();

        assert!(found.contains(&yes1), "yes1 should be in results");
        assert!(found.contains(&yes2), "yes2 should be in results");
        assert!(
            !found.contains(&no.to_owned()),
            "non-matching key should be absent"
        );
    }

    #[tokio::test]
    async fn get_keys_returns_empty_when_no_match() {
        let cache = Cache::for_tests().await;
        let found = cache.get_keys("test:no_such_prefix_xyzzy:*").await.unwrap();
        assert!(found.is_empty());
    }

    // ── cached_response middleware ───────────────────────────────────────────

    #[tokio::test]
    async fn cached_response_on_miss_invokes_fetch_and_stores_result() {
        let cache = Cache::for_tests().await;
        let key = k("cr_miss", "v");
        cache.delete(&key).await.unwrap();

        let result = cache
            .cached_response::<u32, _, _>(&key, 300, || async { Ok(42_u32) })
            .await
            .unwrap();
        assert_eq!(result, 42);

        // Value should now be cached
        let cached: Option<u32> = cache.get(&key).await.unwrap();
        cache.delete(&key).await.unwrap();
        assert_eq!(cached, Some(42), "result should have been stored in cache");
    }

    #[tokio::test]
    async fn cached_response_on_hit_returns_cached_value_not_fetch_fn_result() {
        let cache = Cache::for_tests().await;
        let key = k("cr_hit", "v");
        cache.set(&key, &99_u32, 300).await.unwrap();

        // fetch_fn returns 0, but cached value is 99 — cached value wins
        let result: u32 = cache
            .cached_response(&key, 300, || async { Ok(0_u32) })
            .await
            .unwrap();

        cache.delete(&key).await.unwrap();
        assert_eq!(
            result, 99,
            "should return cached value, not fetch_fn result"
        );
    }
}
