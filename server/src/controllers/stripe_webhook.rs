//! Stripe webhook receiver: signature verification + idempotent event routing.
//!
//! This is the canonical writer for the local `subscriptions` table from
//! Phase 3 onwards. The Phase-2 `?session_id=` stopgap in
//! `controllers/subscription.rs` will be removed once we've observed the
//! webhook running cleanly in production.
//!
//! ## Design
//!
//! - **Public** (no JWT): Stripe signs the request; we verify via HMAC.
//! - **Rate limit**: whitelisted from `actix-governor` (Stripe sends bursts on
//!   retries — we'd otherwise rate-limit ourselves into delivery failures).
//! - **Idempotency**: the `stripe_events` table holds every processed
//!   `event.id`. The handler `INSERT ... ON CONFLICT DO NOTHING RETURNING`s,
//!   and a missing returned row means we've already processed this event so
//!   we 200 immediately. This protects against Stripe's at-least-once delivery
//!   and against client-driven replay from the dashboard.
//! - **Atomicity**: the idempotency insert and the business write happen in
//!   the same sqlx transaction. If the business write panics or returns an
//!   error, the transaction rolls back and the event id is *not* persisted —
//!   so a Stripe retry will reprocess the event.
//!
//! ## User mapping
//!
//! Subscription metadata `{ "user_id": "<id>" }` is stamped during Checkout
//! creation (see `controllers/stripe.rs`). Every `customer.subscription.*`
//! event payload carries this metadata back, so we can resolve `user_id`
//! without an extra Stripe API call. For invoice events (which carry only the
//! subscription id, not the metadata), we look up the user via the existing
//! `(stripe_customer_id, user_id)` pair stored on a prior subscription row.
//!
//! ## What we deliberately don't do
//!
//! - We don't process `checkout.session.completed`. The corresponding
//!   `customer.subscription.created` event arrives with the full subscription
//!   object and metadata, which is everything we need. Recording the event id
//!   is enough to stop Stripe retries.
//! - We don't fetch any extra data via the Stripe API. Every field we need is
//!   already in the webhook payload.

use actix_web::{HttpRequest, HttpResponse, post, web};
use chrono::DateTime;
use log::{error, warn};
use stripe_types::Timestamp;
// Aliased: the external crate `stripe_webhook` collides with this module's
// own name (`controllers::stripe_webhook`).
use ::stripe_webhook::{EventObject, Webhook};

use crate::{
    config::server::StripeConfig,
    entity::subscription::Tier,
    mappers::{stripe_event::StripeEventMapper, subscription::SubscriptionMapper},
    services::{entitlement::EntitlementService, frozen_ics::FrozenIcsService},
};
use secrecy::ExposeSecret as _;
// SubscriptionMapper is used via its static `_in_tx` helpers in
// `dispatch_event` — the handler itself only needs the pool from
// StripeEventMapper to open the outer transaction.

