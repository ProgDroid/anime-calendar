---
name: Co-editor Phase 1 complete (2026-05-06)
description: Invitation lifecycle (send/accept/decline/revoke/resend/preview) shipped. 9 routes live, advisory-locked cap, per-user rate limit, anti-enumeration. Phase 2 (editor mutations + Members tab) is next.
type: project
originSessionId: 6e32f781-1240-465d-af04-4ad77bc7dd15
---
Co-editor Phase 1 (invitation lifecycle) shipped in commit `d5c5b17` on
2026-05-06. The status table in
`docs/superpowers/plans/2026-05-05-co-editor-sharing.md` is up to date.

**Why:** Phase 0 laid the schema + authz spine; Phase 1 is the first
user-visible slice — Pro owners can invite by email, invitees can preview /
accept / decline via token URLs, owners can revoke / resend / list members.
No editor mutation endpoints yet (Phase 2) and no live sync (Phase 3).

**How to apply:** When Phase 2 work starts, use these as known-good
plumbing — do not re-implement:
- `services::invitation_service::InvitationService` — orchestration layer.
- `services::email_validation::{validate_email, check_not_self}`.
- `EmailService::{send_invitation, send_editor_restored}` (plain-text body,
  smtp.host="" WARN-log fallback).
- `Error::Conflict { reason: &'static str }` (409) + `Error::TooManyRequests` (429).
- `CalendarInvitationMapper::count_for_inviter_since(_in_tx)`.
- 9 routes registered under `sharing` OpenAPI tag in `controllers/sharing.rs`
  + `server.rs` `app_data` block; DI block in `main.rs` constructs
  `InvitationService` with a dedicated `invitation_pool`.

**Convention divergences baked into the code (carry forward to Phase 2+):**
- Tokens use deterministic SHA-256 (`auth::generate_random_token` +
  `auth::hash_token`), NOT argon2id + lookup-column. Task 1.5.5 was dropped
  entirely. 256-bit token entropy makes brute force infeasible; matches the
  existing password_reset / email_verification convention.
- No standalone `InvitationToken` service struct.
- Per-user rate limit lives **inside** `InvitationService::send` via the new
  `count_for_inviter_since` mapper query, NOT a custom actix-governor
  `KeyExtractor`. Reason: this codebase has no global auth middleware, so
  reading `Claims` from request extensions in an extractor isn't viable.
- `EmailService` got `send_invitation` + `send_editor_restored` methods
  directly — no `SharingEmail<E: EmailSender>` wrapper.
- Anti-enumeration: every token-failure path collapses to
  `Error::InvalidRequest` (400). Preview, accept, decline all share this
  shape so callers cannot probe valid tokens.
- `username` is used as `owner_display` in invitation emails and previews;
  the preview wire shape carries `owner_avatar: None` for forward
  compatibility (column doesn't exist yet on `users`).

**Verification baseline:** 288 server tests / 412 frontend tests; pedantic
+ nursery clippy clean; redocly lint at 44 errors / 4 warnings (Phase 0
baseline, all pre-existing on existing endpoints — Phase 1 introduced 0
new findings).
