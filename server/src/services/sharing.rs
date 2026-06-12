//! Sharing suspension helpers used by both the Stripe webhook controller
//! and the reconcile safety-net loop.
//!
//! These live in the service layer so neither the controller nor the loop
//! needs to import from each other — the inverted-dependency would otherwise
//! couple controllers → services in the wrong direction.

use crate::ServerResult;
use crate::mappers::calendar_editor::CalendarEditorMapper;
use crate::mappers::calendar_invitation::CalendarInvitationMapper;

/// A calendar-editor/user pair collected during a Paid→Free downgrade so
/// that `MemberLeft` + `kick` events can be published after the outer
/// transaction commits (Redis Pub/Sub must fire post-commit).
#[derive(Debug, Clone)]
pub struct KickRecord {
    pub calendar_id: i32,
    pub user_id: i32,
}

/// A calendar-editor/user pair collected during a Free→Paid upgrade so that
/// "access restored" emails can be sent to each editor after the outer
/// transaction commits (SMTP must fire post-commit).
#[derive(Debug, Clone)]
pub struct RestoredEditor {
    pub calendar_id: i32,
    pub calendar_name: String,
    pub user_id: i32,
}

/// Restore suspended editors and suspended invitations for every calendar owned
/// by `owner_id`, inside an existing transaction. Returns a list of editors
/// that were actually restored (with their calendar names) so that the caller
/// can send "access restored" emails post-commit.
///
/// Only rows where `suspended_at IS NOT NULL` are restored for editors (rows
/// the owner hard-deleted between suspend and re-upgrade are not resurrected).
/// Only `suspended` invitations that are still unexpired are moved back to
/// `pending`.
///
/// Called by both `controllers::stripe_webhook` (inside the webhook
/// transaction) and `services::reconcile` (inside a short restore transaction)
/// after detecting a Free→Paid transition.
///
/// # Errors
/// Returns `Error::Database` if any query fails.
pub async fn restore_owner_sharing_in_tx(
    tx: &mut sqlx::PgConnection,
    owner_id: i32,
) -> ServerResult<Vec<RestoredEditor>> {
    // Fetch all calendar IDs + names owned by this user.
    let calendars = sqlx::query!(
        "SELECT id, name FROM calendars WHERE user_id = $1",
        owner_id
    )
    .fetch_all(&mut *tx)
    .await?;

    let mut restored_editors: Vec<RestoredEditor> = Vec::new();

    for cal in calendars {
        let calendar_id = cal.id;
        let calendar_name = cal.name.clone();

        // Capture suspended editors BEFORE restoring so we know who to email.
        let suspended_user_ids: Vec<i32> = sqlx::query_scalar!(
            "SELECT user_id FROM calendar_editors \
             WHERE calendar_id = $1 AND suspended_at IS NOT NULL",
            calendar_id,
        )
        .fetch_all(&mut *tx)
        .await?;

        // Restore suspended editors.
        CalendarEditorMapper::restore_for_calendar_in_tx(&mut *tx, calendar_id).await?;

        // Restore suspended invitations.
        CalendarInvitationMapper::restore_suspended_for_calendar_in_tx(&mut *tx, calendar_id)
            .await?;

        // Collect restored-editor records for email sending post-commit.
        for user_id in suspended_user_ids {
            restored_editors.push(RestoredEditor {
                calendar_id,
                calendar_name: calendar_name.clone(),
                user_id,
            });
        }
    }

    Ok(restored_editors)
}

