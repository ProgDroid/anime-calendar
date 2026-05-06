use std::{
    collections::HashMap,
    sync::Arc,
};

use redis::{Client, aio::MultiplexedConnection};
use serde::Serialize;
use tokio::sync::{
    RwLock,
    broadcast::{self, Receiver, Sender},
    oneshot,
};

use crate::ServerResult;
use crate::error::Error;

/// Capacity of each per-channel broadcast channel.  Slow consumers that fall
/// more than `BROADCAST_CAPACITY` messages behind will receive a
/// `RecvError::Lagged` on their next `recv()` call.
const BROADCAST_CAPACITY: usize = 64;

/// Redis Pub/Sub wrapper.
///
/// - The publisher side uses a `MultiplexedConnection` (matching [`crate::cache::Cache`]).
/// - The subscriber side opens a dedicated async PubSub connection per unique
///   channel name and fans messages out to in-process `broadcast` channels so
///   multiple in-process consumers can share one Redis subscription.
///
/// `RedisPubSub` is [`Clone`]-able; all clones share the same subscriber map.
#[derive(Clone)]
pub struct RedisPubSub {
    publisher: MultiplexedConnection,
    subscribers: Arc<RwLock<HashMap<String, Sender<String>>>>,
    sub_conn_url: String,
}

impl RedisPubSub {
    /// Create a new `RedisPubSub` connected to `url`.
    ///
    /// # Errors
    /// Returns [`Error::Redis`] if the Redis connection cannot be established.
    pub async fn new(url: &str) -> ServerResult<Self> {
        let client = Client::open(url).map_err(|e| Error::Redis(e.to_string()))?;
        let publisher = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        Ok(Self {
            publisher,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            sub_conn_url: url.to_owned(),
        })
    }

    /// Publish `payload` (serialised as JSON) to `channel`.
    ///
    /// # Errors
    /// Returns [`Error::Redis`] if serialisation or the PUBLISH command fails.
    pub async fn publish<T: Serialize + Sync>(
        &self,
        channel: &str,
        payload: &T,
    ) -> ServerResult<()> {
        let json = serde_json::to_string(payload)
            .map_err(|e| Error::Redis(format!("serialisation failed: {e}")))?;

        let mut conn = self.publisher.clone();
        redis::cmd("PUBLISH")
            .arg(channel)
            .arg(&json)
            .exec_async(&mut conn)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        Ok(())
    }

    /// Subscribe to `channel` and return a [`Receiver`] that yields JSON
    /// strings for every message published to that channel.
    ///
    /// The first call for a given `channel` opens a dedicated Redis PubSub
    /// connection and spawns a background task that drives message delivery.
    /// The method waits until the Redis `SUBSCRIBE` handshake completes before
    /// returning, so callers can immediately publish without a race.
    /// Subsequent calls reuse the existing `broadcast::Sender`.
    ///
    /// # Errors
    /// Returns [`Error::Redis`] if the Redis PubSub connection cannot be
    /// established.
    pub async fn subscribe(&self, channel: &str) -> ServerResult<Receiver<String>> {
        // Fast path: broadcast channel already exists.
        {
            let map = self.subscribers.read().await;
            if let Some(tx) = map.get(channel) {
                return Ok(tx.subscribe());
            }
        }

        // Slow path: first subscriber for this channel — open a dedicated
        // PubSub connection and spawn the listener task.
        let mut map = self.subscribers.write().await;

        // Double-check after acquiring the write lock (another task may have
        // raced us to the slow path).
        if let Some(tx) = map.get(channel) {
            return Ok(tx.subscribe());
        }

        let (tx, rx) = broadcast::channel::<String>(BROADCAST_CAPACITY);
        map.insert(channel.to_owned(), tx.clone());
        drop(map);

        // `ready_tx` fires once the background task has called SUBSCRIBE on
        // Redis, so the caller knows it is safe to publish immediately after
        // `subscribe()` returns.
        let (ready_tx, ready_rx) = oneshot::channel::<()>();

        let url = self.sub_conn_url.clone();
        let channel_owned = channel.to_owned();
        let subscribers = self.subscribers.clone();

        tokio::spawn(async move {
            match Self::run_subscriber_task(&url, &channel_owned, tx, subscribers, ready_tx).await {
                Ok(()) => {}
                Err(e) => log::error!(
                    "redis_pubsub: subscriber task for channel '{}' exited with error: {}",
                    channel_owned,
                    e
                ),
            }
        });

        // Wait for the background task to confirm the SUBSCRIBE handshake.
        // If the task errors before sending ready the receiver will get an
        // error — surface it as a Redis error.
        ready_rx
            .await
            .map_err(|_| Error::Redis("subscriber task exited before becoming ready".to_owned()))?;

        Ok(rx)
    }

    /// Internal: drives a Redis PubSub connection for one channel until the
    /// broadcast sender is closed (no receivers remain).
    async fn run_subscriber_task(
        url: &str,
        channel: &str,
        tx: Sender<String>,
        subscribers: Arc<RwLock<HashMap<String, Sender<String>>>>,
        ready_tx: oneshot::Sender<()>,
    ) -> ServerResult<()> {
        let client = Client::open(url).map_err(|e| Error::Redis(e.to_string()))?;
        let mut pubsub = client
            .get_async_pubsub()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        pubsub
            .subscribe(channel)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        // Signal the caller that the SUBSCRIBE handshake is complete.
        // Ignore the error in case the caller dropped the receiver.
        let _ = ready_tx.send(());

        use futures_util::StreamExt as _;
        let mut stream = pubsub.on_message();

        loop {
            match stream.next().await {
                Some(msg) => {
                    let payload: String = match msg.get_payload() {
                        Ok(p) => p,
                        Err(e) => {
                            log::error!(
                                "redis_pubsub: failed to decode payload on channel '{}': {e}",
                                channel
                            );
                            continue;
                        }
                    };

                    if tx.send(payload).is_err() {
                        // All receivers dropped — no point keeping the
                        // connection alive.
                        log::info!(
                            "redis_pubsub: no receivers for channel '{}', closing subscriber",
                            channel
                        );
                        break;
                    }
                }
                None => {
                    // Stream ended (connection closed by server or dropped).
                    log::info!(
                        "redis_pubsub: message stream ended for channel '{}'",
                        channel
                    );
                    break;
                }
            }
        }

        // Clean up the entry from the subscriber map so the next call to
        // `subscribe()` will open a fresh connection.
        let mut map = subscribers.write().await;
        map.remove(channel);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_redis_url() -> String {
        let host =
            std::env::var("REDIS_HOST").unwrap_or_else(|_| "aegyptvault.local".to_owned());
        let port = std::env::var("REDIS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(2435_u16);
        format!("redis://{host}:{port}")
    }

    #[tokio::test]
    async fn publish_subscribe_roundtrip() {
        let url = test_redis_url();
        let ps = RedisPubSub::new(&url).await.unwrap();
        // subscribe() now blocks until the Redis SUBSCRIBE handshake completes,
        // so it is safe to publish immediately after it returns.
        let mut rx = ps.subscribe("test:chan").await.unwrap();
        ps.publish("test:chan", &serde_json::json!({"hello":"world"}))
            .await
            .unwrap();
        let msg = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            rx.recv(),
        )
        .await
        .expect("timeout")
        .expect("recv failed");
        assert!(msg.contains("hello"));
    }
}
