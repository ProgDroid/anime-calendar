-- Indexes for two hot paths identified by the 2026-06-11 follow-up audit (F2-14).
--
-- 1. Stripe webhook handlers look subscriptions up (and bulk-update them) by
--    stripe_customer_id; only user-keyed and stripe_subscription_id indexes
--    existed, so every customer-keyed event seq-scanned the table.
CREATE INDEX idx_subscriptions_stripe_customer_id
    ON subscriptions (stripe_customer_id);

-- 2. The invitation-send rate limiter COUNTs recent rows by inviter on every
--    send; existing indexes are all calendar_id- or token-keyed.
CREATE INDEX idx_calendar_invitations_inviter_sent
    ON calendar_invitations (inviter_id, sent_at DESC);
