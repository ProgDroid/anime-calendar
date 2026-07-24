---
name: stripe-webhook-handlers-use-a-string-error-chain-not-serverresult
description: "The internal stripe_webhook event handlers return Result<_, String> and there is no From<String> for Error; converting a shared service helper to ServerResult requires bridging Error->String at the webhook call sites."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 1b126c7d-2485-4fb6-9437-7b543abb4528
---

The internal handlers in `controllers/stripe_webhook.rs` (`upsert_subscription`, `handle_subscription_deleted`, `handle_subscription_paused`, `handle_customer_deleted`, …) return `Result<_, String>` — a stringly-typed error chain, NOT the project-standard `ServerResult` / typed `Error`. There is **no `impl From<String> for Error`** anywhere (the enum has only `#[from]`-derived variants: `sqlx::Error`, `config::ConfigError`, `io::Error`, `google_oauth::Error`, `argon2`, `jsonwebtoken`). The webhook `?` operators compile only because the handler fns themselves return `String`.

**Why it bites:** when you convert a *shared* service helper that the webhook calls (e.g. `services::sharing::{suspend,restore}_owner_sharing_in_tx`) from `Result<_, String>` to `ServerResult<_>` (typed `Error`), every webhook `helper(...).await?` site stops compiling — `?` would then need `From<Error> for String`, which doesn't exist. The blast radius is bigger than the helper file.

**How to apply:** bridge at the webhook call sites with `.map_err(|e| e.to_string())?` — this leaves the webhook's own String chain intact (converting that whole chain is a separate, larger task, out of scope for a targeted helper change). Callers already on `ServerResult` need no change: `services/reconcile.rs` uses `match`+`error!`, tests use `.unwrap()` — both `String` and `Error` are `Display`+`Debug`. Use `{e:?}` (not `{e}`) in server-side logs after the switch, since `Error::Database`'s `Display` is the redacted generic string (see [[feedback_error_response_redaction]]); `{e:?}` keeps the inner sqlx detail. Confirmed: audit M-29, commit `cc64ad6`, 2026-06-11 (5 bridge sites). See [[feedback_sharing_helpers_in_services]].
