use crate::{
    ServerResult, config::database::Database as DatabaseConfig, entity::calendar_editor::CalendarEditor,
    mappers::database::Database,
};

#[derive(Clone)]
pub struct CalendarEditorMapper {
    db: Database,
}

impl CalendarEditorMapper {
    /// # Errors
    /// Fails if database connection fails.
    pub async fn new(config: DatabaseConfig) -> ServerResult<Self> {
        Ok(Self {
            db: Database::new(config).await?,
        })
    }

    /// Construct a mapper from a bare pool — for integration tests only.
    #[cfg(test)]
    #[must_use]
    pub const fn from_pool(pool: sqlx::PgPool) -> Self {
        Self {
            db: Database { pool },
        }
    }

    /// Insert an editor row, or set `active=true, suspended_at=NULL` if it
    /// already exists. Used both for first-time invitation acceptance and
    /// for restoring previously-suspended editors after a Pro re-upgrade.
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    pub async fn upsert_active(&self, calendar_id: i32, user_id: i32) -> ServerResult<()> {
        crate::metrics::db::timed("calendar_editor.upsert_active", async {
            Self::upsert_active_in_tx(&mut *self.db.pool.acquire().await?, calendar_id, user_id)
                .await
        })
        .await
    }

    pub(crate) async fn upsert_active_in_tx(
        conn: &mut sqlx::PgConnection,
        calendar_id: i32,
        user_id: i32,
    ) -> ServerResult<()> {
        sqlx::query!(
            "INSERT INTO calendar_editors (calendar_id, user_id, active, suspended_at)
             VALUES ($1, $2, true, NULL)
             ON CONFLICT (calendar_id, user_id) DO UPDATE
                 SET active = true, suspended_at = NULL",
            calendar_id,
            user_id,
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Hard-delete an editor row. Used for owner-initiated removal — there is
    /// no "restore" path, so the row goes away entirely. Returns the number
    /// of rows actually deleted (0 or 1 in practice).
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    pub async fn remove(&self, calendar_id: i32, user_id: i32) -> ServerResult<u64> {
        crate::metrics::db::timed("calendar_editor.remove", async {
            Self::remove_in_tx(&mut *self.db.pool.acquire().await?, calendar_id, user_id).await
        })
        .await
    }

    pub(crate) async fn remove_in_tx(
        conn: &mut sqlx::PgConnection,
        calendar_id: i32,
        user_id: i32,
    ) -> ServerResult<u64> {
        let result = sqlx::query!(
            "DELETE FROM calendar_editors WHERE calendar_id = $1 AND user_id = $2",
            calendar_id,
            user_id,
        )
        .execute(conn)
        .await?;
        Ok(result.rows_affected())
    }

    /// True iff this user is currently an active editor of this calendar.
    /// Suspended editors return `false`.
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    pub async fn is_active_editor(
        &self,
        calendar_id: i32,
        user_id: i32,
    ) -> ServerResult<bool> {
        crate::metrics::db::timed("calendar_editor.is_active_editor", async {
            Self::is_active_editor_in_tx(
                &mut *self.db.pool.acquire().await?,
                calendar_id,
                user_id,
            )
            .await
        })
        .await
    }

    pub(crate) async fn is_active_editor_in_tx(
        conn: &mut sqlx::PgConnection,
        calendar_id: i32,
        user_id: i32,
    ) -> ServerResult<bool> {
        let exists: Option<bool> = sqlx::query_scalar!(
            "SELECT EXISTS(
                SELECT 1 FROM calendar_editors
                WHERE calendar_id = $1 AND user_id = $2 AND active = true
            )",
            calendar_id,
            user_id,
        )
        .fetch_one(conn)
        .await?;
        Ok(exists.unwrap_or(false))
    }

    /// Calendar IDs where this user is currently an active editor.
    /// Used to populate the "shared with me" section of GET /calendars.
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    pub async fn list_calendars_for_user(&self, user_id: i32) -> ServerResult<Vec<i32>> {
        crate::metrics::db::timed("calendar_editor.list_calendars_for_user", async {
            Self::list_calendars_for_user_in_tx(
                &mut *self.db.pool.acquire().await?,
                user_id,
            )
            .await
        })
        .await
    }

    pub(crate) async fn list_calendars_for_user_in_tx(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
    ) -> ServerResult<Vec<i32>> {
        let ids = sqlx::query_scalar!(
            "SELECT calendar_id FROM calendar_editors
             WHERE user_id = $1 AND active = true",
            user_id,
        )
        .fetch_all(conn)
        .await?;
        Ok(ids)
    }

    /// Count active editors on a calendar. Tx-only because the only caller
    /// (cap enforcement) must run it inside the same advisory-locked
    /// transaction as the subsequent INSERT to avoid race conditions.
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    // First production caller lands in Phase 1 (invitation accept cap check).
    #[allow(dead_code)]
    pub(crate) async fn count_active_in_tx(
        conn: &mut sqlx::PgConnection,
        calendar_id: i32,
    ) -> ServerResult<i64> {
        let count: Option<i64> = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM calendar_editors
             WHERE calendar_id = $1 AND active = true",
            calendar_id,
        )
        .fetch_one(conn)
        .await?;
        Ok(count.unwrap_or(0))
    }

