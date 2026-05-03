//! Stripe-facing controller: kicks the user off to a hosted Checkout Session.
//!
//! Phase 2 of Track 4 only handles the *outbound* side of the flow — the user
//! posts a desired billing interval, we create a `CheckoutSession` on Stripe,
//! and return its hosted URL. Phase 3 adds the inbound webhook handler;
//! Phase 4 adds the Customer Portal handoff.

use crate::{
    config::server::{AppBaseUrl, StripeConfig},
    error::Error,
    mappers::{subscription::SubscriptionMapper, user::UserMapper},
    middleware::auth::Claims,
};
use actix_web::{HttpResponse, ResponseError, post, web};
use log::error;
use serde::{Deserialize, Serialize};
use stripe::Client as StripeClient;
use stripe_billing::billing_portal_session::CreateBillingPortalSession;
use stripe_checkout::checkout_session::{
    CreateCheckoutSession, CreateCheckoutSessionLineItems,
    CreateCheckoutSessionPaymentMethodCollection, CreateCheckoutSessionSubscriptionData,
};
use stripe_shared::CheckoutSessionMode;

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BillingInterval {
    Monthly,
    Annual,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CheckoutRequest {
    pub interval: BillingInterval,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CheckoutResponse {
    /// Stripe-hosted Checkout page the frontend should redirect the user to.
    pub url: String,
}

#[utoipa::path(
    post,
    path = "/stripe/checkout",
    tag = "stripe",
    request_body = CheckoutRequest,
    responses(
        (status = 200, body = CheckoutResponse),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
        (status = 500, body = crate::controllers::auth::ErrorResponse,
         description = "Stripe is not configured or returned an error"),
    ),
    security(("bearer_auth" = []))
)]
#[post("/stripe/checkout")]
#[allow(clippy::future_not_send)]
pub async fn create_checkout_session(
    user_mapper: web::Data<UserMapper>,
    sub_mapper: web::Data<SubscriptionMapper>,
    stripe_client: web::Data<StripeClient>,
    stripe_config: web::Data<StripeConfig>,
    app_base_url: web::Data<AppBaseUrl>,
    claims: Claims,
    body: web::Json<CheckoutRequest>,
) -> HttpResponse {
    if !stripe_config.is_configured() {
        return Error::StripeNotConfigured.error_response();
    }

    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(u) => u,
        Err(e) => return e.error_response(),
    };

    let price_id = match body.interval {
        BillingInterval::Monthly => stripe_config.price_id_monthly.as_str(),
        BillingInterval::Annual => stripe_config.price_id_annual.as_str(),
    };
    if price_id.is_empty() {
        // Configured key, missing the requested price id — operator error.
        return Error::StripeNotConfigured.error_response();
    }

    let frontend = app_base_url.as_str().trim_end_matches('/');
    let success_url = format!("{frontend}/upgrade/success?session_id={{CHECKOUT_SESSION_ID}}");
    let cancel_url = format!("{frontend}/upgrade/canceled");

    // Card is required up front — no "trial-then-vanish" abuse pattern.
    // Stamp user_id into the subscription metadata so every downstream
    // `customer.subscription.*` and `invoice.*` webhook event carries the
    // mapping back to our user — no need to fetch the original Checkout
    // session or rely on the customer-id lookup table.
    let mut subscription_data = CreateCheckoutSessionSubscriptionData::new();
    subscription_data.trial_period_days = Some(14);
    subscription_data.metadata = Some(std::collections::HashMap::from([(
        "user_id".to_string(),
        user.id.to_string(),
    )]));

    let mut create = CreateCheckoutSession::new()
        .mode(CheckoutSessionMode::Subscription)
        .line_items(vec![CreateCheckoutSessionLineItems {
            price: Some(price_id.to_string()),
            quantity: Some(1),
            ..Default::default()
        }])
        .success_url(success_url.as_str())
        .cancel_url(cancel_url.as_str())
        .payment_method_collection(CreateCheckoutSessionPaymentMethodCollection::Always)
        .subscription_data(subscription_data)
        .client_reference_id(user.id.to_string());

    // Reuse an existing Stripe customer if the user is re-subscribing after
    // cancellation. Looking up the latest `stripe_customer_id` regardless of
    // subscription status keeps payment-method history attached.
    let existing_customer = match sub_mapper.find_latest_customer_id_for_user(user.id).await {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };
    if let Some(ref cust_id) = existing_customer {
        create = create.customer(cust_id.as_str());
    } else {
        // No prior Stripe customer — pre-fill the email so Stripe can create
        // one on our behalf without a guest-checkout fallback.
        create = create.customer_email(user.email.as_str());
    }

    let session = match create.send(stripe_client.get_ref()).await {
        Ok(s) => s,
        Err(e) => {
            // Don't leak internal Stripe error detail in the response body —
            // log it server-side for debugging, surface generic 500 to client.
            error!("stripe create checkout session failed: {e}");
            return Error::Stripe(e.to_string()).error_response();
        }
    };

    let Some(url) = session.url else {
        error!("stripe checkout session created without a redirect url");
        return Error::Stripe("missing redirect url".into()).error_response();
    };

    HttpResponse::Ok().json(CheckoutResponse { url })
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PortalResponse {
    /// Stripe-hosted Customer Portal page; the frontend should
    /// `window.location.href` to it.
    pub url: String,
}

