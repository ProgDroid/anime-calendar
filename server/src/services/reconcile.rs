//! Hourly reconcile loop — webhook-loss safety net.
//!
//! Webhooks remain the primary path (sub-second upgrade unlock); this loop
//! pulls Stripe truth on a slow tick and conditionally corrects drift. See
//! `docs/superpowers/specs/2026-05-01-track-4-upgrade-flow-design.md` →
//! "Reconcile loop (webhook safety net)" for the design.
//!
//! Multi-replica note: this loop is single-process safe. Multi-replica
//! deployments need either `pg_try_advisory_lock` around the pass or an
//! out-of-process scheduler (k8s `CronJob`). Deferred until that's a real
//! concern.

use std::time::Duration;

use chrono::NaiveDateTime;
use log::{error, info, warn};
use metrics::counter;

use crate::ServerResult;
use crate::mappers::subscription::{ReconcileRow, SubscriptionMapper};
use crate::metrics::names;
use crate::services::calendar_events::{CalendarEvent, CalendarEventPublisher};

/// Optional sharing-suspension/restore dependencies for the reconcile loop.
/// When `Some`, a Paid→Free correction triggers editor + invitation suspension
/// and publishes `MemberLeft` + `kick` events via Redis Pub/Sub; a Free→Paid
/// correction restores editors and invitations and sends restore emails.
/// When `None`, reconcile corrects subscription data only (legacy behaviour,
/// used by `reconcile_for_user` in the CLI and by existing unit tests).
#[derive(Clone)]
pub struct SharingDeps {
    pub editor_mapper: crate::mappers::calendar_editor::CalendarEditorMapper,
    pub invitation_mapper: crate::mappers::calendar_invitation::CalendarInvitationMapper,
    pub publisher: CalendarEventPublisher,
    pub email_service: crate::services::email::EmailService,
    pub user_mapper: crate::mappers::user::UserMapper,
    /// A separate pool used to open the short suspend/restore transaction. Must be
    /// independent of the `SubscriptionMapper`'s pool so the advisory-lock
    /// path in the editors mapper stays clean.
    pub pool: sqlx::PgPool,
}

/// What the reconcile loop needs from Stripe. Generic + native AFIT so we
/// stay free of the `async-trait` macro and let the live + mock impls
/// monomorphize.
///
/// Returns `Ok(None)` when Stripe reports the subscription is missing —
/// treat as a soft no-op so the pass continues. `Err` is reserved for
/// transport/auth failures that should be logged and skipped per-row.
pub trait StripeSubscriptionFetcher: Send + Sync {
    fn retrieve(
        &self,
        sub_id: &str,
    ) -> impl std::future::Future<Output = Result<Option<RemoteSubscription>, String>> + Send;
}

/// Slim projection of a Stripe subscription containing only the fields that
/// participate in drift detection. Keeping the surface narrow means the
/// real client and the test mock both stay simple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSubscription {
    pub status: String,
    pub current_period_end: NaiveDateTime,
    pub cancel_at_period_end: bool,
    pub trial_end: Option<NaiveDateTime>,
}

/// Real-Stripe-backed fetcher. Lives outside the trait so the test path
/// doesn't pay for the async-stripe deserialise overhead.
pub struct LiveStripeFetcher {
    client: stripe::Client,
}

impl LiveStripeFetcher {
    #[must_use]
    pub const fn new(client: stripe::Client) -> Self {
        Self { client }
    }
}

impl StripeSubscriptionFetcher for LiveStripeFetcher {
    async fn retrieve(&self, sub_id: &str) -> Result<Option<RemoteSubscription>, String> {
        use stripe_billing::subscription::RetrieveSubscription;

        let id: stripe_shared::SubscriptionId = match sub_id.parse() {
            Ok(id) => id,
            Err(e) => return Err(format!("invalid stripe subscription id {sub_id}: {e}")),
        };

        let sub = match RetrieveSubscription::new(id).send(&self.client).await {
            Ok(s) => s,
            Err(e) => {
                // async-stripe surfaces 404 as a typed error; we don't have
                // a clean "not found" branch off the public API, so just
                // log + skip. The next pass will retry.
                let msg = format!("{e}");
                if msg.contains("No such subscription") || msg.contains("resource_missing") {
                    return Ok(None);
                }
                return Err(msg);
            }
        };

        let item = sub
            .items
            .data
            .first()
            .ok_or_else(|| "subscription has no items".to_string())?;

        let current_period_end = naive_from_ts(item.current_period_end)
            .ok_or_else(|| "missing current_period_end on item".to_string())?;
        let trial_end = sub.trial_end.and_then(naive_from_ts);

        Ok(Some(RemoteSubscription {
            status: sub.status.as_str().to_string(),
            current_period_end,
            cancel_at_period_end: sub.cancel_at_period_end,
            trial_end,
        }))
    }
}

