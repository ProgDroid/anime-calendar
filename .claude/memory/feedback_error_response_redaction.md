---
name: error-error-response-leaks-interpolated-display-strings
description: Any Error variant whose
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 7b78438c-a9c3-4505-b520-c441c9853c9c
---

`server/src/error.rs` `ResponseError::error_response()` ends with a fallback that serialises `self.to_string()` into the client JSON `{"error": ...}`. So **any `Error` variant whose `#[error("...: {0}")]` interpolates an inner value leaks that value to the client.** As of 2026-06-11 the redacted variants are `Stripe(String)`, `EmailError(String)`, `Redis(String)` — each handled by a dedicated `if let` arm that `log::error!`s the full detail and returns a stable generic code (`stripe_error` / `email_error` / `internal_error`).

**Why:** the `#[from]` variants (`Database`, `Config`, `Server`, `InvalidToken`, `CannotHashPassword`, `CannotGenerateAuthToken`) are safe because their `#[error("...")]` is a STATIC string — the wrapped error is never rendered. The trap is a NEW variant that carries a `String`/detail and interpolates it.

**How to apply:** when adding an `Error` variant that wraps a third-party/internal string, either (a) give it a static `#[error("...")]` (no `{0}`), or (b) add an `if let` redaction arm in `error_response()` before the generic fallback (log detail, return a generic code). The redaction arms must sit AFTER the `PaymentRequired`/`Conflict` early-returns. Generic 5xx codes are safe to change — the frontend only pattern-matches 401/402/409 bodies, never 5xx. Relates to [[feedback_error_variant_for_authz_failure]] (which status for which variant) and [[feedback_anti_enumeration_always_200]].
