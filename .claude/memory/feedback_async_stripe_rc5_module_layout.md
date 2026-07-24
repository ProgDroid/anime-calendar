---
name: async-stripe 1.0.0-rc.5 modular crate type locations
description: Reference for where common Stripe types live across the modular rc.5 sub-crates — saves re-grepping the registry every time
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
**Rule:** async-stripe 1.0.0-rc.5 splits its types across modular sub-crates. The compiler's `consider importing this enum instead` hints aren't always enough — keep this map handy.

**Why:** rc.5 reorganized the type tree across `stripe-checkout`, `stripe-billing`, `stripe-core`, `stripe-shared`, `stripe-types`, and `stripe-webhook`. The same logical type lives in different places than older `stripe` crate versions, and rust-analyzer's auto-import sometimes picks the wrong one (e.g. `Expandable` exists in both `stripe_shared` and `stripe_types`, but only the `stripe_types` re-export is the canonical one).

**How to apply (type → crate map for rc.5):**

| Type | Crate |
|------|-------|
| `Client` | `stripe` (top-level) |
| `Expandable<T>` | `stripe_types` (NOT `stripe_shared`, despite the hint) |
| `Timestamp` | `stripe_types` |
| `CheckoutSession`, `CheckoutSessionId`, `CheckoutSessionMode` | `stripe_shared` |
| `Subscription`, `SubscriptionId`, `SubscriptionStatus`, `SubscriptionItem` | `stripe_shared` |
| `Invoice`, `InvoiceLineItem`, `InvoiceLineItemPeriod` | `stripe_shared` |
| `Customer`, `CustomerId` | `stripe_shared` |
| `Price` | `stripe_shared` |
| `CreateCheckoutSession`, `CreateCheckoutSessionLineItems`, `CreateCheckoutSessionSubscriptionData`, `CreateCheckoutSessionPaymentMethodCollection`, `RetrieveCheckoutSession` | `stripe_checkout::checkout_session` |
| `CreateCheckoutSessionSubscriptionData::trial_period_days` | **field**, not method (`Option<u32>`) — assign with `sd.trial_period_days = Some(14)`, NOT a builder call |
| `Webhook`, `Event`, `EventObject`, `WebhookError` | `stripe_webhook` |

**Crate-name collision gotcha:** the external crate `stripe_webhook` collides with any local module named `stripe_webhook`. Inside such a module, alias the import: `use ::stripe_webhook::{EventObject, Webhook};`

**Feature flags required for the typical set:**
```toml
async-stripe = "=1.0.0-rc.5"
async-stripe-checkout = { version = "=1.0.0-rc.5", features = ["checkout_session", "serialize", "deserialize"] }
async-stripe-billing = { version = "=1.0.0-rc.5", features = ["subscription", "subscription_item", "billing_portal_session", "serialize", "deserialize"] }
async-stripe-core = { version = "=1.0.0-rc.5", features = ["customer", "serialize", "deserialize"] }
async-stripe-shared = { version = "=1.0.0-rc.5", features = ["serialize", "deserialize"] }
async-stripe-types = { version = "=1.0.0-rc.5", features = ["serialize", "deserialize"] }
async-stripe-webhook = { version = "=1.0.0-rc.5", features = ["deserialize", "async-stripe-checkout", "async-stripe-billing", "async-stripe-core"] }
```

Resource feature flags (`subscription`, `customer`, `checkout_session`) are MANDATORY — without them the corresponding type modules don't compile.

**Subscription period boundaries live on items, not the subscription itself:** `sub.items.data[0].current_period_start` / `current_period_end`. The `Subscription` struct has `billing_cycle_anchor` and `trial_end` but not the per-period `current_period_*` fields directly.
