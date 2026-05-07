use redis::{Client, aio::MultiplexedConnection};
use secrecy::{ExposeSecret as _, SecretString};
use serde::{Deserialize, Serialize};

use crate::{ServerResult, services::calendar_events::Viewer};

/// Private Redis-stored shape — not the wire shape.
#[derive(Serialize, Deserialize)]
struct ViewerStored {
    display: String,
}

/// Tracks which users are currently viewing a calendar via Redis TTL keys.
///
/// Each heartbeat sets a key `presence:cal:{calendar_id}:user:{user_id}` with
/// a configured TTL.  `list` uses a SCAN cursor loop (never KEYS) to enumerate
/// active viewers.
///
/// `PresenceService` is [`Clone`]-able because `MultiplexedConnection` is
/// `Clone` (internally `Arc`-backed); all clones share the same connection.
#[derive(Clone)]
pub struct PresenceService {
    redis: MultiplexedConnection,
    ttl_seconds: u64,
}

impl PresenceService {
    /// Connect to Redis and return a ready [`PresenceService`].
    ///
    /// # Errors
    /// Returns [`crate::error::Error::Redis`] if the connection cannot be
    /// established.
    pub async fn new(host: &str, port: u16, password: &str, ttl_seconds: u64) -> ServerResult<Self> {
        let pw = SecretString::from(password.to_owned());
        let url = if pw.expose_secret().is_empty() {
            format!("redis://{host}:{port}")
        } else {
            format!("redis://:{pw_val}@{host}:{port}", pw_val = pw.expose_secret())
        };
        let client = Client::open(url).map_err(|e| crate::error::Error::Redis(e.to_string()))?;
        let redis = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| crate::error::Error::Redis(e.to_string()))?;
        Ok(Self { redis, ttl_seconds })
    }

    /// Refresh viewer heartbeat. Sets the presence key with the configured TTL
    /// and returns the current viewer list for this calendar.
    ///
    /// # Errors
    /// Returns [`crate::error::Error::Redis`] if the SET or SCAN fails.
    pub async fn heartbeat(
        &self,
        calendar_id: i32,
        user_id: i32,
        display: &str,
    ) -> ServerResult<Vec<Viewer>> {
        let key = format!("presence:cal:{calendar_id}:user:{user_id}");
        let val = serde_json::to_string(&ViewerStored {
            display: display.to_owned(),
        })
        .map_err(|e| crate::error::Error::Redis(format!("serialize: {e}")))?;
        let mut conn = self.redis.clone();
        redis::cmd("SET")
            .arg(&key)
            .arg(&val)
            .arg("EX")
            .arg(self.ttl_seconds)
            .exec_async(&mut conn)
            .await
            .map_err(|e| crate::error::Error::Redis(e.to_string()))?;
        self.list(calendar_id).await
    }

    /// List all viewers currently active on a calendar (SCAN cursor loop —
    /// never KEYS).
    ///
    /// # Errors
    /// Returns [`crate::error::Error::Redis`] on scan/get failure.
    pub async fn list(&self, calendar_id: i32) -> ServerResult<Vec<Viewer>> {
        let mut conn = self.redis.clone();
        let pattern = format!("presence:cal:{calendar_id}:user:*");
        let mut cursor: u64 = 0;
        let mut viewers = Vec::new();
        loop {
            let (next, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(50)
                .query_async(&mut conn)
                .await
                .map_err(|e| crate::error::Error::Redis(e.to_string()))?;
            for key in keys {
                if let Ok(raw) = redis::cmd("GET").arg(&key).query_async::<String>(&mut conn).await
                    && let Ok(stored) = serde_json::from_str::<ViewerStored>(&raw)
                {
                    // Extract user_id from the key suffix `…:user:{id}`.
                    let Some(uid) = key
                        .rsplit(':')
                        .next()
                        .and_then(|s| s.parse::<i32>().ok())
                    else {
                        log::warn!("presence: unparseable key suffix in '{key}'; skipping");
                        continue;
                    };
                    viewers.push(Viewer {
                        user_id: uid,
                        display: stored.display,
                    });
                }
            }
            if next == 0 {
                break;
            }
            cursor = next;
        }
        Ok(viewers)
    }

    /// Remove a viewer from presence (e.g. on disconnect).
    ///
    /// # Errors
    /// Returns [`crate::error::Error::Redis`] on DEL failure.
    pub async fn drop_viewer(&self, calendar_id: i32, user_id: i32) -> ServerResult<()> {
        let mut conn = self.redis.clone();
        redis::cmd("DEL")
            .arg(format!("presence:cal:{calendar_id}:user:{user_id}"))
            .exec_async(&mut conn)
            .await
            .map_err(|e| crate::error::Error::Redis(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_redis_params() -> (String, u16) {
        let host =
            std::env::var("REDIS_HOST").unwrap_or_else(|_| "aegyptvault.local".to_owned());
        let port = std::env::var("REDIS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(2435_u16);
        (host, port)
    }

    #[tokio::test]
    async fn heartbeat_and_list_roundtrip() {
        let (host, port) = test_redis_params();
        let svc = PresenceService::new(&host, port, "", 30)
            .await
            .expect("test Redis must be reachable");

        let calendar_id = 999_999_i32;
        let user_id = 42_i32;

        // Clean up any leftover key from a previous run.
        svc.drop_viewer(calendar_id, user_id).await.unwrap();

        let viewers = svc
            .heartbeat(calendar_id, user_id, "TestUser")
            .await
            .unwrap();
        assert!(
            viewers.iter().any(|v| v.user_id == user_id && v.display == "TestUser"),
            "expected TestUser in viewer list; got {viewers:?}"
        );

        // list() independently should return the same result.
        let listed = svc.list(calendar_id).await.unwrap();
        assert!(
            listed.iter().any(|v| v.user_id == user_id),
            "list() should return the viewer; got {listed:?}"
        );

        // drop_viewer removes the entry.
        svc.drop_viewer(calendar_id, user_id).await.unwrap();
        let after_drop = svc.list(calendar_id).await.unwrap();
        assert!(
            !after_drop.iter().any(|v| v.user_id == user_id),
            "viewer should be gone after drop; got {after_drop:?}"
        );
    }
}
