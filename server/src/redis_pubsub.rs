use std::{collections::HashMap, sync::Arc};

use futures_util::StreamExt as _;
use redis::{
    Client,
    aio::{MultiplexedConnection, PubSubSink, PubSubStream},
};
use secrecy::{ExposeSecret as _, SecretString};
use serde::Serialize;
use tokio::sync::{
    Mutex, RwLock,
    broadcast::{self, Receiver, Sender},
};

use crate::ServerResult;
use crate::error::Error;

/// Capacity of each per-channel broadcast channel.  Slow consumers that fall
/// more than `BROADCAST_CAPACITY` messages behind will receive a
/// `RecvError::Lagged` on their next `recv()` call.
const BROADCAST_CAPACITY: usize = 64;

/// Initial back-off delay (seconds) after a subscriber connection drop.
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

/// State shared by every clone of a [`RedisPubSub`] and by the single
/// subscriber driver task.
///
/// **Lock ordering is `subscribers` before `sink`.** Every path that needs both
/// takes them in that order, or takes each in its own scope. The reconnect path
/// in particular must not hold `sink` while reading `subscribers`, or it
/// deadlocks against [`RedisPubSub::subscribe`].
struct Shared {
    /// Connection URL, kept so the driver task can reconnect after a drop.
    /// Held as [`SecretString`] because it embeds the password.
    url: SecretString,
    /// In-process fan-out: one `broadcast::Sender` per live channel. Also the
    /// authoritative list of what the shared connection must be subscribed to,
    /// which is what the reconnect path restores from.
    subscribers: RwLock<HashMap<String, Sender<String>>>,
    /// Sink half of the one shared Redis `PubSub` connection. `None` means the
    /// connection has not been opened yet, or the driver is mid-reconnect.
    sink: RwLock<Option<PubSubSink>>,
    /// Serialises the slow path of [`RedisPubSub::subscribe`]. The `bool` is
    /// "the driver task has been spawned"; it lives inside the mutex because it
    /// is only ever read or written on that slow path.
    driver_started: Mutex<bool>,
}

/// Redis Pub/Sub wrapper.
///
/// - The publisher side uses a `MultiplexedConnection` (matching [`crate::cache::Cache`]).
/// - The subscriber side opens **one** async `PubSub` connection for the whole
///   process, split into a sink (SUBSCRIBE/UNSUBSCRIBE) and a stream (messages).
///   A single driver task demultiplexes incoming messages by channel name into
///   per-channel in-process `broadcast` channels, so multiple in-process
///   consumers share one Redis subscription and every channel shares one socket.
/// - The connection is opened lazily on the first [`Self::subscribe`] call, so a
///   replica with no SSE viewers holds no subscriber connection at all.
/// - The driver reconnects automatically with exponential back-off after a Redis
///   disconnect, restoring **every** live subscription on the new connection.
///
/// This is what keeps the Redis connection budget flat rather than growing with
/// viewer count: `2/calendar + 1/viewer` collapses to one shared socket.
///
/// `RedisPubSub` is [`Clone`]-able; all clones share the same subscriber map and
/// the same Redis connection. The Redis password is stored as [`SecretString`]
/// so it is zeroed on drop and never appears in `Debug` output.
#[derive(Clone)]
pub struct RedisPubSub {
    publisher: MultiplexedConnection,
    shared: Arc<Shared>,
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
    /// Only the publisher connection is opened here. The shared subscriber
    /// connection is opened on the first [`Self::subscribe`] call.
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
            shared: Arc::new(Shared {
                url: SecretString::from(url.to_owned()),
                subscribers: RwLock::new(HashMap::new()),
                sink: RwLock::new(None),
                driver_started: Mutex::new(false),
            }),
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
    /// the channel on this Redis instance. Because each app replica holds at
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
    /// The first call for a given `channel` issues a Redis `SUBSCRIBE` on the
    /// process-wide shared `PubSub` connection (opening it, and spawning the
    /// driver task, if this is the first subscription of any kind). The method
    /// returns only once Redis has acknowledged the `SUBSCRIBE`, so callers can
    /// immediately publish without a race. Subsequent calls for the same
    /// channel reuse the existing `broadcast::Sender` and touch Redis not at
    /// all.
    ///
    /// # Errors
    /// Returns [`Error::Redis`] if the shared subscriber connection cannot be
    /// established, is currently reconnecting, or rejects the `SUBSCRIBE`.
    pub async fn subscribe(&self, channel: &str) -> ServerResult<Receiver<String>> {
        // Fast path: this replica already holds a Redis SUBSCRIBE for the
        // channel, so there is nothing to do but hand out another receiver.
        {
            let map = self.shared.subscribers.read().await;
            if let Some(tx) = map.get(channel) {
                return Ok(tx.subscribe());
            }
        }

        // Slow path: first in-process subscriber for this channel. Serialised
        // so the shared connection is opened exactly once and concurrent
        // callers cannot interleave their SUBSCRIBE round trips.
        let mut driver_started = self.shared.driver_started.lock().await;

        // Another task may have completed the slow path while we waited.
        {
            let map = self.shared.subscribers.read().await;
            if let Some(tx) = map.get(channel) {
                return Ok(tx.subscribe());
            }
        }

        Self::reap_dead_channels(&self.shared).await;
        Self::ensure_connected(&self.shared, &mut driver_started).await?;

        // Clone the sink out before taking the map write lock: the lock order
        // is `subscribers` then `sink`, never the reverse.
        let mut sink = self
            .shared
            .sink
            .read()
            .await
            .clone()
            .ok_or_else(|| Error::Redis("subscriber connection is unavailable".to_owned()))?;

        let (tx, rx) = broadcast::channel::<String>(BROADCAST_CAPACITY);

        // Hold the map write lock across the SUBSCRIBE round trip. No other
        // task may observe the entry until Redis has acknowledged it, or a
        // concurrent fast-path caller could get a Receiver for a channel this
        // replica is not yet subscribed to.
        let mut map = self.shared.subscribers.write().await;
        sink.subscribe(channel)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;
        map.insert(channel.to_owned(), tx);
        drop(map);

        Ok(rx)
    }

