---
name: backend-full-green-test-run-needs-postgres-and-redis-on-2435
description: "cargo test -p server --lib needs a live Redis on port 2435 plus Postgres; a lone failure of subscribe_feed_free_no_blob_no_history_returns_404 panicking at cache.rs:31 means Redis is down, not a code regression."
metadata: 
  node_type: memory
  type: project
  originSessionId: 1b126c7d-2485-4fb6-9437-7b543abb4528
---

A fully-green `cargo test -p server --lib` (and `--workspace --lib`) needs TWO live services: the dev Postgres AND a Redis on port **2435** (env `REDIS_PORT`, default `2435` in the test-only `Cache` helper at `server/src/cache.rs:24-31`, which ends in `.expect("test Redis must be reachable")`).

**Diagnostic shortcut:** if the ONLY failing test is `controllers::calendar::integration_tests::subscribe_feed_free_no_blob_no_history_returns_404` panicking at `cache.rs:31`, that is Redis-not-running — **not a code regression**. Every other test is DB-only or doesn't touch Redis. Confirmed 2026-06-11 (audit Step 5): the suite was 370 passed / 1 failed solely because no Redis was listening on 2435 (the Docker daemon was also down). The standalone `#[tokio::test] senders_noop_when_smtp_host_unconfigured` (email module) needs neither service.

**How to apply:** before claiming a backend test regression, check `Test-NetConnection 127.0.0.1 -Port 2435`. If down, start the dev Redis (docker compose, mapped to 2435) and rerun — don't attribute the `subscribe_feed_*` panic to your change. Pairs with [[project_dev_db_migration_drift]] (the Postgres side of local test infra).
