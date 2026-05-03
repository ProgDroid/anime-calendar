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
    }
}
