import { describe, it, expect, beforeEach } from 'vitest'
import { stashInviteToken, consumeInviteRedirect } from '../inviteRedirect'

describe('inviteRedirect (H-10)', () => {
  beforeEach(() => {
    sessionStorage.clear()
  })

  it('stashInviteToken writes the token under pendingInviteToken', () => {
    stashInviteToken('hex-deadbeef')
    expect(sessionStorage.getItem('pendingInviteToken')).toBe('hex-deadbeef')
  })

  it('consumeInviteRedirect returns the token and clears it', () => {
    sessionStorage.setItem('pendingInviteToken', 'one-shot')
    expect(consumeInviteRedirect()).toBe('one-shot')
    expect(sessionStorage.getItem('pendingInviteToken')).toBeNull()
  })

  it('consumeInviteRedirect returns null when no token is stashed', () => {
    expect(consumeInviteRedirect()).toBeNull()
  })

  it('consume is single-use: a second call returns null', () => {
    stashInviteToken('only-once')
    expect(consumeInviteRedirect()).toBe('only-once')
    expect(consumeInviteRedirect()).toBeNull()
  })

  it('overwriting a stash replaces the prior token', () => {
    stashInviteToken('first')
    stashInviteToken('second')
    expect(consumeInviteRedirect()).toBe('second')
  })
})
