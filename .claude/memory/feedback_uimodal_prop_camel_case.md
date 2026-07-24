---
name: UiModal prop binding must be :ariaLabel (camelCase), not :aria-label
description: Vue 3 + vue-tsc strict mode does not always honor kebab→camel auto-conversion on component prop bindings; pass UiModal's ariaLabel as :ariaLabel
type: feedback
originSessionId: d141fb2f-b707-4ff0-8ba3-228ad77d407f
---
`UiModal.vue` declares `interface Props { open, closeOnScrim?, ariaLabel: string }`. Vue's runtime auto-converts kebab-case bindings to camelCase props, BUT vue-tsc with strict TS configuration sometimes flags `:aria-label="..."` as a missing-required-prop error on the component instance type.

**Why:** Vue's TS plugin generates per-component prop type signatures using camelCase keys. The kebab→camel coercion is a runtime convenience, not a type-system one. Strict-mode `vue-tsc --noEmit` checks the type as written.

**How to apply:**
- Always bind `:ariaLabel="..."` (camelCase) when passing to UiModal — and to any other in-house component using `defineProps<Props>()` with strict types.
- Sibling call sites already follow this convention: `UpgradeInterruptModal.vue`, `InviteEditorModal.vue`, `shared/ConfirmModal.vue`.
- HTML *attributes* like `aria-label` on a native `<button>`/`<input>` are unaffected; this rule applies only to component props.
- This regression slipped through `npm run test:unit` twice in this session (Phase 3 T2 + T6) because vitest does not run vue-tsc; it was caught by `npm run build`. See `feedback_typecheck_gate_separate_from_unit.md`.
