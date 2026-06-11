//! Authorization spine for the co-editor sharing feature.
//!
//! Centralises four role/tier checks in one place so handlers don't ad-hoc
//! re-derive them:
//!
//! | Action          | Owner | Editor | Outsider |
//! |-----------------|-------|--------|----------|
//! | `ItemMutate`    | yes   | yes    | no       |
//! | `MetaMutate`    | yes   | no     | no       |
//! | `ManageEditors` | yes   | no     | no       |
//! | `Invite`        | yes\* | no     | no       |
//!
//! \* Owner must additionally be on the Paid tier — Free owners get
//! `Error::PaymentRequired { reason: "share_calendar" }` so the frontend
//! can route to the existing upgrade-interrupt modal.
//!
//! Both an instance method (`assert_can`) and a connection-bound helper
//! (`assert_can_in_tx`) are exposed: the latter lets cap-enforcement code
//! run the authz check inside the same advisory-locked transaction as the
//! `count_active_in_tx + count_pending_in_tx + INSERT` sequence.

use crate::{
    ServerResult,
    entity::{calendar::Calendar, subscription::Tier},
    error::Error,
    mappers::calendar_editor::CalendarEditorMapper,
    services::entitlement::EntitlementService,
};

/// What the caller is trying to do on a calendar. Kept coarse-grained on
/// purpose: finer roles can be layered on later without rewriting the
/// gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Add/remove items, edit per-item state. Owner + active editors.
    ItemMutate,
    /// Rename the calendar, change `language`/`event_style`, etc. Owner only.
    MetaMutate,
    /// List/remove editors (separate from Invite to keep "send" gated on
    /// tier while letting the owner clean up after a downgrade).
    ManageEditors,
    /// Send an invitation. Owner-only AND requires Paid tier.
    Invite,
}

#[derive(Clone)]
pub struct SharingAuthz {
    editors: CalendarEditorMapper,
    entitlements: EntitlementService,
}

impl SharingAuthz {
    #[must_use]
    pub const fn new(editors: CalendarEditorMapper, entitlements: EntitlementService) -> Self {
        Self {
            editors,
            entitlements,
        }
    }

    /// Authorize `actor_id` to perform `action` on `calendar`. Pool-bound
    /// path: each underlying check acquires its own connection.
    ///
    /// # Errors
    /// - `Error::Forbidden` if the actor lacks the role required.
    /// - `Error::PaymentRequired { reason: Some("share_calendar") }` for
    ///   `Invite` when the owner is on the Free tier.
    /// - Database error variants on infra failure.
    pub async fn assert_can(
        &self,
        actor_id: i32,
        calendar: &Calendar,
        action: Action,
    ) -> ServerResult<()> {
        let is_owner = actor_id == calendar.user_id;
        match action {
            Action::ItemMutate => {
                if is_owner {
                    return Ok(());
                }
                if self.editors.is_active_editor(calendar.id, actor_id).await? {
                    return Ok(());
                }
                Err(Error::Forbidden)
            }
            Action::MetaMutate | Action::ManageEditors => {
                if is_owner {
                    Ok(())
                } else {
                    Err(Error::Forbidden)
                }
            }
            Action::Invite => {
                if !is_owner {
                    return Err(Error::Forbidden);
                }
                if matches!(self.entitlements.effective_tier(actor_id).await?, Tier::Paid) {
                    Ok(())
                } else {
                    Err(Error::PaymentRequired {
                        required_tier: "paid",
                        reason: Some("share_calendar"),
                    })
                }
            }
        }
    }