    /// Soft-suspend every active editor for a calendar (sets `active=false`,
    /// `suspended_at=NOW()`). Used when an owner downgrades from Pro and we
    /// want to preserve the membership history for restore-on-re-upgrade.
    /// Idempotent — already-inactive rows are not re-suspended. Returns the
    /// number of rows actually transitioned.
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    pub async fn suspend_for_calendar(&self, calendar_id: i32) -> ServerResult<u64> {
        crate::metrics::db::timed("calendar_editor.suspend_for_calendar", async {
            Self::suspend_for_calendar_in_tx(&mut *self.db.pool.acquire().await?, calendar_id).await
        })
        .await
    }

    pub(crate) async fn suspend_for_calendar_in_tx(
        conn: &mut sqlx::PgConnection,
        calendar_id: i32,
    ) -> ServerResult<u64> {
        let result = sqlx::query!(
            "UPDATE calendar_editors
             SET active = false, suspended_at = NOW()
             WHERE calendar_id = $1 AND active = true",
            calendar_id,
        )
        .execute(conn)
        .await?;
        Ok(result.rows_affected())
    }

    /// Restore previously-suspended editors. Only flips rows where
    /// `suspended_at IS NOT NULL`, so editors the owner removed between
    /// suspend and restore are NOT resurrected. Returns the number of rows
    /// actually transitioned.
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    // First production caller lands in Phase 4 (Stripe re-upgrade reconcile).
    #[allow(dead_code)]
    pub(crate) async fn restore_for_calendar_in_tx(
        conn: &mut sqlx::PgConnection,
        calendar_id: i32,
    ) -> ServerResult<u64> {
        let result = sqlx::query!(
            "UPDATE calendar_editors
             SET active = true, suspended_at = NULL
             WHERE calendar_id = $1 AND suspended_at IS NOT NULL",
            calendar_id,
        )
        .execute(conn)
        .await?;
        Ok(result.rows_affected())
    }

    /// List active editors for a calendar (oldest joined first).
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    pub async fn list_active(&self, calendar_id: i32) -> ServerResult<Vec<CalendarEditor>> {
        crate::metrics::db::timed("calendar_editor.list_active", async {
            Self::list_active_in_tx(&mut *self.db.pool.acquire().await?, calendar_id).await
        })
        .await
    }

