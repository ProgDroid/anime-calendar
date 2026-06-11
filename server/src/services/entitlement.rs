use std::str::FromStr;

use crate::{
    ServerResult,
    config::server::LimitsConfig,
    entity::subscription::{Entitlement, Tier},
    error::Error,
    mappers::subscription::SubscriptionMapper,
    services::show_count::ShowCountService,
};

/// Effective-tier read service. Wraps `SubscriptionMapper` so handlers and
/// middleware never need to know the entitlement rules — just call
/// `effective_tier(user_id)` or `entitlement(user_id)`.
///
/// v1 deliberately does not cache: the read is a single indexed lookup and
/// caching introduces invalidation bugs the moment Stripe changes a sub.
#[derive(Clone)]
pub struct EntitlementService {
    mapper: SubscriptionMapper,
    show_count: ShowCountService,
    free_calendar_limit: i64,
    free_show_cap: i64,
}

impl EntitlementService {
    #[must_use]
    pub fn new(
        mapper: SubscriptionMapper,
        show_count: ShowCountService,
        limits: &LimitsConfig,
    ) -> Self {
        Self {
            mapper,
            show_count,
            free_calendar_limit: i64::from(limits.free_calendar_limit),
            free_show_cap: i64::from(limits.free_show_cap),
        }
    }

