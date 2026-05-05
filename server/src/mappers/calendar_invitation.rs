use chrono::NaiveDateTime;

use crate::{
    ServerResult,
    config::database::Database as DatabaseConfig,
    entity::calendar_invitation::{CalendarInvitation, InvitationStatus},
    error::Error,
    mappers::database::Database,
};

#[derive(Clone)]
pub struct CalendarInvitationMapper {
    db: Database,
}

impl CalendarInvitationMapper {
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

    /// Insert a new pending invitation.
    ///
    /// The partial unique index on `(calendar_id, lower(invitee_email))
    /// WHERE status='pending'` rejects a second pending invite for the same
    /// calendar/email pair until the first is resolved (accepted/declined/
    /// revoked/expired/suspended).
    ///
    /// # Errors
    /// Returns the underlying sqlx error. The unique-violation surfaces as
    /// `sqlx::Error::Database` with constraint name
    /// `idx_calendar_invitations_pending_unique`.
    // First production caller lands in Phase 1 (POST /calendars/:id/invitations).
    #[allow(dead_code)]
    pub async fn create(
        &self,
        calendar_id: i32,
        inviter_id: i32,
        invitee_email: &str,
        token_hash: &str,
        expires_at: NaiveDateTime,
    ) -> ServerResult<CalendarInvitation> {
        crate::metrics::db::timed("calendar_invitation.create", async {
            Self::create_in_tx(
                &mut *self.db.pool.acquire().await?,
                calendar_id,
                inviter_id,
                invitee_email,
                token_hash,
                expires_at,
            )
            .await
        })
        .await
    }

    pub(crate) async fn create_in_tx(
        conn: &mut sqlx::PgConnection,
        calendar_id: i32,
        inviter_id: i32,
        invitee_email: &str,
        token_hash: &str,
        expires_at: NaiveDateTime,
    ) -> ServerResult<CalendarInvitation> {
        let row = sqlx::query!(
            "INSERT INTO calendar_invitations
                (calendar_id, inviter_id, invitee_email, token_hash, expires_at)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id,
                       calendar_id,
                       inviter_id,
                       invitee_email,
                       token_hash,
                       status as \"status: InvitationStatus\",
                       expires_at,
                       sent_at,
                       resolved_at",
            calendar_id,
            inviter_id,
            invitee_email,
            token_hash,
            expires_at,
        )
        .fetch_one(conn)
        .await?;

        Ok(CalendarInvitation {
            id: row.id,
            calendar_id: row.calendar_id,
            inviter_id: row.inviter_id,
            invitee_email: row.invitee_email,
            token_hash: row.token_hash,
            status: row.status,
            expires_at: row.expires_at,
            sent_at: row.sent_at,
            resolved_at: row.resolved_at,
        })
    }

    /// Look up a pending invitation by its hashed token, gated on
    /// `expires_at > NOW()`. Returns `None` for missing, expired, or
    /// already-resolved tokens — callers cannot distinguish these cases,
    /// which is intentional (anti-enumeration).
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    // First production caller lands in Phase 1 (POST /invitations/accept).
    #[allow(dead_code)]
    pub async fn find_pending_by_token_hash(
        &self,
        token_hash: &str,
    ) -> ServerResult<Option<CalendarInvitation>> {
        crate::metrics::db::timed("calendar_invitation.find_pending_by_token_hash", async {
            Self::find_pending_by_token_hash_in_tx(
                &mut *self.db.pool.acquire().await?,
                token_hash,
            )
            .await
        })
        .await
    }

