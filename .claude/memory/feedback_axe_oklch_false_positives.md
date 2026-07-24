---
name: axe-core 4.10 oklch parsing breaks color-contrast rule
description: axe-core's color-contrast on this codebase produces false positives because oklch()/oklab() resolution is broken; always re-verify via canvas + alpha-composite
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
axe-core 4.10's `color-contrast` rule misparses CSS `oklch()` and `oklab()` colors and returns wildly incorrect sRGB values. On 2026-05-01 the `/my-calendars` light/coral sweep flagged "Create New Calendar" at 3.85:1 — actual canvas-sampled ratio was ~6.0:1, well over AA.

**Why:** The design system here lives entirely on OKLCH tokens (`tokens.css`). Every `bg-accent-1`, `text-fg-1`, `bg-bg-2` etc. resolves to `oklch(...)` at runtime. axe doesn't reliably round-trip those into sRGB.

**How to apply:** When running an axe sweep against this app:
1. Trust axe for ARIA / structural / heading-order / landmark findings.
2. **Don't** trust raw `color-contrast` violations. Re-verify each one by:
   - Canvas-rendering both fg and bg via `ctx.fillStyle = c; ctx.fillRect(); getImageData()` — this gives the browser-resolved sRGB.
   - Alpha-composite the bg stack: walk up `parentElement`, accumulate transparent backgrounds (e.g. `bg-accent-1-soft` is 12 % alpha coral over `bg-bg-1`), and Porter-Duff over them onto the first opaque ancestor. Skipping this step turns every chip / soft surface into a false positive.
   - Compute WCAG ratio with relative-luminance formula on the resolved sRGB.
3. Reconstruct axe's targeted element with `target[target.length - 1]` — joining the array with `' '` builds a *descendant* selector and matches unrelated DOM elements.
4. Disable transitions globally (`*, *::before, *::after { transition-duration: 0s !important }`) and wait ≥400 ms after toggling `data-theme`/`data-accent` before measuring. The system has 240 ms `transition-all` on most surfaces; mid-fade frames produce phantom 3:1 ratios on segmented controls.

The verified probe + summarize scripts and per-route axe JSON outputs are saved under `.playwright-mcp/track-2-a11y-auth-sweep/`.
