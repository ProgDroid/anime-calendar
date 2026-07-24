---
name: Tailwind v4 @theme + var() chain works at runtime, but verify with hard reload
description: Per-[data-theme] token overrides DO propagate through @theme to utility classes via var() — but Vite HMR can leave stale computed values around; always hard-reload before measuring contrast/colors
type: feedback
originSessionId: ae44063a-601c-45c4-b9ac-ac2c2751589f
---
Tailwind v4's `@theme` block generates utility CSS that references `var(--color-*)` at runtime. So defining `--color-accent-1: var(--accent-1)` in `@theme` and then overriding `--accent-1` inside a `[data-theme='light']` selector DOES work — utilities like `bg-accent-1` pick up the new value via the var() chain.

**Why:** Tailwind v4 doesn't resolve var() references at build time. The generated rule is literally `.bg-accent-1 { background-color: var(--color-accent-1); }`, and `--color-accent-1` itself is `var(--accent-1)` at the `@theme` level. Cascade does the rest at runtime.

**How to apply when verifying:**
- Don't trust live `getComputedStyle()` readings after editing `tokens.css` while the dev server is running. **Hard-reload the page** (or click "reload page" in Playwright after the file edit) before measuring.
- Vite HMR for CSS-token-only changes is sometimes flaky — the page may report old computed values for a few seconds after HMR claims a successful update.
- Symptom that pointed here: contrast sweep across all 5 accents passed; then a single re-run after a token edit reported coral as still failing while iris/matcha/sakura/citron passed. The bg was reading `oklab(0.72 ...)` (dark-theme value) for ~30s after HMR. Fresh page navigation showed the correct light-theme `oklch(0.52 ...)` and zero violations.

Pattern that's known to work for theming via `@theme`:
```css
:root {
  --accent-1-l: 72%; /* default per theme */
}
[data-theme='light'] { --accent-1-l: 52%; }
[data-theme='light'] {
  --accent-1: oklch(var(--accent-1-l) 0.20 var(--accent-h1));
}
@theme {
  --color-accent-1: var(--accent-1);
}
```