fn naive_from_ts(ts: stripe_types::Timestamp) -> Option<NaiveDateTime> {
    chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.naive_utc())
}

/// Run a single reconcile pass. Logs + skips per-row errors so one bad sub
/// doesn't poison the whole pass.
///
/// Returns the number of rows that drifted (were corrected) — used by the
/// CLI to print a summary and by tests to assert convergence.
///
/// When `sharing` is `Some`, a Paid→Free correction additionally suspends
/// all editors and pending invitations for the owner's calendars and
/// publishes `MemberLeft` + `kick` events via Redis Pub/Sub (best-effort;
/// failures are logged but don't count as errors).
///
/// # Errors
/// Returns an error only on a top-level DB failure (couldn't list rows).
/// Per-row errors are logged and counted in
/// `entitlement_reconcile_errors_total` but not bubbled.
pub async fn run_pass<F: StripeSubscriptionFetcher>(
    mapper: &SubscriptionMapper,
    fetcher: &F,
    sharing: Option<&SharingDeps>,
) -> ServerResult<u64> {
    let rows = mapper.list_for_reconcile().await?;
    let total = rows.len();
    let mut corrected: u64 = 0;

    for row in rows {
        match reconcile_one(mapper, fetcher, &row, sharing).await {
            Ok(true) => corrected += 1,
            Ok(false) => {}
            Err(e) => {
                error!(
                    "reconcile: row id={} sub={} failed: {e}",
                    row.id, row.stripe_subscription_id
                );
                counter!(names::ENTITLEMENT_RECONCILE_ERRORS_TOTAL).increment(1);
            }
        }
    }

    info!("reconcile: pass complete, scanned={total} corrected={corrected}");
    Ok(corrected)
}

async fn reconcile_one<F: StripeSubscriptionFetcher>(
    mapper: &SubscriptionMapper,
    fetcher: &F,
    row: &ReconcileRow,
    sharing: Option<&SharingDeps>,
) -> Result<bool, String> {
    let remote = match fetcher.retrieve(&row.stripe_subscription_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            warn!(
                "reconcile: stripe reports {} missing — skipping",
                row.stripe_subscription_id
            );
            return Ok(false);
        }
        Err(e) => return Err(format!("stripe retrieve failed: {e}")),
    };

    let drifted_fields = collect_drift(row, &remote);
    if drifted_fields.is_empty() {
        return Ok(false);
    }

    // Detect potential tier transitions before applying the update, so we can
    // decide whether sharing suspension or restore is needed afterward.
    let was_paid_status = matches!(row.status.as_str(), "active" | "trialing");
    let becomes_free_status = remote.status == "canceled";
    let was_free_status = matches!(row.status.as_str(), "canceled" | "past_due" | "unpaid");
    let becomes_paid_status = matches!(remote.status.as_str(), "active" | "trialing");

    // Conditional UPDATE guarded on local current_period_end <= stripe's.
    // rows_affected = 0 means a webhook landed mid-pass and moved us past
    // Stripe's period_end; that's the safe outcome — we don't regress it.
    let rows_affected = mapper
        .apply_reconcile_update(
            row.id,
            &remote.status,
            remote.current_period_end,
            remote.cancel_at_period_end,
            remote.trial_end,
        )
        .await
        .map_err(|e| format!("apply_reconcile_update failed: {e}"))?;

    if rows_affected == 0 {
        // Race: a fresher webhook write beat us to it. Not an error.
        return Ok(false);
    }

    for field in &drifted_fields {
        counter!(names::ENTITLEMENT_RECONCILE_DRIFT_TOTAL, names::LABEL_FIELD => *field)
            .increment(1);
        warn!(
            "reconcile: drift corrected on sub {} field={field}",
            row.stripe_subscription_id
        );
    }

    // After a confirmed Paid→Free correction, suspend sharing for all
    // calendars owned by this user. Best-effort: log failures but don't
    // propagate them as reconcile errors — the subscription data is already
    // corrected, which is the primary goal.
    if was_paid_status && becomes_free_status
        && let Some(deps) = sharing
    {
        apply_sharing_side_effects(row.user_id, deps).await;
    }

    // After a confirmed Free→Paid correction, restore sharing for all
    // calendars owned by this user and send restore emails. Best-effort.
    if was_free_status && becomes_paid_status
        && let Some(deps) = sharing
    {
        apply_restore_sharing_side_effects(row.user_id, deps).await;
    }

    Ok(true)
}

