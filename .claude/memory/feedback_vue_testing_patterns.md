---
name: Vue testing patterns — router guards and form interaction
description: Selectors and techniques learned writing Vue Test Utils specs; avoids class-selector collisions, router guard traps, and form interaction edge cases
type: feedback
originSessionId: 09480f2a-20ee-482c-9ba8-4bfed05009a3
---
## Class selector collisions — use data-testid for interactive elements

`.find('.some-class')` returns the **first DOM match**, so if you add a new element that shares the same CSS classes, existing tests silently start clicking the wrong element.

The toggle button in `LoginPage.vue` uses `class="link link-primary"`. Adding the "Forgot password?" `router-link` (also `link link-primary`) to the form caused `.find('.link.link-primary')` to hit the wrong element in login mode.

Fix: add `data-testid="toggle-mode"` to the toggle button; update tests to use `'[data-testid="toggle-mode"]'`.

**Why:** Class selectors encode visual intent, not element identity. Any style change or new component can break them. `data-testid` is stable and self-documenting.

**How to apply:** Whenever writing a test that clicks a button or link by CSS class, ask whether another element on the same page could share that class. If yes, add `data-testid` to the element instead.

---

## Router guard testing

Don't import the real `router/index.ts` — it uses `createWebHistory()` which doesn't work in jsdom and has lazy-loaded components that complicate setup. Instead:
1. Use `createMemoryHistory()` for the test router
2. Copy the `beforeEach` guard logic verbatim into the test file
3. Mock `useAuthStore` and `useUserSettingsStore` via `vi.mock()`
4. Assert on `router.currentRoute.value.path` after `await router.push()`

**Why:** The real router's components don't need to render for guard tests; only the `beforeEach` logic matters.

**How to apply:** When testing navigation guard behaviour, create a test-only router that mirrors the real guard rather than mounting the full app.

---

## CalendarSettingsForm — no `<form>` tag

`CalendarSettingsForm.vue` uses `div.form-control` elements, not a `<form>` element. The save button is:
```html
<button data-testid="submit-btn" @click="emit('submit')">
```

Use `wrapper.find('[data-testid="submit-btn"]').trigger('click')` — `wrapper.find('form')` returns an empty DOMWrapper.

**Why:** The form emits a custom event to CalendarPage which calls `submitCalendar`. There is no form submit handler.

---

## ConfirmModal selectors

From `src/components/shared/ConfirmModal.vue`:
- Modal container: `[data-testid="modal-box"]`
- Cancel button: `[data-testid="cancel-btn"]`
- Confirm button: `[data-testid="confirm-btn"]`

Don't use `[data-testid="modal-confirm"]` — that attribute doesn't exist.

---

## TypeScript strict: array index access in VTU tests

`wrapper.findAll('selector')[0]` returns `T | undefined` under `noUncheckedIndexedAccess`. Use `[0]!` non-null assertion when you know the element exists (verified by test preconditions).

Same applies to `wrapper.findAll('input')[1]!.setValue(...)` and `forms[forms.length - 1]!.trigger(...)`.

**Why:** Vue Test Utils + TS strict catches these at `vue-tsc --build` time even though vitest runs fine (it skips type-checking).

**How to apply:** Always add `!` after indexed VTU array access in test assertions.

---

## HTMLInputElement cast for `.value` access

`wrapper.findAll('input[type="radio"]').map(r => r.element.value)` fails: `element` is typed as `Element`, not `HTMLInputElement`. Cast: `(r.element as HTMLInputElement).value`.

---

## Pinia mock partial objects — `as unknown as ReturnType<typeof useStore>`

Partial mock objects passed to `vi.mocked(useStore).mockReturnValue({...})` must be cast with double-assertion:
```ts
} as unknown as ReturnType<typeof useStore>)
```
`as ReturnType<typeof useStore>` alone is rejected because Pinia stores include `$state`, `$patch`, `$subscribe`, etc. that partial mocks lack. The `unknown` intermediate breaks the overlap check.

**Why:** Single `as T` requires overlap between source and target. `as unknown as T` is the explicit "I know better" escape hatch — cleaner than `as any`.

---

## `defineProps<...>()` — don't assign when only used in template

In `<script setup>`, `const props = defineProps<{...}>()` triggers `no-unused-vars` when the script body never references `props.xxx` directly. Template accesses props without the `props.` prefix automatically. Just call `defineProps<{...}>()` without assignment.

---

## `vue/multi-word-component-names` — use `defineOptions` to avoid renaming files

When a `.vue` file has a single-word name (`Toast.vue`, `Register.vue`) and renaming would break imports, add inside `<script setup>`:
```ts
defineOptions({ name: 'AppToast' })
```
The ESLint rule checks the explicit `name` option first; the filename fallback is not used when `name` is set.

---

## CalendarPage save — items must be non-empty

`submitCalendar()` has an early return guard:
```js
if (itemsInCalendar.value.length === 0) {
  calendarError.value = t('calendar.noItemsSelected')
  return // never reaches api.put
}
```

For tests that need to hit the PUT call, mock the GET response with at least one item. Minimum viable item shape: `{ id, title, image, episodes, status, next_airing_episode, schedule }`.
