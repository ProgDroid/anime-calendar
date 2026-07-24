---
name: ref-on-component focus trap (Vue 3)
description: Calling .focus() through a template ref placed on a Vue component (not a raw DOM element) silently no-ops. The component must defineExpose a focus wrapper for the chain to actually fire.
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
When a template ref is placed on a Vue component (e.g. `<UiInput ref="searchInputRef" />`), `searchInputRef.value` is the component's exposed proxy — NOT the underlying `<input>` element. Calling `.focus()` on that proxy is a silent no-op unless the component has explicitly exposed a `focus` method via `defineExpose`. There is no runtime warning; TypeScript types it as `HTMLInputElement | null` if you ask, and the call doesn't throw.

**Why:** Track 3 Task 11's FAB-jump-to-search feature looked correct in the code, all tests passed, and the bug was only caught in code review. The focus chain `searchPanelRef.value?.searchInputRef.value?.focus()` traversed three component refs, every one of which was a component instance, none of which exposed `.focus()`.

**How to apply:**
1. When designing a UI primitive that wraps an interactive element (input, button, etc.), add `defineExpose({ focus: () => innerEl.value?.focus() })` so callers can drive focus through the wrapper. Do this proactively on `UiInput`, `UiTextarea`, `UiButton` etc.
2. When parent components forward focus through nested wrappers, expose a `focus()` wrapper at every level — never expose a raw ref expecting the caller to traverse.
3. **Always pair focus features with a `document.activeElement` test assertion** — the test must mount with `attachTo: document.body` and query the inner DOM element (the `data-testid` is usually on the wrapper div, the `<input>` is one level deeper).
4. If you see a focus-trap-like bug in a code review, suspect this pattern first.
