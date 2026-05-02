import api from '@/config/api'

export type Tier = 'free' | 'paid'

export type BillingInterval = 'monthly' | 'annual'

export interface Entitlement {
  tier: Tier
  status: string | null
  current_period_end: string | null
  cancel_at_period_end: boolean
  trial_end: string | null
}

/**
 * Phase 2 of Track 4: kicks off a Stripe Checkout Session for the requested
 * billing interval. The server returns a Stripe-hosted URL we redirect to.
 *
 * No client-side caching — the server is authoritative on price ids and the
 * Checkout URL contains a single-use session token.
 */
export async function createCheckoutSession(
  interval: BillingInterval,
): Promise<{ url: string }> {
  const res = await api.post<{ url: string }>('/stripe/checkout', { interval })
  return res.data
}

/**
 * Read the current user's effective entitlement. Pass `sessionId` from the
 * /upgrade/success page so the server can pre-populate the local row before
 * the webhook arrives (Phase-2 stopgap, removed in Phase 3).
 */
export async function getMySubscription(sessionId?: string): Promise<Entitlement> {
  const res = await api.get<Entitlement>('/subscription/me', {
    params: sessionId ? { session_id: sessionId } : undefined,
  })
  return res.data
}

/**
 * Phase 4: ask the server to mint a Stripe Customer Portal session. The
 * server looks up our `stripe_customer_id` from JWT claims — we never pass
 * one in, which is defense in depth (no caller-supplied customer id to
 * tamper with). Caller redirects to `result.url` via `window.location.href`.
 * Stripe sends the user back to `/account/subscription` when they finish.
 */
export async function createPortalSession(): Promise<{ url: string }> {
  const res = await api.post<{ url: string }>('/stripe/portal')
  return res.data
}
