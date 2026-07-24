---
name: UiModal Teleport breaks wrapper.find() in vue-test-utils
description: Tests asserting on modal-internal DOM must query document.body, not the wrapper
type: feedback
originSessionId: 7a8d378a-90cb-41a0-b043-addec4ba2a44
---
`UiModal` (and `ConfirmModal` which wraps it) uses `<Teleport to="body">` to render the dialog outside the component tree. This breaks the standard `wrapper.find('[data-testid="...modal-inner..."]')` pattern because the teleported nodes are NOT children of the wrapper — they're attached directly to `document.body`.

**Why:** `wrapper.find()` walks the wrapper's component tree. Teleported content is rendered into a different DOM subtree at runtime. `vue-test-utils` doesn't follow Teleport portals.

**How to apply:**

1. **Mount with `attachTo: document.body`**:
   ```ts
   const wrapper = mount(Component, { attachTo: document.body, ... })
   ```
   This ensures the wrapper's own DOM is in the document so Teleport can resolve `to: "body"`. Without this, modal contents render into a detached body.

2. **Query teleported content via `document.body.querySelector`**, not the wrapper:
   ```ts
   // ❌ won't find the modal contents
   wrapper.find('[data-testid="cancel-btn"]')
   // ✅ finds it
   document.body.querySelector('[data-testid="cancel-btn"]')
   ```

3. **Click teleported buttons via the raw DOM node**:
   ```ts
   const cancel = document.body.querySelector('[data-testid="cancel-btn"]') as HTMLElement
   cancel.click()
   await flushPromises()
   ```

4. **Tear down with `wrapper.unmount()`** — Vue's Teleport correctly cleans up the body content on unmount, but only if you actually unmount (not just discard the wrapper).

This came up during Phase E1+E2 (toast/modal migration) and again during FU-4 — three test files needed updating: `ConfirmModal.spec.ts`, `MyCalendarsPage.spec.ts`, `errorScenarios.spec.ts`. Existing tests asserting on non-teleported parts of the components (the trigger button, the page chrome) work fine without changes.

Same gotcha applies to `UiToastHost` if/when tests need to assert on toast contents — toasts render via Teleport-equivalent reactive DOM in `document.body`.
