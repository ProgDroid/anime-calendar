---
name: feedback_redis_pubsub_shared_subscriber
description: "redis-rs 1.0.3 split-pubsub: two API facts docs.rs/latest gets wrong for this version, plus the lock-ordering deadlock and the quiet-channel leak a shared subscriber introduces."
metadata:
  node_type: memory
  type: feedback
  originSessionId: 30c24443-62ce-4076-9320-3334d0c8b47e
  modified: 2026-09-22T19:27:23.372Z
---

Multiplexing many Redis channels onto one `SUBSCRIBE` connection (Task 16) turns on `PubSub::split() -> (PubSubSink, PubSubStream)`: the sink issues `SUBSCRIBE`/`UNSUBSCRIBE` while the stream is being polled.

**Two API facts, both settled by reading `~/.cargo/registry/.../redis-1.0.3/src/aio/pubsub.rs`, both of which docs.rs/latest reports differently:**

1. `PubSubSink::subscribe` is `(&mut self, channel_name: impl ToRedisArgs)` at `:297`. docs.rs/latest shows `impl IntoIterator<Item = impl Into<Bytes>>` — a **newer signature that has not shipped in 1.0.3**. Context7 returns the latest-version page, so it was wrong here.
2. `PubSubSink::send_recv` (`:270`) goes through an internally-spawned `PipelineSink`, **not** through whoever polls `PubSubStream`. That is what makes it safe for `subscribe()` to await the `SUBSCRIBE` ack while the driver task is between polls. Had it been the other way, awaiting the ack from a caller would deadlock.

**Two hazards the multiplexing introduces, neither obvious from the diff:**

- **Lock ordering.** With `subscribers: RwLock<HashMap>` and `sink: RwLock<Option<PubSubSink>>`, the reconnect path naturally reads the channel set *while holding* the sink write lock — which deadlocks against `subscribe()` holding `subscribers` and wanting `sink`. Fix: `subscribers` before `sink` everywhere, and scope each acquisition separately in the reconnect path.
- **The quiet-channel leak.** The driver only learns a `broadcast` is empty when the **next message arrives on it**, so a channel nobody publishes to keeps its Redis subscription — and gets restored on every reconnect — for the life of the process. A sweep on the `subscribe` slow path closes it with no timer.

**Testing it.** Assert against Redis, not against the code's own belief: diff `CLIENT LIST TYPE pubsub` around N `subscribe()` calls and require growth of exactly one. For the reconnect test, kill the **real** socket with `CLIENT KILL ID <id>`, where the id comes from diffing that same list around the test's own setup — `CLIENT KILL TYPE pubsub` would collaterally kill parallel tests' connections. Both tests were confirmed by mutation (`&channels[0..1]` on reconnect; disabling the sweep), each failing exactly one test.

**Why:** a shared connection converts a per-channel failure into an all-channel failure, so the reconnect path stops being boilerplate and becomes the correctness requirement. See [[project_deploy_readiness_and_open_decisions]] and the project CLAUDE.md's Redis convention.

**How to apply:** when a "cheap-looking constructor called in a loop" shows up (this is the same defect class as the fifteen Postgres pools in Task 15), check whether the protocol permits sharing before capping. And read the vendored crate for any signature you are about to write — Context7 answers for `latest`, not for the locked version.
