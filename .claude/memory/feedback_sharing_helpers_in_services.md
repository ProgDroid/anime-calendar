---
name: Feedback: Sharing cross-cutting helpers belong in services/sharing.rs
description: suspend_owner_sharing_in_tx / restore_owner_sharing_in_tx must live in services/, not controllers/ — avoids inverted dependency that breaks reconcile.
type: feedback
originSessionId: 5f88ca2f-c4a5-4cba-99c3-92cedfbba71c
---
Cross-cutting sharing helpers (functions called by both a controller and a service) must live in `server/src/services/sharing.rs`, not in `server/src/controllers/stripe_webhook.rs`.

**Why:** The spec says "Modify: controllers/stripe_webhook.rs" so implementers naturally add `suspend_owner_sharing_in_tx` there. But `services/reconcile.rs` also needs this function. A service importing from a controller is an inverted dependency — controllers call services, not the reverse. This pattern caused 2 correction cycles in Phase 4.

**How to apply:** Whenever a helper is needed by both a controller and a service (e.g., Stripe webhook + reconcile loop), put it in a service module. If no suitable service exists, create `services/sharing.rs`. Import it in the controller via `use crate::services::sharing::...`. The implementer brief must name the target module explicitly, not just say "inside the webhook handler".
