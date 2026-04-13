use redis::{aio::MultiplexedConnection, Client, RedisResult};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Cache TTL for item lookups and single-resource responses (1 hour).
pub const CACHE_TTL_ITEM: u64 = 3600;
/// Cache TTL for search results (30 minutes).
pub const CACHE_TTL_SEARCH: u64 = 1800;
/// Cache TTL for calendar export responses (2 hours).
pub const CACHE_TTL_CALENDAR: u64 = 7200;

#[derive(Clone)]
pub struct Cache {
    connection: Arc<Mutex<MultiplexedConnection>>,
    metrics: Arc<Mutex<CacheMetrics>>,
}

impl Cache {
    /// Connect to a test Redis instance.  Host/port are read from the
    /// `REDIS_HOST` / `REDIS_PORT` environment variables, falling back to the
    /// development defaults.  Panics if the connection fails.
    #[cfg(test)]
    pub async fn for_tests() -> Self {
        let host =
            std::env::var("REDIS_HOST").unwrap_or_else(|_| "aegyptvault.local".to_owned());
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

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            metrics: Arc::new(Mutex::new(CacheMetrics::default())),
        })
    }

    /// # Errors
    /// Fails if Redis query fails.
    pub async fn get<T>(&self, key: &str) -> RedisResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        let value: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut *self.connection.lock().await)
            .await?;

        if let Some(v) = value {
            self.metrics.lock().await.increment_hit();
            if let Ok(parsed) = serde_json::from_str(&v) {
                Ok(Some(parsed))
            } else {
                self.metrics.lock().await.increment_miss();
                Ok(None) // Return None if deserialization fails
            }
        } else {
            self.metrics.lock().await.increment_miss();
            Ok(None)
        }
    }

    /// # Errors
    /// Fails if Redis query fails.
    pub async fn delete(&self, key: &str) -> RedisResult<()> {
        redis::cmd("DEL")
            .arg(key)
            .exec_async(&mut *self.connection.lock().await)
            .await?;
        Ok(())
    }

    /// # Errors
    /// Fails if Redis query fails.
    pub async fn exists(&self, key: &str) -> RedisResult<bool> {
        let result: i64 = redis::cmd("EXISTS")
            .arg(key)
            .query_async(&mut *self.connection.lock().await)
            .await?;
        Ok(result > 0)
    }

    /// # Errors
    /// Fails if Redis query fails.
    pub async fn get_keys(&self, pattern: &str) -> RedisResult<Vec<String>> {
        let mut keys: Vec<String> = Vec::new();
        let mut cursor: u64 = 0;
        loop {
            let (next_cursor, batch): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut *self.connection.lock().await)
                .await?;
            keys.extend(batch);
            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }
        Ok(keys)
    }

    /// # Errors
    /// Fails if Redis query fails.
    pub async fn flush(&self) -> RedisResult<()> {
        redis::cmd("FLUSHALL")
            .exec_async(&mut *self.connection.lock().await)
            .await?;
        Ok(())
    }

    /// # Errors
    /// Fails if Redis query fails or value cannot be serialised.
    pub async fn set<T>(&self, key: &str, value: &T, ttl_seconds: u64) -> RedisResult<()>
    where
        T: Serialize + Sync,
    {
        let serialized = serde_json::to_string(value).map_err(|e| {
            redis::RedisError::from((redis::ErrorKind::Io, "Serialization failed", e.to_string()))
        })?;

        redis::cmd("SET")
            .arg(key)
            .arg(serialized)
            .arg("EX")
            .arg(ttl_seconds)
            .exec_async(&mut *self.connection.lock().await)
            .await?;

        Ok(())
    }

    // Middleware-style caching function with metrics
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

    // Get cache metrics
    pub async fn get_metrics(&self) -> CacheMetrics {
        self.metrics.lock().await.clone()
    }

    // Reset cache metrics
    pub async fn reset_metrics(&self) {
        *self.metrics.lock().await = CacheMetrics::default();
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
        self.delete(&generate_calendar_items_key(calendar_id)).await?;
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
        let key = generate_user_paged_calendars_key(user_id);
        self.delete(&key).await?;
        Ok(())
    }

    // Monitor cache performance
    /// # Errors
    /// Fails if Redis query fails.
    pub async fn monitor_performance(
        &self,
    ) -> Result<CachePerformanceMetrics, Box<dyn std::error::Error>> {
        let metrics = self.get_metrics().await;
        let keys = self.get_keys("*").await?;

        let performance = CachePerformanceMetrics {
            hit_rate: metrics.cache_hit_rate,
            total_requests: metrics.total_requests,
            cache_size: keys.len(),
            hits: metrics.hits,
            misses: metrics.misses,
        };

        Ok(performance)
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

// Cache metrics structure with atomic counters for thread safety
#[derive(Debug, Clone, Serialize)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_requests: u64,
    pub cache_hit_rate: f64,
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            evictions: 0,
            total_requests: 0,
            cache_hit_rate: 0.0,
        }
    }
}

impl CacheMetrics {
    pub fn increment_hit(&mut self) {
        self.hits += 1;
        self.total_requests += 1;
        self.calculate_hit_rate();
    }

    pub fn increment_miss(&mut self) {
        self.misses += 1;
        self.total_requests += 1;
        self.calculate_hit_rate();
    }

    pub const fn increment_eviction(&mut self) {
        self.evictions += 1;
    }

    #[allow(clippy::cast_precision_loss)]
    fn calculate_hit_rate(&mut self) {
        if self.total_requests > 0 {
            self.cache_hit_rate = (self.hits as f64 / self.total_requests as f64) * 100.0;
        } else {
            self.cache_hit_rate = 0.0;
        }
    }
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

// Cache performance metrics structure
#[derive(Debug, Clone)]
pub struct CachePerformanceMetrics {
    pub hit_rate: f64,
    pub total_requests: u64,
    pub cache_size: usize,
    pub hits: u64,
    pub misses: u64,
}