/// `POST /api/stripe/webhook` — Stripe-hosted webhook receiver.
///
/// Returns 200 on:
/// - successfully processed events
/// - replays of already-seen events (idempotency short-circuit)
/// - events we don't subscribe to (recorded + ignored so Stripe stops retrying)
///
/// Returns 400 on signature verification failure (Stripe retries 4xx as well,
/// but a sustained 4xx eventually disables the endpoint — which is the
/// correct outcome if the webhook secret is wrong).
///
/// Returns 500 on database errors so Stripe will retry.
#[post("/stripe/webhook")]
#[allow(clippy::future_not_send)]
pub async fn stripe_webhook(
    req: HttpRequest,
    body: web::Bytes,
    event_mapper: web::Data<StripeEventMapper>,
    stripe_config: web::Data<StripeConfig>,
    frozen_ics: web::Data<FrozenIcsService>,
) -> HttpResponse {
    if !stripe_config.is_configured() {
        // No Stripe credentials in this deployment — nothing should be
        // hitting this endpoint. Return 400 so the sender retries fewer times
        // than a 500 would, and log loudly.
        warn!("stripe webhook hit but Stripe is not configured on this deployment");
        return HttpResponse::BadRequest().json(serde_json::json!({"error":"not configured"}));
    }

    let Some(sig_header) = req
        .headers()
        .get("Stripe-Signature")
        .and_then(|h| h.to_str().ok())
    else {
        warn!("stripe webhook missing Stripe-Signature header");
        return HttpResponse::BadRequest().json(serde_json::json!({"error":"missing signature"}));
    };

    let Ok(payload) = std::str::from_utf8(&body) else {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error":"invalid payload encoding"}));
    };

    let secret = stripe_config.webhook_secret.expose_secret();
    if secret.is_empty() {
        warn!("stripe webhook hit but webhook_secret is empty — refusing to process");
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error":"webhook secret not configured"}));
    }

    let event = match Webhook::construct_event(payload, sig_header, secret) {
        Ok(e) => e,
        Err(e) => {
            warn!("stripe webhook signature verification failed: {e}");
            return HttpResponse::BadRequest()
                .json(serde_json::json!({"error":"invalid signature"}));
        }
    };

    let event_id = event.id.to_string();
    let event_type = event.type_.as_str();

    // Open the outer transaction. The idempotency insert + every downstream
    // write live inside this scope so a business-logic failure rolls them
    // back together and a Stripe retry will reprocess.
    let pool = event_mapper.pool();
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("stripe webhook: failed to begin transaction: {e}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error":"db error"}));
        }
    };

    let first_time =
        match StripeEventMapper::record_first_time_in_tx(&mut tx, &event_id, event_type).await {
            Ok(b) => b,
            Err(e) => {
                error!("stripe webhook: idempotency insert failed for {event_id}: {e}");
                let _ = tx.rollback().await;
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error":"db error"}));
            }
        };
    if !first_time {
        // Replay — commit so the no-op insert query result doesn't leak a
        // dangling tx, then 200.
        let _ = tx.commit().await;
        return HttpResponse::Ok().json(serde_json::json!({"received":true,"replay":true}));
    }

    let result = dispatch_event(&mut tx, event.data.object).await;

    match result {
        Ok(action) => {
            if let Err(e) = tx.commit().await {
                error!("stripe webhook: commit failed for {event_id}: {e}");
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error":"db error"}));
            }
            // Post-commit: dispatch the frozen-blob lifecycle action that the
            // tier transition (if any) implied. Never run before commit — if
            // the outer txn rolls back we must not act on a phantom transition.
            // FrozenIcsService uses its own pool, so its writes are independent
            // of `tx`. We deliberately swallow errors here: the webhook itself
            // succeeded, and a failed regenerate is recoverable via the lazy
            // safety net on `GET /api/calendars/subscribe/:token`.
            dispatch_frozen_action(&frozen_ics, action).await;
            HttpResponse::Ok().json(serde_json::json!({"received":true}))
        }
        Err(e) => {
            error!("stripe webhook: handler failed for {event_id} ({event_type}): {e}");
            let _ = tx.rollback().await;
            // 500 → Stripe retries. The event id was rolled back with the
            // rest of the work so the next delivery re-enters the handler.
            HttpResponse::InternalServerError().json(serde_json::json!({"error":"handler failed"}))
        }
    }
}

/// What `FrozenIcsService` action (if any) should fire after the outer
/// webhook transaction commits. Computed inside `dispatch_event` by comparing
/// the user's effective tier before vs. after the subscription mapper write.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum FrozenAction {
    /// No tier transition; nothing to do.
    None,
    /// Free → Paid: null the user's frozen blobs (Pro serves live data).
    Clear(i32),
    /// Paid → Free: regenerate frozen blobs from current state.
    Regenerate(i32),
}

/// Pure mapping from (old, new) tier to the implied frozen-blob action.
/// Same-tier and any non-transition combination → `None`.
const fn transition_action(user_id: i32, old: Tier, new: Tier) -> FrozenAction {
    match (old, new) {
        (Tier::Free, Tier::Paid) => FrozenAction::Clear(user_id),
        (Tier::Paid, Tier::Free) => FrozenAction::Regenerate(user_id),
        _ => FrozenAction::None,
    }
}

/// Apply a `FrozenAction` against `FrozenIcsService`. Errors are logged but
/// never propagated — the caller has already committed the outer transaction
/// and returned 200 to Stripe.
async fn dispatch_frozen_action(frozen_ics: &FrozenIcsService, action: FrozenAction) {
    match action {
        FrozenAction::None => {}
        FrozenAction::Clear(uid) => {
            if let Err(e) = frozen_ics.clear_for_user(uid).await {
                error!("stripe webhook: clear_for_user({uid}) failed after commit: {e:?}");
            }
        }
        FrozenAction::Regenerate(uid) => match frozen_ics.regenerate_for_user(uid).await {
            Ok(0) => {}
            Ok(n) => warn!(
                "stripe webhook: regenerate_for_user({uid}): {n} calendar(s) failed to render"
            ),
            Err(e) => error!("stripe webhook: regenerate_for_user({uid}) errored: {e:?}"),
        },
    }
}

