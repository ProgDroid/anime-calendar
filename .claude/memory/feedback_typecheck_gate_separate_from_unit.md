---
name: npm run build is a separate gate from npm run test:unit
description: Vitest does not invoke vue-tsc; component-prop or import-type changes must be verified with npm run build before declaring DONE
type: feedback
originSessionId: d141fb2f-b707-4ff0-8ba3-228ad77d407f
---
`npm run test:unit` runs Vitest only. `npm run build` runs `vue-tsc --noEmit && rolldown-vite build`. Vue templates can compile + render correctly at runtime while vue-tsc rejects them in strict mode.

**Symptoms that hide behind passing unit tests:**
- Component prop name mismatches (kebab vs camel — see `feedback_uimodal_prop_camel_case.md`)
- Missing required props masked by Vue's runtime warning (which is a console message, not a test failure)
- `as any` / `as unknown as ...` casts that get tightened by a dependency upgrade
- Missing imports of types only used in template type narrowing

**Why:** Phase 3 T2 (DangerZoneTab + UiModal) merged with `npm run test:unit` green but `npm run build` failed. The regression sat through 5 other commits before T6's pre-commit verification caught it. Two-cycle bug, ~10 min of debug.

**How to apply:**
- Implementer briefs that touch any `.vue` template MUST list `npm run build` as an explicit acceptance gate, not just `npm run test:unit`.
- Spec/quality reviewers checking Vue work MUST run `npm run build` themselves; do not rely on the unit-test pass.
- If shipping a multi-task branch, run `npm run build` at every commit boundary, not just at the final cumulative review.
- `npm run lint` is *also* a separate gate — it does not run vue-tsc; it runs oxlint + eslint. All three must pass.
