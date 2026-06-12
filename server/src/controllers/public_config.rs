use actix_web::{HttpResponse, get, web};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Public, non-secret configuration that the SPA needs at bootstrap.
///
/// Constructed once at server start from `ServerConfig`, then injected as
/// `web::Data<PublicConfig>`. Acts as a deny-by-default surface — only fields
/// explicitly placed here can be exposed to the browser, so other parts of
/// `ServerConfig` (DB creds, JWT secret, Stripe secret, etc.) cannot be
/// leaked by an accidental controller change.
#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct PublicConfig {
    /// Google OAuth client ID — same value the backend uses to verify Google
    /// credential JWTs in `oauth::google_oauth`. Sourced from `config.toml`
    /// at startup so the frontend Docker image can be environment-agnostic.
    pub google_client_id: String,
    /// Free-tier caps + Pro ceiling, mirrored from `LimitsConfig` so the SPA
    /// renders the real numbers (pricing copy, cap-interrupt modals) instead
    /// of hardcoding them and silently drifting from the server (M-10).
    pub limits: PublicLimits,
    /// Presence heartbeat cadence (seconds), mirrored from `SharingConfig` so
    /// the client's heartbeat timer tracks the server's TTL instead of a
    /// hardcoded constant (L-11).
    pub presence_heartbeat_seconds: u32,
}

/// Browser-facing projection of `LimitsConfig` (the entitlement caps the SPA
/// needs to display). Numbers only — no behaviour, no secrets.
#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct PublicLimits {
    pub free_calendar_limit: u32,
    pub free_show_cap: u32,
    pub pro_max_reminders: u32,
}

#[utoipa::path(
    get,
    path = "/public-config",
    operation_id = "get_public_config",
    tag = "config",
    responses((status = 200, body = PublicConfig)),
)]
#[get("/public-config")]
async fn get(config: web::Data<PublicConfig>) -> HttpResponse {
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "public, max-age=300"))
        .json(config.get_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test};

    #[actix_web::test]
    async fn returns_google_client_id_from_injected_config() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(PublicConfig {
                    google_client_id: "test-cid.apps.googleusercontent.com".to_owned(),
                    limits: PublicLimits {
                        free_calendar_limit: 3,
                        free_show_cap: 25,
                        pro_max_reminders: 5,
                    },
                    presence_heartbeat_seconds: 30,
                }))
                .service(get),
        )
        .await;

        let req = test::TestRequest::get().uri("/public-config").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let cache_control = resp
            .headers()
            .get("Cache-Control")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        assert_eq!(cache_control, "public, max-age=300");

        let body: PublicConfig = test::read_body_json(resp).await;
        assert_eq!(body.google_client_id, "test-cid.apps.googleusercontent.com");
        assert_eq!(body.limits.free_calendar_limit, 3);
        assert_eq!(body.limits.free_show_cap, 25);
        assert_eq!(body.limits.pro_max_reminders, 5);
        assert_eq!(body.presence_heartbeat_seconds, 30);
    }
}
