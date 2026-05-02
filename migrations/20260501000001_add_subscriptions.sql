-- Add subscriptions table for Pro tier entitlement, mirroring Stripe's
-- subscription object so webhook handlers can map fields 1:1.
--
-- Source-of-truth model: Stripe is authoritative. We mirror status, period
-- boundaries, trial state, and cancel-at-period-end into this table on every
-- relevant Stripe webhook event. The reconcile loop (see services/reconcile.rs,
-- added in Track 4 phase 6) catches any drift hourly as a safety net.
--
-- A user is "paid" iff they have a row whose status is in
-- ('trialing','active','past_due') AND current_period_end > NOW().
-- past_due is included to grant a grace period during dunning; final
-- cancellation flips status to 'canceled' and the row stays for history.
--
-- The single composite index covers the hot read path (entitlement check
-- per request), ordered to match the WHERE clause in
-- subscription_mapper::find_active_for_user.

CREATE TABLE subscriptions (
    id                       SERIAL       PRIMARY KEY,
    user_id                  INTEGER      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tier                     VARCHAR(32)  NOT NULL,
    status                   VARCHAR(32)  NOT NULL,
    stripe_customer_id       VARCHAR(255) NOT NULL,
    stripe_subscription_id   VARCHAR(255) NOT NULL UNIQUE,
    stripe_price_id          VARCHAR(255) NOT NULL,
    current_period_start     TIMESTAMP    NOT NULL,
    current_period_end       TIMESTAMP    NOT NULL,
    trial_end                TIMESTAMP,
    cancel_at_period_end     BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_subscriptions_user_active
    ON subscriptions(user_id, status, current_period_end);
