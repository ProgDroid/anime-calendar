use crate::{
    ServerResult, config::database::Database as DatabaseConfig, entity::subscription::Subscription,
    mappers::database::Database,
};

/// Slim row used by the reconcile loop. We only project the fields that
/// participate in drift detection — adding more columns here means more
/// work per pass on the DB and Stripe side.
#[derive(Debug, Clone)]
pub struct ReconcileRow {
    pub id: i32,
    pub user_id: i32,
    pub stripe_subscription_id: String,
    pub status: String,
    pub current_period_end: chrono::NaiveDateTime,
    pub cancel_at_period_end: bool,
    pub trial_end: Option<chrono::NaiveDateTime>,
}

/// Repository for the `subscriptions` table. Handles the read path used by the
/// entitlement service plus the writes used by webhook handlers and the
/// reconcile loop.
#[derive(Clone)]
pub struct SubscriptionMapper {
    db: Database,
}

impl SubscriptionMapper {
    /// # Errors
    /// Fails if the database connection cannot be established.
    pub async fn new(config: DatabaseConfig) -> ServerResult<Self> {
        Ok(Self {
            db: Database::new(config).await?,
        })
    }

    /// Construct from a bare pool — used by integration tests and by the
    /// `set_subscription` CLI's `--reconcile-from-stripe` mode (where the
    /// pool comes from the same dev `database.toml` and we don't want a
    /// second mapper-construction round-trip).
    #[must_use]
    pub const fn from_pool(pool: sqlx::PgPool) -> Self {
        Self {
            db: Database { pool },
        }
    }

    /// Borrow the underlying connection pool. Used by the reconcile loop to
    /// acquire a dedicated connection for the cross-replica advisory lock.
    #[must_use]
    pub const fn pool(&self) -> &sqlx::PgPool {
        &self.db.pool
    }