    pub(crate) async fn find_pending_by_token_hash_in_tx(
        conn: &mut sqlx::PgConnection,
        token_hash: &str,
    ) -> ServerResult<Option<CalendarInvitation>> {
        let row = sqlx::query!(
            "SELECT id,
                    calendar_id,
                    inviter_id,
                    invitee_email,
                    token_hash,
                    status as \"status: InvitationStatus\",
                    expires_at,
                    sent_at,
                    resolved_at
             FROM calendar_invitations
             WHERE token_hash = $1
               AND status = 'pending'
               AND expires_at > NOW()",
            token_hash,
        )
        .fetch_optional(conn)
        .await?;

        Ok(row.map(|r| CalendarInvitation {
            id: r.id,
            calendar_id: r.calendar_id,
            inviter_id: r.inviter_id,
            invitee_email: r.invitee_email,
            token_hash: r.token_hash,
            status: r.status,
            expires_at: r.expires_at,
            sent_at: r.sent_at,
            resolved_at: r.resolved_at,
        }))
    }

    /// Look up an invitation by its primary key, ignoring status.
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    // First production caller lands in Phase 1 (DELETE /invitations/:id revoke).
    #[allow(dead_code)]
    pub async fn find_by_id(&self, id: i64) -> ServerResult<Option<CalendarInvitation>> {
        crate::metrics::db::timed("calendar_invitation.find_by_id", async {
            Self::find_by_id_in_tx(&mut *self.db.pool.acquire().await?, id).await
        })
        .await
    }

    pub(crate) async fn find_by_id_in_tx(
        conn: &mut sqlx::PgConnection,
        id: i64,
    ) -> ServerResult<Option<CalendarInvitation>> {
        let row = sqlx::query!(
            "SELECT id,
                    calendar_id,
                    inviter_id,
                    invitee_email,
                    token_hash,
                    status as \"status: InvitationStatus\",
                    expires_at,
                    sent_at,
                    resolved_at
             FROM calendar_invitations
             WHERE id = $1",
            id,
        )
        .fetch_optional(conn)
        .await?;

        Ok(row.map(|r| CalendarInvitation {
            id: r.id,
            calendar_id: r.calendar_id,
            inviter_id: r.inviter_id,
            invitee_email: r.invitee_email,
            token_hash: r.token_hash,
            status: r.status,
            expires_at: r.expires_at,
            sent_at: r.sent_at,
            resolved_at: r.resolved_at,
        }))
    }

    /// All pending invitations for a calendar, newest first. Used by the
    /// owner's editor-management UI.
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    // First production caller lands in Phase 1 (GET /calendars/:id/invitations).
    #[allow(dead_code)]
    pub async fn list_pending_for_calendar(
        &self,
        calendar_id: i32,
    ) -> ServerResult<Vec<CalendarInvitation>> {
        crate::metrics::db::timed("calendar_invitation.list_pending_for_calendar", async {
            Self::list_pending_for_calendar_in_tx(
                &mut *self.db.pool.acquire().await?,
                calendar_id,
            )
            .await
        })
        .await
    }

