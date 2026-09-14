use actix_web::{HttpResponse, get};
use serde::Serialize;

/// Liveness payload returned by [`health`].
///
/// Deliberately static and dependency-free: the endpoint answers "is this
/// process up and serving HTTP?", not "are Postgres and Redis reachable?".
/// A readiness check that fans out to backing services would make a Redis
/// blip roll the whole Cloud Run revision.
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

/// `GET /health` — unauthenticated liveness probe.
///
/// Registered without an `/api` prefix like every other backend route: nginx
/// proxies `location /api/` to `http://server:8080/` with a trailing slash,
/// which strips the prefix before the request reaches actix. The deploy
/// workflow's smoke test curls `https://${DOMAIN}/api/health` and lands here.
#[get("/health")]
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse { status: "ok" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test};

    #[actix_web::test]
    async fn health_returns_200_ok() {
        let app = test::init_service(App::new().service(health)).await;
        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn health_returns_status_ok_json() {
        let app = test::init_service(App::new().service(health)).await;
        let req = test::TestRequest::get().uri("/health").to_request();
        let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(body["status"], "ok");
    }

    /// Guards the prefix decision above: if someone "fixes" the route back to
    /// `/api/health`, the nginx-proxied smoke test starts 404ing in CI.
    #[actix_web::test]
    async fn health_is_not_registered_under_api_prefix() {
        let app = test::init_service(App::new().service(health)).await;
        let req = test::TestRequest::get().uri("/api/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }
}
