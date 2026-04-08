# Dev Setup Note: rolldown-vite

## What It Is

The frontend uses `rolldown-vite` (aliased as `vite` in `package.json`) rather than standard Vite:

```json
"vite": "npm:rolldown-vite@latest"
```

rolldown-vite is a Vite fork that replaces the Rollup bundler with [Rolldown](https://rolldown.rs/), a Rollup-compatible bundler written in Rust. It is being developed by the Vite core team as the future production bundler for Vite.

## Benefits

- Significantly faster production builds (Rust vs JavaScript bundler)
- Dev server behavior is identical to standard Vite
- API-compatible with standard Vite plugins

## Caveats

- It is pre-stable — some Rollup/Vite plugins may have edge-case incompatibilities
- If a Vite plugin behaves unexpectedly, check whether it works with standard Vite first to isolate whether rolldown-vite is the cause
- `npm:rolldown-vite@latest` always resolves to the latest release; pin to a specific version if builds become unstable

## Compatibility

The following plugins are confirmed working in this project:
- `@vitejs/plugin-vue`
- `@tailwindcss/vite`
- `vite-plugin-vue-devtools`
