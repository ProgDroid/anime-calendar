//! In-house actix middleware wrapping the MIT-licensed `governor` crate.
//! Replaces `actix-governor` (GPL-3.0-or-later) per AUDIT.md CD-LICENSE-1.
//!
//! Semantics preserved:
//! - 60-request burst, 1-request-per-second steady-state replenishment.
//! - Per-IP keying with IPv6 /56-prefix bucketing (bytes 0–6 preserved,
//!   bytes 7–15 zeroed).
//! - Path-exempt: `/stripe/webhook` always allowed (Stripe retries failed
//!   deliveries in bursts; HTTP 429 there would trip its endpoint-disabled
//!   heuristic).
//! - 429 response body: `{"error":"rate limited","retry_after":<secs>}`.
//!
//! ## Key bookkeeping
//!
//! `DefaultKeyedRateLimiter` is backed by a concurrent `DashMap` keyed by
//! `IpAddr`. The map has no automatic eviction, so on a public-internet
//! deployment a broad port-scan or DDoS attempt would otherwise grow it
//! unboundedly (≈ 250 B per distinct source IP — 1 M IPs ≈ 250 MB).
//! `RateLimit::new` therefore spawns a background task that calls
//! `RateLimiter::retain_recent` every 60 seconds; the task holds only a
//! `Weak` reference to the limiter, so it exits cleanly once the last
//! `RateLimit` clone is dropped (e.g. on graceful shutdown).

use std::{
    future::{Ready, ready},
    net::{IpAddr, Ipv6Addr},
    num::NonZeroU32,
    sync::{Arc, Weak},
    time::Duration,
};

use actix_web::{
    Error, HttpResponse,
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use governor::{
    DefaultKeyedRateLimiter, Quota, RateLimiter,
    clock::{Clock as _, DefaultClock},
};

const WEBHOOK_EXEMPT_PATH: &str = "/stripe/webhook";

/// Actix-web middleware that enforces a token-bucket rate limit per client IP.
///
/// Build with [`RateLimit::new`] and attach via `.wrap(rate_limit)` on the
/// `App`. The factory is cheaply `Clone`d per worker via the inner `Arc`.
#[derive(Clone)]
pub struct RateLimit {
    limiter: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    trust_proxy_header: bool,
}

impl RateLimit {
    /// Create a rate limiter allowing `burst` requests of capacity that
    /// replenishes at one request per `period`.
    ///
    /// `trust_proxy_header` selects the keying source. Behind the shipped
    /// nginx proxy every connection's peer address IS the proxy, which would
    /// collapse all clients into one shared bucket — pass `true` there so the
    /// key comes from the rightmost `X-Forwarded-For` entry (the hop the
    /// proxy itself appended; earlier entries are client-controlled and
    /// spoofable). Pass `false` for direct-to-server deployments, where the
    /// header cannot be trusted at all.
    ///
    /// Spawns a background tokio task that calls
    /// [`RateLimiter::retain_recent`] every 60 seconds to evict idle keys
    /// (see module docs for rationale). The task only holds a [`Weak`]
    /// reference to the limiter, so it terminates once the last `RateLimit`
    /// clone is dropped.
    ///
    /// # Panics
    /// Panics if `burst` is 0 or if `period` is zero (programming error).
    #[must_use]
    pub fn new(burst: u32, period: Duration, trust_proxy_header: bool) -> Self {
        let quota = Quota::with_period(period)
            .expect("rate limit period must be > 0")
            .allow_burst(NonZeroU32::new(burst).expect("burst must be > 0"));
        let limiter = Arc::new(RateLimiter::keyed(quota));

        // Background eviction: holds Weak so it exits when the last RateLimit
        // is dropped, rather than pinning the limiter alive forever.
        let weak = Arc::downgrade(&limiter);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            // First tick fires immediately; skip it so we don't churn at startup.
            interval.tick().await;
            loop {
                interval.tick().await;
                let Some(limiter) = Weak::upgrade(&weak) else {
                    break;
                };
                limiter.retain_recent();
            }
        });

        Self {
            limiter,
            trust_proxy_header,
        }
    }
}

/// Resolve the IP address a request should be rate-limited under.
///
/// With `trust_proxy_header` set, prefers the rightmost `X-Forwarded-For`
/// entry — the one appended by our own proxy and therefore the only one a
/// client cannot forge. Falls back to the TCP peer address when the header
/// is absent or unparseable (e.g. health checks hitting the backend port
/// directly inside the compose network).
fn client_ip(req: &ServiceRequest, trust_proxy_header: bool) -> Option<IpAddr> {
    if trust_proxy_header {
        let forwarded = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .map(str::trim)
            .and_then(|ip| ip.parse::<IpAddr>().ok());
        if let Some(ip) = forwarded {
            return Some(ip);
        }
    }
    req.peer_addr().map(|s| s.ip())
}

impl<S, B> Transform<S, ServiceRequest> for RateLimit
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = RateLimitMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddleware {
            service,
            limiter: Arc::clone(&self.limiter),
            trust_proxy_header: self.trust_proxy_header,
        }))
    }
}

