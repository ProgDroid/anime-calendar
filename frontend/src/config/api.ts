import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  withCredentials: true,
})

/**
 * Returns an absolute URL for a backend path.
 * Used for subscription/export links that are pasted into external calendar clients.
 * Must be absolute so Google Calendar / Apple Calendar can fetch them.
 */
export const getApiUrl = (path: string): string => {
  return `${window.location.origin}/api${path}`
}

// Public route prefixes that must NOT be force-redirected to /login on 401.
// Kept in sync with `meta: { public: true }` routes in `router/index.ts`.
// We match here on `window.location.pathname` rather than importing the router
// because api.ts is imported by stores/services that the router itself loads —
// importing `@/router` here would create a circular module-init dependency.
// See ~/.claude/projects/.../feedback_axios_refresh_interceptor.md.
const PUBLIC_ROUTE_PREFIXES = [
  '/login',
  '/register',
  '/forgot-password',
  '/reset-password',
  '/verify-email', // covers /verify-email and /verify-email/pending
]

export const isPublicRoute = (pathname: string): boolean =>
  PUBLIC_ROUTE_PREFIXES.some(
    (prefix) => pathname === prefix || pathname.startsWith(`${prefix}/`) || pathname.startsWith(`${prefix}?`),
  )

// 401 interceptor: attempt a silent token refresh, then retry the original request
// once. On refresh failure redirect to /login so the user can re-authenticate —
// EXCEPT when the user is already on a public auth route (e.g. landed via a
// password-reset email link). In that case redirecting would dead-end them on
// /login and break the flow.
api.interceptors.response.use(
  (response) => response,
  async (error) => {
    const originalRequest = error.config as typeof error.config & { _retried?: boolean }
    if (
      axios.isAxiosError(error) &&
      error.response?.status === 401 &&
      !originalRequest._retried
    ) {
      originalRequest._retried = true
      try {
        await api.post('/auth/refresh')
        return api(originalRequest)
      } catch {
        if (!isPublicRoute(window.location.pathname)) {
          window.location.href = '/login'
        }
      }
    }
    return Promise.reject(error)
  },
)

export default api
