-- Stripe webhook idempotency key store.
--
-- Every Stripe webhook event has a unique stripe_event_id (e.g. "evt_abc123").
-- The webhook handler INSERTs this key with ON CONFLICT DO NOTHING; if the
-- insert affects 0 rows, the event was already processed and the handler
-- short-circuits with 200 (Stripe stops retrying).
--
-- This is intentionally a small append-only table. event_type and received_at
-- exist for operational debugging only; the primary key is what enforces
-- idempotency.

CREATE TABLE stripe_events (
    stripe_event_id  VARCHAR(255)  PRIMARY KEY,
    event_type       VARCHAR(64)   NOT NULL,
    received_at      TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP
);