/// Route a verified Stripe event object to the right writer. Returns the
/// `FrozenAction` (if any) implied by a tier transition the event caused, so
/// the outer handler can fire the corresponding `FrozenIcsService` call
/// **after** the transaction commits.
///
/// Events we deliberately ignore return `Ok(FrozenAction::None)` — the
/// idempotency insert is enough to stop Stripe retries. Invoice events do
/// not contribute to tier-transition detection (out of scope for Phase 1.4
/// — the lazy-regen safety net on the subscribe endpoint covers any holes).
async fn dispatch_event(
    tx: &mut sqlx::PgConnection,
    object: EventObject,
) -> Result<FrozenAction, String> {
    match object {
        // checkout.session.completed: Stripe also fires
        // customer.subscription.created with the full subscription object and
        // our metadata, so we don't write here. The event id is already
        // persisted by record_first_time_in_tx, so 200 is correct.
        EventObject::CustomerSubscriptionCreated(sub)
        | EventObject::CustomerSubscriptionUpdated(sub) => upsert_subscription(tx, &sub).await,

        EventObject::CustomerSubscriptionDeleted(sub) => {
            handle_subscription_deleted(tx, &sub).await
        }

        EventObject::InvoicePaymentSucceeded(invoice) => {
            handle_invoice_succeeded(tx, &invoice).await?;
            Ok(FrozenAction::None)
        }

        EventObject::InvoicePaymentFailed(invoice) => {
            handle_invoice_failed(tx, &invoice).await?;
            Ok(FrozenAction::None)
        }

        // Any other event: idempotency-only path. Nothing to write.
        EventObject::CheckoutSessionCompleted(_) | _ => Ok(FrozenAction::None),
    }
}

/// Resolve the `user_id` referenced by a subscription payload, then handle
/// a `customer.subscription.deleted` event by flipping its row to
/// `canceled`. Reads effective tier on the same `tx` before and after the
/// write, so the returned `FrozenAction` reflects whether downgrading this
/// subscription actually moved the user from Paid to Free (other rows in
/// other states could keep them Paid).
async fn handle_subscription_deleted(
    tx: &mut sqlx::PgConnection,
    sub: &stripe_shared::Subscription,
) -> Result<FrozenAction, String> {
    let sub_id = sub.id.as_str();
    let existing = SubscriptionMapper::find_by_stripe_id_with(&mut *tx, sub_id)
        .await
        .map_err(|e| format!("find_by_stripe_id_with failed: {e}"))?;
    let Some(row) = existing else {
        warn!("stripe webhook: subscription.deleted for unknown stripe_subscription_id {sub_id}");
        return Ok(FrozenAction::None);
    };
    let user_id = row.user_id;
    let old_tier = EntitlementService::effective_tier_in_tx(&mut *tx, user_id)
        .await
        .map_err(|e| format!("effective_tier_in_tx (old) failed: {e}"))?;
    let updated =
        SubscriptionMapper::update_status_by_subscription_id_in_tx(&mut *tx, sub_id, "canceled")
            .await
            .map_err(|e| format!("update_status_by_subscription_id failed: {e}"))?;
    if updated == 0 {
        // Defensive: we just read the row above on this same tx, so it
        // exists. Log loudly if Stripe ever sends a delete for a row we
        // can't update (e.g. concurrent admin tooling).
        warn!(
            "stripe webhook: subscription.deleted update affected 0 rows for {sub_id} \
             despite earlier find succeeding"
        );
        return Ok(FrozenAction::None);
    }
    let new_tier = EntitlementService::effective_tier_in_tx(&mut *tx, user_id)
        .await
        .map_err(|e| format!("effective_tier_in_tx (new) failed: {e}"))?;
    Ok(transition_action(user_id, old_tier, new_tier))
}

