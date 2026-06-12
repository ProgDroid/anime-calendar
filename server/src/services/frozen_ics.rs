use sqlx::PgPool;

use crate::ServerResult;
use crate::services::ics_export::IcsExportService;

/// Manages the `calendars.frozen_subscribe_ics` lifecycle.
///
/// The frozen blob is what Free-tier subscribers receive when they hit
/// the public subscribe URL. It captures the calendar state at the moment
/// the user last edited (or downgraded), and only refreshes when the user
/// edits again. Pro users serve live data; their blobs are nulled out.
///
/// All write paths log per-calendar failures but don't abort whole-user
/// loops — partial frozen state is preferable to none.
#[derive(Clone)]
pub struct FrozenIcsService {
    pool: PgPool,
    ics_export: IcsExportService,
}

impl FrozenIcsService {
    #[must_use]
    pub const fn new(pool: PgPool, ics_export: IcsExportService) -> Self {
        Self { pool, ics_export }
    }

    /// Render the calendar's current state and store the result in
    /// `frozen_subscribe_ics`. Idempotent — calling twice with no calendar
    /// changes produces the same blob.
    ///
    /// # Errors
    /// Renders or DB writes can fail; both surface to the caller.
    pub async fn regenerate(&self, calendar_id: i32) -> ServerResult<()> {
        let blob = self.ics_export.render(calendar_id).await?;
        sqlx::query!(
            "UPDATE calendars SET frozen_subscribe_ics = $1 WHERE id = $2",
            blob,
            calendar_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Regenerate frozen blobs for every non-deleted calendar owned by the
    /// user. Per-calendar failures are logged and counted but don't abort
    /// the loop. Returns the number of failures so the caller can decide
    /// whether to alert.
    ///
    /// # Errors
    /// Returns errors only if the *list* query fails. Per-calendar errors
    /// during render/write are absorbed into the failure count.
    pub async fn regenerate_for_user(&self, user_id: i32) -> ServerResult<usize> {
        let calendar_ids: Vec<i32> = sqlx::query_scalar!(
            "SELECT id FROM calendars WHERE user_id = $1 AND deleted_at IS NULL",
            user_id,
        )
        .fetch_all(&self.pool)
        .await?;

        let total = calendar_ids.len();
        let mut failures = 0_usize;
        for cid in calendar_ids {
            if let Err(e) = self.regenerate(cid).await {
                log::error!("FrozenIcsService::regenerate_for_user: calendar {cid} failed: {e:?}");
                failures += 1;
            }
        }
        log::info!(
            "FrozenIcsService::regenerate_for_user: user {user_id} regenerated {ok}/{total} calendars",
            ok = total - failures,
        );
        Ok(failures)
    }

    /// Null the frozen blob for every calendar owned by the user. Called
    /// on Free → Pro transition (Pro users serve live data).
    ///
    /// # Errors
    /// Database error.
    pub async fn clear_for_user(&self, user_id: i32) -> ServerResult<()> {
        sqlx::query!(
            "UPDATE calendars SET frozen_subscribe_ics = NULL WHERE user_id = $1",
            user_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::config::server::CacheConfig;
    use crate::mappers::anilist::Anilist;
    use crate::services::cached_anilist::CachedAnilist;

    async fn build_services(pool: PgPool) -> (IcsExportService, FrozenIcsService) {
        use crate::config::server::LimitsConfig;
        use crate::mappers::subscription::SubscriptionMapper;
        use crate::mappers::user_settings::UserSettingsMapper;
        use crate::services::entitlement::EntitlementService;
        use crate::services::show_count::ShowCountService;

        let cached = CachedAnilist::new(
            Anilist::default(),
            Cache::for_tests().await,
            &CacheConfig::default(),
        );
        let user_settings_mapper = UserSettingsMapper::from_pool(pool.clone());
        let entitlement = EntitlementService::new(
            SubscriptionMapper::from_pool(pool.clone()),
            ShowCountService::new(pool.clone()),
            &LimitsConfig::default(),
        );
        let ics_export =
            IcsExportService::new(pool.clone(), cached, user_settings_mapper, entitlement);
        let frozen = FrozenIcsService::new(pool, ics_export.clone());
        (ics_export, frozen)
    }

    async fn seed_user(pool: &PgPool) -> i32 {
        let n: u64 = rand::random();
        let username = format!("frozen_{n}");
        let email = format!("frozen_{n}@test.com");
        sqlx::query_scalar(
            "INSERT INTO users (username, email, password_hash) \
             VALUES ($1, $2, 'hash') RETURNING id",
        )
        .bind(username)
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn seed_calendar(pool: &PgPool, user_id: i32, name: &str) -> i32 {
        let n: u64 = rand::random();
        let token = format!("tok-{n}");
        sqlx::query_scalar(
            "INSERT INTO calendars (name, language, user_id, subscription_token) \
             VALUES ($1, 'english'::language, $2, $3) RETURNING id",
        )
        .bind(name)
        .bind(user_id)
        .bind(token)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn frozen_blob(pool: &PgPool, calendar_id: i32) -> Option<String> {
        sqlx::query_scalar("SELECT frozen_subscribe_ics FROM calendars WHERE id = $1")
            .bind(calendar_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn cleanup_user(pool: &PgPool, user_id: i32) {
        sqlx::query(
            "DELETE FROM calendar_items WHERE calendar_id IN \
             (SELECT id FROM calendars WHERE user_id = $1)",
        )
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("DELETE FROM calendars WHERE user_id = $1")
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

    #[tokio::test]
    async fn regenerate_writes_non_null_blob() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user(&pool).await;
        let cal_id = seed_calendar(&pool, user_id, "Cal").await;

        let (_, frozen) = build_services(pool.clone()).await;
        frozen.regenerate(cal_id).await.expect("regenerate");

        let blob = frozen_blob(&pool, cal_id).await;
        assert!(blob.is_some());
        assert!(blob.unwrap().contains("BEGIN:VCALENDAR"));

        cleanup_user(&pool, user_id).await;
    }

    #[tokio::test]
    async fn regenerate_is_idempotent() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user(&pool).await;
        let cal_id = seed_calendar(&pool, user_id, "Cal").await;

        let (_, frozen) = build_services(pool.clone()).await;
        frozen.regenerate(cal_id).await.expect("first");
        let first = frozen_blob(&pool, cal_id).await;
        frozen.regenerate(cal_id).await.expect("second");
        let second = frozen_blob(&pool, cal_id).await;

        assert_eq!(first, second);

        cleanup_user(&pool, user_id).await;
    }

    #[tokio::test]
    async fn regenerate_for_user_writes_all_calendars() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user(&pool).await;
        let c1 = seed_calendar(&pool, user_id, "A").await;
        let c2 = seed_calendar(&pool, user_id, "B").await;
        let c3 = seed_calendar(&pool, user_id, "C").await;

        let (_, frozen) = build_services(pool.clone()).await;
        let failures = frozen.regenerate_for_user(user_id).await.expect("loop");

        assert_eq!(failures, 0);
        for cid in [c1, c2, c3] {
            assert!(frozen_blob(&pool, cid).await.is_some());
        }

        cleanup_user(&pool, user_id).await;
    }

    #[tokio::test]
    async fn regenerate_for_user_skips_other_users() {
        let pool = crate::test_helpers::test_pool().await;
        let user_a = seed_user(&pool).await;
        let user_b = seed_user(&pool).await;
        let a1 = seed_calendar(&pool, user_a, "A1").await;
        let b1 = seed_calendar(&pool, user_b, "B1").await;

        let (_, frozen) = build_services(pool.clone()).await;
        frozen.regenerate_for_user(user_a).await.expect("loop");

        assert!(frozen_blob(&pool, a1).await.is_some());
        assert!(frozen_blob(&pool, b1).await.is_none());

        cleanup_user(&pool, user_a).await;
        cleanup_user(&pool, user_b).await;
    }

    #[tokio::test]
    async fn regenerate_for_user_skips_soft_deleted() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user(&pool).await;
        let active = seed_calendar(&pool, user_id, "Active").await;
        let deleted = seed_calendar(&pool, user_id, "Deleted").await;
        sqlx::query("UPDATE calendars SET deleted_at = NOW() WHERE id = $1")
            .bind(deleted)
            .execute(&pool)
            .await
            .unwrap();

        let (_, frozen) = build_services(pool.clone()).await;
        frozen.regenerate_for_user(user_id).await.expect("loop");

        assert!(frozen_blob(&pool, active).await.is_some());
        assert!(frozen_blob(&pool, deleted).await.is_none());

        cleanup_user(&pool, user_id).await;
    }

    #[tokio::test]
    async fn clear_for_user_nulls_all_blobs() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user(&pool).await;
        let c1 = seed_calendar(&pool, user_id, "A").await;
        let c2 = seed_calendar(&pool, user_id, "B").await;

        let (_, frozen) = build_services(pool.clone()).await;
        frozen.regenerate_for_user(user_id).await.expect("seed");
        assert!(frozen_blob(&pool, c1).await.is_some());
        assert!(frozen_blob(&pool, c2).await.is_some());

        frozen.clear_for_user(user_id).await.expect("clear");
        assert!(frozen_blob(&pool, c1).await.is_none());
        assert!(frozen_blob(&pool, c2).await.is_none());

        cleanup_user(&pool, user_id).await;
    }
}
