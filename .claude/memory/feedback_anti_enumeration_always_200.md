---
name: Anti-enumeration — resend/forgot endpoints must return 200 on ALL paths including internal errors
description: Returning 500 on replace_token/send_email failure leaks account existence; all paths must return the constant 200 ok response
type: feedback
originSessionId: 3ec09e38-54a3-4383-9c58-30ad8b46dfa5
---
Endpoints designed for anti-enumeration (forgot-password, resend-verification) must return the constant 200 response on EVERY code path — including internal failures like `replace_token` error or `send_verification_email` error.

**Why:** A timing-aware attacker observing "email not found → 200" vs "email found but DB error → 500" can enumerate valid accounts. The anti-enumeration guarantee is broken the moment any non-200 status escapes.

**How to apply:** Use a factory closure (`let ok = || HttpResponse::Ok().json(...)`) instead of a single bound `ok` variable (HttpResponse is not Clone). Both the success path and any internal error paths call `ok()`. Log the internal error with `error!("{e}")` before returning `ok()`. The `(status = 500, ...)` entry must also be removed from the utoipa OpenAPI annotation.
