---
name: actix-governor path-based exemption via custom KeyExtractor
description: To exempt a specific path from rate-limiting, write a custom KeyExtractor that maps the path to a sentinel-key in whitelisted_keys; don't try to scope-split the App
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
**Rule:** When you need to exempt a specific path from `actix-governor` rate-limiting, implement a custom `KeyExtractor` that returns a constant sentinel-key for that path and lists that key in `whitelisted_keys()`. Avoid restructuring the `App::new()` into nested scopes just to wrap Governor selectively.

**Why:** `actix-governor` 0.7's `whitelisted_keys()` works on key *values* (e.g. IP addresses), not paths. The framework's "right" answer for path exemption is a custom extractor — it's a 30-line implementation that preserves the default per-peer-IP behavior for everything else, while a scope-based exemption would require splitting registration into multiple sub-scopes and reasoning about middleware ordering across them.

This came up wiring the Stripe webhook in Phase 3 of Track 4 (`server/src/server.rs::WebhookExemptKeyExtractor`). Stripe sends bursts on retries; rate-limiting the webhook would trip its endpoint-disabled heuristic.

**How to apply:**
- Define a `#[derive(Clone, Copy, Debug)]` struct (zero-sized — no state needed for path-based dispatch).
- `impl KeyExtractor` with `type Key = IpAddr` (matches the upstream `PeerIpKeyExtractor`).
- In `extract`: check `req.path()` first; if it matches the exempt path, return `Ok(SENTINEL_IP)`. Otherwise fall back to the default per-peer-IP logic (including the IPv6 /56-prefix bucketing — copy that verbatim from `PeerIpKeyExtractor` source).
- Override `whitelisted_keys()` to return `vec![SENTINEL_IP]`.
- Override `exceed_rate_limit_response` to keep error bodies as JSON (matches our `Error::*` JSON convention from CLAUDE.md).
- Wire via `GovernorConfigBuilder::default().key_extractor(YourExtractor)`.