    pub(crate) async fn list_pending_for_calendar_in_tx(
        conn: &mut sqlx::PgConnection,
        calendar_id: i32,
    ) -> ServerResult<Vec<CalendarInvitation>> {
        let rows = sqlx::query!(
            "SELECT id,
                    calendar_id,
                    inviter_id,
                    invitee_email,
                    token_hash,
                    status as \"status: InvitationStatus\",
                    expires_at,
                    sent_at,
                    resolved_at
             FROM calendar_invitations
             WHERE calendar_id = $1 AND status = 'pending'
             ORDER BY sent_at DESC",
            calendar_id,
        )
        .fetch_all(conn)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| CalendarInvitation {
                id: r.id,
                calendar_id: r.calendar_id,
                inviter_id: r.inviter_id,
                invitee_email: r.invitee_email,
                token_hash: r.token_hash,
                status: r.status,
                expires_at: r.expires_at,
                sent_at: r.sent_at,
                resolved_at: r.resolved_at,
            })
            .collect())
    }

    /// Count pending invitations for a calendar. Used together with
    /// `CalendarEditorMapper::count_active_in_tx` under an advisory lock to
    /// enforce the editor cap (each pending invite "reserves" a slot).
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    // First production caller lands in Phase 1 (cap-enforced invite create).
    #[allow(dead_code)]
    pub(crate) async fn count_pending_in_tx(
        conn: &mut sqlx::PgConnection,
        calendar_id: i32,
    ) -> ServerResult<i64> {
        let count: Option<i64> = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM calendar_invitations
             WHERE calendar_id = $1 AND status = 'pending'",
            calendar_id,
        )
        .fetch_one(conn)
        .await?;
        Ok(count.unwrap_or(0))
    }

    /// Count invitations sent by `inviter_id` to `invitee_email` since
    /// `since`, regardless of status. Used for the per-(inviter,email)
    /// hourly rate-limit on resends.
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    // First production caller lands in Phase 1 (invite rate limit guard).
    #[allow(dead_code)]
    pub async fn count_pending_for_inviter_since(
        &self,
        inviter_id: i32,
        invitee_email: &str,
        since: NaiveDateTime,
    ) -> ServerResult<i64> {
        crate::metrics::db::timed(
            "calendar_invitation.count_pending_for_inviter_since",
            async {
                Self::count_pending_for_inviter_since_in_tx(
                    &mut *self.db.pool.acquire().await?,
                    inviter_id,
                    invitee_email,
                    since,
                )
                .await
            },
        )
        .await
    }

    pub(crate) async fn count_pending_for_inviter_since_in_tx(
        conn: &mut sqlx::PgConnection,
        inviter_id: i32,
        invitee_email: &str,
        since: NaiveDateTime,
    ) -> ServerResult<i64> {
        let count: Option<i64> = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM calendar_invitations
             WHERE inviter_id = $1
               AND lower(invitee_email::text) = lower($2)
               AND sent_at >= $3",
            inviter_id,
            invitee_email,
            since,
        )
        .fetch_one(conn)
        .await?;
        Ok(count.unwrap_or(0))
    }

    /// Transition a pending invitation to a terminal state, stamping
    /// `resolved_at = NOW()`. Returns 0 if the row was not pending (e.g.
    /// already accepted in a concurrent request) — callers must treat that
    /// as "lost the race" rather than success.
    ///
    /// # Errors
    /// Returns the underlying sqlx error or `Error::InvalidRequest` if the
    /// caller passes `Pending` or `Suspended` (invalid terminal targets
    /// for this method).
    // First production caller lands in Phase 1 (accept/decline/revoke).
    #[allow(dead_code)]
    pub async fn mark_resolved(
        &self,
        id: i64,
        status: InvitationStatus,
    ) -> ServerResult<u64> {
        crate::metrics::db::timed("calendar_invitation.mark_resolved", async {
            Self::mark_resolved_in_tx(&mut *self.db.pool.acquire().await?, id, status).await
        })
        .await
    }

    pub(crate) async fn mark_resolved_in_tx(
        conn: &mut sqlx::PgConnection,
        id: i64,
        status: InvitationStatus,
    ) -> ServerResult<u64> {
        if matches!(status, InvitationStatus::Pending | InvitationStatus::Suspended) {
            return Err(Error::InvalidRequest);
        }
        let result = sqlx::query!(
            "UPDATE calendar_invitations
             SET status = $2, resolved_at = NOW()
             WHERE id = $1 AND status = 'pending'",
            id,
            status as InvitationStatus,
        )
        .execute(conn)
        .await?;
        Ok(result.rows_affected())
    }

    /// Move every pending invitation for a calendar to `suspended`. Used
    /// when the owner downgrades from Pro: pending invites freeze in place
    /// so the recipient cannot accept while sharing is disabled. Tx-only
    /// because the caller batches this with `CalendarEditorMapper::
    /// suspend_for_calendar_in_tx` under one transaction.
    ///
    /// Note: `resolved_at` is **not** stamped — `suspended` is reversible.
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    // First production caller lands in Phase 4 (Stripe downgrade reconcile).
    #[allow(dead_code)]
    pub(crate) async fn suspend_pending_for_calendar_in_tx(
        conn: &mut sqlx::PgConnection,
        calendar_id: i32,
    ) -> ServerResult<u64> {
        let result = sqlx::query!(
            "UPDATE calendar_invitations
             SET status = 'suspended'
             WHERE calendar_id = $1 AND status = 'pending'",
            calendar_id,
        )
        .execute(conn)
        .await?;
        Ok(result.rows_affected())
    }

    /// Move suspended invitations back to pending **iff still unexpired**.
    /// Used on Pro re-upgrade. Expired suspended rows are left alone (a
    /// follow-up sweep transitions them to `expired`).
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    // First production caller lands in Phase 4 (Stripe re-upgrade reconcile).
    #[allow(dead_code)]
    pub(crate) async fn restore_suspended_for_calendar_in_tx(
        conn: &mut sqlx::PgConnection,
        calendar_id: i32,
    ) -> ServerResult<u64> {
        let result = sqlx::query!(
            "UPDATE calendar_invitations
             SET status = 'pending'
             WHERE calendar_id = $1
               AND status = 'suspended'
               AND expires_at > NOW()",
            calendar_id,
        )
        .execute(conn)
        .await?;
        Ok(result.rows_affected())
    }

    /// Push a pending invitation's `expires_at` forward. Used by the
    /// resend flow so a recipient can request a fresh deadline without
    /// the inviter creating a brand-new row (and tripping the partial
    /// unique index).
    ///
    /// # Errors
    /// Returns the underlying sqlx error.
    // First production caller lands in Phase 1 (POST /invitations/:id/resend).
    #[allow(dead_code)]
    pub async fn bump_expiry(
        &self,
        id: i64,
        new_expires_at: NaiveDateTime,
    ) -> ServerResult<u64> {
        crate::metrics::db::timed("calendar_invitation.bump_expiry", async {
            Self::bump_expiry_in_tx(
                &mut *self.db.pool.acquire().await?,
                id,
                new_expires_at,
            )
            .await
        })
        .await
    }

    pub(crate) async fn bump_expiry_in_tx(
        conn: &mut sqlx::PgConnection,
        id: i64,
        new_expires_at: NaiveDateTime,
    ) -> ServerResult<u64> {
        let result = sqlx::query!(
            "UPDATE calendar_invitations
             SET expires_at = $2
             WHERE id = $1 AND status = 'pending'",
            id,
            new_expires_at,
        )
        .execute(conn)
        .await?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::calendar::{Calendar, Language};
    use crate::mappers::{calendar::CalendarMapper, user::UserMapper};
    use chrono::{Duration, Utc};

    async fn create_test_user(conn: &mut sqlx::PgConnection) -> i32 {
        let n: u64 = rand::random();
        UserMapper::create_user_with(
            conn,
            &format!("invitetest_{n}"),
            &format!("invitetest_{n}@example.com"),
            None,
        )
        .await
        .unwrap()
        .id
    }

    async fn create_test_calendar(conn: &mut sqlx::PgConnection, owner_id: i32) -> i32 {
        CalendarMapper::insert_calendar_with(
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

    fn future_expiry() -> NaiveDateTime {
        (Utc::now() + Duration::days(7)).naive_utc()
    }

    fn past_expiry() -> NaiveDateTime {
        (Utc::now() - Duration::days(1)).naive_utc()
    }

    #[tokio::test]
    async fn create_inserts_pending_row_with_defaults() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        let inv = CalendarInvitationMapper::create_in_tx(
            &mut tx,
            cal,
            owner,
            "Bob@Example.COM",
            "hash_a",
            future_expiry(),
        )
        .await
        .unwrap();

        assert_eq!(inv.calendar_id, cal);
        assert_eq!(inv.inviter_id, owner);
        // citext preserves the input casing on read; uniqueness is case-insensitive.
        assert_eq!(inv.invitee_email, "Bob@Example.COM");
        assert_eq!(inv.token_hash, "hash_a");
        assert_eq!(inv.status, InvitationStatus::Pending);
        assert!(inv.resolved_at.is_none());

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn partial_unique_index_blocks_second_pending_for_same_email() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "dup@example.com", "h1", future_expiry(),
        )
        .await
        .unwrap();

        // Same email, different casing — must still be rejected (citext + lower()).
        let result = CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "DUP@example.com", "h2", future_expiry(),
        )
        .await;

        assert!(result.is_err(), "duplicate pending invite must be rejected");

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn revoke_then_recreate_succeeds() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        let first = CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "user@example.com", "h1", future_expiry(),
        )
        .await
        .unwrap();

        // Revoke the first
        let affected = CalendarInvitationMapper::mark_resolved_in_tx(
            &mut tx,
            first.id,
            InvitationStatus::Revoked,
        )
        .await
        .unwrap();
        assert_eq!(affected, 1);

        // Now a fresh pending invite is allowed
        let second = CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "user@example.com", "h2", future_expiry(),
        )
        .await
        .unwrap();
        assert_eq!(second.status, InvitationStatus::Pending);
        assert_ne!(second.id, first.id);

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn find_pending_by_token_hash_excludes_expired_and_resolved() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        // Active row
        let active = CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "active@example.com", "active_hash", future_expiry(),
        )
        .await
        .unwrap();
        assert!(
            CalendarInvitationMapper::find_pending_by_token_hash_in_tx(&mut tx, "active_hash")
                .await
                .unwrap()
                .is_some()
        );

        // Expired row
        let expired = CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "expired@example.com", "expired_hash", future_expiry(),
        )
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE calendar_invitations SET expires_at = $2 WHERE id = $1",
            expired.id,
            past_expiry(),
        )
        .execute(&mut *tx)
        .await
        .unwrap();
        assert!(
            CalendarInvitationMapper::find_pending_by_token_hash_in_tx(&mut tx, "expired_hash")
                .await
                .unwrap()
                .is_none()
        );

        // Resolved row
        CalendarInvitationMapper::mark_resolved_in_tx(
            &mut tx,
            active.id,
            InvitationStatus::Accepted,
        )
        .await
        .unwrap();
        assert!(
            CalendarInvitationMapper::find_pending_by_token_hash_in_tx(&mut tx, "active_hash")
                .await
                .unwrap()
                .is_none()
        );

        // Unknown token
        assert!(
            CalendarInvitationMapper::find_pending_by_token_hash_in_tx(&mut tx, "no_such_hash")
                .await
                .unwrap()
                .is_none()
        );

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn count_pending_in_tx_excludes_terminal_statuses() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        let pending = CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "pending@example.com", "h_p", future_expiry(),
        )
        .await
        .unwrap();
        let to_accept = CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "accepted@example.com", "h_a", future_expiry(),
        )
        .await
        .unwrap();
        let to_revoke = CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "revoked@example.com", "h_r", future_expiry(),
        )
        .await
        .unwrap();

        CalendarInvitationMapper::mark_resolved_in_tx(
            &mut tx,
            to_accept.id,
            InvitationStatus::Accepted,
        )
        .await
        .unwrap();
        CalendarInvitationMapper::mark_resolved_in_tx(
            &mut tx,
            to_revoke.id,
            InvitationStatus::Revoked,
        )
        .await
        .unwrap();

        let count = CalendarInvitationMapper::count_pending_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert_eq!(count, 1, "only pending invitations count");

        let listed = CalendarInvitationMapper::list_pending_for_calendar_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, pending.id);

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn mark_resolved_rejects_non_terminal_status() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;
        let inv = CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "x@example.com", "h", future_expiry(),
        )
        .await
        .unwrap();

        assert!(
            CalendarInvitationMapper::mark_resolved_in_tx(
                &mut tx,
                inv.id,
                InvitationStatus::Pending
            )
            .await
            .is_err()
        );
        assert!(
            CalendarInvitationMapper::mark_resolved_in_tx(
                &mut tx,
                inv.id,
                InvitationStatus::Suspended
            )
            .await
            .is_err()
        );

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn mark_resolved_returns_zero_for_already_resolved() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;
        let inv = CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "x@example.com", "h", future_expiry(),
        )
        .await
        .unwrap();

        let first = CalendarInvitationMapper::mark_resolved_in_tx(
            &mut tx,
            inv.id,
            InvitationStatus::Accepted,
        )
        .await
        .unwrap();
        assert_eq!(first, 1);

        // Race-loser: same row, different terminal status — must affect zero rows.
        let second = CalendarInvitationMapper::mark_resolved_in_tx(
            &mut tx,
            inv.id,
            InvitationStatus::Declined,
        )
        .await
        .unwrap();
        assert_eq!(second, 0);

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn suspend_then_restore_only_restores_unexpired() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        let fresh = CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "fresh@example.com", "h_f", future_expiry(),
        )
        .await
        .unwrap();
        let stale = CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "stale@example.com", "h_s", future_expiry(),
        )
        .await
        .unwrap();

        let suspended = CalendarInvitationMapper::suspend_pending_for_calendar_in_tx(&mut tx, cal)
            .await
            .unwrap();
        assert_eq!(suspended, 2);

        // Push the second one's expiry into the past while suspended.
        sqlx::query!(
            "UPDATE calendar_invitations SET expires_at = $2 WHERE id = $1",
            stale.id,
            past_expiry(),
        )
        .execute(&mut *tx)
        .await
        .unwrap();

        let restored =
            CalendarInvitationMapper::restore_suspended_for_calendar_in_tx(&mut tx, cal)
                .await
                .unwrap();
        assert_eq!(restored, 1, "only the unexpired suspended row is restored");

        let still_suspended = CalendarInvitationMapper::find_by_id_in_tx(&mut tx, stale.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(still_suspended.status, InvitationStatus::Suspended);

        let restored_row = CalendarInvitationMapper::find_by_id_in_tx(&mut tx, fresh.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(restored_row.status, InvitationStatus::Pending);

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn count_pending_for_inviter_since_is_case_insensitive() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;
        let an_hour_ago = (Utc::now() - Duration::hours(1)).naive_utc();

        CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "Same@Example.COM", "h1", future_expiry(),
        )
        .await
        .unwrap();
        // Resolve so the next create can succeed under the partial unique index.
        let to_revoke = CalendarInvitationMapper::find_pending_by_token_hash_in_tx(&mut tx, "h1")
            .await
            .unwrap()
            .unwrap();
        CalendarInvitationMapper::mark_resolved_in_tx(
            &mut tx,
            to_revoke.id,
            InvitationStatus::Revoked,
        )
        .await
        .unwrap();

        CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "same@example.com", "h2", future_expiry(),
        )
        .await
        .unwrap();

        let count = CalendarInvitationMapper::count_pending_for_inviter_since_in_tx(
            &mut tx,
            owner,
            "SAME@example.com",
            an_hour_ago,
        )
        .await
        .unwrap();
        assert_eq!(count, 2, "case-insensitive match across invites");

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn bump_expiry_only_affects_pending_rows() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;
        // Postgres TIMESTAMP truncates to microseconds; build an instant
        // with zero sub-second to avoid round-trip mismatch.
        let new_deadline = chrono::NaiveDate::from_ymd_opt(2099, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let inv = CalendarInvitationMapper::create_in_tx(
            &mut tx, cal, owner, "bump@example.com", "h", future_expiry(),
        )
        .await
        .unwrap();

        let bumped = CalendarInvitationMapper::bump_expiry_in_tx(&mut tx, inv.id, new_deadline)
            .await
            .unwrap();
        assert_eq!(bumped, 1);

        let after = CalendarInvitationMapper::find_by_id_in_tx(&mut tx, inv.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.expires_at, new_deadline);

        // Resolve, then bump again — must not change anything.
        CalendarInvitationMapper::mark_resolved_in_tx(
            &mut tx,
            inv.id,
            InvitationStatus::Accepted,
        )
        .await
        .unwrap();
        let bumped_again =
            CalendarInvitationMapper::bump_expiry_in_tx(&mut tx, inv.id, future_expiry())
                .await
                .unwrap();
        assert_eq!(bumped_again, 0);

        tx.rollback().await.unwrap();
    }
}