    pub(crate) async fn list_active_in_tx(
        conn: &mut sqlx::PgConnection,
        calendar_id: i32,
    ) -> ServerResult<Vec<CalendarEditor>> {
        let rows = sqlx::query!(
            "SELECT calendar_id, user_id, active, suspended_at, joined_at
             FROM calendar_editors
             WHERE calendar_id = $1 AND active = true
             ORDER BY joined_at",
            calendar_id,
        )
        .fetch_all(conn)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| CalendarEditor {
                calendar_id: r.calendar_id,
                user_id: r.user_id,
                active: r.active,
                suspended_at: r.suspended_at,
                joined_at: r.joined_at,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::calendar::{Calendar, Language};
    use crate::mappers::{calendar::CalendarMapper, user::UserMapper};

    async fn create_test_user(conn: &mut sqlx::PgConnection) -> i32 {
        let n: u64 = rand::random();
        UserMapper::create_user_in_tx(
            conn,
            &format!("editortest_{n}"),
            &format!("editortest_{n}@example.com"),
            None,
        )
        .await
        .unwrap()
        .id
    }

    async fn create_test_calendar(conn: &mut sqlx::PgConnection, owner_id: i32) -> i32 {
        CalendarMapper::insert_calendar_in_tx(
            conn,
            Calendar {
                id: 0,
                name: "Test".to_string(),
                item_ids: vec![],
                language: Language::English,
                subscription_token: String::new(),
                user_id: owner_id,
                created_at: chrono::NaiveDateTime::default(),
                updated_at: chrono::NaiveDateTime::default(),
                event_style: "timed".to_owned(),
                frozen_subscribe_ics: None,
                meta_version: 1,
            },
        )
        .await
        .unwrap()
        .id
    }

    #[tokio::test]
    async fn remove_returns_one_when_row_exists_and_zero_otherwise() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let editor = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal, editor)
            .await
            .unwrap();

        let removed = CalendarEditorMapper::remove_in_tx(&mut tx, cal, editor)
            .await
            .unwrap();
        assert_eq!(removed, 1);

        let removed_again = CalendarEditorMapper::remove_in_tx(&mut tx, cal, editor)
            .await
            .unwrap();
        assert_eq!(removed_again, 0, "second remove finds nothing");

        let active = CalendarEditorMapper::list_active_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert!(active.is_empty());

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn is_active_editor_distinguishes_active_suspended_and_missing() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let active_editor = create_test_user(&mut tx).await;
        let suspended_editor = create_test_user(&mut tx).await;
        let stranger = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal, active_editor)
            .await
            .unwrap();
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal, suspended_editor)
            .await
            .unwrap();
        // Suspend only the second editor (one-off, can't use suspend_for_calendar
        // because that would suspend both).
        sqlx::query!(
            "UPDATE calendar_editors SET active = false, suspended_at = NOW()
             WHERE calendar_id = $1 AND user_id = $2",
            cal,
            suspended_editor,
        )
        .execute(&mut *tx)
        .await
        .unwrap();

        assert!(
            CalendarEditorMapper::is_active_editor_in_tx(&mut tx, cal, active_editor)
                .await
                .unwrap()
        );
        assert!(
            !CalendarEditorMapper::is_active_editor_in_tx(&mut tx, cal, suspended_editor)
                .await
                .unwrap()
        );
        assert!(
            !CalendarEditorMapper::is_active_editor_in_tx(&mut tx, cal, stranger)
                .await
                .unwrap()
        );

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn list_calendars_for_user_returns_only_active_memberships() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let editor = create_test_user(&mut tx).await;
        let cal_a = create_test_calendar(&mut tx, owner).await;
        let cal_b = create_test_calendar(&mut tx, owner).await;
        let cal_c = create_test_calendar(&mut tx, owner).await;

        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal_a, editor)
            .await
            .unwrap();
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal_b, editor)
            .await
            .unwrap();
        // Membership in C exists but is suspended — must not appear.
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal_c, editor)
            .await
            .unwrap();
        CalendarEditorMapper::suspend_for_calendar_in_tx(&mut tx, cal_c)
            .await
            .unwrap();

        let mut ids = CalendarEditorMapper::list_calendars_for_user_in_tx(&mut tx, editor)
            .await
            .unwrap();
        ids.sort_unstable();
        let mut expected = vec![cal_a, cal_b];
        expected.sort_unstable();
        assert_eq!(ids, expected);

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn count_active_matches_list_active_length() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        for _ in 0..3 {
            let editor = create_test_user(&mut tx).await;
            CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal, editor)
                .await
                .unwrap();
        }
        // Add a fourth editor and immediately suspend everyone — count must be 0.
        let extra = create_test_user(&mut tx).await;
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal, extra)
            .await
            .unwrap();

        let count = CalendarEditorMapper::count_active_in_tx(&mut tx, cal)
            .await
            .unwrap();
        let len = CalendarEditorMapper::list_active_in_tx(&mut tx, cal)
            .await
            .unwrap()
            .len();
        assert_eq!(count, i64::try_from(len).unwrap());
        assert_eq!(count, 4);

        CalendarEditorMapper::suspend_for_calendar_in_tx(&mut tx, cal)
            .await
            .unwrap();
        let count_after = CalendarEditorMapper::count_active_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert_eq!(count_after, 0);

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn suspend_then_restore_round_trips() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let editor_a = create_test_user(&mut tx).await;
        let editor_b = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal, editor_a)
            .await
            .unwrap();
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal, editor_b)
            .await
            .unwrap();

        let suspended = CalendarEditorMapper::suspend_for_calendar_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert_eq!(suspended, 2);

        // After suspend, list_active is empty
        let active = CalendarEditorMapper::list_active_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert!(active.is_empty());

        // suspend is idempotent — running again finds zero rows to suspend
        let suspended_again = CalendarEditorMapper::suspend_for_calendar_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert_eq!(suspended_again, 0);

        // Restore brings them back
        let restored = CalendarEditorMapper::restore_for_calendar_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert_eq!(restored, 2);

        let active_after = CalendarEditorMapper::list_active_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert_eq!(active_after.len(), 2);
        assert!(active_after.iter().all(|e| e.suspended_at.is_none()));

        // Restore is idempotent — no suspended rows means zero affected
        let restored_again = CalendarEditorMapper::restore_for_calendar_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert_eq!(restored_again, 0);

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn restore_does_not_resurrect_removed_rows() {
        // If owner removes an editor between downgrade-suspend and re-upgrade-restore,
        // restore must not bring that editor back. Only suspended rows are restored.
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let kept = create_test_user(&mut tx).await;
        let removed = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal, kept)
            .await
            .unwrap();
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal, removed)
            .await
            .unwrap();

        CalendarEditorMapper::suspend_for_calendar_in_tx(&mut tx, cal)
            .await
            .unwrap();

        // Owner removes the second editor while suspended (hard delete)
        sqlx::query!(
            "DELETE FROM calendar_editors WHERE calendar_id = $1 AND user_id = $2",
            cal,
            removed,
        )
        .execute(&mut *tx)
        .await
        .unwrap();

        let restored = CalendarEditorMapper::restore_for_calendar_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert_eq!(restored, 1, "only the kept editor should be restored");

        let active = CalendarEditorMapper::list_active_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].user_id, kept);

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn upsert_active_is_idempotent() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let editor = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal, editor)
            .await
            .unwrap();
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal, editor)
            .await
            .unwrap();

        let editors = CalendarEditorMapper::list_active_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert_eq!(editors.len(), 1);
        assert_eq!(editors[0].user_id, editor);
        assert_eq!(editors[0].calendar_id, cal);
        assert!(editors[0].active);
        assert!(editors[0].suspended_at.is_none());

        tx.rollback().await.unwrap();
    }
}
