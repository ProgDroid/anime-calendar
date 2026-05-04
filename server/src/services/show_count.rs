use sqlx::PgPool;

/// Counts cap-relevant resources owned by a user. Backs the entitlement
/// service's free-tier cap checks.
#[derive(Clone)]
pub struct ShowCountService {
    pool: PgPool,
}

impl ShowCountService {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Distinct `AniList` media ids tracked across all of the user's
    /// non-deleted calendars. The "show cap" is metered against this
    /// count; duplicates across calendars cost one slot, not two.
    ///
    /// # Errors
    /// Returns the underlying sqlx error on query failure.
    pub async fn count_distinct_for_user(&self, user_id: i32) -> sqlx::Result<i64> {
        sqlx::query_scalar!(
            r#"SELECT COUNT(DISTINCT ci.item_id) AS "count!"
               FROM calendar_items ci
               JOIN calendars c ON c.id = ci.calendar_id
               WHERE c.user_id = $1
                 AND c.deleted_at IS NULL"#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Whether the user already tracks the given media id in any of their
    /// (non-deleted) calendars. Used by the "idempotent add" path so adds
    /// of an already-tracked id never trigger a 402 even when the user is
    /// at their cap.
    ///
    /// # Errors
    /// Returns the underlying sqlx error on query failure.
    pub async fn user_already_tracks(&self, user_id: i32, item_id: i32) -> sqlx::Result<bool> {
        sqlx::query_scalar!(
            r#"SELECT EXISTS(
                 SELECT 1 FROM calendar_items ci
                 JOIN calendars c ON c.id = ci.calendar_id
                 WHERE c.user_id = $1
                   AND c.deleted_at IS NULL
                   AND ci.item_id = $2
               ) AS "exists!""#,
            user_id,
            item_id,
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Count non-deleted calendars owned by the user.
    ///
    /// # Errors
    /// Returns the underlying sqlx error on query failure.
    pub async fn count_calendars_for_user(&self, user_id: i32) -> sqlx::Result<i64> {
        sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "count!"
               FROM calendars
               WHERE user_id = $1 AND deleted_at IS NULL"#,
            user_id,
        )
        .fetch_one(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::calendar::Calendar;
    use crate::mappers::calendar::CalendarMapper;

    async fn create_test_user(pool: &PgPool) -> i32 {
        let n: u64 = rand::random();
        sqlx::query_scalar("INSERT INTO users (username, email) VALUES ($1, $2) RETURNING id")
            .bind(format!("countuser_{n}"))
            .bind(format!("count_{n}@example.com"))
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn insert_calendar(pool: &PgPool, user_id: i32, items: Vec<i32>) -> i32 {
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

    async fn cleanup(pool: &PgPool, user_id: i32) {
        sqlx::query(
            "DELETE FROM calendar_items WHERE calendar_id IN (SELECT id FROM calendars WHERE user_id = $1)",
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
    async fn count_distinct_excludes_duplicates() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        insert_calendar(&pool, user_id, vec![100, 200]).await;
        insert_calendar(&pool, user_id, vec![100]).await;
        let svc = ShowCountService::new(pool.clone());
        let count = svc.count_distinct_for_user(user_id).await.unwrap();
        assert_eq!(count, 2);
        cleanup(&pool, user_id).await;
    }

    #[tokio::test]
    async fn count_distinct_excludes_other_users() {
        let pool = crate::test_helpers::test_pool().await;
        let user_a = create_test_user(&pool).await;
        let user_b = create_test_user(&pool).await;
        insert_calendar(&pool, user_a, vec![1]).await;
        let svc = ShowCountService::new(pool.clone());
        assert_eq!(svc.count_distinct_for_user(user_a).await.unwrap(), 1);
        assert_eq!(svc.count_distinct_for_user(user_b).await.unwrap(), 0);
        cleanup(&pool, user_a).await;
        cleanup(&pool, user_b).await;
    }

    #[tokio::test]
    async fn count_distinct_excludes_soft_deleted_calendars() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        let cal_id = insert_calendar(&pool, user_id, vec![42]).await;
        sqlx::query("UPDATE calendars SET deleted_at = NOW() WHERE id = $1")
            .bind(cal_id)
            .execute(&pool)
            .await
            .unwrap();
        let svc = ShowCountService::new(pool.clone());
        assert_eq!(svc.count_distinct_for_user(user_id).await.unwrap(), 0);
        cleanup(&pool, user_id).await;
    }

    #[tokio::test]
    async fn user_already_tracks_returns_true_for_any_calendar() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        insert_calendar(&pool, user_id, vec![42]).await;
        let svc = ShowCountService::new(pool.clone());
        assert!(svc.user_already_tracks(user_id, 42).await.unwrap());
        assert!(!svc.user_already_tracks(user_id, 99).await.unwrap());
        cleanup(&pool, user_id).await;
    }

    #[tokio::test]
    async fn count_calendars_for_user_excludes_soft_deleted() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = create_test_user(&pool).await;
        insert_calendar(&pool, user_id, vec![]).await;
        insert_calendar(&pool, user_id, vec![]).await;
        let third = insert_calendar(&pool, user_id, vec![]).await;
        sqlx::query("UPDATE calendars SET deleted_at = NOW() WHERE id = $1")
            .bind(third)
            .execute(&pool)
            .await
            .unwrap();
        let svc = ShowCountService::new(pool.clone());
        assert_eq!(svc.count_calendars_for_user(user_id).await.unwrap(), 2);
        cleanup(&pool, user_id).await;
    }
}
