/**
 * Centralised client-side logger (L-15).
 *
 * All app `console.error`/`console.warn` calls route through here so there is a
 * single chokepoint to silence them in production (or later swap for a remote
 * sink). Gated on `import.meta.env.DEV`: dev builds log to the console; prod
 * builds stay quiet by default. This is the *only* place `console.*` is allowed
 * in `src/` — components and composables call `logger.*` instead.
 */
export const logger = {
  error(...args: unknown[]): void {
    if (import.meta.env.DEV) console.error(...args)
  },
  warn(...args: unknown[]): void {
    if (import.meta.env.DEV) console.warn(...args)
  },
}
