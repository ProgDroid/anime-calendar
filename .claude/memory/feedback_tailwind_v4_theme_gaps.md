---
name: Tailwind v4 @theme has gaps vs tokens.css; font-display includes font-size
description: Common pitfalls when using OKLCH tokens via Tailwind v4's @theme block
type: feedback
originSessionId: 7a8d378a-90cb-41a0-b043-addec4ba2a44
---
Two related gotchas when wiring `frontend/src/assets/tokens.css` CSS variables to Tailwind v4 utilities via the `@theme` block in `main.css`.

## Not every token in tokens.css is exposed as a utility

`tokens.css` defines a richer palette than what's reachable via Tailwind class names. Notably:
- `--accent-2-soft` exists as a CSS variable but is **not** mapped through `@theme`, so `bg-accent-2-soft` / `text-accent-2-soft` won't compile to anything.
- Same for `--shadow-lg` (used inline as `box-shadow: var(--shadow-lg)` rather than as a Tailwind utility).
- `--bg-inset` is exposed in some setups but not all.

**How to apply:**
- Before using a `bg-foo` / `text-foo` / `ring-foo` class, grep `frontend/src/assets/main.css` for the `@theme` block and confirm the corresponding `--color-foo` (or `--shadow-foo`, etc.) is defined there.
- Fallbacks when the utility is missing:
  - For accent-soft surfaces, use `bg-accent-1/15` (alpha-blended primary accent). Visually similar to `accent-2-soft`.
  - For inline gradients, use `style="background: …var(--accent-2-soft)…"` directly. The CSS variable IS in scope at runtime even though no Tailwind utility exists.
- If you find yourself using a token consistently, add it to `@theme` rather than alpha-blend forever. Mention the addition in the commit body.

## `.font-display` is a CSS shorthand, not just font-family

`.font-display` in `tokens.css` uses the `font:` shorthand which sets ALL font properties — family, size, weight, line-height, etc. So `class="font-display text-base"` doesn't actually shrink the text — the shorthand wins.

**How to apply:**
- When you want only the display **font family** (Instrument Serif) without forcing the display size, use `style="font-family: var(--font-display)"` directly.
- Use the `font-display` class only when you also want the full display typography preset (e.g. for a hero numeral or a wordmark flourish at its natural size).
- This came up when the topbar wordmark needed Instrument Serif italic on "Calendar" without inheriting the 56px display size.

## Icon SFCs render at intrinsic 24×24 with no size prop

Icons under `frontend/src/components/ui/icons/` are SFCs with hardcoded `width="24" height="24"`. They accept `ariaLabel` only — no `size`/`s` prop.

**How to apply:**
- Resize via Tailwind arbitrary variants on the wrapper: `class="[&_svg]:w-3.5 [&_svg]:h-3.5"` (= 14px).
- CSS dimensions override the SVG's intrinsic width/height attributes.
- For 14px (matches design's `Icon.X s={14}` calls): `[&_svg]:w-3.5 [&_svg]:h-3.5`.
- For 22px: `[&_svg]:w-[22px] [&_svg]:h-[22px]`.
- DON'T modify the icon SFCs to accept a size prop — they're ports of the design handoff `foundations.jsx` and the per-call wrapper class is the established pattern in this codebase.