/// Suspend all editors + invitations for `owner_id`'s calendars and publish
/// `MemberLeft` + `kick` events. Called after a confirmed Paid→Free reconcile
/// correction. All errors are logged and swallowed — the subscription data
/// is already correct; Redis/DB failures here are recoverable on next page load.
async fn apply_sharing_side_effects(owner_id: i32, deps: &SharingDeps) {
    use crate::services::sharing::suspend_owner_sharing_in_tx;

    // Open a short transaction for the suspend writes.
    let mut tx = match deps.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("reconcile: failed to begin sharing suspend tx for user {owner_id}: {e}");
            return;
        }
    };

    let kicks = match suspend_owner_sharing_in_tx(&mut tx, owner_id).await {
        Ok(k) => k,
        Err(e) => {
            error!("reconcile: suspend_owner_sharing_in_tx failed for user {owner_id}: {e:?}");
            let _ = tx.rollback().await;
            return;
        }
    };

    if let Err(e) = tx.commit().await {
        error!("reconcile: sharing suspend commit failed for user {owner_id}: {e}");
        return;
    }

    // Publish MemberLeft + kick for each suspended editor (best-effort).
    for kick in kicks {
        let _ = deps
            .publisher
            .publish_calendar(
                kick.calendar_id,
                &CalendarEvent::MemberLeft {
                    user_id: kick.user_id.to_string(),
                    actor: "system".to_string(),
                    reason: "downgrade".to_string(),
                },
            )
            .await
            .map_err(|e| {
                error!(
                    "reconcile: publish MemberLeft downgrade cal={} uid={}: {e}",
                    kick.calendar_id, kick.user_id
                );
            });
        let _ = deps
            .publisher
            .publish_kick(kick.user_id, "owner_downgrade")
            .await
            .map_err(|e| {
                error!(
                    "reconcile: publish kick downgrade uid={}: {e}",
                    kick.user_id
                );
            });
    }
}

/// Restore all editors + invitations for `owner_id`'s calendars and send
/// "access restored" emails post-commit. Called after a confirmed Free→Paid
/// reconcile correction. All errors are logged and swallowed — the
/// subscription data is already correct; SMTP failures here are recoverable.
async fn apply_restore_sharing_side_effects(owner_id: i32, deps: &SharingDeps) {
    use crate::services::sharing::restore_owner_sharing_in_tx;

    // Open a short transaction for the restore writes.
    let mut tx = match deps.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("reconcile: failed to begin sharing restore tx for user {owner_id}: {e}");
            return;
        }
    };

    let restored = match restore_owner_sharing_in_tx(&mut tx, owner_id).await {
        Ok(r) => r,
        Err(e) => {
            error!("reconcile: restore_owner_sharing_in_tx failed for user {owner_id}: {e:?}");
            let _ = tx.rollback().await;
            return;
        }
    };

    if let Err(e) = tx.commit().await {
        error!("reconcile: sharing restore commit failed for user {owner_id}: {e}");
        return;
    }

    // Send "access restored" emails for each restored editor (best-effort).
    for r in restored {
        match deps.user_mapper.get_user_by_id(r.user_id).await {
            Ok(user) => {
                deps.email_service
                    .send_editor_restored(&user.email, &r.calendar_name)
                    .await
                    .ok();
            }
            Err(e) => {
                error!(
                    "reconcile: get_user_by_id({}) for restore email failed: {e:?}",
                    r.user_id
                );
            }
        }
    }
}

fn collect_drift(local: &ReconcileRow, remote: &RemoteSubscription) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if local.status != remote.status {
        fields.push("status");
    }
    if local.current_period_end != remote.current_period_end {
        fields.push("period");
    }
    if local.cancel_at_period_end != remote.cancel_at_period_end {
        fields.push("cancel_flag");
    }
    if local.trial_end != remote.trial_end {
        fields.push("trial");
    }
    fields
}

