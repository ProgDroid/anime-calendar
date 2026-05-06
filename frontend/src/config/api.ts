import axios from 'axios'
import { useUpgradeInterrupt, type UpgradeReason } from '@/composables/useUpgradeInterrupt'

const api = axios.create({
  baseURL: '/api',
  withCredentials: true,
})

/**
 * Body shape of a 402 PaymentRequired response from the backend
 * (`Error::PaymentRequired` in `server/src/error.rs`):
 *
 * ```json
 * { "error": "upgrade_required", "required_tier": "paid", "reason": "cap_calendars" }
 * ```
 *
 * The `reason` field is optional — the backend's accent-enforcement path
 * omits it. The interceptor below defaults to `pro_accent` in that case.
 */
interface PaymentRequiredBody {
  error?: string
  required_tier?: string
  reason?: string
}

const VALID_REASONS: ReadonlySet<UpgradeReason> = new Set([
  'cap_calendars',
  'cap_shows',
  'pro_accent',
  'share_calendar',
])

function coerceReason(raw: unknown): UpgradeReason {
  return typeof raw === 'string' && VALID_REASONS.has(raw as UpgradeReason)
    ? (raw as UpgradeReason)
    : 'pro_accent'
}

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

// 402 interceptor: when the backend returns PaymentRequired with
// `required_tier: "paid"`, surface the upgrade modal globally. The reason
// code routes the modal copy. Errors are still re-thrown so call sites can
// render their own inline state if they care (e.g. cap-aware editor flows
// that already disable the trigger button optimistically).
api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (axios.isAxiosError(error) && error.response?.status === 402) {
      const body = (error.response.data ?? {}) as PaymentRequiredBody
      if (body.required_tier === 'paid') {
        const { openUpgradeModal } = useUpgradeInterrupt()
        openUpgradeModal(coerceReason(body.reason))
      }
    }
    return Promise.reject(error)
  },
)

export default api
