use std::{collections::HashMap, sync::Arc};

use futures_util::StreamExt as _;
use redis::{Client, aio::MultiplexedConnection};
use secrecy::{ExposeSecret as _, SecretString};
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

/// Initial back-off delay (seconds) after a subscriber task disconnect.
const BACKOFF_START_SECS: u64 = 1;
/// Maximum back-off delay (seconds) between reconnect attempts.
const BACKOFF_MAX_SECS: u64 = 30;

/// Build a Redis URL from its components.  The password is exposed only
/// transiently inside this function and never written to a struct field or log.
fn build_url(host: &str, port: u16, password: &SecretString) -> String {
    let pw = password.expose_secret();
    if pw.is_empty() {
        format!("redis://{host}:{port}")
    } else {
        format!("redis://:{pw}@{host}:{port}")
    }
}

/// Redis Pub/Sub wrapper.
///
/// - The publisher side uses a `MultiplexedConnection` (matching [`crate::cache::Cache`]).
/// - The subscriber side opens a dedicated async PubSub connection per unique
///   channel name and fans messages out to in-process `broadcast` channels so
///   multiple in-process consumers can share one Redis subscription.
/// - Subscriber tasks reconnect automatically with exponential back-off after a
///   Redis disconnect.
///
/// `RedisPubSub` is [`Clone`]-able; all clones share the same subscriber map.
/// The Redis password is stored as [`SecretString`] so it is zeroed on drop
/// and never appears in `Debug` output.
#[derive(Clone)]
pub struct RedisPubSub {
    publisher: MultiplexedConnection,
    subscribers: Arc<RwLock<HashMap<String, Sender<String>>>>,
    /// Connection URL, kept so subscriber tasks can reconnect after a drop.
    /// Held as [`SecretString`] because it embeds the password — same
    /// protection the separate password field used to provide, now covering
    /// the whole string.
    url: SecretString,
}

impl RedisPubSub {
    /// Create a new `RedisPubSub` that connects to `host:port`.
    ///
    /// The URL is assembled internally; the password is never stored as a plain
    /// `String` field.
    ///
    /// # Errors
    /// Returns [`Error::Redis`] if the publisher connection cannot be
    /// established.
    pub async fn new(host: &str, port: u16, password: &str) -> ServerResult<Self> {
        let password = SecretString::from(password.to_owned());
        Self::from_url(&build_url(host, port, &password)).await
    }

