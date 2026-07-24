---
name: feedback_public_config_extension
description: "Extending /public-config — update loadPublicConfig mapping AND the e2e stub (bootstrap hard-fails), and interpolate display from the same source as enforcement"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: ab83a158-ffc2-4fb3-a1dd-2857b860d545
---

When adding a field to the `/public-config` surface (the runtime SPA bootstrap config — backend `PublicConfig` in `controllers/public_config.rs`, frontend `services/publicConfig.ts`):

1. **Update three places or it breaks:** the backend struct + `server.rs` construction, the `loadPublicConfig` response→camelCase mapping, **and** the e2e stub in `frontend/e2e/fixtures.ts`. The SPA *hard-fails its bootstrap* (refuses to mount) if `loadPublicConfig` can't read an expected field, so an e2e stub that omits a newly-added field turns the **entire** e2e suite red. The fixture's own comment warns about rejection; schema *additions* are the subtler trap.

2. **Module-const-from-config with fallback** is the de-hardcode pattern (M-10/F2-28d): `const L = (()=>{ try { return getPublicConfig().limits } catch { return <backend defaults> } })(); export const FREE_SHOW_CAP = L.freeShowCap`. Works because the store is lazy-imported *after* `loadPublicConfig()` resolves in `main.ts`; the `catch` covers unit tests, which never bootstrap the config cache.

3. **Display must share its source with enforcement.** When interpolating a limit into UI copy (i18n `{count}`), pull the number from the *same* config-derived constant the enforcement computeds use. Interpolating display from config while enforcement stays hardcoded creates display-vs-enforcement drift that is *worse* than a hardcoded string (UI says 30, the cap blocks at 25).

**Why:** these three coupling points are invisible until a gate goes red or a user sees a wrong number. **How to apply:** treat `/public-config` changes as a 3-file checklist + the single-source rule. Related: [[feedback_vite_docker_env_vars]], [[feedback_tier_gated_apply_pattern]], [[feedback_i18n_t_named_slots]].