/// One-shot reconcile for a single user.
///
/// Used by the `set_subscription` CLI's `--reconcile-from-stripe` mode.
/// Returns whether a drift correction was applied (`Ok(Some(true))`),
/// no drift was found (`Ok(Some(false))`), or the user has no eligible
/// subscription row to reconcile (`Ok(None)`).
/// # Errors
/// Returns a string describing a DB or Stripe transport failure.
pub async fn reconcile_for_user<F: StripeSubscriptionFetcher>(
    mapper: &SubscriptionMapper,
    fetcher: &F,
    user_id: i32,
) -> Result<Option<bool>, String> {
    let sub = mapper
        .find_active_for_user(user_id)
        .await
        .map_err(|e| format!("find_active_for_user failed: {e}"))?;
    let Some(sub) = sub else {
        return Ok(None);
    };
    let row = ReconcileRow {
        id: sub.id,
        user_id: sub.user_id,
        stripe_subscription_id: sub.stripe_subscription_id.clone(),
        status: sub.status,
        current_period_end: sub.current_period_end,
        cancel_at_period_end: sub.cancel_at_period_end,
        trial_end: sub.trial_end,
    };
    // reconcile_for_user is used by the CLI, which has no sharing deps.
    let corrected = reconcile_one(mapper, fetcher, &row, None).await?;
    Ok(Some(corrected))
}