/// Suspend all active editors and pending invitations for every calendar owned
/// by `owner_id`, collecting a `KickRecord` for each editor that was active
/// before the suspend. All writes happen on `tx` so they are rolled back
/// atomically with the caller's transaction if anything fails downstream.
///
/// Called by both `controllers::stripe_webhook` (inside the webhook
/// transaction) and `services::reconcile` (inside a short suspend transaction)
/// after detecting a Paid→Free transition.
///
/// # Errors
/// Returns `Error::Database` if any query fails.
pub async fn suspend_owner_sharing_in_tx(
    tx: &mut sqlx::PgConnection,
    owner_id: i32,
) -> ServerResult<Vec<KickRecord>> {
    // Fetch all calendar IDs owned by this user.
    let calendar_ids: Vec<i32> =
        sqlx::query_scalar!("SELECT id FROM calendars WHERE user_id = $1", owner_id)
            .fetch_all(&mut *tx)
            .await?;

    let mut kick_records: Vec<KickRecord> = Vec::new();

    for calendar_id in calendar_ids {
        // Capture active editors BEFORE suspending so we know who to kick.
        let active_editors = CalendarEditorMapper::list_active_in_tx(&mut *tx, calendar_id).await?;

        // Suspend active editors.
        CalendarEditorMapper::suspend_for_calendar_in_tx(&mut *tx, calendar_id).await?;

        // Suspend pending invitations.
        CalendarInvitationMapper::suspend_pending_for_calendar_in_tx(&mut *tx, calendar_id).await?;

        // Collect kick records for every editor that was active.
        for editor in active_editors {
            kick_records.push(KickRecord {
                calendar_id,
                user_id: editor.user_id,
            });
        }
    }

    Ok(kick_records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mappers::calendar_editor::CalendarEditorMapper;
    use crate::mappers::calendar_invitation::CalendarInvitationMapper;
    use chrono::{Duration, Utc};

    async fn seed_user(conn: &mut sqlx::PgConnection) -> i32 {
        let n: u64 = rand::random();
        sqlx::query_scalar::<_, i32>(
            "INSERT INTO users (username, email, password_hash) \
             VALUES ($1, $2, 'hash') RETURNING id",
        )
        .bind(format!("sharingtest_{n}"))
        .bind(format!("sharingtest_{n}@example.com"))
        .fetch_one(&mut *conn)
        .await
        .unwrap()
    }

    async fn seed_calendar(conn: &mut sqlx::PgConnection, user_id: i32, name: &str) -> i32 {
        let n: u64 = rand::random();
        sqlx::query_scalar::<_, i32>(
            "INSERT INTO calendars (name, language, user_id, subscription_token) \
             VALUES ($1, 'english'::language, $2, $3) RETURNING id",
        )
        .bind(name)
        .bind(user_id)
        .bind(format!("tok-restore-{n}"))
        .fetch_one(&mut *conn)
        .await
        .unwrap()
    }

    /// Free→Paid upgrade: `restore_owner_sharing_in_tx` must:
    /// - Flip `active=true, suspended_at=NULL` for every suspended editor row.
    /// - Flip `status='pending'` for every suspended invitation still unexpired.
    /// - Return one `RestoredEditor` per restored editor with the correct
    ///   `calendar_id`, `user_id`, and `calendar_name`.
    #[tokio::test]
    async fn upgrade_restores_editors_and_invites() {
        let mut tx = crate::test_helpers::test_tx().await;

        // Seed owner with 2 calendars.
        let owner_id = seed_user(&mut tx).await;
        let cal_a = seed_calendar(&mut tx, owner_id, "Calendar Alpha").await;
        let cal_b = seed_calendar(&mut tx, owner_id, "Calendar Beta").await;

        // Seed 2 editors per calendar.
        let editor_a1 = seed_user(&mut tx).await;
        let editor_a2 = seed_user(&mut tx).await;
        let editor_b1 = seed_user(&mut tx).await;
        let editor_b2 = seed_user(&mut tx).await;

        let inv_expires = (Utc::now() + Duration::days(7)).naive_utc();

        // Insert active editors then suspend them all.
        CalendarEditorMapper::upsert_active_in_tx(&mut *tx, cal_a, editor_a1)
            .await
            .unwrap();
        CalendarEditorMapper::upsert_active_in_tx(&mut *tx, cal_a, editor_a2)
            .await
            .unwrap();
        CalendarEditorMapper::upsert_active_in_tx(&mut *tx, cal_b, editor_b1)
            .await
            .unwrap();
        CalendarEditorMapper::upsert_active_in_tx(&mut *tx, cal_b, editor_b2)
            .await
            .unwrap();
        CalendarEditorMapper::suspend_for_calendar_in_tx(&mut *tx, cal_a)
            .await
            .unwrap();
        CalendarEditorMapper::suspend_for_calendar_in_tx(&mut *tx, cal_b)
            .await
            .unwrap();

        // Seed 1 suspended invitation per calendar.
        let na: u64 = rand::random();
        CalendarInvitationMapper::create_in_tx(
            &mut *tx,
            cal_a,
            owner_id,
            &format!("inv_a_{na}@example.com"),
            &format!("hash_a_{na}"),
            inv_expires,
        )
        .await
        .unwrap();
        CalendarInvitationMapper::suspend_pending_for_calendar_in_tx(&mut *tx, cal_a)
            .await
            .unwrap();

        let nb: u64 = rand::random();
        CalendarInvitationMapper::create_in_tx(
            &mut *tx,
            cal_b,
            owner_id,
            &format!("inv_b_{nb}@example.com"),
            &format!("hash_b_{nb}"),
            inv_expires,
        )
        .await
        .unwrap();
        CalendarInvitationMapper::suspend_pending_for_calendar_in_tx(&mut *tx, cal_b)
            .await
            .unwrap();

        // Act: call the function under test using the same transaction connection.
        let restored = restore_owner_sharing_in_tx(&mut *tx, owner_id)
            .await
            .unwrap();

        // Assert: all editors are now active with suspended_at = NULL.
        let active_a: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM calendar_editors \
             WHERE calendar_id = $1 AND active = true AND suspended_at IS NULL",
        )
        .bind(cal_a)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(active_a, 2, "cal_a: 2 active editors after restore");

        let active_b: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM calendar_editors \
             WHERE calendar_id = $1 AND active = true AND suspended_at IS NULL",
        )
        .bind(cal_b)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(active_b, 2, "cal_b: 2 active editors after restore");

        // Assert: all invitations are now pending.
        let pending_a: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM calendar_invitations \
             WHERE calendar_id = $1 AND status = 'pending'",
        )
        .bind(cal_a)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(pending_a, 1, "cal_a: 1 pending invitation after restore");

        let pending_b: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM calendar_invitations \
             WHERE calendar_id = $1 AND status = 'pending'",
        )
        .bind(cal_b)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(pending_b, 1, "cal_b: 1 pending invitation after restore");

        // Assert: RestoredEditor list covers all 4 editors with correct names.
        assert_eq!(restored.len(), 4, "exactly 4 restored-editor records");
        let mut pairs: Vec<(i32, i32, String)> = restored
            .iter()
            .map(|r| (r.calendar_id, r.user_id, r.calendar_name.clone()))
            .collect();
        pairs.sort_unstable_by_key(|(cal, uid, _)| (*cal, *uid));

        let mut expected = vec![
            (cal_a, editor_a1, "Calendar Alpha".to_string()),
            (cal_a, editor_a2, "Calendar Alpha".to_string()),
            (cal_b, editor_b1, "Calendar Beta".to_string()),
            (cal_b, editor_b2, "Calendar Beta".to_string()),
        ];
        expected.sort_unstable_by_key(|(cal, uid, _)| (*cal, *uid));

        assert_eq!(pairs, expected, "restored records must match all editors");

        // tx drops here and rolls back automatically — no manual cleanup needed.
        tx.rollback().await.unwrap();
    }
}