async fn upsert_subscription(
    tx: &mut sqlx::PgConnection,
    sub: &stripe_shared::Subscription,
) -> Result<FrozenAction, String> {
    let sub_id = sub.id.as_str();
    let stripe_customer_id = match &sub.customer {
        stripe_types::Expandable::Object(c) => c.id.to_string(),
        stripe_types::Expandable::Id(id) => id.to_string(),
    };

    // Resolve user_id: prefer subscription.metadata.user_id (stamped during
    // Checkout creation). Fall back to a customer→user lookup against rows
    // we've already written, which covers Stripe-dashboard-created subs and
    // any legacy data.
    let user_id = if let Some(uid) = sub
        .metadata
        .get("user_id")
        .and_then(|v| v.parse::<i32>().ok())
    {
        uid
    } else if let Some(uid) =
        SubscriptionMapper::find_user_id_by_customer_in_tx(&mut *tx, &stripe_customer_id)
            .await
            .map_err(|e| format!("find_user_id_by_customer failed: {e}"))?
    {
        uid
    } else {
        // Out-of-band Stripe activity for a customer we've never seen.
        // Return Ok so Stripe stops retrying — the idempotency row is
        // already in place and there's nothing actionable here. Loudly log
        // because this means a real subscription exists in Stripe with no
        // local user mapping; an operator needs to backfill manually.
        warn!(
            "stripe webhook: subscription event for {sub_id} has no resolvable user_id \
             (metadata empty, no prior customer row for {stripe_customer_id})"
        );
        return Ok(FrozenAction::None);
    };

    let item = sub
        .items
        .data
        .first()
        .ok_or_else(|| "subscription has no items".to_string())?;
    let price_id = item.price.id.to_string();

    let period_start = naive_from_timestamp(item.current_period_start)
        .ok_or_else(|| "missing current_period_start on item".to_string())?;
    let period_end = naive_from_timestamp(item.current_period_end)
        .ok_or_else(|| "missing current_period_end on item".to_string())?;
    let trial_end = sub.trial_end.and_then(naive_from_timestamp);

    // Read effective tier *before* the upsert so we can detect a transition
    // after the write below. The read is on the same `tx`, so it observes
    // any earlier writes in this webhook delivery.
    let old_tier = EntitlementService::effective_tier_in_tx(&mut *tx, user_id)
        .await
        .map_err(|e| format!("effective_tier_in_tx (old) failed: {e}"))?;

    SubscriptionMapper::upsert_from_stripe_in_tx(
        &mut *tx,
        user_id,
        sub.status.as_str(),
        &stripe_customer_id,
        sub_id,
        &price_id,
        period_start,
        period_end,
        trial_end,
        sub.cancel_at_period_end,
    )
    .await
    .map_err(|e| format!("upsert_from_stripe_in_tx failed: {e}"))?;

    let new_tier = EntitlementService::effective_tier_in_tx(&mut *tx, user_id)
        .await
        .map_err(|e| format!("effective_tier_in_tx (new) failed: {e}"))?;

    Ok(transition_action(user_id, old_tier, new_tier))
}

async fn handle_invoice_succeeded(
    tx: &mut sqlx::PgConnection,
    invoice: &stripe_shared::Invoice,
) -> Result<(), String> {
    // The invoice's lines.data has the canonical period boundaries for the
    // billing cycle this invoice paid for. Sub events also carry period info,
    // but using the invoice line keeps this handler self-contained — Stripe
    // sometimes fires invoice.payment_succeeded *before* the corresponding
    // customer.subscription.updated, which would otherwise leave us with a
    // stale `current_period_end` for a few seconds.
    let Some(line) = invoice.lines.data.first() else {
        warn!("stripe webhook: invoice.payment_succeeded with no lines.data");
        return Ok(());
    };
    let Some(sub_field) = &line.subscription else {
        // One-off invoice (not a subscription renewal) — nothing to update.
        return Ok(());
    };
    let sub_id = match sub_field {
        stripe_types::Expandable::Object(s) => s.id.to_string(),
        stripe_types::Expandable::Id(id) => id.to_string(),
    };
    let Some(period_end) = naive_from_timestamp(line.period.end) else {
        warn!("stripe webhook: invoice line missing period.end for sub {sub_id}");
        return Ok(());
    };
    let updated =
        SubscriptionMapper::update_period_end_by_subscription_id_in_tx(tx, &sub_id, period_end)
            .await
            .map_err(|e| format!("update_period_end_by_subscription_id failed: {e}"))?;
    if updated == 0 {
        warn!(
            "stripe webhook: invoice.payment_succeeded for unknown stripe_subscription_id {sub_id}"
        );
    }
    Ok(())
}