/// Spawn the reconcile loop on the current Tokio runtime. `interval_secs = 0`
/// returns without spawning, so dev / test deployments can opt out.
///
/// Pass `sharing = Some(deps)` to enable sharing suspension on Paid→Free
/// corrections. Pass `None` if the sharing services are not available
/// (e.g. deployment without Redis).
pub fn spawn_loop<F>(
    mapper: SubscriptionMapper,
    fetcher: F,
    interval_secs: u64,
    sharing: Option<SharingDeps>,
)
where
    F: StripeSubscriptionFetcher + 'static,
{
    if interval_secs == 0 {
        info!("reconcile: interval_secs=0, loop disabled");
        return;
    }
    info!("reconcile: spawning loop at {interval_secs}s interval");

    actix_web::rt::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        // Skip the immediate first tick — the server has things to do at
        // startup. Default `Burst` behavior would fire one right away.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            if let Err(e) = run_pass(&mapper, &fetcher, sharing.as_ref()).await {
                error!("reconcile: pass failed at top level: {e}");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as ChronoDuration, Utc};
    use std::collections::HashMap;
    use std::sync::Mutex;

    // Tests share a real DATABASE_URL via `test_pool()`. To stay isolated
    // from other tests' rows that `list_for_reconcile()` would also pick up,
    // we use random sub_ids and assert on per-row outcomes (not global
    // `corrected` counts, which would race with parallel tests).
    fn unique_sub_id(prefix: &str) -> String {
        let n: u64 = rand::random();
        format!("sub_recon_{prefix}_{n}")
    }

    /// Postgres TIMESTAMP stores microseconds; chrono's `now()` gives
    /// nanoseconds. Round-trip through the DB drops the trailing nanos, so
    /// any post-write equality check must truncate the original value to
    /// microseconds first.
    fn micros(dt: chrono::NaiveDateTime) -> chrono::NaiveDateTime {
        use chrono::Timelike;
        let n = dt.nanosecond();
        dt.with_nanosecond((n / 1000) * 1000).unwrap()
    }

    /// In-memory fixture. Stores Stripe-side truth keyed by subscription id;
    /// `retrieve` looks up + clones. `Mutex<HashMap>` is enough — tests are
    /// single-threaded per case.
    struct MockFetcher {
        store: Mutex<HashMap<String, Option<RemoteSubscription>>>,
    }

    impl MockFetcher {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }
        fn set(&self, id: &str, remote: RemoteSubscription) {
            self.store
                .lock()
                .unwrap()
                .insert(id.to_string(), Some(remote));
        }
        fn set_missing(&self, id: &str) {
            self.store.lock().unwrap().insert(id.to_string(), None);
        }
    }

    impl StripeSubscriptionFetcher for MockFetcher {
        async fn retrieve(&self, sub_id: &str) -> Result<Option<RemoteSubscription>, String> {
            Ok(self
                .store
                .lock()
                .unwrap()
                .get(sub_id)
                .cloned()
                .unwrap_or(None))
        }
    }

    async fn seed_user(pool: &sqlx::PgPool) -> i32 {
        let n: u64 = rand::random();
        sqlx::query_scalar::<_, i32>(
            "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, 'hash') RETURNING id",
        )
        .bind(format!("recon_{n}"))
        .bind(format!("recon_{n}@test.com"))
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn seed_subscription(
        pool: &sqlx::PgPool,
        user_id: i32,
        sub_id: &str,
        status: &str,
        period_end: NaiveDateTime,
        cancel_at_period_end: bool,
        trial_end: Option<NaiveDateTime>,
    ) {
        let n: u64 = rand::random();
        let period_start = (Utc::now() - ChronoDuration::days(30)).naive_utc();
        sqlx::query!(
            "INSERT INTO subscriptions \
             (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
              stripe_price_id, current_period_start, current_period_end, \
              cancel_at_period_end, trial_end) \
             VALUES ($1, 'paid', $2, $3, $4, 'price_test', $5, $6, $7, $8)",
            user_id,
            status,
            format!("cus_{n}"),
            sub_id,
            period_start,
            period_end,
            cancel_at_period_end,
            trial_end,
        )
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn pass_corrects_status_drift() {
        let pool = crate::test_helpers::test_pool().await;
        let mapper = SubscriptionMapper::from_pool(pool.clone());
        let user_id = seed_user(&pool).await;
        let sub_id = unique_sub_id("status");
        let period_end = (Utc::now() + ChronoDuration::days(14)).naive_utc();
        seed_subscription(&pool, user_id, &sub_id, "active", period_end, false, None).await;

        let fetcher = MockFetcher::new();
        fetcher.set(
            &sub_id,
            RemoteSubscription {
                status: "past_due".to_string(),
                current_period_end: period_end,
                cancel_at_period_end: false,
                trial_end: None,
            },
        );

        run_pass(&mapper, &fetcher, None).await.unwrap();

        let row = sqlx::query!(
            "SELECT status FROM subscriptions WHERE stripe_subscription_id = $1",
            sub_id,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.status, "past_due");
    }

    #[tokio::test]
    async fn pass_corrects_period_end_drift() {
        let pool = crate::test_helpers::test_pool().await;
        let mapper = SubscriptionMapper::from_pool(pool.clone());
        let user_id = seed_user(&pool).await;
        let sub_id = unique_sub_id("period");
        let local_end = micros((Utc::now() + ChronoDuration::days(7)).naive_utc());
        let stripe_end = micros((Utc::now() + ChronoDuration::days(37)).naive_utc());
        seed_subscription(&pool, user_id, &sub_id, "active", local_end, false, None).await;

        let fetcher = MockFetcher::new();
        fetcher.set(
            &sub_id,
            RemoteSubscription {
                status: "active".to_string(),
                current_period_end: stripe_end,
                cancel_at_period_end: false,
                trial_end: None,
            },
        );

        run_pass(&mapper, &fetcher, None).await.unwrap();

        let row = sqlx::query!(
            "SELECT current_period_end FROM subscriptions WHERE stripe_subscription_id = $1",
            sub_id,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.current_period_end, stripe_end);
    }

    #[tokio::test]
    async fn pass_skips_when_local_period_end_is_newer() {
        // Webhook-landed-mid-pass scenario: local has the newer period_end.
        // Reconcile must NOT regress it back to Stripe's stale value.
        let pool = crate::test_helpers::test_pool().await;
        let mapper = SubscriptionMapper::from_pool(pool.clone());
        let user_id = seed_user(&pool).await;
        let sub_id = unique_sub_id("race");
        let local_end = micros((Utc::now() + ChronoDuration::days(37)).naive_utc());
        let stripe_end = micros((Utc::now() + ChronoDuration::days(7)).naive_utc());
        seed_subscription(&pool, user_id, &sub_id, "active", local_end, false, None).await;

        let fetcher = MockFetcher::new();
        fetcher.set(
            &sub_id,
            RemoteSubscription {
                status: "active".to_string(),
                current_period_end: stripe_end,
                cancel_at_period_end: false,
                trial_end: None,
            },
        );

        run_pass(&mapper, &fetcher, None).await.unwrap();

        let row = sqlx::query!(
            "SELECT current_period_end FROM subscriptions WHERE stripe_subscription_id = $1",
            sub_id,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.current_period_end, local_end, "local must not regress");
    }

    #[tokio::test]
    async fn apply_reconcile_update_no_drift_when_period_unchanged() {
        // No-drift case is hard to assert on a shared pool because the
        // global pass touches every row. Test the mapper helper directly
        // instead — calling with values that match the local row should be
        // a no-op now (the H-7 guard tightened from `<=` to a strict-newer
        // OR equal-period-different-status check, so equal-everything is
        // skipped rather than re-touching `updated_at`).
        let pool = crate::test_helpers::test_pool().await;
        let mapper = SubscriptionMapper::from_pool(pool.clone());
        let user_id = seed_user(&pool).await;
        let sub_id = unique_sub_id("clean");
        let period_end = (Utc::now() + ChronoDuration::days(14)).naive_utc();
        seed_subscription(&pool, user_id, &sub_id, "active", period_end, false, None).await;

        let row_id: i32 = sqlx::query_scalar!(
            "SELECT id FROM subscriptions WHERE stripe_subscription_id = $1",
            sub_id,
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let affected = mapper
            .apply_reconcile_update(row_id, "active", period_end, false, None)
            .await
            .unwrap();
        // Equal period + equal status => no-op under the H-7 guard.
        assert_eq!(affected, 0);
    }

    #[tokio::test]
    async fn pass_skips_canceled_rows() {
        // Canceled rows are filtered by list_for_reconcile, so even with a
        // mocked Stripe value the row stays untouched.
        let pool = crate::test_helpers::test_pool().await;
        let mapper = SubscriptionMapper::from_pool(pool.clone());
        let user_id = seed_user(&pool).await;
        let sub_id = unique_sub_id("dead");
        let period_end = (Utc::now() - ChronoDuration::days(2)).naive_utc();
        seed_subscription(&pool, user_id, &sub_id, "canceled", period_end, true, None).await;

        let fetcher = MockFetcher::new();
        fetcher.set(
            &sub_id,
            RemoteSubscription {
                status: "active".to_string(),
                current_period_end: (Utc::now() + ChronoDuration::days(30)).naive_utc(),
                cancel_at_period_end: false,
                trial_end: None,
            },
        );

        run_pass(&mapper, &fetcher, None).await.unwrap();

        let row = sqlx::query!(
            "SELECT status FROM subscriptions WHERE stripe_subscription_id = $1",
            sub_id,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.status, "canceled");
    }

    #[tokio::test]
    async fn pass_continues_on_per_row_error() {
        // Stripe says ghost is missing (soft no-op). real should still drift.
        let pool = crate::test_helpers::test_pool().await;
        let mapper = SubscriptionMapper::from_pool(pool.clone());
        let user_id = seed_user(&pool).await;
        let ghost_id = unique_sub_id("ghost");
        let real_id = unique_sub_id("real");
        let period_end = (Utc::now() + ChronoDuration::days(14)).naive_utc();
        seed_subscription(&pool, user_id, &ghost_id, "active", period_end, false, None).await;
        seed_subscription(&pool, user_id, &real_id, "active", period_end, false, None).await;

        let fetcher = MockFetcher::new();
        fetcher.set_missing(&ghost_id);
        fetcher.set(
            &real_id,
            RemoteSubscription {
                status: "past_due".to_string(),
                current_period_end: period_end,
                cancel_at_period_end: false,
                trial_end: None,
            },
        );

        run_pass(&mapper, &fetcher, None).await.unwrap();

        let real = sqlx::query!(
            "SELECT status FROM subscriptions WHERE stripe_subscription_id = $1",
            real_id,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(real.status, "past_due");

        let ghost = sqlx::query!(
            "SELECT status FROM subscriptions WHERE stripe_subscription_id = $1",
            ghost_id,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        // Ghost row was soft-skipped — its local status is untouched.
        assert_eq!(ghost.status, "active");
    }

    #[test]
    fn collect_drift_detects_each_field() {
        let now = chrono::NaiveDateTime::parse_from_str("2026-05-02 00:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap();
        let local = ReconcileRow {
            id: 1,
            user_id: 1,
            stripe_subscription_id: "sub_x".to_string(),
            status: "active".to_string(),
            current_period_end: now,
            cancel_at_period_end: false,
            trial_end: None,
        };

        let same = RemoteSubscription {
            status: "active".to_string(),
            current_period_end: now,
            cancel_at_period_end: false,
            trial_end: None,
        };
        assert!(collect_drift(&local, &same).is_empty());

        let status_drift = RemoteSubscription {
            status: "past_due".to_string(),
            ..same
        };
        assert_eq!(collect_drift(&local, &status_drift), vec!["status"]);

        let cancel_drift = RemoteSubscription {
            cancel_at_period_end: true,
            ..same.clone()
        };
        assert_eq!(collect_drift(&local, &cancel_drift), vec!["cancel_flag"]);

        let trial_drift = RemoteSubscription {
            trial_end: Some(now),
            ..same
        };
        assert_eq!(collect_drift(&local, &trial_drift), vec!["trial"]);
    }
}
