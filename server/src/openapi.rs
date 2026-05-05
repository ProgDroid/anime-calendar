#![allow(clippy::needless_for_each)]

use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

use crate::{
    controllers::{
        account::UsageResponse,
        auth::{AuthResponse, ErrorResponse, LoginRequest, RegisterRequest},
        calendar::{
            CalendarOwner, CalendarRequest, CalendarsResponse, PageCalendar, PaginatedResponse,
            PaginationInfo, SharedPageCalendar,
        },
        oauth::{GoogleOAuthRequest, GoogleOAuthResponse},
        public_config::PublicConfig,
        stripe::{BillingInterval, CheckoutRequest, CheckoutResponse, PortalResponse},
        subscription::SubscriptionResponse,
        user::{UpdatePasswordRequest, UpdateUserRequest, UserResponse},
    },
    entity::{
        calendar::Language as EntityLanguage,
        user_settings::{SiteLanguage, Theme, UserSettings},
    },
};

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Anime Calendar API",
        version = "1.0.0",
        description = "REST API for managing anime calendars with Anilist integration",
        license(name = "Proprietary", identifier = "LicenseRef-anime-calendar")
    ),
    servers(
        (url = "/", description = "Same-origin (proxied via Vite/nginx)"),
    ),
    paths(
        // auth
        crate::controllers::auth::login,
        crate::controllers::auth::register,
        crate::controllers::auth::get_current_user,
        crate::controllers::auth::verify_token_endpoint,
        crate::controllers::auth::logout,
        // refresh
        crate::controllers::refresh::refresh,
        // password reset
        crate::controllers::password_reset::forgot_password,
        crate::controllers::password_reset::reset_password,
        // email verification
        crate::controllers::email_verification::verify_email,
        crate::controllers::email_verification::resend_verification,
        // oauth
        crate::controllers::oauth::google_oauth,
        // account (free-tier usage counts)
        crate::controllers::account::get_usage,
        // user
        crate::controllers::user::get_user_details,
        crate::controllers::user::update_user,
        crate::controllers::user::delete_user,
        crate::controllers::user::update_password,
        crate::controllers::user::get_user_settings,
        crate::controllers::user::update_user_settings,
        // calendars
        crate::controllers::calendar::get_calendars,
        crate::controllers::calendar::get_calendar,
        crate::controllers::calendar::put,
        crate::controllers::calendar::delete_calendar,
        crate::controllers::calendar::export,
        crate::controllers::calendar::subscribe_feed,
        // items
        crate::controllers::item::get,
        crate::controllers::items::get,
        crate::controllers::items::search,
        // stripe / subscription
        crate::controllers::stripe::create_checkout_session,
        crate::controllers::stripe::create_portal_session,
        crate::controllers::subscription::get_my_subscription,
        // public config
        crate::controllers::public_config::get,
    ),
    components(schemas(
        // auth types
        LoginRequest,
        AuthResponse,
        RegisterRequest,
        ErrorResponse,
        // password reset types
        crate::controllers::password_reset::ForgotPasswordRequest,
        crate::controllers::password_reset::ResetPasswordRequest,
        crate::controllers::auth::MessageResponse,
        // email verification types
        crate::controllers::email_verification::VerifyEmailRequest,
        crate::controllers::email_verification::ResendVerificationRequest,
        // oauth types
        GoogleOAuthRequest,
        GoogleOAuthResponse,
        // user types
        UserResponse,
        UpdateUserRequest,
        UpdatePasswordRequest,
        UserSettings,
        Theme,
        SiteLanguage,
        // calendar types
        // PaginationParams is `IntoParams`, not `ToSchema` — registered via
        // `params(PaginationParams)` on each handler that uses it, not in
        // the global schemas list (see feedback_utoipa_schema_patterns).
        CalendarRequest,
        PageCalendar,
        PaginatedResponse,
        PaginationInfo,
        CalendarOwner,
        SharedPageCalendar,
        CalendarsResponse,
        EntityLanguage,
        // account types
        UsageResponse,
        // subscription / stripe types
        BillingInterval,
        CheckoutRequest,
        CheckoutResponse,
        PortalResponse,
        SubscriptionResponse,
        // public config types
        PublicConfig,
        // common types
        common::calendar::Calendar,
        common::item::Item,
        common::item::Type,
        common::language::Language,
        common::title::Title,
        common::media_cover::MediaCover,
        common::schedule::Schedule,
        common::recommendation::Recommendation,
        common::recommendation::RecommendationMedia,
    )),
    modifiers(&BearerAuth),
    tags(
        (name = "auth", description = "Authentication — register, login, token verification"),
        (name = "user", description = "User profile and settings management"),
        (name = "calendars", description = "Calendar CRUD, iCal export, and subscription feeds"),
        (name = "items", description = "Anilist media item search and retrieval"),
        (name = "stripe", description = "Stripe Checkout and billing integration"),
        (name = "subscription", description = "Effective tier / entitlement read endpoints"),
        (name = "config", description = "Public bootstrap configuration for the SPA"),
    )
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_spec_generates_successfully() {
        let spec = ApiDoc::openapi();
        let json = spec
            .to_pretty_json()
            .expect("OpenAPI spec should serialize to JSON");
        assert!(!json.is_empty(), "OpenAPI spec should not be empty");
    }
}
