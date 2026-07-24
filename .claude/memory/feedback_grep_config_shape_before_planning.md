---
name: Verify config schema shape before drafting plans
description: Plans that specify TOML schemas (e.g. [stripe.prices]) without checking the existing struct shape produce stub instructions that can't be followed verbatim
type: feedback
originSessionId: be1596ff-ca01-4f16-80bf-1a49c8d8e222
---
When a plan stub specifies a config-file schema like:

```toml
[stripe.prices]
monthly = "price_..."
annual  = "price_..."
```

… and claims "no code changes — the existing flow already uses `config.stripe.prices.monthly`", **verify the schema shape against the actual `Config` struct before treating the plan as gospel**. The plan author may have inferred a nested layout that doesn't exist.

**Real example (2026-05-04 monetisation plan):** plan specified `[stripe.prices]`. Actual code (`server/src/config/server.rs:137`) reads flat `price_id_monthly` / `price_id_annual` directly under `[stripe]`. Following the plan literally would have meant:
- Restructure `[stripe]` in `config.toml.dist` (visible config shape change for users)
- Add a `prices: PricesConfig` substruct to the `Stripe` config + serde rename
- Change controller call sites from `stripe_config.price_id_monthly` to `stripe_config.prices.monthly`
- All "no code changes" — none of which were intended.

**Why:** The plan was written without grepping for the actual field names; it inferred from the suggested commit message. Plans drift from code in this way silently.

**How to apply:** When a plan asks for any config schema change:
1. `grep -rn "<config_field_name>" server/src/config/ server/src/controllers/` — confirm the existing schema.
2. If the plan's shape doesn't match, push back to the plan author or downgrade the change scope (e.g. just annotate existing keys instead of restructuring).
3. The "no code changes" claim is the tell — if a schema change is truly free, the plan probably already matches reality. If it doesn't match, the change has cascading effects.

This generalises the lesson in `feedback_verify_before_assuming_code_shape.md` to config files specifically.
