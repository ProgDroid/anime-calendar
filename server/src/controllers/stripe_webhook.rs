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
    mappers::{stripe_event::StripeEventMapper, subscription::SubscriptionMapper},
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
        Ok(()) => {
            if let Err(e) = tx.commit().await {
                error!("stripe webhook: commit failed for {event_id}: {e}");
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error":"db error"}));
            }
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

/// Route a verified Stripe event object to the right writer. Returns `Ok(())`
/// for events we deliberately ignore — the idempotency insert is enough to
/// stop Stripe retries.
async fn dispatch_event(tx: &mut sqlx::PgConnection, object: EventObject) -> Result<(), String> {
    match object {
        // checkout.session.completed: Stripe also fires
        // customer.subscription.created with the full subscription object and
        // our metadata, so we don't write here. The event id is already
        // persisted by record_first_time_in_tx, so 200 is correct.
        EventObject::CustomerSubscriptionCreated(sub)
        | EventObject::CustomerSubscriptionUpdated(sub) => upsert_subscription(tx, &sub).await,

        EventObject::CustomerSubscriptionDeleted(sub) => {
            let sub_id = sub.id.as_str();
            let updated =
                SubscriptionMapper::update_status_by_subscription_id_in_tx(tx, sub_id, "canceled")
                    .await
                    .map_err(|e| format!("update_status_by_subscription_id failed: {e}"))?;
            if updated == 0 {
                warn!(
                    "stripe webhook: subscription.deleted for unknown stripe_subscription_id {sub_id}"
                );
            }
            Ok(())
        }

        EventObject::InvoicePaymentSucceeded(invoice) => {
            handle_invoice_succeeded(tx, &invoice).await
        }

        EventObject::InvoicePaymentFailed(invoice) => handle_invoice_failed(tx, &invoice).await,

        // Any other event: idempotency-only path. Nothing to write.
        EventObject::CheckoutSessionCompleted(_) | _ => Ok(()),
    }
}

async fn upsert_subscription(
    tx: &mut sqlx::PgConnection,
    sub: &stripe_shared::Subscription,
) -> Result<(), String> {
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
        SubscriptionMapper::find_user_id_by_customer_in_tx(tx, &stripe_customer_id)
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
        return Ok(());
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

    SubscriptionMapper::upsert_from_stripe_in_tx(
        tx,
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
    .map_err(|e| format!("upsert_from_stripe_in_tx failed: {e}"))
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
