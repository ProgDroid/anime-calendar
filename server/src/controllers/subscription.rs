//! Read-side endpoint for the current user's effective entitlement.
//!
//! The local `subscriptions` table is written by the Stripe webhook handler
//! (`controllers/stripe_webhook.rs`); this endpoint just reads the result.
//! The Phase-2 `?session_id=` query-string stopgap was removed once Phase 3
//! landed — the webhook is now the canonical writer.

use crate::{
    entity::subscription::Entitlement,
    mappers::user::UserMapper,
    middleware::auth::Claims,
    services::entitlement::EntitlementService,
};
use actix_web::{HttpResponse, ResponseError, get, web};
use serde::Serialize;

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
    entitlement: web::Data<EntitlementService>,
    claims: Claims,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(u) => u,
        Err(e) => return e.error_response(),
    };

    match entitlement.entitlement(user.id).await {
        Ok(ent) => HttpResponse::Ok().json(SubscriptionResponse::from(ent)),
        Err(e) => e.error_response(),
    }
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
