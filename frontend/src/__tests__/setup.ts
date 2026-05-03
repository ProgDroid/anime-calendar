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