    /// Return just the tier — for fast gates that only branch on free/paid.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn effective_tier(&self, user_id: i32) -> ServerResult<Tier> {
        let sub = self.mapper.find_active_for_user(user_id).await?;
        Ok(sub.as_ref().map_or(Tier::Free, |s| {
            Tier::from_str(&s.tier).unwrap_or(Tier::Free)
        }))
    }

    /// Return the full entitlement view — for the Subscription tab and
    /// anywhere the UI needs status / period / trial context.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn entitlement(&self, user_id: i32) -> ServerResult<Entitlement> {
        let sub = self.mapper.find_active_for_user(user_id).await?;
        Ok(sub
            .as_ref()
            .map_or_else(Entitlement::free, Entitlement::from_subscription))
    }

    /// Returns Ok if the user can create another calendar at their tier.
    /// Pro users always pass; Free users blocked at `free_calendar_limit`.
    ///
    /// # Errors
    /// `Error::PaymentRequired { reason: Some("cap_calendars") }` if the cap
    /// would be exceeded. Database error variants on count failure.
    pub async fn assert_can_create_calendar(&self, user_id: i32) -> ServerResult<()> {
        if matches!(self.effective_tier(user_id).await?, Tier::Paid) {
            return Ok(());
        }
        let count = self.show_count.count_calendars_for_user(user_id).await?;
        if count >= self.free_calendar_limit {
            return Err(Error::PaymentRequired {
                required_tier: "paid",
                reason: Some("cap_calendars"),
            });
        }
        Ok(())
    }

    /// Returns Ok if the user can add `item_id` to a calendar without
    /// breaching the show cap. Idempotent: an item the user already tracks
    /// in any of their non-deleted calendars passes regardless of cap state.
    ///
    /// # Errors
    /// `Error::PaymentRequired { reason: Some("cap_shows") }` when adding a
    /// new (untracked) item at the free cap. Database errors otherwise.
    pub async fn assert_can_add_show(&self, user_id: i32, item_id: i32) -> ServerResult<()> {
        if matches!(self.effective_tier(user_id).await?, Tier::Paid) {
            return Ok(());
        }
        if self.show_count.user_already_tracks(user_id, item_id).await? {
            return Ok(()); // idempotent — already counted
        }
        let count = self.show_count.count_distinct_for_user(user_id).await?;
        if count >= self.free_show_cap {
            return Err(Error::PaymentRequired {
                required_tier: "paid",
                reason: Some("cap_shows"),
            });
        }
        Ok(())
    }

    /// In-transaction variant of `effective_tier`. Reads the active
    /// subscription on the caller's connection so callers that hold a
    /// `pg_advisory_xact_lock` observe a consistent snapshot.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn effective_tier_in_tx(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
    ) -> ServerResult<Tier> {
        let sub = SubscriptionMapper::find_active_for_user_in_tx(conn, user_id).await?;
        Ok(sub.as_ref().map_or(Tier::Free, |s| {
            Tier::from_str(&s.tier).unwrap_or(Tier::Free)
        }))
    }

    /// In-transaction variant of `assert_can_create_calendar`. Runs the cap
    /// query against the caller's connection — the only correct place to
    /// enforce the cap when a `pg_advisory_xact_lock` is held, because the
    /// pool-bound version observes pre-INSERT state from concurrent txs.
    ///
    /// # Errors
    /// `Error::PaymentRequired { reason: Some("cap_calendars") }` if the cap
    /// would be exceeded; database errors on count failure.
    pub async fn assert_can_create_calendar_in_tx(
        &self,
        conn: &mut sqlx::PgConnection,
        user_id: i32,
    ) -> ServerResult<()> {
        if matches!(Self::effective_tier_in_tx(&mut *conn, user_id).await?, Tier::Paid) {
            return Ok(());
        }
        let count = ShowCountService::count_calendars_for_user_in_tx(conn, user_id).await?;
        if count >= self.free_calendar_limit {
            return Err(Error::PaymentRequired {
                required_tier: "paid",
                reason: Some("cap_calendars"),
            });
        }
        Ok(())
    }

    /// In-transaction variant of `assert_can_add_show`. Same idempotency
    /// rule (already-tracked items pass even at cap), but observes the
    /// caller's tx snapshot — pair with `pg_advisory_xact_lock` to make
    /// the cap honest under concurrent writers.
    ///
    /// # Errors
    /// `Error::PaymentRequired { reason: Some("cap_shows") }` when adding a
    /// new (untracked) item at the free cap; database errors otherwise.
    pub async fn assert_can_add_show_in_tx(
        &self,
        conn: &mut sqlx::PgConnection,
        user_id: i32,
        item_id: i32,
    ) -> ServerResult<()> {
        if matches!(Self::effective_tier_in_tx(&mut *conn, user_id).await?, Tier::Paid) {
            return Ok(());
        }
        if ShowCountService::user_already_tracks_in_tx(&mut *conn, user_id, item_id).await? {
            return Ok(()); // idempotent — already counted
        }
        let count = ShowCountService::count_distinct_for_user_in_tx(conn, user_id).await?;
        if count >= self.free_show_cap {
            return Err(Error::PaymentRequired {
                required_tier: "paid",
                reason: Some("cap_shows"),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mappers::calendar::CalendarMapper;
    use crate::entity::calendar::Calendar;

    async fn create_test_user(pool: &sqlx::PgPool) -> i32 {
        let n: u64 = rand::random();
        sqlx::query_scalar("INSERT INTO users (username, email) VALUES ($1, $2) RETURNING id")
            .bind(format!("entituser_{n}"))
            .bind(format!("entit_{n}@example.com"))
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn make_paid(pool: &sqlx::PgPool, user_id: i32) {
        use chrono::{Duration, Utc};
        let now = Utc::now().naive_utc();
        sqlx::query(
            "INSERT INTO subscriptions \
             (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
              stripe_price_id, current_period_start, current_period_end, \
              cancel_at_period_end) \
             VALUES ($1, 'paid', 'active', $2, $3, 'price_test', $4, $5, false)",
        )
        .bind(user_id)
        .bind(format!("cus_test_{user_id}"))
        .bind(format!("sub_test_{user_id}"))
        .bind(now)
        .bind(now + Duration::days(30))
        .execute(pool)
        .await
        .unwrap();
    }

    async fn insert_calendar(pool: &sqlx::PgPool, user_id: i32, items: Vec<i32>) -> i32 {
        let cal = CalendarMapper::from_pool(pool.clone())
            .insert_calendar(Calendar {
                user_id,
                item_ids: items,
                ..Calendar::default()
            })
            .await
            .unwrap();
        cal.id
    }

    fn limits(free_calendar_limit: u32, free_show_cap: u32) -> LimitsConfig {
        LimitsConfig {
            free_calendar_limit,
            free_show_cap,
            pro_max_reminders: 5,
        }
    }

    fn build_service(pool: &sqlx::PgPool, limits: &LimitsConfig) -> EntitlementService {
        EntitlementService::new(
            SubscriptionMapper::from_pool(pool.clone()),
            ShowCountService::new(pool.clone()),
            limits,
        )
    }

    #[tokio::test]
    async fn assert_can_create_calendar_pro_passes() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        make_paid(&pool, user_id).await;
        // Even with N calendars at the free cap, paid is unlimited.
        for _ in 0..5 {
            insert_calendar(&pool, user_id, vec![]).await;
        }
        let svc = build_service(&pool, &limits(3, 25));
        svc.assert_can_create_calendar(user_id).await.unwrap();

        // Cleanup
        sqlx::query("DELETE FROM subscriptions WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM calendars WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn assert_can_create_calendar_free_under_cap_passes() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        // 2 calendars, cap = 3 → still room.
        insert_calendar(&pool, user_id, vec![]).await;
        insert_calendar(&pool, user_id, vec![]).await;
        let svc = build_service(&pool, &limits(3, 25));
        svc.assert_can_create_calendar(user_id).await.unwrap();

        sqlx::query("DELETE FROM calendars WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn assert_can_create_calendar_free_at_cap_blocks() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        // 3 calendars at cap 3 → blocked.
        for _ in 0..3 {
            insert_calendar(&pool, user_id, vec![]).await;
        }
        let svc = build_service(&pool, &limits(3, 25));
        let err = svc.assert_can_create_calendar(user_id).await.unwrap_err();
        match err {
            Error::PaymentRequired {
                required_tier,
                reason,
            } => {
                assert_eq!(required_tier, "paid");
                assert_eq!(reason, Some("cap_calendars"));
            }
            other => panic!("expected PaymentRequired, got {other:?}"),
        }

        sqlx::query("DELETE FROM calendars WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn assert_can_add_show_pro_unlimited() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        make_paid(&pool, user_id).await;
        // Cap is 2 but paid user has 5 distinct items already → still passes.
        let cal_id = insert_calendar(&pool, user_id, vec![1, 2, 3, 4, 5]).await;
        let svc = build_service(&pool, &limits(3, 2));
        svc.assert_can_add_show(user_id, 999).await.unwrap();

        sqlx::query("DELETE FROM calendar_items WHERE calendar_id = $1")
            .bind(cal_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM subscriptions WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM calendars WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn assert_can_add_show_free_under_cap_passes() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        let cal_id = insert_calendar(&pool, user_id, vec![1, 2]).await;
        let svc = build_service(&pool, &limits(3, 5));
        svc.assert_can_add_show(user_id, 99).await.unwrap();

        sqlx::query("DELETE FROM calendar_items WHERE calendar_id = $1")
            .bind(cal_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM calendars WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn assert_can_add_show_idempotent_on_already_tracked() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        // At cap (2 distinct items), but adding an already-tracked item → passes.
        let cal_id = insert_calendar(&pool, user_id, vec![10, 20]).await;
        let svc = build_service(&pool, &limits(3, 2));
        svc.assert_can_add_show(user_id, 10).await.unwrap();

        sqlx::query("DELETE FROM calendar_items WHERE calendar_id = $1")
            .bind(cal_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM calendars WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn effective_tier_in_tx_returns_paid_for_active_sub() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        make_paid(&pool, user_id).await;

        let mut conn = pool.acquire().await.unwrap();
        let tier = EntitlementService::effective_tier_in_tx(&mut conn, user_id)
            .await
            .unwrap();
        assert!(matches!(tier, Tier::Paid));

        sqlx::query("DELETE FROM subscriptions WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn assert_can_create_calendar_in_tx_blocks_at_cap() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        for _ in 0..3 {
            insert_calendar(&pool, user_id, vec![]).await;
        }
        let svc = build_service(&pool, &limits(3, 25));

        let mut conn = pool.acquire().await.unwrap();
        let err = svc
            .assert_can_create_calendar_in_tx(&mut conn, user_id)
            .await
            .unwrap_err();
        match err {
            Error::PaymentRequired { reason, .. } => {
                assert_eq!(reason, Some("cap_calendars"));
            }
            other => panic!("expected PaymentRequired, got {other:?}"),
        }

        sqlx::query("DELETE FROM calendars WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn assert_can_add_show_in_tx_passes_for_already_tracked_at_cap() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        let cal_id = insert_calendar(&pool, user_id, vec![10, 20]).await;
        let svc = build_service(&pool, &limits(3, 2));

        let mut conn = pool.acquire().await.unwrap();
        // Already tracked → passes even at cap.
        svc.assert_can_add_show_in_tx(&mut conn, user_id, 10)
            .await
            .unwrap();
        // New at cap → blocked.
        let err = svc
            .assert_can_add_show_in_tx(&mut conn, user_id, 999)
            .await
            .unwrap_err();
        match err {
            Error::PaymentRequired { reason, .. } => {
                assert_eq!(reason, Some("cap_shows"));
            }
            other => panic!("expected PaymentRequired, got {other:?}"),
        }

        sqlx::query("DELETE FROM calendar_items WHERE calendar_id = $1")
            .bind(cal_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM calendars WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn assert_can_add_show_blocks_new_item_at_cap() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        let cal_id = insert_calendar(&pool, user_id, vec![10, 20]).await;
        let svc = build_service(&pool, &limits(3, 2));
        let err = svc.assert_can_add_show(user_id, 999).await.unwrap_err();
        match err {
            Error::PaymentRequired {
                required_tier,
                reason,
            } => {
                assert_eq!(required_tier, "paid");
                assert_eq!(reason, Some("cap_shows"));
            }
            other => panic!("expected PaymentRequired, got {other:?}"),
        }

        sqlx::query("DELETE FROM calendar_items WHERE calendar_id = $1")
            .bind(cal_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM calendars WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
    }
}
