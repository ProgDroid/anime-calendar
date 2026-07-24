---
name: SHA-256 over argon2id for high-entropy random tokens
description: When storing 256-bit OS-RNG tokens (invitation, password reset, email verify), use deterministic SHA-256, not argon2id. Argon2id only matters for low-entropy secrets (passwords).
type: feedback
originSessionId: 6e32f781-1240-465d-af04-4ad77bc7dd15
---
For server-minted secret tokens carrying 256 bits of `rand::rng()` entropy
(e.g. invitation tokens, password-reset tokens, email-verification tokens),
store them as a **deterministic SHA-256 hash** of the raw token, NOT
argon2id. Use `auth::generate_random_token` + `auth::hash_token` in
`server/src/services/auth.rs` — both already exist and are the codebase
convention.

**Why:** Argon2id is a slow / memory-hard hash specifically designed to
defeat brute-force on **low-entropy** secrets (passwords). For 32 random
bytes from the OS RNG, brute force is mathematically infeasible
regardless of hash speed — 2^256 is unreachable even at extreme hashing
rates. SHA-256 gives the same one-way property AND enables direct index
lookup (the database does the constant-time comparison via the index).
The plan for the co-editor invitation flow originally proposed argon2id
+ a separate SHA-256 lookup column; that's pure ceremony for this threat
model. Existing `password_reset` and `email_verification` controllers
already use the simpler pattern — the invitation lifecycle adopted it on
2026-05-06.

**How to apply:** When a plan or spec calls for argon2id on a token:
1. Check the entropy source. If it's `rand::rng()` filling ≥ 16 bytes, the
   token is high-entropy — use SHA-256.
2. If it's a user-supplied secret (password) or a low-entropy code (PIN,
   recovery code under 64 bits), keep argon2id.
3. Verify against the existing `auth::hash_token` / `auth::generate_random_token`
   helpers before rolling new infra. The Item / Mapper / column already
   exists for the simpler pattern in `password_reset_tokens`,
   `email_verifications`, and `calendar_invitations.token_hash`.
4. Always pair with: short expiry (`expires_at > NOW()` in lookup),
   single-use (status flip on accept), and no logging of the raw token.
   These are what actually defend against compromise — not the hash
   algorithm.