    /// Open the shared subscriber connection and spawn its driver task, unless
    /// that has already happened.
    ///
    /// Only ever called with the `driver_started` mutex held. If the driver is
    /// already running but the connection is down, this fails fast rather than
    /// blocking the caller for the whole back-off — the driver owns recovery.
    async fn ensure_connected(shared: &Arc<Shared>, driver_started: &mut bool) -> ServerResult<()> {
        if *driver_started {
            return if shared.sink.read().await.is_some() {
                Ok(())
            } else {
                Err(Error::Redis(
                    "subscriber connection is reconnecting".to_owned(),
                ))
            };
        }

        let (sink, stream) = Self::connect(&shared.url).await.map_err(Error::Redis)?;

        let driver_shared = Arc::clone(shared);
        tokio::spawn(async move {
            Self::run_driver(driver_shared, stream).await;
        });

        *shared.sink.write().await = Some(sink);
        *driver_started = true;
        Ok(())
    }

    /// Open one Redis `PubSub` connection and split it into its sink and stream
    /// halves.
    async fn connect(url: &SecretString) -> Result<(PubSubSink, PubSubStream), String> {
        let client = Client::open(url.expose_secret()).map_err(|e| e.to_string())?;
        let pubsub = client.get_async_pubsub().await.map_err(|e| e.to_string())?;
        Ok(pubsub.split())
    }

    /// Internal: drives the one shared Redis `PubSub` connection for the whole
    /// process, demultiplexing messages to the per-channel broadcast senders.
    ///
    /// On a disconnect it waits with exponential back-off (1s → 2s → … → 30s)
    /// and reconnects. Unlike the per-channel tasks this replaced, it never
    /// exits: a channel losing its last receiver costs an `UNSUBSCRIBE`, not
    /// the connection, which every other channel is still riding on.
    async fn run_driver(shared: Arc<Shared>, mut stream: PubSubStream) {
        let mut backoff_secs = BACKOFF_START_SECS;

        loop {
            // ── Deliver messages until the connection drops ──────────────
            while let Some(msg) = stream.next().await {
                // Traffic proves the connection is healthy.
                backoff_secs = BACKOFF_START_SECS;

                let channel = msg.get_channel_name().to_owned();
                let payload: String = match msg.get_payload() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!(
                            "redis_pubsub: failed to decode payload on channel '{channel}': {e}"
                        );
                        continue;
                    }
                };

                let tx = { shared.subscribers.read().await.get(&channel).cloned() };

                let Some(tx) = tx else {
                    // Subscribed on Redis with no local fan-out — shouldn't
                    // happen, but drop the subscription rather than keep
                    // receiving into nothing.
                    log::debug!(
                        "redis_pubsub: message on channel '{channel}' with no local fan-out; unsubscribing"
                    );
                    Self::unsubscribe_channel(&shared, &channel).await;
                    continue;
                };

                if tx.send(payload).is_err() {
                    // Last in-process receiver dropped. Give up the Redis
                    // subscription but keep the connection.
                    log::debug!(
                        "redis_pubsub: no receivers for channel '{channel}', unsubscribing"
                    );
                    shared.subscribers.write().await.remove(&channel);
                    Self::unsubscribe_channel(&shared, &channel).await;
                }
            }

