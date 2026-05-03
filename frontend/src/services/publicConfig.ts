import api from '@/config/api'

export interface PublicConfig {
  googleClientId: string
}

interface PublicConfigResponse {
  google_client_id: string
}

let cached: PublicConfig | null = null

/**
 * Fetch the SPA's public bootstrap config from the backend.
 *
 * Called once during app bootstrap (in `main.ts`) before `app.mount`. The
 * resolved value is then read synchronously by components via
 * `getPublicConfig()`. Failure here is treated as a hard error — the SPA
 * cannot meaningfully start without these values, so the bootstrap promise
 * is rejected and the caller renders an error shell instead of mounting.
 *
 * Idempotent: subsequent calls return the cached config without re-fetching.
 */
export async function loadPublicConfig(): Promise<PublicConfig> {
  if (cached) return cached
  const { data } = await api.get<PublicConfigResponse>('/public-config')
  cached = { googleClientId: data.google_client_id }
  return cached
}

/**
 * Synchronous accessor for the loaded config. Throws if called before
 * `loadPublicConfig` has resolved — guarantees that consumers see a fully
 * populated config rather than an empty/undefined value that silently
 * breaks downstream behavior (e.g. an empty Google client_id renders a
 * non-functional sign-in button).
 */
export function getPublicConfig(): PublicConfig {
  if (!cached) {
    throw new Error('Public config accessed before bootstrap completed')
  }
  return cached
}

// Test-only escape hatch: allows specs that mock `@/services/publicConfig`
// to be bypassed in favour of priming the real cache directly. Not exported
// from a public surface — kept internal to this module.
export function __resetPublicConfigForTesting(): void {
  cached = null
}