async fn handle_invoice_failed(
    tx: &mut sqlx::PgConnection,
    invoice: &stripe_shared::Invoice,
) -> Result<(), String> {
    let Some(sub_field) = invoice
        .lines
        .data
        .first()
        .and_then(|l| l.subscription.as_ref())
    else {
        // Not a subscription invoice — nothing to flip.
        return Ok(());
    };
    let sub_id = match sub_field {
        stripe_types::Expandable::Object(s) => s.id.to_string(),
        stripe_types::Expandable::Id(id) => id.to_string(),
    };
    let updated =
        SubscriptionMapper::update_status_by_subscription_id_in_tx(tx, &sub_id, "past_due")
            .await
            .map_err(|e| format!("update_status_by_subscription_id (past_due) failed: {e}"))?;
    if updated == 0 {
        warn!("stripe webhook: invoice.payment_failed for unknown stripe_subscription_id {sub_id}");
    }
    Ok(())
}

fn naive_from_timestamp(ts: Timestamp) -> Option<chrono::NaiveDateTime> {
    DateTime::from_timestamp(ts, 0).map(|dt| dt.naive_utc())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::config::server::CacheConfig;
    use crate::mappers::anilist::Anilist;
    use crate::services::cached_anilist::CachedAnilist;
    use crate::services::ics_export::IcsExportService;
    use chrono::{Duration, Utc};
    use sqlx::PgPool;

    // ---- transition_action: pure unit tests --------------------------------

    #[test]
    fn transition_action_free_to_paid_clears() {
        assert_eq!(
            transition_action(42, Tier::Free, Tier::Paid),
            FrozenAction::Clear(42),
        );
    }

    #[test]
    fn transition_action_paid_to_free_regenerates() {
        assert_eq!(
            transition_action(7, Tier::Paid, Tier::Free),
            FrozenAction::Regenerate(7),
        );
    }

    #[test]
    fn transition_action_same_tier_is_noop() {
        assert_eq!(
            transition_action(1, Tier::Free, Tier::Free),
            FrozenAction::None,
        );
        assert_eq!(
            transition_action(1, Tier::Paid, Tier::Paid),
            FrozenAction::None,
        );
    }

    // ---- end-to-end pipeline tests -----------------------------------------
    //
    // These deliberately bypass `EventObject` reconstruction (constructing
    // `stripe_shared::Subscription` from scratch is impractical with the
    // async-stripe rc.5 type layout). Instead they exercise the same
    // primitives `dispatch_event` calls — `effective_tier_in_tx`,
    // `update_status_by_subscription_id_in_tx`, `FrozenIcsService` — and
    // assert the implied transition matches what `transition_action`
    // produces. EventObject parsing is upstream of the logic under test.

    async fn build_frozen_ics(pool: PgPool) -> FrozenIcsService {
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
        let ics_export = IcsExportService::new(
            pool.clone(),
            cached,
            user_settings_mapper,
            entitlement,
        );
        FrozenIcsService::new(pool, ics_export)
    }

    async fn seed_user(pool: &PgPool) -> i32 {
        let n: u64 = rand::random();
        sqlx::query_scalar(
            "INSERT INTO users (username, email, password_hash) \
             VALUES ($1, $2, 'hash') RETURNING id",
        )
        .bind(format!("webhook_{n}"))
        .bind(format!("webhook_{n}@test.com"))
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn seed_calendar(pool: &PgPool, user_id: i32) -> i32 {
        let n: u64 = rand::random();
        sqlx::query_scalar(
            "INSERT INTO calendars (name, language, user_id, subscription_token) \
             VALUES ('Cal', 'english'::language, $1, $2) RETURNING id",
        )
        .bind(user_id)
        .bind(format!("tok-{n}"))
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn seed_active_paid_subscription(pool: &PgPool, user_id: i32) -> String {
        let n: u64 = rand::random();
        let stripe_sub_id = format!("sub_test_{n}");
        let now = Utc::now().naive_utc();
        sqlx::query(
            "INSERT INTO subscriptions \
             (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
              stripe_price_id, current_period_start, current_period_end, \
              cancel_at_period_end) \
             VALUES ($1, 'paid', 'active', $2, $3, 'price_test', $4, $5, false)",
        )
        .bind(user_id)
        .bind(format!("cus_test_{n}"))
        .bind(&stripe_sub_id)
        .bind(now)
        .bind(now + Duration::days(30))
        .execute(pool)
        .await
        .unwrap();
        stripe_sub_id
    }

    async fn frozen_blob(pool: &PgPool, calendar_id: i32) -> Option<String> {
        sqlx::query_scalar("SELECT frozen_subscribe_ics FROM calendars WHERE id = $1")
            .bind(calendar_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn cleanup_user(pool: &PgPool, user_id: i32) {
        sqlx::query("DELETE FROM subscriptions WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
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

    /// Paid → Free downgrade via `update_status_by_subscription_id_in_tx`
    /// must produce a `Regenerate` action and `regenerate_for_user` must
    /// then leave the calendar's `frozen_subscribe_ics` non-NULL.
    #[tokio::test]
    async fn paid_to_free_transition_regenerates_blob() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user(&pool).await;
        let cal_id = seed_calendar(&pool, user_id).await;
        let stripe_sub_id = seed_active_paid_subscription(&pool, user_id).await;

        // Pro users serve live data — start with NULL frozen blob.
        assert!(frozen_blob(&pool, cal_id).await.is_none());

        // Read old tier (Paid), flip status to canceled, read new tier (Free).
        let mut tx = pool.begin().await.unwrap();
        let old_tier = EntitlementService::effective_tier_in_tx(&mut tx, user_id)
            .await
            .unwrap();
        assert_eq!(old_tier, Tier::Paid);

        let updated = SubscriptionMapper::update_status_by_subscription_id_in_tx(
            &mut tx,
            &stripe_sub_id,
            "canceled",
        )
        .await
        .unwrap();
        assert_eq!(updated, 1);

        let new_tier = EntitlementService::effective_tier_in_tx(&mut tx, user_id)
            .await
            .unwrap();
        assert_eq!(new_tier, Tier::Free);
        tx.commit().await.unwrap();

        let action = transition_action(user_id, old_tier, new_tier);
        assert_eq!(action, FrozenAction::Regenerate(user_id));

        // Now apply the action — same call the post-commit dispatcher would.
        let frozen = build_frozen_ics(pool.clone()).await;
        let failures = frozen.regenerate_for_user(user_id).await.unwrap();
        assert_eq!(failures, 0);
        assert!(
            frozen_blob(&pool, cal_id).await.is_some(),
            "frozen blob must be populated after regenerate_for_user",
        );

        cleanup_user(&pool, user_id).await;
    }

    /// Free → Paid upgrade: a user with pre-existing frozen blobs (i.e. they
    /// were previously a free subscriber) gets all blobs nulled by
    /// `clear_for_user` after the transition fires.
    #[tokio::test]
    async fn free_to_paid_transition_clears_blobs() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user(&pool).await;
        let cal_id = seed_calendar(&pool, user_id).await;

        // User starts on Free with a populated frozen blob (matches the
        // steady-state for Free subscribers — the subscribe endpoint or a
        // prior edit primed it).
        let frozen = build_frozen_ics(pool.clone()).await;
        frozen.regenerate(cal_id).await.unwrap();
        assert!(frozen_blob(&pool, cal_id).await.is_some());

        // Old tier is Free (no subscription rows at all).
        let mut tx = pool.begin().await.unwrap();
        let old_tier = EntitlementService::effective_tier_in_tx(&mut tx, user_id)
            .await
            .unwrap();
        assert_eq!(old_tier, Tier::Free);
        tx.commit().await.unwrap();

        // Insert a Paid sub row — equivalent to the upsert the webhook
        // performs on subscription.created.
        let _stripe_sub_id = seed_active_paid_subscription(&pool, user_id).await;

        let mut tx2 = pool.begin().await.unwrap();
        let new_tier = EntitlementService::effective_tier_in_tx(&mut tx2, user_id)
            .await
            .unwrap();
        assert_eq!(new_tier, Tier::Paid);
        tx2.commit().await.unwrap();

        let action = transition_action(user_id, old_tier, new_tier);
        assert_eq!(action, FrozenAction::Clear(user_id));

        frozen.clear_for_user(user_id).await.unwrap();
        assert!(
            frozen_blob(&pool, cal_id).await.is_none(),
            "blob must be NULL after clear_for_user — Pro serves live data",
        );

        cleanup_user(&pool, user_id).await;
    }
}
