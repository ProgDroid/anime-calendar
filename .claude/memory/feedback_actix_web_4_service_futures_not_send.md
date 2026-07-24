---
name: actix-web 4 service futures are NOT Send
description: Boxed futures inside actix Service::call must be LocalBoxFuture, never BoxFuture+Send — actix-web 4 uses per-worker LocalSets
type: feedback
originSessionId: d141fb2f-b707-4ff0-8ba3-228ad77d407f
---
When implementing an actix-web 4 `Service` / `Transform` middleware, the boxed future returned from `call` MUST be `LocalBoxFuture<'static, ...>` from `futures_util::future`, NOT `BoxFuture<'static, ... + Send>`.

**Why:** actix-web 4 runs each App on a per-worker `tokio::task::LocalSet`. Service futures are pinned to the LocalSet's local pool and never moved across threads. Adding `+ Send` forces every type captured by the future to be `Send`, but typical handler types (Rc, RefCell, !Send extractors, route service futures from inner services) are not. The `+ Send` bound will compile-error at the call site of any non-Send inner service.

**How to apply:**
- New middleware: type alias `type ResFuture<B> = LocalBoxFuture<'static, Result<ServiceResponse<B>, Error>>;`
- Reviewers proposing `+ Send` in middleware briefs: push back. They are confusing actix-web 3 conventions (or non-actix middleware ecosystems like axum/tower) with actix-web 4.
- The compiler error you'll see if you ignore this: `the trait bound \`X: Send\` is not satisfied` somewhere deep inside `actix_web::Resource::route` or similar — the error never points at your middleware directly.

This came up live during T1 (CD-LICENSE-1) when the spec reviewer prescribed `+ Send` on the in-house RateLimit wrapper around the MIT `governor` crate. The implementer correctly switched to `LocalBoxFuture` and the user reviewed + approved the pushback. The MIT replacement now lives at `server/src/middleware/rate_limit.rs`.