    /// Connection-bound variant. Use when the caller is already inside a
    /// transaction (e.g. cap-enforcement under `pg_advisory_xact_lock`)
    /// so all reads observe the same snapshot.
    ///
    /// # Errors
    /// Same variants as `assert_can`.
    pub(crate) async fn assert_can_in_tx(
        conn: &mut sqlx::PgConnection,
        actor_id: i32,
        calendar: &Calendar,
        action: Action,
    ) -> ServerResult<()> {
        let is_owner = actor_id == calendar.user_id;
        match action {
            Action::ItemMutate => {
                if is_owner {
                    return Ok(());
                }
                if CalendarEditorMapper::is_active_editor_in_tx(conn, calendar.id, actor_id)
                    .await?
                {
                    return Ok(());
                }
                Err(Error::Forbidden)
            }
            Action::MetaMutate | Action::ManageEditors => {
                if is_owner {
                    Ok(())
                } else {
                    Err(Error::Forbidden)
                }
            }
            Action::Invite => {
                if !is_owner {
                    return Err(Error::Forbidden);
                }
                if matches!(
                    EntitlementService::effective_tier_in_tx(conn, actor_id).await?,
                    Tier::Paid
                ) {
                    Ok(())
                } else {
                    Err(Error::PaymentRequired {
                        required_tier: "paid",
                        reason: Some("share_calendar"),
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::calendar::{Calendar, Language};
    use crate::mappers::{
        calendar::CalendarMapper, calendar_editor::CalendarEditorMapper, user::UserMapper,
    };
    use chrono::{Duration, Utc};

    async fn create_test_user(conn: &mut sqlx::PgConnection) -> i32 {
        let n: u64 = rand::random();
        UserMapper::create_user_in_tx(
            conn,
            &format!("authz_{n}"),
            &format!("authz_{n}@example.com"),
            None,
        )
        .await
        .unwrap()
        .id
    }

    async fn create_test_calendar(conn: &mut sqlx::PgConnection, owner_id: i32) -> Calendar {
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
    }

    /// Insert a paid, active subscription row for `user_id`. Mirrors the
    /// `make_paid` helper in `entitlement.rs::tests`.
    async fn make_paid(conn: &mut sqlx::PgConnection, user_id: i32) {
        let now = Utc::now().naive_utc();
        sqlx::query(
            "INSERT INTO subscriptions \
             (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
              stripe_price_id, current_period_start, current_period_end, \
              cancel_at_period_end) \
             VALUES ($1, 'paid', 'active', $2, $3, 'price_test', $4, $5, false)",
        )
        .bind(user_id)
        .bind(format!("cus_authz_{user_id}"))
        .bind(format!("sub_authz_{user_id}"))
        .bind(now)
        .bind(now + Duration::days(30))
        .execute(conn)
        .await
        .unwrap();
    }

    fn assert_forbidden(result: &ServerResult<()>) {
        assert!(matches!(result, Err(Error::Forbidden)), "{result:?}");
    }

    fn assert_share_calendar_payment_required(result: &ServerResult<()>) {
        match result {
            Err(Error::PaymentRequired {
                required_tier,
                reason,
            }) => {
                assert_eq!(*required_tier, "paid");
                assert_eq!(*reason, Some("share_calendar"));
            }
            other => panic!("expected PaymentRequired share_calendar, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn owner_can_meta_mutate_and_manage_editors() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        SharingAuthz::assert_can_in_tx(&mut tx, owner, &cal, Action::MetaMutate)
            .await
            .unwrap();
        SharingAuthz::assert_can_in_tx(&mut tx, owner, &cal, Action::ManageEditors)
            .await
            .unwrap();

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn editor_cannot_meta_mutate_or_manage_editors() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let editor = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal.id, editor)
            .await
            .unwrap();

        assert_forbidden(&
            SharingAuthz::assert_can_in_tx(&mut tx, editor, &cal, Action::MetaMutate).await,
        );
        assert_forbidden(&
            SharingAuthz::assert_can_in_tx(&mut tx, editor, &cal, Action::ManageEditors).await,
        );

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn owner_and_editor_can_item_mutate() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let editor = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal.id, editor)
            .await
            .unwrap();

        SharingAuthz::assert_can_in_tx(&mut tx, owner, &cal, Action::ItemMutate)
            .await
            .unwrap();
        SharingAuthz::assert_can_in_tx(&mut tx, editor, &cal, Action::ItemMutate)
            .await
            .unwrap();

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn outsider_cannot_item_mutate() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let stranger = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        assert_forbidden(&
            SharingAuthz::assert_can_in_tx(&mut tx, stranger, &cal, Action::ItemMutate).await,
        );

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn suspended_editor_cannot_item_mutate() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let editor = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal.id, editor)
            .await
            .unwrap();
        // Owner downgrades from Pro → suspend the editor row.
        CalendarEditorMapper::suspend_for_calendar_in_tx(&mut tx, cal.id)
            .await
            .unwrap();

        assert_forbidden(&
            SharingAuthz::assert_can_in_tx(&mut tx, editor, &cal, Action::ItemMutate).await,
        );

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn free_owner_cannot_invite() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        assert_share_calendar_payment_required(&
            SharingAuthz::assert_can_in_tx(&mut tx, owner, &cal, Action::Invite).await,
        );

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn paid_owner_can_invite() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        make_paid(&mut tx, owner).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        SharingAuthz::assert_can_in_tx(&mut tx, owner, &cal, Action::Invite)
            .await
            .unwrap();

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn editor_cannot_invite_even_when_paid() {
        // Owner-only check beats the tier check: an editor on Pro still gets
        // Forbidden, not PaymentRequired.
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let editor = create_test_user(&mut tx).await;
        make_paid(&mut tx, editor).await;
        let cal = create_test_calendar(&mut tx, owner).await;
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal.id, editor)
            .await
            .unwrap();

        assert_forbidden(&
            SharingAuthz::assert_can_in_tx(&mut tx, editor, &cal, Action::Invite).await,
        );

        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn outsider_cannot_invite() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner = create_test_user(&mut tx).await;
        let stranger = create_test_user(&mut tx).await;
        make_paid(&mut tx, stranger).await;
        let cal = create_test_calendar(&mut tx, owner).await;

        assert_forbidden(&
            SharingAuthz::assert_can_in_tx(&mut tx, stranger, &cal, Action::Invite).await,
        );

        tx.rollback().await.unwrap();
    }
}