    /// Return the user's currently-entitling subscription row, if any.
    ///
    /// "Entitling" = status in (trialing, active, `past_due`) and the period has
    /// not yet ended. The WHERE clause must stay in lockstep with
    /// `Status::grants_access`.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn find_active_for_user(&self, user_id: i32) -> ServerResult<Option<Subscription>> {
        crate::metrics::db::timed("subscription.find_active_for_user", async {
            Self::find_active_for_user_in_tx(&mut *self.db.pool.acquire().await?, user_id).await
        })
        .await
    }

    pub(crate) async fn find_active_for_user_in_tx(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
    ) -> ServerResult<Option<Subscription>> {
        let row = sqlx::query_as!(
            Subscription,
            "SELECT id, user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
             stripe_price_id, current_period_start, current_period_end, trial_end, \
             cancel_at_period_end, created_at, updated_at \
             FROM subscriptions \
             WHERE user_id = $1 \
               AND status IN ('trialing','active','past_due') \
               AND current_period_end > NOW() \
             ORDER BY current_period_end DESC \
             LIMIT 1",
            user_id,
        )
        .fetch_optional(conn)
        .await?;
        Ok(row)
    }

    /// Look up a subscription by its Stripe subscription id. Used by webhook
    /// handlers to find the row to update.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn find_by_stripe_id(
        &self,
        stripe_subscription_id: &str,
    ) -> ServerResult<Option<Subscription>> {
        crate::metrics::db::timed("subscription.find_by_stripe_id", async {
            Self::find_by_stripe_id_in_tx(
                &mut *self.db.pool.acquire().await?,
                stripe_subscription_id,
            )
            .await
        })
        .await
    }

    pub(crate) async fn find_by_stripe_id_in_tx(
        conn: &mut sqlx::PgConnection,
        stripe_subscription_id: &str,
    ) -> ServerResult<Option<Subscription>> {
        let row = sqlx::query_as!(
            Subscription,
            "SELECT id, user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
             stripe_price_id, current_period_start, current_period_end, trial_end, \
             cancel_at_period_end, created_at, updated_at \
             FROM subscriptions \
             WHERE stripe_subscription_id = $1",
            stripe_subscription_id,
        )
        .fetch_optional(conn)
        .await?;
        Ok(row)
    }

    /// Returns the user's most recent `stripe_customer_id`, regardless of
    /// status. Used at Checkout time to reuse a Stripe customer when a user
    /// re-subscribes after cancellation — keeps payment-method history
    /// attached to the same Stripe customer object.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn find_latest_customer_id_for_user(
        &self,
        user_id: i32,
    ) -> ServerResult<Option<String>> {
        crate::metrics::db::timed("subscription.find_latest_customer_id_for_user", async {
            let row = sqlx::query_scalar!(
                "SELECT stripe_customer_id FROM subscriptions \
                 WHERE user_id = $1 \
                 ORDER BY created_at DESC \
                 LIMIT 1",
                user_id,
            )
            .fetch_optional(&self.db.pool)
            .await?;
            Ok(row)
        })
        .await
    }

    /// Insert or refresh a subscription row from a Stripe payload. Phase-2-only
    /// stopgap used by `GET /api/subscription/me?session_id=` when the
    /// webhook hasn't arrived yet; Phase 3's webhook handler will become the
    /// canonical writer and this stays for safety/replay.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_from_stripe(
        &self,
        user_id: i32,
        status: &str,
        stripe_customer_id: &str,
        stripe_subscription_id: &str,
        stripe_price_id: &str,
        current_period_start: chrono::NaiveDateTime,
        current_period_end: chrono::NaiveDateTime,
        trial_end: Option<chrono::NaiveDateTime>,
        cancel_at_period_end: bool,
    ) -> ServerResult<()> {
        crate::metrics::db::timed("subscription.upsert_from_stripe", async {
            sqlx::query!(
                "INSERT INTO subscriptions \
                 (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
                  stripe_price_id, current_period_start, current_period_end, trial_end, \
                  cancel_at_period_end) \
                 VALUES ($1, 'paid', $2, $3, $4, $5, $6, $7, $8, $9) \
                 ON CONFLICT (stripe_subscription_id) DO UPDATE SET \
                   status = EXCLUDED.status, \
                   stripe_price_id = EXCLUDED.stripe_price_id, \
                   current_period_start = EXCLUDED.current_period_start, \
                   current_period_end = EXCLUDED.current_period_end, \
                   trial_end = EXCLUDED.trial_end, \
                   cancel_at_period_end = EXCLUDED.cancel_at_period_end, \
                   updated_at = NOW()",
                user_id,
                status,
                stripe_customer_id,
                stripe_subscription_id,
                stripe_price_id,
                current_period_start,
                current_period_end,
                trial_end,
                cancel_at_period_end,
            )
            .execute(&self.db.pool)
            .await?;
            Ok(())
        })
        .await
    }

    /// Same as `upsert_from_stripe` but participates in an existing sqlx
    /// transaction so the webhook handler can commit the idempotency insert
    /// (`stripe_events`) and the subscription write atomically.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_from_stripe_in_tx(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
        status: &str,
        stripe_customer_id: &str,
        stripe_subscription_id: &str,
        stripe_price_id: &str,
        current_period_start: chrono::NaiveDateTime,
        current_period_end: chrono::NaiveDateTime,
        trial_end: Option<chrono::NaiveDateTime>,
        cancel_at_period_end: bool,
    ) -> ServerResult<()> {
        sqlx::query!(
            "INSERT INTO subscriptions \
             (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
              stripe_price_id, current_period_start, current_period_end, trial_end, \
              cancel_at_period_end) \
             VALUES ($1, 'paid', $2, $3, $4, $5, $6, $7, $8, $9) \
             ON CONFLICT (stripe_subscription_id) DO UPDATE SET \
               status = EXCLUDED.status, \
               stripe_price_id = EXCLUDED.stripe_price_id, \
               current_period_start = EXCLUDED.current_period_start, \
               current_period_end = EXCLUDED.current_period_end, \
               trial_end = EXCLUDED.trial_end, \
               cancel_at_period_end = EXCLUDED.cancel_at_period_end, \
               updated_at = NOW()",
            user_id,
            status,
            stripe_customer_id,
            stripe_subscription_id,
            stripe_price_id,
            current_period_start,
            current_period_end,
            trial_end,
            cancel_at_period_end,
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Resolve a `user_id` from a Stripe customer id by looking at any prior
    /// subscription row. Used by subscription/invoice webhook events that
    /// don't carry `client_reference_id` and don't have user metadata
    /// populated (legacy rows or out-of-band Stripe activity).
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn find_user_id_by_customer_in_tx(
        conn: &mut sqlx::PgConnection,
        stripe_customer_id: &str,
    ) -> ServerResult<Option<i32>> {
        let row = sqlx::query_scalar!(
            "SELECT user_id FROM subscriptions \
             WHERE stripe_customer_id = $1 \
             ORDER BY created_at DESC \
             LIMIT 1",
            stripe_customer_id,
        )
        .fetch_optional(conn)
        .await?;
        Ok(row)
    }

    /// Update only the `status` column for a subscription, by stripe id.
    /// Used by `customer.subscription.deleted` (→ `canceled`) and
    /// `invoice.payment_failed` (→ `past_due`). Returns the number of rows
    /// affected so callers can decide whether to log a "row not found" warning.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn update_status_by_subscription_id_in_tx(
        conn: &mut sqlx::PgConnection,
        stripe_subscription_id: &str,
        status: &str,
    ) -> ServerResult<u64> {
        let result = sqlx::query!(
            "UPDATE subscriptions SET status = $1, updated_at = NOW() \
             WHERE stripe_subscription_id = $2",
            status,
            stripe_subscription_id,
        )
        .execute(conn)
        .await?;
        Ok(result.rows_affected())
    }

    /// Cancel every subscription row for a given Stripe customer id, in one
    /// statement. Used by `customer.deleted` so the local rows don't go stale
    /// once Stripe forgets about the customer (the reconcile loop would
    /// otherwise hit 404 on every pass).
    ///
    /// Returns the number of rows affected — caller logs if zero.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn update_all_status_by_customer_in_tx(
        conn: &mut sqlx::PgConnection,
        stripe_customer_id: &str,
        status: &str,
    ) -> ServerResult<u64> {
        let result = sqlx::query!(
            "UPDATE subscriptions SET status = $1, updated_at = NOW() \
             WHERE stripe_customer_id = $2",
            status,
            stripe_customer_id,
        )
        .execute(conn)
        .await?;
        Ok(result.rows_affected())
    }

    /// Update `current_period_end` on a successful renewal invoice. Also
    /// flips `status` to `active` if the subscription was previously in
    /// `past_due` — a successful charge clears the dunning state. Returns
    /// rows affected for the same reason as `update_status_by_subscription_id_in_tx`.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn update_period_end_by_subscription_id_in_tx(
        conn: &mut sqlx::PgConnection,
        stripe_subscription_id: &str,
        current_period_end: chrono::NaiveDateTime,
    ) -> ServerResult<u64> {
        let result = sqlx::query!(
            "UPDATE subscriptions SET \
                current_period_end = $1, \
                status = CASE WHEN status = 'past_due' THEN 'active' ELSE status END, \
                updated_at = NOW() \
             WHERE stripe_subscription_id = $2",
            current_period_end,
            stripe_subscription_id,
        )
        .execute(conn)
        .await?;
        Ok(result.rows_affected())
    }

    /// Snapshot of a subscription row used by the reconcile loop. Slim
    /// projection so the loop doesn't drag the full Subscription entity
    /// (and its calendar of unused columns) through every pass.
    /// # Errors
    /// - `sqlx::Error`: database error
    pub async fn list_for_reconcile(&self) -> ServerResult<Vec<ReconcileRow>> {
        crate::metrics::db::timed("subscription.list_for_reconcile", async {
            let rows = sqlx::query_as!(
                ReconcileRow,
                "SELECT id, user_id, stripe_subscription_id, status, current_period_end, \
                        cancel_at_period_end, trial_end \
                 FROM subscriptions \
                 WHERE status <> 'canceled' \
                 ORDER BY updated_at ASC",
            )
            .fetch_all(&self.db.pool)
            .await?;
            Ok(rows)
        })
        .await
    }

    /// Apply a Stripe-side truth update to a local row, guarded so a webhook
    /// write that landed mid-pass doesn't get clobbered.
    ///
    /// `rows_affected = 0` means a fresher webhook write already landed and
    /// drift correction is unnecessary. The guard fires when:
    ///
    /// - `current_period_end` is *strictly newer* than the local row, OR
    /// - `current_period_end` is equal but the `status` differs (so we
    ///   converge on Stripe's status when the equal-period webhook hasn't yet
    ///   touched the row, e.g. a webhook landed `active` first and the next
    ///   reconcile pass sees `past_due` from Stripe with the same period).
    ///
    /// Both halves of the guard are required: without the status check the
    /// loop could regress an `active`-by-webhook row back to `past_due` if
    /// Stripe still reports the older state for the same period boundary; without
    /// the period check it would regress a fresher period back to an older one.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn apply_reconcile_update(
        &self,
        id: i32,
        stripe_status: &str,
        stripe_period_end: chrono::NaiveDateTime,
        stripe_cancel_at_period_end: bool,
        stripe_trial_end: Option<chrono::NaiveDateTime>,
    ) -> ServerResult<u64> {
        crate::metrics::db::timed("subscription.apply_reconcile_update", async {
            // sqlx::query! note: PostgreSQL PREPARE rejects the same $N appearing
            // in both `SET col = $N` and a comparison like `col <> $N`. Use
            // distinct params ($6, $7) for the WHERE-clause comparisons and
            // clone the small input fields to feed them.
            let status_for_set = stripe_status.to_owned();
            let status_for_cmp = stripe_status.to_owned();
            let period_for_set = stripe_period_end;
            let period_for_cmp = stripe_period_end;
            let result = sqlx::query!(
                "UPDATE subscriptions SET \
                    status = $2, \
                    current_period_end = $3, \
                    cancel_at_period_end = $4, \
                    trial_end = $5, \
                    updated_at = NOW() \
                 WHERE id = $1 \
                   AND (current_period_end < $6 \
                        OR (current_period_end = $7 AND status <> $8))",
                id,
                status_for_set,
                period_for_set,
                stripe_cancel_at_period_end,
                stripe_trial_end,
                period_for_cmp,
                period_for_cmp,
                status_for_cmp,
            )
            .execute(&self.db.pool)
            .await?;
            Ok(result.rows_affected())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    async fn seed_user(conn: &mut sqlx::PgConnection) -> i32 {
        let n: u64 = rand::random();
        sqlx::query_scalar(
            "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, 'hash') RETURNING id",
        )
        .bind(format!("subtester_{n}"))
        .bind(format!("subtester_{n}@test.com"))
        .fetch_one(conn)
        .await
        .unwrap()
    }

    async fn insert_subscription(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
        status: &str,
        period_end_offset_days: i64,
    ) {
        let n: u64 = rand::random();
        let period_end = (Utc::now() + Duration::days(period_end_offset_days)).naive_utc();
        let period_start = (Utc::now() - Duration::days(30)).naive_utc();
        sqlx::query!(
            "INSERT INTO subscriptions \
             (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
              stripe_price_id, current_period_start, current_period_end, cancel_at_period_end) \
             VALUES ($1, 'paid', $2, $3, $4, 'price_test', $5, $6, false)",
            user_id,
            status,
            format!("cus_{n}"),
            format!("sub_{n}"),
            period_start,
            period_end,
        )
        .execute(&mut *conn)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn find_active_returns_active_subscription() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;
        insert_subscription(&mut tx, user_id, "active", 14).await;

        let sub = SubscriptionMapper::find_active_for_user_in_tx(&mut tx, user_id)
            .await
            .unwrap();
        assert!(sub.is_some());
        let sub = sub.unwrap();
        assert_eq!(sub.user_id, user_id);
        assert_eq!(sub.status, "active");
        assert_eq!(sub.tier, "paid");
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn find_active_returns_trialing_subscription() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;
        insert_subscription(&mut tx, user_id, "trialing", 7).await;

        let sub = SubscriptionMapper::find_active_for_user_in_tx(&mut tx, user_id)
            .await
            .unwrap();
        assert!(sub.is_some());
        assert_eq!(sub.unwrap().status, "trialing");
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn find_active_returns_past_due_subscription() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;
        insert_subscription(&mut tx, user_id, "past_due", 3).await;

        let sub = SubscriptionMapper::find_active_for_user_in_tx(&mut tx, user_id)
            .await
            .unwrap();
        assert!(sub.is_some());
        assert_eq!(sub.unwrap().status, "past_due");
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn find_active_skips_canceled_subscription() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;
        insert_subscription(&mut tx, user_id, "canceled", 14).await;

        let sub = SubscriptionMapper::find_active_for_user_in_tx(&mut tx, user_id)
            .await
            .unwrap();
        assert!(sub.is_none());
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn find_active_skips_expired_subscription() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;
        // active status but period_end in the past.
        insert_subscription(&mut tx, user_id, "active", -1).await;

        let sub = SubscriptionMapper::find_active_for_user_in_tx(&mut tx, user_id)
            .await
            .unwrap();
        assert!(sub.is_none());
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn find_active_returns_none_for_user_with_no_subscription() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;

        let sub = SubscriptionMapper::find_active_for_user_in_tx(&mut tx, user_id)
            .await
            .unwrap();
        assert!(sub.is_none());
        tx.rollback().await.unwrap();
    }

    // ---- apply_reconcile_update guard tests --------------------------------

    /// Insert a subscription row and return its `id`. Uses `pool`-bound
    /// queries because `apply_reconcile_update` is an instance method on
    /// `SubscriptionMapper` that only takes `&self` and reaches the pool
    /// itself, so we can't share a `tx` here.
    async fn seed_reconcile_row(
        pool: &sqlx::PgPool,
        user_id: i32,
        status: &str,
        period_end: chrono::NaiveDateTime,
    ) -> i32 {
        let n: u64 = rand::random();
        let period_start = (Utc::now() - Duration::days(30)).naive_utc();
        sqlx::query_scalar(
            "INSERT INTO subscriptions \
             (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
              stripe_price_id, current_period_start, current_period_end, cancel_at_period_end) \
             VALUES ($1, 'paid', $2, $3, $4, 'price_test', $5, $6, false) RETURNING id",
        )
        .bind(user_id)
        .bind(status)
        .bind(format!("cus_recon_{n}"))
        .bind(format!("sub_recon_{n}"))
        .bind(period_start)
        .bind(period_end)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn seed_user_in_pool(pool: &sqlx::PgPool) -> i32 {
        let n: u64 = rand::random();
        sqlx::query_scalar(
            "INSERT INTO users (username, email, password_hash) \
             VALUES ($1, $2, 'hash') RETURNING id",
        )
        .bind(format!("recontester_{n}"))
        .bind(format!("recontester_{n}@test.com"))
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn cleanup_user_pool(pool: &sqlx::PgPool, user_id: i32) {
        sqlx::query("DELETE FROM subscriptions WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
    }

    /// H-7: reconcile must converge on Stripe's `status` when the local row has
    /// the same `current_period_end` but a different status. First call updates
    /// (status diverges); a second call with the same status is a no-op.
    #[tokio::test]
    async fn apply_reconcile_update_flips_status_at_equal_period() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user_in_pool(&pool).await;
        let period_end = (Utc::now() + Duration::days(14)).naive_utc();
        let row_id = seed_reconcile_row(&pool, user_id, "active", period_end).await;

        let mapper = SubscriptionMapper::from_pool(pool.clone());

        // First call: same period, different status → must update.
        let updated = mapper
            .apply_reconcile_update(row_id, "past_due", period_end, false, None)
            .await
            .unwrap();
        assert_eq!(updated, 1, "status divergence must drive an update");

        // Second call: same period, same status → no-op.
        let updated_again = mapper
            .apply_reconcile_update(row_id, "past_due", period_end, false, None)
            .await
            .unwrap();
        assert_eq!(
            updated_again, 0,
            "matching status + matching period must be a no-op",
        );

        cleanup_user_pool(&pool, user_id).await;
    }

    /// H-7: a strictly older Stripe period must never overwrite a fresher local
    /// row (this is the original guard that we kept).
    #[tokio::test]
    async fn apply_reconcile_update_skips_strictly_older_period() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user_in_pool(&pool).await;
        let local_period_end = (Utc::now() + Duration::days(14)).naive_utc();
        let stripe_period_end = local_period_end - Duration::seconds(10);
        let row_id = seed_reconcile_row(&pool, user_id, "active", local_period_end).await;

        let mapper = SubscriptionMapper::from_pool(pool.clone());

        let updated = mapper
            .apply_reconcile_update(row_id, "active", stripe_period_end, false, None)
            .await
            .unwrap();
        assert_eq!(
            updated, 0,
            "older Stripe period must not overwrite a fresher local row",
        );

        cleanup_user_pool(&pool, user_id).await;
    }

    #[tokio::test]
    async fn find_by_stripe_id_returns_match() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;
        let period_end = (Utc::now() + Duration::days(14)).naive_utc();
        let period_start = (Utc::now() - Duration::days(30)).naive_utc();
        sqlx::query!(
            "INSERT INTO subscriptions \
             (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
              stripe_price_id, current_period_start, current_period_end, cancel_at_period_end) \
             VALUES ($1, 'paid', 'active', 'cus_known', 'sub_known', 'price_test', $2, $3, false)",
            user_id,
            period_start,
            period_end,
        )
        .execute(&mut *tx)
        .await
        .unwrap();

        let sub = SubscriptionMapper::find_by_stripe_id_in_tx(&mut tx, "sub_known")
            .await
            .unwrap();
        assert!(sub.is_some());
        assert_eq!(sub.unwrap().user_id, user_id);

        let none = SubscriptionMapper::find_by_stripe_id_in_tx(&mut tx, "sub_missing")
            .await
            .unwrap();
        assert!(none.is_none());
        tx.rollback().await.unwrap();
    }
}
