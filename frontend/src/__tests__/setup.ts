/**
 * Filter cosmetic intlify warnings emitted by vue-i18n 11 when `<i18n-t>` is
 * rendered inside a Composition-API parent. The warning fires from
 * `getComposer(..., useComponent=true)` because the composer doesn't have
 * vue-i18n's internal `InejctWithOptionSymbol` (only set via legacy options).
 * The component still resolves the translation via the global scope, so the
 * warning is purely cosmetic and unactionable from user code.
 *
 * See node_modules/vue-i18n/dist/vue-i18n.mjs `NOT_FOUND_PARENT_SCOPE`.
 */
const originalWarn = console.warn.bind(console)
console.warn = (...args: unknown[]) => {
  const first = args[0]
  if (typeof first === 'string' && first.includes('[intlify] Not found parent scope')) {
    return
  }
  originalWarn(...args)
}

// ── MockEventSource ────────────────────────────────────────────────────────
// Global mock used by usePresence tests. Assigned to global.EventSource so
// components can be mounted in jsdom without a real server connection.

export class MockEventSource {
  static lastInstance: MockEventSource
  static instances: MockEventSource[] = []
  url: string
  withCredentials: boolean
  closed = false
  private listeners: Record<string, Array<(e: MessageEvent) => void>> = {}
  onmessage: ((e: MessageEvent) => void) | null = null

  constructor(url: string, opts?: { withCredentials?: boolean }) {
    this.url = url
    this.withCredentials = opts?.withCredentials ?? false
    MockEventSource.lastInstance = this
    MockEventSource.instances.push(this)
  }

  close() {
    this.closed = true
  }

  emit(name: string, data: string) {
    const e = { data } as MessageEvent
    if (name === 'message' && this.onmessage) this.onmessage(e)
    ;(this.listeners[name] ?? []).forEach((l) => l(e))
  }

  addEventListener(name: string, listener: (e: MessageEvent) => void) {
    ;(this.listeners[name] ??= []).push(listener)
  }
}

;(global as unknown as { EventSource: typeof MockEventSource }).EventSource = MockEventSource