pub struct RateLimitMiddleware<S> {
    service: S,
    limiter: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    trust_proxy_header: bool,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    // Actix-web 4 runs each worker on its own `LocalSet`, so service futures
    // are NOT required to be `Send` (handlers may hold non-Send state like
    // `Rc`). We use `LocalBoxFuture` to make the non-Send contract explicit
    // and to match the convention in `server/src/middleware/auth.rs`.
    type Future = futures::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Webhook exemption — Stripe retries in bursts; never 429 this path.
        if req.path() == WEBHOOK_EXEMPT_PATH {
            let fut = self.service.call(req);
            return Box::pin(async move { fut.await.map(ServiceResponse::map_into_left_body) });
        }

        // Resolve the client IP, applying /56-prefix bucketing for IPv6.
        let key = match client_ip(&req, self.trust_proxy_header) {
            Some(IpAddr::V4(v4)) => IpAddr::V4(v4),
            Some(IpAddr::V6(v6)) => {
                let mut octets = v6.octets();
                // Zero the host portion (bytes 7-15) so all addresses in the
                // same /56 share one bucket, matching the original behaviour.
                octets[7..16].fill(0);
                IpAddr::V6(Ipv6Addr::from(octets))
            }
            None => {
                log::error!("rate_limit: could not extract a client IP from request");
                let resp = HttpResponse::InternalServerError()
                    .content_type("application/json")
                    .body(r#"{"error":"internal server error"}"#);
                return Box::pin(async move { Ok(req.into_response(resp).map_into_right_body()) });
            }
        };

        match self.limiter.check_key(&key) {
            Ok(()) => {
                let fut = self.service.call(req);
                Box::pin(async move { fut.await.map(ServiceResponse::map_into_left_body) })
            }
            Err(negative) => {
                let wait = negative
                    .wait_time_from(DefaultClock::default().now())
                    .as_secs();
                let body = format!(r#"{{"error":"rate limited","retry_after":{wait}}}"#);
                let resp = HttpResponse::TooManyRequests()
                    .content_type("application/json")
                    .insert_header(("Retry-After", wait.to_string()))
                    .body(body);
                Box::pin(async move { Ok(req.into_response(resp).map_into_right_body()) })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{
        App, HttpResponse,
        http::StatusCode,
        test::{call_service, init_service},
        web,
    };

    async fn ok_handler() -> HttpResponse {
        HttpResponse::Ok().finish()
    }

    /// Helper: build a test app with the given `RateLimit` and a handler
    /// mounted at `GET /` and `GET /stripe/webhook`.
    macro_rules! test_app {
        ($rl:expr) => {
            init_service(
                App::new()
                    .wrap($rl)
                    .route("/", web::get().to(ok_handler))
                    .route("/stripe/webhook", web::get().to(ok_handler)),
            )
            .await
        };
    }

    /// Convenience: build a `TestRequest` with a fixed peer addr.
    fn req_with_peer(uri: &str, peer: &str) -> actix_web::test::TestRequest {
        actix_web::test::TestRequest::get()
            .uri(uri)
            .peer_addr(peer.parse().unwrap())
    }

    #[actix_web::test]
    async fn burst_of_60_all_allowed() {
        // Burst = 60, very slow replenishment (1000 s) so no refill during test.
        let rl = RateLimit::new(60, Duration::from_secs(1000), false);
        let app = test_app!(rl);

        for _ in 0..60 {
            let req = req_with_peer("/", "1.2.3.4:5678").to_request();
            let resp = call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                StatusCode::OK,
                "expected OK within burst window"
            );
        }
    }

    #[actix_web::test]
    async fn over_burst_returns_429() {
        let rl = RateLimit::new(60, Duration::from_secs(1000), false);
        let app = test_app!(rl);

        // Exhaust the burst.
        for _ in 0..60 {
            let req = req_with_peer("/", "10.0.0.1:1111").to_request();
            let _ = call_service(&app, req).await;
        }

        // 61st request must be rate limited.
        let req = req_with_peer("/", "10.0.0.1:1111").to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

        // Body must contain the correct shape.
        let body = actix_web::test::read_body(resp).await;
        let body_str = std::str::from_utf8(&body).unwrap();
        assert!(
            body_str.contains(r#""error":"rate limited""#),
            "unexpected body: {body_str}"
        );
        assert!(
            body_str.contains(r#""retry_after":"#),
            "missing retry_after field: {body_str}"
        );
    }

    #[actix_web::test]
    async fn webhook_path_always_allowed() {
        // Tiny burst of 2 — the webhook path should bypass this entirely.
        let rl = RateLimit::new(2, Duration::from_secs(1000), false);
        let app = test_app!(rl);

        // Send 100 requests to the webhook path from the same IP.
        for i in 0..100u32 {
            let req = req_with_peer("/stripe/webhook", "5.5.5.5:80").to_request();
            let resp = call_service(&app, req).await;
            assert_ne!(
                resp.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "webhook request {i} was rate limited — should be exempt"
            );
        }
    }

    #[actix_web::test]
    async fn per_ip_isolation() {
        // Burst = 1 — IP A can get 1 through; IP B should still get 1 through.
        let rl = RateLimit::new(1, Duration::from_secs(1000), false);
        let app = test_app!(rl);

        // Exhaust IP A's burst.
        let req_a = req_with_peer("/", "192.168.1.1:9000").to_request();
        assert_eq!(call_service(&app, req_a).await.status(), StatusCode::OK);

        // IP A's 2nd request is blocked.
        let req_a2 = req_with_peer("/", "192.168.1.1:9000").to_request();
        assert_eq!(
            call_service(&app, req_a2).await.status(),
            StatusCode::TOO_MANY_REQUESTS
        );

        // IP B (different key) is still allowed.
        let req_b = req_with_peer("/", "192.168.1.2:9000").to_request();
        assert_eq!(
            call_service(&app, req_b).await.status(),
            StatusCode::OK,
            "different IP should have its own independent bucket"
        );
    }

    #[actix_web::test]
    async fn xff_is_ignored_when_proxy_is_not_trusted() {
        // trust_proxy_header = false: a client-supplied X-Forwarded-For must
        // not let it hop buckets — keying stays on the TCP peer.
        let rl = RateLimit::new(1, Duration::from_secs(1000), false);
        let app = test_app!(rl);

        let req = req_with_peer("/", "10.1.1.1:1000")
            .insert_header(("X-Forwarded-For", "9.9.9.9"))
            .to_request();
        assert_eq!(call_service(&app, req).await.status(), StatusCode::OK);

        // Same peer, different forged header → same bucket → limited.
        let req = req_with_peer("/", "10.1.1.1:1000")
            .insert_header(("X-Forwarded-For", "8.8.8.8"))
            .to_request();
        assert_eq!(
            call_service(&app, req).await.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "forged XFF must not split buckets when the proxy is untrusted"
        );
    }

    #[actix_web::test]
    async fn trusted_proxy_keys_on_rightmost_xff_entry() {
        // trust_proxy_header = true: all requests share the proxy's peer IP;
        // buckets must follow the rightmost (proxy-appended) XFF entry, and
        // client-controlled earlier entries must be ignored.
        let rl = RateLimit::new(1, Duration::from_secs(1000), true);
        let app = test_app!(rl);

        let req = req_with_peer("/", "172.18.0.2:80")
            .insert_header(("X-Forwarded-For", "6.6.6.6, 1.1.1.1"))
            .to_request();
        assert_eq!(call_service(&app, req).await.status(), StatusCode::OK);

        // Different forged left entry, same rightmost → same bucket.
        let req = req_with_peer("/", "172.18.0.2:80")
            .insert_header(("X-Forwarded-For", "7.7.7.7, 1.1.1.1"))
            .to_request();
        assert_eq!(
            call_service(&app, req).await.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "rightmost XFF entry must select the bucket"
        );

        // Different rightmost entry (another real client) → its own bucket,
        // even though the TCP peer (the proxy) is identical.
        let req = req_with_peer("/", "172.18.0.2:80")
            .insert_header(("X-Forwarded-For", "6.6.6.6, 2.2.2.2"))
            .to_request();
        assert_eq!(
            call_service(&app, req).await.status(),
            StatusCode::OK,
            "distinct clients behind the proxy must not share a bucket"
        );
    }

    #[actix_web::test]
    async fn trusted_proxy_falls_back_to_peer_without_header() {
        // trust_proxy_header = true but no XFF (e.g. an in-network health
        // check hitting the backend port directly): fall back to peer IP.
        let rl = RateLimit::new(1, Duration::from_secs(1000), true);
        let app = test_app!(rl);

        let req = req_with_peer("/", "10.2.2.2:1000").to_request();
        assert_eq!(call_service(&app, req).await.status(), StatusCode::OK);

        let req = req_with_peer("/", "10.2.2.2:1000").to_request();
        assert_eq!(
            call_service(&app, req).await.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "peer fallback must still rate-limit"
        );
    }

    #[actix_web::test]
    async fn ipv6_56_prefix_bucketing() {
        // Two IPv6 addresses sharing the same /56 prefix (bytes 0-6 identical,
        // bytes 7-15 differ) should share a bucket.
        let rl = RateLimit::new(1, Duration::from_secs(1000), false);
        let app = test_app!(rl);

        // Address A: 2001:db8:1:2:3:4:5:6
        let req_a = req_with_peer("/", "[2001:db8:1:2:3:4:5:6]:80").to_request();
        assert_eq!(call_service(&app, req_a).await.status(), StatusCode::OK);

        // Address B shares the /56 prefix (bytes 0-6 = 2001:db8:1:2:3:4:XX)
        // so after zeroing bytes 7-15 both collapse to the same bucket key.
        let req_b = req_with_peer("/", "[2001:db8:1:2:3:4:5:7]:80").to_request();
        let resp_b = call_service(&app, req_b).await;
        assert_eq!(
            resp_b.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "IPv6 addresses in the same /56 should share a rate-limit bucket"
        );
    }
}
