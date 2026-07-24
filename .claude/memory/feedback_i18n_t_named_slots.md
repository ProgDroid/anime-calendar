---
name: vue-i18n `<i18n-t>` named slots for inline markup
description: Use `<i18n-t keypath="...">` with named slots to mix Vue template fragments into translation strings without HTML in the locale JSON. Tests need a helper to reconstruct rendered output.
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
When a translation needs inline markup (italic flourish, link, `<strong>` etc.), don't put the tags in the locale string and don't `v-html` it. Use vue-i18n's component interpolation:

```json
// en.json
{
  "pricing": {
    "heading": "More seasons, more colors, more {emphasis}.",
    "headingEmphasis": "control"
  }
}
```

```vue
<i18n-t keypath="pricing.heading" tag="h1" class="font-display text-4xl">
  <template #emphasis>
    <i>{{ t('pricing.headingEmphasis') }}</i>
  </template>
</i18n-t>
```

The `{emphasis}` placeholder in the source string maps to the `#emphasis` named slot. Translators only deal with text + a placeholder — never with markup that could break across locales.

**Why:**
- Translators don't need HTML literacy and can't accidentally break the markup.
- No `v-html`, so no XSS risk introduced through the i18n surface.
- Different locales can position the emphasis word differently (e.g., word order swaps in pt-br, ja, etc.) without coordinating with the template.
- Survives lint rules that forbid `v-html`.

**How to apply:**
- Use named slots, not positional ones — `{0}`, `{1}` is harder to maintain across locales.
- Pair the keypath with a sibling key for the slot's text content (`headingEmphasis` next to `heading`). Keeps the locale-parity test happy and lets translators see them together.
- The `<i18n-t>` component is auto-registered when you install vue-i18n via `app.use(i18n)`. No import needed in the template.
- vue-i18n logs `[intlify] Not found parent scope. use the global scope.` in tests when the component is mounted without a parent i18n composition scope. It's a benign warning, not a failure — `mount(...)` with `global.plugins: [i18n]` works fine.

**Testing pitfall:** rendered text won't contain the literal locale string `"...more {emphasis}."` because vue-i18n interpolates at render time. Tests asserting `wrapper.text()` need to reconstruct what the user actually sees:

```ts
function renderedHeading(messages) {
  const lead = messages.pricing.heading.split('{emphasis}')[0] ?? ''
  return (lead + messages.pricing.headingEmphasis).trim()
}
expect(wrapper.text()).toContain(renderedHeading(enMessages))
```

This is robust to copy changes — only break if the placeholder name changes.

**Anti-patterns:**
- Putting `<i>...</i>` directly in the JSON value and using `v-html` — fragile, XSS-adjacent, lint warns.
- Splitting the heading into 3 sibling keys (`headingPart1`, `headingPart2`, `headingPart3`) — translator-hostile (can't see the whole sentence) and locale-fragile (word order assumed).
- Using `{0}` / `{1}` positional slots when there's only one inline element — named is clearer at the call site.
