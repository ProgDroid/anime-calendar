import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

// Bypass the global mock from setup.ts — we want to exercise the real
// interceptor logic in this file.
vi.unmock('../config/api')
vi.unmock('@/config/api')

type LocationStub = { pathname: string; href: string; origin: string }

const setLocation = (pathname: string): LocationStub => {
  const stub: LocationStub = {
    pathname,
    href: `http://localhost${pathname}`,
    origin: 'http://localhost',
  }
  Object.defineProperty(window, 'location', {
    configurable: true,
    writable: true,
    value: stub,
  })
  return stub
}

const buildAxiosError = (status: number, config: Record<string, unknown> = {}) => {
  // Mimic an axios error shape so `axios.isAxiosError(err)` returns true.
  // axios.isAxiosError checks for `isAxiosError === true` on the object.
  const err: Record<string, unknown> = new Error('Request failed') as unknown as Record<
    string,
    unknown
  >
  err.isAxiosError = true
  err.response = { status }
  err.config = config
  err.toJSON = () => ({})
  return err
}

describe('api interceptor', () => {
  beforeEach(() => {
    vi.resetModules()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('passes through successful responses unchanged', async () => {
    setLocation('/my-calendars')
    const { default: api } = await import('@/config/api')

    // Re-derive the response handler the same way axios runs it: we reach
    // into the interceptor handlers directly. The response handler is the
    // 1st arg of `use()`.
    const handlers = (api.interceptors.response as unknown as {
      handlers: Array<{ fulfilled: (r: unknown) => unknown; rejected: (e: unknown) => unknown }>
    }).handlers
    const handler = handlers[handlers.length - 1]!

    const response = { status: 200, data: { ok: true } }
    expect(handler.fulfilled(response)).toEqual(response)
  })

  it('redirects to /login when refresh fails on a private route', async () => {
    const loc = setLocation('/my-calendars')
    const { default: api } = await import('@/config/api')

    const postSpy = vi.spyOn(api, 'post').mockRejectedValueOnce(new Error('refresh failed'))

    const handlers = (api.interceptors.response as unknown as {
      handlers: Array<{ fulfilled: (r: unknown) => unknown; rejected: (e: unknown) => Promise<unknown> }>
    }).handlers
    const handler = handlers[handlers.length - 1]!

    const config = { url: '/user' }
    const err = buildAxiosError(401, config)

    await expect(handler.rejected(err)).rejects.toBe(err)
    expect(postSpy).toHaveBeenCalledWith('/auth/refresh')
    expect(loc.href).toBe('/login')
    expect((config as { _retried?: boolean })._retried).toBe(true)
  })

  it('does NOT redirect on a public route when refresh fails', async () => {
    const loc = setLocation('/forgot-password')
    const initialHref = loc.href
    const { default: api } = await import('@/config/api')

    vi.spyOn(api, 'post').mockRejectedValueOnce(new Error('refresh failed'))

    const handlers = (api.interceptors.response as unknown as {
      handlers: Array<{ fulfilled: (r: unknown) => unknown; rejected: (e: unknown) => Promise<unknown> }>
    }).handlers
    const handler = handlers[handlers.length - 1]!

    const config = { url: '/user' }
    const err = buildAxiosError(401, config)

    await expect(handler.rejected(err)).rejects.toBe(err)
    expect(loc.href).toBe(initialHref)
    expect((config as { _retried?: boolean })._retried).toBe(true)
  })

  it('retries the original request and sets _retried when refresh succeeds', async () => {
    setLocation('/my-calendars')
    const { default: api } = await import('@/config/api')

    // Stub the axios adapter on this instance so the retry doesn't hit the
    // network. The first call is `api.post('/auth/refresh')` (refresh OK);
    // the second is `api(originalRequest)` (the retry).
    const adapter = vi
      .fn()
      .mockResolvedValueOnce({
        status: 200,
        data: {},
        headers: {},
        config: {},
        statusText: 'OK',
      })
      .mockResolvedValueOnce({
        status: 200,
        data: { retried: true },
        headers: {},
        config: {},
        statusText: 'OK',
      })
    api.defaults.adapter = adapter as never

    const handlers = (api.interceptors.response as unknown as {
      handlers: Array<{ fulfilled: (r: unknown) => unknown; rejected: (e: unknown) => Promise<unknown> }>
    }).handlers
    const handler = handlers[handlers.length - 1]!

    const config = { url: '/user', method: 'get', headers: {} }
    const err = buildAxiosError(401, config)

    const result = (await handler.rejected(err)) as { data: { retried: boolean } }
    expect((config as { _retried?: boolean })._retried).toBe(true)
    // Adapter called twice: once for refresh, once for retry of original.
    expect(adapter).toHaveBeenCalledTimes(2)
    expect(result.data.retried).toBe(true)
  })

  it('does not loop: a second 401 with _retried already set falls through', async () => {
    setLocation('/my-calendars')
    const { default: api } = await import('@/config/api')

    const postSpy = vi.spyOn(api, 'post')

    const handlers = (api.interceptors.response as unknown as {
      handlers: Array<{ fulfilled: (r: unknown) => unknown; rejected: (e: unknown) => Promise<unknown> }>
    }).handlers
    const handler = handlers[handlers.length - 1]!

    const config = { url: '/user', _retried: true }
    const err = buildAxiosError(401, config)

    await expect(handler.rejected(err)).rejects.toBe(err)
    expect(postSpy).not.toHaveBeenCalled()
  })
})

describe('isPublicRoute', () => {
  it('matches public route prefixes and rejects private paths', async () => {
    const { isPublicRoute } = await import('@/config/api')
    expect(isPublicRoute('/login')).toBe(true)
    expect(isPublicRoute('/register')).toBe(true)
    expect(isPublicRoute('/forgot-password')).toBe(true)
    expect(isPublicRoute('/reset-password')).toBe(true)
    expect(isPublicRoute('/verify-email')).toBe(true)
    expect(isPublicRoute('/verify-email/pending')).toBe(true)
    expect(isPublicRoute('/my-calendars')).toBe(false)
    expect(isPublicRoute('/account/profile')).toBe(false)
    expect(isPublicRoute('/calendar/123')).toBe(false)
  })
})