            // ── Connection lost — every channel went with it ─────────────
            *shared.sink.write().await = None;
            log::error!(
                "redis_pubsub: shared subscriber connection lost; retrying in {backoff_secs}s"
            );

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(BACKOFF_MAX_SECS);

                match Self::reconnect(&shared).await {
                    Ok(new_stream) => {
                        stream = new_stream;
                        break;
                    }
                    Err(e) => log::error!(
                        "redis_pubsub: reconnect failed: {e}; retrying in {backoff_secs}s"
                    ),
                }
            }
        }
    }

    /// Reopen the shared subscriber connection and restore **every** live
    /// subscription on it.
    ///
    /// This is the part that a single shared connection makes load-bearing: a
    /// drop now costs all channels at once rather than one, so restoring the
    /// full set — not the channel that happened to notice — is the correctness
    /// requirement. The set comes from `subscribers`, which is why that map is
    /// the authoritative record of what should be subscribed.
    async fn reconnect(shared: &Arc<Shared>) -> Result<PubSubStream, String> {
        let (mut sink, stream) = Self::connect(&shared.url).await?;

        let channels: Vec<String> = { shared.subscribers.read().await.keys().cloned().collect() };

        // SUBSCRIBE with no arguments is an error; a connection with nothing
        // live on it is legitimate (every viewer left while we were down).
        if !channels.is_empty() {
            sink.subscribe(&channels).await.map_err(|e| e.to_string())?;
        }

        *shared.sink.write().await = Some(sink);
        log::info!(
            "redis_pubsub: subscriber connection restored with {} channel(s)",
            channels.len()
        );
        Ok(stream)
    }

    /// Issue `UNSUBSCRIBE` for `channel` on the shared connection, if it is up.
    ///
    /// Best-effort: if the connection is mid-reconnect there is nothing to
    /// unsubscribe from, and the channel is already gone from `subscribers` so
    /// [`Self::reconnect`] will not restore it.
    async fn unsubscribe_channel(shared: &Arc<Shared>, channel: &str) {
        let sink = { shared.sink.read().await.clone() };
        if let Some(mut sink) = sink
            && let Err(e) = sink.unsubscribe(channel).await
        {
            log::error!("redis_pubsub: UNSUBSCRIBE '{channel}' failed: {e}");
        }
    }

    /// Drop Redis subscriptions for channels whose last in-process receiver has
    /// gone.
    ///
    /// The driver only learns that a broadcast is empty when the *next* message
    /// arrives on it, so a channel that simply goes quiet would otherwise keep
    /// its subscription — and be restored on every reconnect — for the life of
    /// the process. Piggybacking the sweep on the `subscribe` slow path keeps
    /// it free of a timer while bounding the map to periods of real activity.
    async fn reap_dead_channels(shared: &Arc<Shared>) {
        let dead: Vec<String> = {
            let mut map = shared.subscribers.write().await;
            let dead: Vec<String> = map
                .iter()
                .filter(|(_, tx)| tx.receiver_count() == 0)
                .map(|(channel, _)| channel.clone())
                .collect();
            for channel in &dead {
                map.remove(channel);
            }
            dead
        };

        for channel in dead {
            log::debug!("redis_pubsub: reaping idle channel '{channel}'");
            Self::unsubscribe_channel(shared, &channel).await;
        }
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
    use std::collections::HashSet;
    use std::time::Duration;

    use super::*;

    /// Serialises the tests that open a Redis `PubSub` *connection*.
    ///
    /// `connection_identity` below works by diffing `CLIENT LIST TYPE pubsub`
    /// around its own connection setup, which is only unambiguous if no other
    /// test opens one concurrently. Nothing outside this module does: the
    /// `for_tests()` call sites in `controllers/calendar.rs` only publish and
    /// run `PUBSUB NUMSUB`, both of which go over the publisher's
    /// `MultiplexedConnection`, and the connection here is opened lazily on the
    /// first `subscribe()`.
    static PUBSUB_CONNECTION: Mutex<()> = Mutex::const_new(());

    fn test_redis_params() -> (String, u16) {
        let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "aegyptvault.local".to_owned());
        let port = std::env::var("REDIS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(2435_u16);
        (host, port)
    }

    /// A channel name unique to one test run, so concurrent tests never share a
    /// Redis channel.
    fn unique_channel(prefix: &str) -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        format!(
            "test:{prefix}:{}:{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    }

    /// The Redis client ids of every connection currently in subscriber mode.
    async fn pubsub_client_ids(ps: &RedisPubSub) -> HashSet<u64> {
        let mut conn = ps.publisher.clone();
        let list: String = redis::cmd("CLIENT")
            .arg("LIST")
            .arg("TYPE")
            .arg("pubsub")
            .query_async(&mut conn)
            .await
            .expect("CLIENT LIST failed");

        list.lines()
            .filter_map(|line| {
                line.split_whitespace()
                    .find_map(|field| field.strip_prefix("id="))
                    .and_then(|id| id.parse::<u64>().ok())
            })
            .collect()
    }

    /// Wait until `rx` yields a message, or fail.
    async fn expect_message(rx: &mut Receiver<String>, what: &str) -> String {
        tokio::time::timeout(Duration::from_secs(10), rx.recv())
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
            .unwrap_or_else(|e| panic!("recv failed for {what}: {e}"))
    }

    #[tokio::test]
    async fn publish_subscribe_roundtrip() {
        let _guard = PUBSUB_CONNECTION.lock().await;
        let (host, port) = test_redis_params();
        let ps = RedisPubSub::new(&host, port, "").await.unwrap();
        let channel = unique_channel("roundtrip");
        // subscribe() blocks until the Redis SUBSCRIBE handshake completes,
        // so it is safe to publish immediately after it returns.
        let mut rx = ps.subscribe(&channel).await.unwrap();
        ps.publish(&channel, &serde_json::json!({"hello": "world"}))
            .await
            .unwrap();
        let msg = expect_message(&mut rx, "roundtrip").await;
        assert!(msg.contains("hello"));
    }

    /// The point of Task 16: subscribing to more channels must not open more
    /// connections. Asserted against Redis's own `CLIENT LIST`, not against our
    /// belief about what the code does.
    #[tokio::test]
    async fn many_channels_share_one_connection() {
        let _guard = PUBSUB_CONNECTION.lock().await;
        let (host, port) = test_redis_params();
        let ps = RedisPubSub::new(&host, port, "").await.unwrap();

        let before = pubsub_client_ids(&ps).await;

        let channels: Vec<String> = (0..5).map(|_| unique_channel("multiplex")).collect();
        let mut receivers = Vec::new();
        for channel in &channels {
            receivers.push(ps.subscribe(channel).await.unwrap());
        }

        let after = pubsub_client_ids(&ps).await;
        let opened: Vec<u64> = after.difference(&before).copied().collect();
        assert_eq!(
            opened.len(),
            1,
            "5 channels must ride one connection; CLIENT LIST TYPE pubsub grew by {}",
            opened.len()
        );

        // And all five still actually work over that one socket.
        for (channel, rx) in channels.iter().zip(receivers.iter_mut()) {
            ps.publish(channel, &serde_json::json!({"c": channel}))
                .await
                .unwrap();
            let msg = expect_message(rx, channel).await;
            assert!(msg.contains(channel.as_str()));
        }
    }

    /// The failure mode multiplexing introduces: one dropped connection now
    /// costs *every* channel, so reconnect has to restore the whole live set
    /// rather than the one channel that noticed. Kills the real socket from the
    /// Redis side — a simulated disconnect would not prove the stream ends.
    #[tokio::test]
    async fn reconnect_restores_every_live_channel() {
        let _guard = PUBSUB_CONNECTION.lock().await;
        let (host, port) = test_redis_params();
        let ps = RedisPubSub::new(&host, port, "").await.unwrap();

        let chan_a = unique_channel("reconnect-a");
        let chan_b = unique_channel("reconnect-b");

        let before = pubsub_client_ids(&ps).await;
        let mut rx_a = ps.subscribe(&chan_a).await.unwrap();
        let mut rx_b = ps.subscribe(&chan_b).await.unwrap();
        let after = pubsub_client_ids(&ps).await;

        let ours: Vec<u64> = after.difference(&before).copied().collect();
        assert_eq!(
            ours.len(),
            1,
            "expected exactly one new pubsub connection to identify"
        );

        // Confirm both channels are live before the kill, so a failure after it
        // cannot be blamed on a subscription that never worked.
        ps.publish(&chan_a, &serde_json::json!({"phase": "before"}))
            .await
            .unwrap();
        assert!(
            expect_message(&mut rx_a, "chan_a before kill")
                .await
                .contains("before")
        );

        // Kill only our own connection, by id.
        let mut conn = ps.publisher.clone();
        let killed: i64 = redis::cmd("CLIENT")
            .arg("KILL")
            .arg("ID")
            .arg(ours[0])
            .query_async(&mut conn)
            .await
            .expect("CLIENT KILL failed");
        assert_eq!(killed, 1, "CLIENT KILL ID did not kill our connection");

        // The driver backs off for BACKOFF_START_SECS before its first retry,
        // then re-SUBSCRIBEs both channels. Publishing into the gap would be
        // lost (Redis pub/sub has no replay), so wait for the subscription to
        // be restored, then publish.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        loop {
            let live_a = ps.channel_subscriber_count(&chan_a).await.unwrap_or(0);
            let live_b = ps.channel_subscriber_count(&chan_b).await.unwrap_or(0);
            if live_a == 1 && live_b == 1 {
                break;
            }
            // Report both counts: a *partial* restore is the failure this test
            // exists for, and "nothing came back" and "only one came back" want
            // very different investigations.
            assert!(
                tokio::time::Instant::now() < deadline,
                "subscriptions not restored within 20s (chan_a={live_a}, chan_b={live_b}); \
                 a non-zero count here means reconnect restored only part of the live set"
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        ps.publish(&chan_a, &serde_json::json!({"phase": "after"}))
            .await
            .unwrap();
        ps.publish(&chan_b, &serde_json::json!({"phase": "after"}))
            .await
            .unwrap();

        assert!(
            expect_message(&mut rx_a, "chan_a after reconnect")
                .await
                .contains("after"),
            "channel A did not resume"
        );
        assert!(
            expect_message(&mut rx_b, "chan_b after reconnect")
                .await
                .contains("after"),
            "channel B did not resume — reconnect restored only part of the live set"
        );
    }

    /// A channel whose last receiver has gone must give up its Redis
    /// subscription, but must not take the shared connection down with it.
    #[tokio::test]
    async fn dropping_a_receiver_unsubscribes_without_closing_the_connection() {
        let _guard = PUBSUB_CONNECTION.lock().await;
        let (host, port) = test_redis_params();
        let ps = RedisPubSub::new(&host, port, "").await.unwrap();

        let doomed = unique_channel("reaped");
        let survivor = unique_channel("survivor");

        let before = pubsub_client_ids(&ps).await;
        let rx_doomed = ps.subscribe(&doomed).await.unwrap();
        let mut rx_survivor = ps.subscribe(&survivor).await.unwrap();
        let connection = {
            let after = pubsub_client_ids(&ps).await;
            let ours: Vec<u64> = after.difference(&before).copied().collect();
            assert_eq!(ours.len(), 1);
            ours[0]
        };

        drop(rx_doomed);
        assert_eq!(ps.channel_subscriber_count(&doomed).await.unwrap(), 1);

        // The sweep runs on the subscribe slow path.
        let third = unique_channel("third");
        let _rx_third = ps.subscribe(&third).await.unwrap();

        assert_eq!(
            ps.channel_subscriber_count(&doomed).await.unwrap(),
            0,
            "the abandoned channel kept its Redis subscription"
        );
        assert!(
            pubsub_client_ids(&ps).await.contains(&connection),
            "reaping a channel closed the shared connection"
        );

        // The surviving channel is untouched.
        ps.publish(&survivor, &serde_json::json!({"still": "here"}))
            .await
            .unwrap();
        assert!(
            expect_message(&mut rx_survivor, "survivor")
                .await
                .contains("still")
        );
    }
}
