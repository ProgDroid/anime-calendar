---
name: Design tokens — text variants (accent-1-text, danger-text)
description: Two extra tokens added 2026-05-01 for AA-compliant text contrast; use these for text on neutral backgrounds, keep --accent-1 / --danger for surfaces
type: project
originSessionId: ae44063a-601c-45c4-b9ac-ac2c2751589f
---
The Track 2 a11y remediation introduced two text-variant tokens distinct from their surface counterparts. Pick the right one when writing UI.

## --accent-1-text vs --accent-1

- **`--accent-1`** — accent surfaces: button bg, chip bg, hover ring, focus ring offset color. Brand-vibrant, lighter in dark theme (~72% L), darker in light theme (~52% L).
- **`--accent-1-text`** — accent text rendered on **neutral** background (bg-0 / bg-1, or `bg-accent-1-soft` tinted bg). Always passes 4.5:1 against bg-0 in both themes. Lighter in dark (~78% L), darker in light (~45% L).

**Tailwind utilities:** `bg-accent-1`, `text-accent-1-text`. Use `text-accent-1` only for hover affordances (e.g. `hover:text-accent-1` on a wordmark) where contrast against neutral bg isn't a body-text AA case.

## --danger-text vs --danger

- **`--danger`** — danger surfaces (button bg, badge bg, card border, danger card tint via `bg-danger/10`).
- **`--danger-text`** — error microcopy: form errors (UiInput error helper, login/register/reset errors, account tab errors, danger-zone eyebrow, toast danger variant). Higher contrast against neutral bg.

**Tailwind utilities:** `bg-danger`, `border-danger`, `text-danger-text`. Don't use bare `text-danger` for error text — that's the surface token and won't hit 4.5:1 in light theme.

## Per-hue lightness tuning

`tokens.css` defines `--accent-1-l` and `--accent-1-text-l` per theme as the default lightness, then per-`[data-accent]` overrides for the hues that need extra darkening in light theme (matcha → 44%, citron → 47%) because their perceived L* runs hot at the same numeric lightness. Add new accents by following the same pattern; verify with live axe before merging.

## When extending

- **Adding a new accent hue**: only adjust `--accent-h1` in the `[data-accent="..."]` selector. If contrast fails in light theme, add a `[data-theme='light'][data-accent="<name>"]` block overriding `--accent-1-l` / `--accent-1-text-l`. Verify all 5 accents × 2 themes via live axe at 375×812.
- **New error/warning microcopy**: reach for `text-danger-text` (or `text-warning-text` if/when added) over the bare surface tokens.
- **New accent-tinted chip / link / avatar**: text uses `text-accent-1-text`, bg uses `bg-accent-1-soft` (or `bg-accent-1/15` for ad-hoc tints).
