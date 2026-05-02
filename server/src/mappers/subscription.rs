use crate::{
    ServerResult, config::database::Database as DatabaseConfig, entity::subscription::Subscription,
    mappers::database::Database,
};

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

    /// Construct from a bare pool — for integration tests only.
    #[cfg(test)]
    #[must_use]
    pub const fn from_pool(pool: sqlx::PgPool) -> Self {
        Self {
            db: Database { pool },
        }
    }

    /// Return the user's currently-entitling subscription row, if any.
    ///
    /// "Entitling" = status in (trialing, active, past_due) and the period has
    /// not yet ended. The WHERE clause must stay in lockstep with
    /// `Status::grants_access`.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn find_active_for_user(&self, user_id: i32) -> ServerResult<Option<Subscription>> {
        crate::metrics::db::timed("subscription.find_active_for_user", async {
            Self::find_active_for_user_with(&mut *self.db.pool.acquire().await?, user_id).await
        })
        .await
    }

    pub(crate) async fn find_active_for_user_with(
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
            Self::find_by_stripe_id_with(
                &mut *self.db.pool.acquire().await?,
                stripe_subscription_id,
            )
            .await
        })
        .await
    }

    pub(crate) async fn find_by_stripe_id_with(
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

        let sub = SubscriptionMapper::find_active_for_user_with(&mut tx, user_id)
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

        let sub = SubscriptionMapper::find_active_for_user_with(&mut tx, user_id)
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

        let sub = SubscriptionMapper::find_active_for_user_with(&mut tx, user_id)
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

        let sub = SubscriptionMapper::find_active_for_user_with(&mut tx, user_id)
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

        let sub = SubscriptionMapper::find_active_for_user_with(&mut tx, user_id)
            .await
            .unwrap();
        assert!(sub.is_none());
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn find_active_returns_none_for_user_with_no_subscription() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;

        let sub = SubscriptionMapper::find_active_for_user_with(&mut tx, user_id)
            .await
            .unwrap();
        assert!(sub.is_none());
        tx.rollback().await.unwrap();
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

        let sub = SubscriptionMapper::find_by_stripe_id_with(&mut tx, "sub_known")
            .await
            .unwrap();
        assert!(sub.is_some());
        assert_eq!(sub.unwrap().user_id, user_id);

        let none = SubscriptionMapper::find_by_stripe_id_with(&mut tx, "sub_missing")
            .await
            .unwrap();
        assert!(none.is_none());
        tx.rollback().await.unwrap();
    }
}
