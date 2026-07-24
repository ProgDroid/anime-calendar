---
name: No global auth middleware → can't read Claims in actix-governor KeyExtractor
description: This codebase decodes Claims per-handler via FromRequest, not in a global middleware. Therefore actix-governor KeyExtractors cannot read Claims for per-user rate limiting. Implement per-user budgets in the service layer instead.
type: feedback
originSessionId: 6e32f781-1240-465d-af04-4ad77bc7dd15
---
The anime-calendar server has **no global auth middleware** that decodes
the JWT and inserts `Claims` into `req.extensions()`. Instead,
`server/src/middleware/auth.rs` implements `actix_web::FromRequest` for
`Claims`, so each handler that wants the caller's identity declares
`claims: Claims` in its argument list and the decode happens at
extractor time.

**Why:** This means `actix-governor::KeyExtractor::extract` runs **before**
the handler — its only access to the request is `ServiceRequest`, which
doesn't carry the decoded `Claims` because nothing has put them there
yet. Re-decoding the cookie/JWT inside the extractor would duplicate the
auth logic and is fragile. The plan for Phase 1 of the co-editor
sharing feature originally specified a `PerUserKey` extractor for the
20-invites-per-hour rate limit; that approach was abandoned on
2026-05-06 in favor of a service-layer counted-window query.

**How to apply:** When a feature wants per-user (NOT per-IP) rate
limiting, do NOT reach for `actix-governor` with a custom
`KeyExtractor`. Instead:
1. Add a counting helper to the relevant mapper:
   `count_for_actor_since(actor_id: i32, since: NaiveDateTime) -> ServerResult<i64>`.
2. Run the check inside the service method that mutates the resource,
   before the actual write. Compare against a config-driven budget.
3. Return `Error::TooManyRequests` (HTTP 429, body `{"error":"rate_limited"}`)
   when over budget. The variant exists from Phase 1.
4. Tight races at the window boundary are acceptable for these flows —
   the integrity-critical check is the cap inside the advisory lock,
   not the rate limit.

If global per-IP rate limiting is needed (e.g. webhook protection),
`actix-governor` with `WebhookExemptKeyExtractor` in `server/src/server.rs`
is the right tool — it works because `IpAddr` IS available on
`ServiceRequest` without auth context. See
`feedback_actix_governor_path_exempt` for that pattern.
