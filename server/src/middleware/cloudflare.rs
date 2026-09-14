//! Origin lock: reject requests that did not arrive via Cloudflare.
//!
//! Cloud Run services carry a public `*.run.app` URL that stays reachable even
//! once DNS points at Cloudflare, so the WAF, rate limiting and bot rules at
//! the edge can be bypassed by addressing the origin directly. A Cloudflare
//! Transform Rule adds `X-CF-Origin-Secret` to every proxied request; anything
//! without the matching value is refused here with 403.
//!
//! Disabled when no secret is configured, which is the local-dev default —
//! there is no Cloudflare in front of `cargo run`.

use std::rc::Rc;

use actix_web::{
    Error, HttpResponse,
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::{LocalBoxFuture, Ready, ok};

/// Header Cloudflare is configured to inject.
const HEADER: &str = "X-CF-Origin-Secret";

/// Compare without an early return on the first differing byte.
///
/// The margin this buys over a network is tiny, but the comparison guards a
/// shared secret and a branch-free version costs nothing. Length is not
/// concealed — only the contents.
fn secrets_match(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0_u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub struct CloudflareOrigin {
    /// `None` (or an empty string) disables the check entirely.
    secret: Option<Rc<str>>,
}

impl CloudflareOrigin {
    /// Build from the configured secret. `None` or `Some("")` yields a
    /// pass-through middleware, so local dev needs no special casing at the
    /// call site.
    #[must_use]
    pub fn new(secret: Option<String>) -> Self {
        Self {
            secret: secret
                .filter(|s| !s.is_empty())
                .map(|s| Rc::from(s.as_str())),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for CloudflareOrigin
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = CloudflareOriginMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(CloudflareOriginMiddleware {
            service: Rc::new(service),
            secret: self.secret.clone(),
        })
    }
}

pub struct CloudflareOriginMiddleware<S> {
    service: Rc<S>,
    secret: Option<Rc<str>>,
}

impl<S, B> Service<ServiceRequest> for CloudflareOriginMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = Rc::clone(&self.service);

        // No secret configured: the lock is off (local dev, or an origin not
        // fronted by Cloudflare).
        let Some(secret) = self.secret.clone() else {
            return Box::pin(async move {
                svc.call(req).await.map(ServiceResponse::map_into_left_body)
            });
        };

        let presented = req
            .headers()
            .get(HEADER)
            .map(|v| v.as_bytes().to_vec())
            .unwrap_or_default();

        Box::pin(async move {
            if secrets_match(&presented, secret.as_bytes()) {
                svc.call(req).await.map(ServiceResponse::map_into_left_body)
            } else {
                // Deliberately identical for a missing and a wrong header, and
                // says nothing about which: the response should not help an
                // attacker learn whether they found the right header name.
                let response =
                    HttpResponse::Forbidden().json(serde_json::json!({"error": "forbidden"}));
                Ok(req.into_response(response).map_into_right_body())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, HttpResponse, get, test as actix_test};

    #[get("/ping")]
    async fn ping() -> HttpResponse {
        HttpResponse::Ok().finish()
    }

    // actix-web 4 runs each worker on its own `LocalSet`, so service futures are
    // deliberately not `Send` (see the same note in `middleware/rate_limit.rs`).
    // A test helper that holds the test service across an await inherits that.
    #[allow(clippy::future_not_send)]
    async fn call_with(secret: Option<&str>, header: Option<&str>) -> actix_web::http::StatusCode {
        let app = actix_test::init_service(
            App::new()
                .wrap(CloudflareOrigin::new(secret.map(ToOwned::to_owned)))
                .service(ping),
        )
        .await;
        let mut req = actix_test::TestRequest::get().uri("/ping");
        if let Some(h) = header {
            req = req.insert_header((HEADER, h));
        }
        actix_test::call_service(&app, req.to_request())
            .await
            .status()
    }

    #[actix_web::test]
    async fn missing_header_returns_403() {
        assert_eq!(call_with(Some("secret123"), None).await, 403);
    }

    #[actix_web::test]
    async fn wrong_secret_returns_403() {
        assert_eq!(call_with(Some("secret123"), Some("wrong")).await, 403);
    }

    /// A prefix of the real secret must not pass — guards against a comparison
    /// that stops at the shorter length.
    #[actix_web::test]
    async fn prefix_of_secret_returns_403() {
        assert_eq!(call_with(Some("secret123"), Some("secret")).await, 403);
    }

    #[actix_web::test]
    async fn correct_secret_passes_through() {
        assert_eq!(call_with(Some("secret123"), Some("secret123")).await, 200);
    }

    /// Local dev has no Cloudflare in front of it.
    #[actix_web::test]
    async fn no_configured_secret_disables_the_check() {
        assert_eq!(call_with(None, None).await, 200);
    }

    /// An empty string in config is "unset", not "the empty secret" — otherwise
    /// a blank env var would lock every request out.
    #[actix_web::test]
    async fn empty_configured_secret_disables_the_check() {
        assert_eq!(call_with(Some(""), None).await, 200);
    }

    #[test]
    fn secrets_match_rejects_differing_length_and_content() {
        assert!(secrets_match(b"abc", b"abc"));
        assert!(!secrets_match(b"abc", b"abd"));
        assert!(!secrets_match(b"abc", b"ab"));
        assert!(!secrets_match(b"", b"a"));
        assert!(secrets_match(b"", b""));
    }
}