/// `POST /api/stripe/portal` — generate a Stripe Customer Portal session for
/// the calling user and return its hosted URL.
///
/// The user must already have a `stripe_customer_id` on a previous
/// subscription row — i.e. they must have completed at least one Checkout.
/// Free users hitting this endpoint receive 400; the frontend gates the
/// "Manage subscription" CTA so this should never trip in the happy path.
///
/// **Authorization defense in depth:** the lookup goes through
/// `find_latest_customer_id_for_user(claims.user_id)`, so the resulting
/// portal session can only ever target the calling user's customer. There's
/// no caller-supplied customer id to validate against — that's by design.
#[utoipa::path(
    post,
    path = "/stripe/portal",
    tag = "stripe",
    responses(
        (status = 200, body = PortalResponse),
        (status = 400, body = crate::controllers::auth::ErrorResponse,
         description = "Caller has no Stripe customer id (never subscribed)"),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
        (status = 500, body = crate::controllers::auth::ErrorResponse,
         description = "Stripe is not configured or returned an error"),
    ),
    security(("bearer_auth" = []))
)]
#[post("/stripe/portal")]
#[allow(clippy::future_not_send)]
pub async fn create_portal_session(
    user_mapper: web::Data<UserMapper>,
    sub_mapper: web::Data<SubscriptionMapper>,
    stripe_client: web::Data<StripeClient>,
    stripe_config: web::Data<StripeConfig>,
    app_base_url: web::Data<AppBaseUrl>,
    claims: Claims,
) -> HttpResponse {
    if !stripe_config.is_configured() {
        return Error::StripeNotConfigured.error_response();
    }

    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(u) => u,
        Err(e) => return e.error_response(),
    };

    let customer_id = match sub_mapper.find_latest_customer_id_for_user(user.id).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            // User has never subscribed — nothing for the portal to manage.
            // 400 with a clear error matches the frontend's expectation that
            // the Manage CTA is only shown to paid/past_due/trialing users.
            return Error::InvalidRequest.error_response();
        }
        Err(e) => return e.error_response(),
    };

    let frontend = app_base_url.as_str().trim_end_matches('/');
    let return_url = format!("{frontend}/account/subscription");

    let session = match CreateBillingPortalSession::new()
        .customer(customer_id.as_str())
        .return_url(return_url.as_str())
        .send(stripe_client.get_ref())
        .await
    {
        Ok(s) => s,
        Err(e) => {
            error!("stripe create billing portal session failed: {e}");
            return Error::Stripe(e.to_string()).error_response();
        }
    };

    HttpResponse::Ok().json(PortalResponse { url: session.url })
}
