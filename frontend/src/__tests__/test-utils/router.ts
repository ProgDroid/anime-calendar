import { createMemoryHistory, createRouter, type Router } from 'vue-router'
import { defineComponent, h } from 'vue'

const stub = defineComponent({ render: () => h('div') })

/**
 * All public routes that mobile-smoke and other spec files navigate to or
 * render `<RouterLink>` to. Centralized so a new `<RouterLink to="/foo">`
 * in a page only requires a single update here, not in every spec router.
 *
 * Pages mounted directly in tests don't need their own route registered
 * unless something `router.push`es to them.
 */
// Mirrors the public paths declared in `src/router/index.ts`. Keep in sync
// when adding a new top-level route — otherwise specs that don't register
// it will warn "No match found for location" on RouterLink resolution.
const PUBLIC_ROUTES = [
  '/',
  '/login',
  '/register',
  '/forgot-password',
  '/reset-password',
  '/verify-email',
  '/verify-email/pending',
  '/my-calendars',
  '/account',
  '/account/profile',
  '/account/preferences',
  '/account/subscription',
  '/account/password',
  '/account/danger',
  '/upgrade',
  '/upgrade/success',
  '/upgrade/canceled',
  '/calendar/:id',
] as const

/**
 * Build a memory-history router for spec tests with all public routes
 * pre-registered. Avoids "No match found for location" warnings from
 * `<RouterLink>` resolution and from initial-route resolution on memory
 * histories that haven't been pushed yet.
 *
 * Tests that need a custom route (e.g. mounting a view via RouterView)
 * should pass `extraRoutes` rather than rebuild the list.
 */
export function makeSmokeRouter(
  initialPath: string = '/',
  extraRoutes: { path: string; component?: unknown }[] = [],
): Router {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      ...PUBLIC_ROUTES.map((path) => ({ path, component: stub })),
      ...extraRoutes.map((r) => ({ path: r.path, component: (r.component ?? stub) as never })),
    ],
  })
  // Push synchronously so memory history isn't sitting at the default
  // unresolved location when the component mounts.
  void router.push(initialPath)
  return router
}
