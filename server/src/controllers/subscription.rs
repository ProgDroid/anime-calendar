//! Read-side endpoint for the current user's effective entitlement.
//!
//! Phase 2 includes a `?session_id=` stopgap that fetches a CheckoutSession
//! from Stripe and provisionally writes the local subscription row when the
//! webhook hasn't arrived yet. The frontend success page polls this endpoint
//! after returning from Stripe-hosted checkout. Phase 3 introduces the
//! webhook handler and removes the stopgap.

use std::str::FromStr;

use crate::{
    config::server::StripeConfig,
    entity::subscription::Entitlement,
    mappers::{subscription::SubscriptionMapper, user::UserMapper},
    middleware::auth::Claims,
    services::entitlement::EntitlementService,
};
use actix_web::{HttpResponse, ResponseError, get, web};
use chrono::DateTime;
use log::warn;
use serde::{Deserialize, Serialize};
use stripe::Client as StripeClient;
use stripe_checkout::checkout_session::RetrieveCheckoutSession;
use stripe_shared::CheckoutSessionId;
use stripe_types::{Expandable, Timestamp};

#[derive(Deserialize)]
pub struct SubscriptionMeQuery {
    /// Optional Stripe Checkout session id forwarded by the success page.
    /// Phase 2 only: lets us pre-populate the local subscription row before
    /// the webhook arrives. Phase 3 removes this query parameter handling.
    pub session_id: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SubscriptionResponse {
    pub tier: String,
    pub status: Option<String>,
    pub current_period_end: Option<chrono::NaiveDateTime>,
    pub cancel_at_period_end: bool,
    pub trial_end: Option<chrono::NaiveDateTime>,
}

impl From<Entitlement> for SubscriptionResponse {
    fn from(e: Entitlement) -> Self {
        Self {
            tier: e.tier.to_string(),
            status: e.status,
            current_period_end: e.current_period_end,
            cancel_at_period_end: e.cancel_at_period_end,
            trial_end: e.trial_end,
        }
    }
}

#[utoipa::path(
    get,
    path = "/subscription/me",
    tag = "subscription",
    params(
        ("session_id" = Option<String>, Query,
         description = "Phase-2 stopgap: forward the Stripe Checkout session id so the server \
                        can pre-populate the local subscription row before the webhook arrives."),
    ),
    responses(
        (status = 200, body = SubscriptionResponse),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[get("/subscription/me")]
#[allow(clippy::future_not_send)]
pub async fn get_my_subscription(
    user_mapper: web::Data<UserMapper>,
    sub_mapper: web::Data<SubscriptionMapper>,
    entitlement: web::Data<EntitlementService>,
    stripe_client: web::Data<StripeClient>,
    stripe_config: web::Data<StripeConfig>,
    claims: Claims,
    query: web::Query<SubscriptionMeQuery>,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(u) => u,
        Err(e) => return e.error_response(),
    };

    // Phase-2 stopgap: if the request includes a session id and we don't yet
    // have an entitling row, pull the session from Stripe and write the row
    // ourselves. Webhook-driven writes are still the canonical path.
    if let Some(session_id) = query.session_id.as_deref()
        && stripe_config.is_configured()
    {
        match sub_mapper.find_active_for_user(user.id).await {
            Ok(None) => {
                if let Err(e) =
                    backfill_from_session(&stripe_client, &sub_mapper, user.id, session_id).await
                {
                    // Non-fatal: the webhook will eventually deliver the
                    // same data. Log and continue with the read.
                    warn!(
                        "subscription/me stopgap backfill failed for user {}: {e}",
                        user.id
                    );
                }
            }
            Ok(Some(_)) => { /* already have a row; webhook beat us to it */ }
            Err(e) => return e.error_response(),
        }
    }

    match entitlement.entitlement(user.id).await {
        Ok(ent) => HttpResponse::Ok().json(SubscriptionResponse::from(ent)),
        Err(e) => e.error_response(),
    }
}

async fn backfill_from_session(
    client: &StripeClient,
    sub_mapper: &SubscriptionMapper,
    user_id: i32,
    session_id: &str,
) -> Result<(), String> {
    let id = CheckoutSessionId::from_str(session_id)
        .map_err(|e| format!("invalid session id: {e}"))?;

    let session = RetrieveCheckoutSession::new(id)
        .expand(["subscription".to_string(), "subscription.items".to_string()])
        .send(client)
        .await
        .map_err(|e| format!("retrieve session failed: {e}"))?;

    // The session was anchored on this user via client_reference_id; if it
    // doesn't match, refuse to write — defensive against forged session ids.
    let claimed = session
        .client_reference_id
        .as_deref()
        .and_then(|s| s.parse::<i32>().ok());
    if claimed != Some(user_id) {
        return Err(format!(
            "session client_reference_id {claimed:?} does not match caller {user_id}"
        ));
    }

    let Some(sub_field) = session.subscription else {
        return Err("session has no linked subscription".into());
    };
    let subscription = match sub_field {
        Expandable::Object(s) => *s,
        Expandable::Id(id) => return Err(format!("subscription not expanded: {id}")),
    };

    let stripe_customer_id = match session.customer {
        Some(Expandable::Object(c)) => c.id.to_string(),
        Some(Expandable::Id(id)) => id.to_string(),
        None => return Err("session has no customer".into()),
    };

    let item = subscription
        .items
        .data
        .first()
        .ok_or_else(|| "subscription has no items".to_string())?;
    let price_id = item.price.id.to_string();

    let period_start = naive_from_timestamp(item.current_period_start)
        .ok_or_else(|| "missing current_period_start on item".to_string())?;
    let period_end = naive_from_timestamp(item.current_period_end)
        .ok_or_else(|| "missing current_period_end on item".to_string())?;
    let trial_end = subscription.trial_end.and_then(naive_from_timestamp);

    sub_mapper
        .upsert_from_stripe(
            user_id,
            subscription.status.as_str(),
            &stripe_customer_id,
            subscription.id.as_str(),
            &price_id,
            period_start,
            period_end,
            trial_end,
            subscription.cancel_at_period_end,
        )
        .await
        .map_err(|e| format!("upsert failed: {e}"))?;

    Ok(())
}

fn naive_from_timestamp(ts: Timestamp) -> Option<chrono::NaiveDateTime> {
    DateTime::from_timestamp(ts, 0).map(|dt| dt.naive_utc())
}

#[cfg(test)]
mod tests {
    use super::SubscriptionResponse;
    use crate::entity::subscription::Entitlement;

    #[test]
    fn free_entitlement_serialises_with_tier_free_and_no_extras() {
        let resp = SubscriptionResponse::from(Entitlement::free());
        assert_eq!(resp.tier, "free");
        assert!(resp.status.is_none());
        assert!(resp.current_period_end.is_none());
        assert!(!resp.cancel_at_period_end);
        assert!(resp.trial_end.is_none());
    }
}