    /// Create a `RedisPubSub` from a full Redis URL. `rediss://` selects TLS,
    /// which managed providers generally require.
    ///
    /// # Errors
    /// Returns [`Error::Redis`] if the publisher connection cannot be
    /// established.
    pub async fn from_url(url: &str) -> ServerResult<Self> {
        let client = Client::open(url).map_err(|e| Error::Redis(e.to_string()))?;
        let publisher = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        Ok(Self {
            publisher,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            url: SecretString::from(url.to_owned()),
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

    /// Return the number of subscribers on `channel` across the whole fleet.
    ///
    /// Uses Redis `PUBSUB NUMSUB`, which counts every connection subscribed to
    /// the channel on this Redis instance. Because each app replica opens at
    /// most one Redis `SUBSCRIBE` per channel (see [`Self::subscribe`]) and all
    /// replicas share one Redis, a non-zero count means *some* replica has a
    /// live SSE subscriber — the correct global "is anyone watching?" gate
    /// (local `Sender::receiver_count` would only see this replica).
    ///
    /// # Errors
    /// Returns [`Error::Redis`] if the `PUBSUB NUMSUB` command fails.
    pub async fn channel_subscriber_count(&self, channel: &str) -> ServerResult<u64> {
        let mut conn = self.publisher.clone();
        // NUMSUB replies as a flat 2-element array: [channel, count].
        let (_chan, count): (String, u64) = redis::cmd("PUBSUB")
            .arg("NUMSUB")
            .arg(channel)
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;
        Ok(count)
    }

    /// Subscribe to `channel` and return a [`Receiver`] that yields JSON
    /// strings for every message published to that channel.
    ///
    /// The first call for a given `channel` opens a dedicated Redis PubSub
    /// connection and spawns a background task that drives message delivery
    /// (with automatic exponential back-off reconnection).  The method blocks
    /// until the Redis `SUBSCRIBE` handshake completes before returning, so
    /// callers can immediately publish without a race.
    /// Subsequent calls reuse the existing `broadcast::Sender`.
    ///
    /// # Errors
    /// Returns [`Error::Redis`] if the initial Redis PubSub connection cannot
    /// be established (i.e. the ready signal never arrives).
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
        let map = self.subscribers.write().await;

        // Double-check after acquiring the write lock (another task may have
        // raced us to the slow path).
        if let Some(tx) = map.get(channel) {
            return Ok(tx.subscribe());
        }

        let (tx, rx) = broadcast::channel::<String>(BROADCAST_CAPACITY);

        // `ready_tx` fires once the background task has issued SUBSCRIBE on
        // Redis, confirming it is alive and safe to publish to immediately.
        let (ready_tx, ready_rx) = oneshot::channel::<()>();

        // Drop the write lock before we await the ready signal; otherwise the
        // lock would be held across the network round-trip.
        drop(map);

        let url = self.url.clone();
        let channel_owned = channel.to_owned();
        let subscribers = self.subscribers.clone();
        let tx_for_task = tx.clone();

        tokio::spawn(async move {
            Self::run_subscriber_task(url, &channel_owned, tx_for_task, subscribers, ready_tx)
                .await;
        });

        // Wait for the background task to confirm the SUBSCRIBE handshake.
        // If the task fails before sending, surface as a Redis error.
        ready_rx
            .await
            .map_err(|_| Error::Redis("subscriber task exited before becoming ready".to_owned()))?;

        // Only insert the Sender into the map AFTER we know the task is alive,
        // so a concurrent fast-path caller never gets a dead Sender.
        self.subscribers
            .write()
            .await
            .insert(channel.to_owned(), tx);

        Ok(rx)
    }

    /// Internal: drives a Redis PubSub connection for one channel.  On any
    /// disconnect or error the task waits with exponential back-off (1s → 2s →
    /// … → 30s) and reconnects, unless all in-process receivers have been
    /// dropped — in which case the task exits cleanly.
    #[allow(clippy::too_many_arguments)]
    async fn run_subscriber_task(
        url: SecretString,
        channel: &str,
        tx: Sender<String>,
        subscribers: Arc<RwLock<HashMap<String, Sender<String>>>>,
        ready_tx: oneshot::Sender<()>,
    ) {
        let mut backoff_secs = BACKOFF_START_SECS;
        let mut ready_tx = Some(ready_tx);

        loop {
            // If all receivers dropped while we were waiting to reconnect,
            // there is no point opening a new connection.
            if tx.receiver_count() == 0 {
                log::debug!(
                    "redis_pubsub: no receivers for channel '{}', subscriber task exiting",
                    channel
                );
                break;
            }

            let connect_result = async {
                let client = Client::open(url.expose_secret()).map_err(|e| e.to_string())?;
                let mut pubsub = client.get_async_pubsub().await.map_err(|e| e.to_string())?;
                pubsub.subscribe(channel).await.map_err(|e| e.to_string())?;
                Ok::<_, String>(pubsub)
            }
            .await;

            let mut pubsub = match connect_result {
                Ok(p) => p,
                Err(e) => {
                    log::error!(
                        "redis_pubsub: failed to connect/subscribe on channel '{}': {e}; \
                         retrying in {backoff_secs}s",
                        channel
                    );
                    // Signal failure on the very first attempt so the caller
                    // of subscribe() gets an error instead of hanging.
                    if let Some(tx) = ready_tx.take() {
                        drop(tx); // closes the oneshot → RecvError on caller side
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(BACKOFF_MAX_SECS);
                    continue;
                }
            };

            // Connected — signal readiness on the first successful handshake.
            if let Some(rtx) = ready_tx.take() {
                let _ = rtx.send(());
            }
            // Reset back-off after a successful connect.
            backoff_secs = BACKOFF_START_SECS;

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
                            // All receivers dropped — shut down cleanly.
                            log::debug!(
                                "redis_pubsub: no receivers for channel '{}', closing subscriber",
                                channel
                            );
                            // Break out of both loops.
                            let mut map = subscribers.write().await;
                            map.remove(channel);
                            return;
                        }
                    }
                    None => {
                        // Stream ended — Redis disconnected.
                        log::error!(
                            "redis_pubsub: connection lost on channel '{}'; \
                             retrying in {backoff_secs}s",
                            channel
                        );
                        break; // break inner loop → retry outer loop
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
            backoff_secs = (backoff_secs * 2).min(BACKOFF_MAX_SECS);
        }

        // Clean up the map entry so a future subscribe() call starts fresh.
        let mut map = subscribers.write().await;
        map.remove(channel);
    }
}

#[cfg(test)]
impl RedisPubSub {
    /// Construct a `RedisPubSub` connected to the test Redis instance.
    pub async fn for_tests() -> Self {
        let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "aegyptvault.local".to_owned());
        let port = std::env::var("REDIS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(2435_u16);
        Self::new(&host, port, "")
            .await
            .expect("test Redis must be reachable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_redis_params() -> (String, u16) {
        let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "aegyptvault.local".to_owned());
        let port = std::env::var("REDIS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(2435_u16);
        (host, port)
    }

    #[tokio::test]
    async fn publish_subscribe_roundtrip() {
        let (host, port) = test_redis_params();
        let ps = RedisPubSub::new(&host, port, "").await.unwrap();
        // subscribe() blocks until the Redis SUBSCRIBE handshake completes,
        // so it is safe to publish immediately after it returns.
        let mut rx = ps.subscribe("test:chan").await.unwrap();
        ps.publish("test:chan", &serde_json::json!({"hello": "world"}))
            .await
            .unwrap();
        let msg = tokio::time::timeout(std::time::Duration::from_secs(3), rx.recv())
            .await
            .expect("timeout")
            .expect("recv failed");
        assert!(msg.contains("hello"));
    }
}
