/**
 * Stash + consume a co-editor invite token across an auth handoff (H-10).
 *
 * Why sessionStorage and not a `?redirect=/invite/{token}` URL query:
 *   - URL queries land in browser history, bookmarks, browser sync, extensions
 *     with tab-read permission, screenshots, and shared screenshares.
 *   - The token grants calendar access on possession alone, so any of those
 *     leak vectors is equivalent to an access leak.
 *   - sessionStorage is tab-scoped and never serialized into the URL bar, so
 *     none of the above vectors apply.
 *
 * The token is consumed at most once (the read clears the stash) so a stale
 * stash from a previous flow can't redirect a fresh login somewhere
 * unexpected.
 */

const KEY = 'pendingInviteToken'

export function stashInviteToken(token: string): void {
  try {
    sessionStorage.setItem(KEY, token)
  } catch {
    // sessionStorage can throw in private-browsing or quota-exceeded modes.
    // In those cases the user has to re-paste the invite URL; not worth
    // bubbling.
  }
}

export function consumeInviteRedirect(): string | null {
  let token: string | null = null
  try {
    token = sessionStorage.getItem(KEY)
    if (token !== null) sessionStorage.removeItem(KEY)
  } catch {
    return null
  }
  return token
}
