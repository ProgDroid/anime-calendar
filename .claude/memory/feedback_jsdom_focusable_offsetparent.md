---
name: jsdom + Teleport breaks offsetParent visibility filter
description: When testing focus management in components that use Teleport, do NOT filter focusable elements by `el.offsetParent !== null` — jsdom returns null for everything inside Teleported nodes
type: feedback
originSessionId: ae44063a-601c-45c4-b9ac-ac2c2751589f
---
When implementing a focus trap (or any focusable-element discovery) in a Vue component that uses `<Teleport to="body">` (UiModal, dropdowns, etc.), don't filter the focusable list by `el.offsetParent !== null`.

**Why:** jsdom doesn't compute layout, so `offsetParent` is `null` for nodes that don't have a CSS layout box — which includes all children of Teleported `<div>`s. Real DOM returns the nearest positioned ancestor instead. The filter that "works in browser to skip hidden elements" silently empties the focusable list in tests, making focus-trap specs fail without obvious cause.

**How to apply:** In `getFocusable()` style helpers, filter only on `disabled` (and hidden via `aria-hidden`/`hidden` attribute if you need it). Skip `offsetParent`. If you need true visibility checks, do them explicitly in `e2e` tests where layout is real.

Example fix that landed in `frontend/src/components/ui/UiModal.vue`:

```ts
// Bad — empty result inside Teleport in jsdom:
return Array.from(dialogEl.value.querySelectorAll(SEL))
  .filter(el => !el.hasAttribute('disabled') && el.offsetParent !== null)

// Good:
return Array.from(dialogEl.value.querySelectorAll(SEL))
  .filter(el => !el.hasAttribute('disabled'))
```

Symptom that points here: focus-trap unit tests show `expected first-button to be focused, received body`. Switch to integration via Playwright if you really need visibility filtering.
