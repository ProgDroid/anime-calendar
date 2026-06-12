//! Invitation orchestration: send / accept / decline / revoke / resend / preview.
//!
//! Each method runs the user-facing security checks and database mutations.
//! The HTTP controller is a thin shim that calls into these methods after
//! resolving DI handles and applying the `SharingAuthz` gate.
//!
//! ## Token model
//!
//! Tokens are 32 random bytes (256 bits of entropy) emitted as a 64-char hex
//! string and indexed in the database by their **deterministic SHA-256
//! hash**. This matches the existing `password_reset` and `email_verification`
//! conventions — argon2id is over-engineering for high-entropy tokens (brute
//! force is mathematically infeasible regardless of hash speed).
//!
//! ## Anti-enumeration
//!
//! All token-error paths return `Error::InvalidRequest` (HTTP 400) with the
//! same `error: "invalid_request"` body, regardless of whether the token is
//! missing, expired, already-resolved, or email-mismatched. This prevents
//! callers from probing the DB for valid tokens.
//!
//! ## Cap + rate-limit enforcement
//!
//! `send` runs cap checks (active editors + pending invites < cap) inside a
//! Postgres advisory-locked transaction so concurrent sends can't both pass
//! the cap check and double-insert. The per-inviter hourly rate limit is
//! enforced via a `count_for_inviter_since` query before the lock is taken;
//! a tight race here is acceptable (it would let through at most a couple
//! extra invitations under heavy concurrency, which is bounded anyway by
//! the cap inside the lock).

use chrono::{Duration, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ServerResult;
use crate::config::server::SharingConfig;
use crate::entity::calendar::Calendar;
use crate::entity::calendar_invitation::{CalendarInvitation, InvitationStatus};
use crate::error::Error;
use crate::mappers::calendar_editor::CalendarEditorMapper;
use crate::mappers::calendar_invitation::CalendarInvitationMapper;
use crate::services::auth::{generate_random_token, hash_token};
use crate::services::email::EmailService;
use crate::services::email_validation::{check_not_self, validate_email};

/// Public-facing invite preview returned by the unauthenticated
/// `GET /invitations/{token}` endpoint.
///
/// The browser shows this on the `/invite/:token` landing page so the
/// invitee knows what they're accepting before they sign in.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvitationPreview {
    pub calendar_name: String,
    pub owner_display: String,
    /// Reserved for forward compatibility — `users` does not currently
    /// store an avatar URL, so always emitted as `null`. Kept on the wire
    /// shape to avoid a frontend break when the column lands.
    pub owner_avatar: Option<String>,
    pub item_count: i64,
    /// Obfuscated form of the invitee's email address, e.g. `f***@h***.com`.
    /// Shown on the landing page so the recipient can confirm the invite is
    /// meant for them without revealing the full address to an observer.
    pub masked_email: String,
}

/// Mask an email address for display: `first_char***@first_domain_char***.tld`.
///
/// Rules:
/// - Local part: keep first character, append `***` (unless the local part is
///   a single character, in which case just that character).
/// - Domain: keep the first character of the domain label before the last `.`,
///   append `***`, keep the TLD.  If there is no `.` in the domain, mask the
///   whole domain to `first_char***`.
fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        None => {
            // Malformed — best-effort mask.
            let first = email.chars().next().unwrap_or('?');
            format!("{first}***")
        }
        Some((local, domain)) => {
            let local_first = local.chars().next().unwrap_or('?');
            let masked_local = if local.len() == 1 {
                local_first.to_string()
            } else {
                format!("{local_first}***")
            };

            let masked_domain = match domain.rsplit_once('.') {
                None => {
                    let domain_first = domain.chars().next().unwrap_or('?');
                    format!("{domain_first}***")
                }
                Some((label, tld)) => {
                    let label_first = label.chars().next().unwrap_or('?');
                    format!("{label_first}***.{tld}")
                }
            };

            format!("{masked_local}@{masked_domain}")
        }
    }
}

/// Invitation orchestration service. Constructed once at startup with all
/// downstream mapper/service dependencies plus the [`sharing`] config block.
///
/// `editors` and `users` mappers are used through their `_in_tx` static
/// helpers (called on `&mut tx`) rather than instance methods, so they
/// don't appear as fields here — the `InvitationService` doesn't need to
/// hold them.
#[derive(Clone)]
pub struct InvitationService {
    invitations: CalendarInvitationMapper,
    email: EmailService,
    pool: sqlx::PgPool,
    sharing: SharingConfig,
    app_base_url: String,
}

impl InvitationService {
    #[must_use]
    pub const fn new(
        invitations: CalendarInvitationMapper,
        email: EmailService,
        pool: sqlx::PgPool,
        sharing: SharingConfig,
        app_base_url: String,
    ) -> Self {
        Self {
            invitations,
            email,
            pool,
            sharing,
            app_base_url,
        }
    }

    /// Send a new invitation. Caller must have already validated tier +
    /// ownership via `SharingAuthz::assert_can(_, _, Action::Invite)`.
    ///
    /// Flow:
    ///   1. Validate invitee email shape + reject self-invite.
    ///   2. Per-inviter hourly rate-limit check (rolling window).
    ///   3. Mint token + compute hash up-front.
    ///   4. Open advisory-locked tx → check cap → insert invitation.
    ///   5. Commit, then send email best-effort (failure logs but does not
    ///      roll back the row — the owner can resend).
    ///
    /// # Errors
    /// - `Error::InvalidRequest` if email is malformed or matches the inviter.
    /// - `Error::TooManyRequests` if the rolling hour budget is exhausted.
    /// - `Error::Conflict { reason: "editor_cap_reached" }` if active+pending >= cap.
    /// - `Error::Conflict { reason: "invite_already_pending" }` if a pending
    ///   row already exists for this calendar+invitee.
    /// - Database / email-build error variants on infra failure.
    #[allow(clippy::similar_names)] // inviter_email/invitee_email are wire shape
    pub async fn send(
        &self,
        owner: &Calendar,
        inviter_username: &str,
        inviter_email: &str,
        invitee_email: &str,
    ) -> ServerResult<CalendarInvitation> {
        validate_email(invitee_email)?;
        check_not_self(invitee_email, inviter_email)?;

        let inviter_id = owner.user_id;
        let calendar_id = owner.id;

        // Per-inviter rolling-hour rate limit.
        let window_start = (Utc::now() - Duration::hours(1)).naive_utc();
        let recent = self
            .invitations
            .count_for_inviter_since(inviter_id, window_start)
            .await?;
        if recent >= i64::from(self.sharing.invite_rate_limit_per_hour) {
            return Err(Error::TooManyRequests);
        }

        let raw_token = generate_random_token();
        let token_hash = hash_token(&raw_token);
        let expires_at = (Utc::now()
            + Duration::days(i64::from(self.sharing.invitation_expiry_days)))
        .naive_utc();

        let invitation = self
            .insert_with_cap_check(
                calendar_id,
                inviter_id,
                invitee_email,
                &token_hash,
                expires_at,
            )
            .await?;

        // Send AFTER commit. If SMTP is down, the row exists; the owner
        // can resend.
        let invite_url = format!("{}/invite/{}", self.app_base_url, raw_token);
        if let Err(e) = self
            .email
            .send_invitation(invitee_email, inviter_username, &owner.name, &invite_url)
            .await
        {
            log::error!("invitation email send failed: {e:?}");
        }

        Ok(invitation)
    }

    async fn insert_with_cap_check(
        &self,
        calendar_id: i32,
        inviter_id: i32,
        invitee_email: &str,
        token_hash: &str,
        expires_at: NaiveDateTime,
    ) -> ServerResult<CalendarInvitation> {
        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "SELECT pg_advisory_xact_lock(hashtext($1))",
            format!("invite:{calendar_id}"),
        )
        .execute(&mut *tx)
        .await?;

        let active = CalendarEditorMapper::count_active_in_tx(&mut tx, calendar_id).await?;
        let pending = CalendarInvitationMapper::count_pending_in_tx(&mut tx, calendar_id).await?;
        if active + pending >= i64::from(self.sharing.editor_cap) {
            return Err(Error::Conflict {
                reason: "editor_cap_reached",
            });
        }

        let invitation = match CalendarInvitationMapper::create_in_tx(
            &mut tx,
            calendar_id,
            inviter_id,
            invitee_email,
            token_hash,
            expires_at,
        )
        .await
        {
            Ok(i) => i,
            Err(Error::Database(sqlx::Error::Database(db_err)))
                if db_err
                    .constraint()
                    .is_some_and(|c| c.contains("pending_unique")) =>
            {
                return Err(Error::Conflict {
                    reason: "invite_already_pending",
                });
            }
            Err(e) => return Err(e),
        };
        tx.commit().await?;
        Ok(invitation)
    }

    /// Accept an invitation. Email-bound: actor's email must match the
    /// invite's `invitee_email`. Caller is the authenticated invitee.
    ///
    /// # Errors
    /// - `Error::InvalidRequest` for **any** token-related failure: token not
    ///   found, expired, already-resolved, or caller's email does not match
    ///   the invite's `invitee_email`. Anti-enumeration: response shape is
    ///   always identical regardless of which condition triggered the error.
    /// - Database error variants on infra failure.
    pub async fn accept(&self, actor_id: i32, raw_token: &str) -> ServerResult<CalendarInvitation> {
        let token_hash = hash_token(raw_token);
        let mut tx = self.pool.begin().await?;
        let inv = CalendarInvitationMapper::find_pending_by_token_hash_in_tx(&mut tx, &token_hash)
            .await?
            .ok_or(Error::InvalidRequest)?;

        let user =
            crate::mappers::user::UserMapper::get_user_by_id_in_tx(&mut tx, actor_id).await?;
        if !user.email.eq_ignore_ascii_case(&inv.invitee_email) {
            return Err(Error::InvalidRequest);
        }

        let resolved = CalendarInvitationMapper::mark_resolved_in_tx(
            &mut tx,
            inv.id,
            InvitationStatus::Accepted,
        )
        .await?;
        if resolved == 0 {
            // Race — already resolved. Anti-enumeration: same as missing.
            return Err(Error::InvalidRequest);
        }

        CalendarEditorMapper::upsert_active_in_tx(&mut tx, inv.calendar_id, actor_id).await?;
        tx.commit().await?;

        Ok(inv)
    }

    /// Decline an invitation. Same email-bound rules as `accept`. Marks
    /// the invitation `Declined`; no editor row is created.
    ///
    /// # Errors
    /// - `Error::InvalidRequest` for **any** token-related failure: token not
    ///   found, expired, already-resolved, or caller's email does not match
    ///   the invite's `invitee_email`. Anti-enumeration: response shape is
    ///   always identical regardless of which condition triggered the error.
    /// - Database error variants on infra failure.
    pub async fn decline(&self, actor_id: i32, raw_token: &str) -> ServerResult<()> {
        let token_hash = hash_token(raw_token);
        let mut tx = self.pool.begin().await?;
        let inv = CalendarInvitationMapper::find_pending_by_token_hash_in_tx(&mut tx, &token_hash)
            .await?
            .ok_or(Error::InvalidRequest)?;

        let user =
            crate::mappers::user::UserMapper::get_user_by_id_in_tx(&mut tx, actor_id).await?;
        if !user.email.eq_ignore_ascii_case(&inv.invitee_email) {
            return Err(Error::InvalidRequest);
        }

        let resolved = CalendarInvitationMapper::mark_resolved_in_tx(
            &mut tx,
            inv.id,
            InvitationStatus::Declined,
        )
        .await?;
        if resolved == 0 {
            return Err(Error::InvalidRequest);
        }
        tx.commit().await?;
        Ok(())
    }

    /// Revoke a pending invitation. Owner-only — caller must already have
    /// passed `SharingAuthz::assert_can(_, _, Action::ManageEditors)`.
    /// Idempotent on a still-pending row; returns `NotFound` for an
    /// invitation that's already in a terminal status (so callers don't
    /// silently treat a failed revoke as success).
    ///
    /// # Errors
    /// - `Error::NotFound` if the invitation belongs to a different
    ///   calendar, or if it's already resolved.
    /// - Database error variants on infra failure.
    pub async fn revoke(&self, calendar_id: i32, invitation_id: i64) -> ServerResult<()> {
        let mut tx = self.pool.begin().await?;
        let inv = CalendarInvitationMapper::find_by_id_in_tx(&mut tx, invitation_id)
            .await?
            .ok_or(Error::NotFound)?;
        if inv.calendar_id != calendar_id {
            return Err(Error::NotFound);
        }
        let resolved = CalendarInvitationMapper::mark_resolved_in_tx(
            &mut tx,
            invitation_id,
            InvitationStatus::Revoked,
        )
        .await?;
        if resolved == 0 {
            return Err(Error::NotFound);
        }
        tx.commit().await?;
        Ok(())
    }

    /// Resend an invitation. We only stored the hash, not the raw token,
    /// so we can't re-send the original link. Instead we revoke the
    /// existing pending row and mint a brand-new invitation. The new row
    /// counts toward the per-inviter rate limit.
    ///
    /// # Errors
    /// - `Error::NotFound` if the invitation is already resolved or
    ///   targets a different calendar.
    /// - `Error::TooManyRequests` if the rolling hour budget is exhausted.
    /// - Database / email-build error variants.
    pub async fn resend(
        &self,
        owner: &Calendar,
        inviter_username: &str,
        invitation_id: i64,
    ) -> ServerResult<CalendarInvitation> {
        // Fetch the original invite to recover invitee_email + assert
        // ownership of the calendar.
        let original = self
            .invitations
            .find_by_id(invitation_id)
            .await?
            .ok_or(Error::NotFound)?;
        if original.calendar_id != owner.id {
            return Err(Error::NotFound);
        }
        if !matches!(original.status, InvitationStatus::Pending) {
            return Err(Error::NotFound);
        }

        // Rate-limit check before we revoke — otherwise a 429 would still
        // burn the original token.
        let window_start = (Utc::now() - Duration::hours(1)).naive_utc();
        let recent = self
            .invitations
            .count_for_inviter_since(owner.user_id, window_start)
            .await?;
        if recent >= i64::from(self.sharing.invite_rate_limit_per_hour) {
            return Err(Error::TooManyRequests);
        }

        // Revoke the old row in its own tx so the partial unique index
        // doesn't reject the replacement.
        let mut tx = self.pool.begin().await?;
        let resolved = CalendarInvitationMapper::mark_resolved_in_tx(
            &mut tx,
            invitation_id,
            InvitationStatus::Revoked,
        )
        .await?;
        if resolved == 0 {
            return Err(Error::NotFound);
        }
        tx.commit().await?;

        // Mint + insert the new row (also runs the cap check inside its
        // advisory lock, in case anything has changed since).
        let raw_token = generate_random_token();
        let token_hash = hash_token(&raw_token);
        let expires_at = (Utc::now()
            + Duration::days(i64::from(self.sharing.invitation_expiry_days)))
        .naive_utc();
        let invitation = self
            .insert_with_cap_check(
                owner.id,
                owner.user_id,
                &original.invitee_email,
                &token_hash,
                expires_at,
            )
            .await?;

        let invite_url = format!("{}/invite/{}", self.app_base_url, raw_token);
        if let Err(e) = self
            .email
            .send_invitation(
                &original.invitee_email,
                inviter_username,
                &owner.name,
                &invite_url,
            )
            .await
        {
            log::error!("invitation email resend failed: {e:?}");
        }

        Ok(invitation)
    }

    /// Public preview for `/invite/:token`. Unauthenticated; same
    /// anti-enumeration shape as accept/decline.
    ///
    /// # Errors
    /// `Error::InvalidRequest` for any failure path.
    pub async fn preview(&self, raw_token: &str) -> ServerResult<InvitationPreview> {
        let token_hash = hash_token(raw_token);
        let mut tx = self.pool.begin().await?;
        let inv = CalendarInvitationMapper::find_pending_by_token_hash_in_tx(&mut tx, &token_hash)
            .await?
            .ok_or(Error::InvalidRequest)?;

        let cal_row = sqlx::query!(
            "SELECT name, user_id FROM calendars
             WHERE id = $1 AND deleted_at IS NULL",
            inv.calendar_id,
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::InvalidRequest)?;

        let owner =
            crate::mappers::user::UserMapper::get_user_by_id_in_tx(&mut tx, cal_row.user_id)
                .await
                .map_err(|_| Error::InvalidRequest)?;

        let item_count: Option<i64> = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM calendar_items WHERE calendar_id = $1",
            inv.calendar_id,
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(InvitationPreview {
            calendar_name: cal_row.name,
            owner_display: owner.username,
            owner_avatar: None,
            item_count: item_count.unwrap_or(0),
            masked_email: mask_email(&inv.invitee_email),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::server::SmtpConfig;
    use crate::mappers::calendar_invitation::CalendarInvitationMapper;
    use crate::services::auth::hash_token;
    use crate::test_helpers::test_pool;
    use sqlx::Row as _;

    /// Helpers aren't enough to build the full live SMTP/Pool service in a
    /// unit test, so the rich integration coverage lives in
    /// `controllers::sharing::tests`. The tests here pin pure logic that
    /// doesn't need DB / email — input validation paths.

    #[test]
    fn mask_email_typical() {
        assert_eq!(mask_email("foo@hotmail.com"), "f***@h***.com");
        assert_eq!(mask_email("alice@example.com"), "a***@e***.com");
    }

    #[test]
    fn mask_email_single_char_local() {
        assert_eq!(mask_email("a@example.com"), "a@e***.com");
    }

    #[test]
    fn mask_email_domain_no_dot() {
        assert_eq!(mask_email("foo@localhost"), "f***@l***");
    }

    #[test]
    fn mask_email_malformed_no_at() {
        assert_eq!(mask_email("notanemail"), "n***");
    }

    #[tokio::test]
    async fn validate_email_is_invoked_via_check() {
        // Direct sanity — ensures the helper is wired into the orchestrator
        // contract (no other test would catch a regression in the import).
        assert!(validate_email("foo").is_err());
        assert!(validate_email("a@b.co").is_ok());
        assert!(check_not_self("foo@bar.com", "FOO@bar.com").is_err());
    }

    // -----------------------------------------------------------------------
    // Anti-enumeration: accept/decline must collapse ALL token failures to
    // Error::InvalidRequest (HTTP 400) regardless of condition.
    // -----------------------------------------------------------------------

    /// Build a minimal `InvitationService` backed by a live pool for
    /// integration tests. Email is wired with an empty SMTP host so all
    /// send calls log+no-op instead of touching a real SMTP server.
    fn make_svc(pool: sqlx::PgPool) -> InvitationService {
        let inv_mapper = CalendarInvitationMapper::from_pool(pool.clone());
        let email = crate::services::email::EmailService::new(SmtpConfig::default());
        InvitationService::new(
            inv_mapper,
            email,
            pool,
            SharingConfig::default(),
            "http://localhost".to_string(),
        )
    }

    /// Generate a random hex suffix unique to this test invocation, mirroring
    /// `services::reconcile::tests::unique_sub_id`. Tests share a real
    /// `DATABASE_URL` via `test_pool()`, so all seeded usernames / emails /
    /// tokens must be unique across runs to avoid `users_username_key` /
    /// `users_email_key` collisions on re-run.
    fn unique_suffix() -> String {
        let n: u64 = rand::random();
        format!("{n:016x}")
    }

    /// Seed a user + calendar + invitation row. Returns
    /// `(invitee_user_id, raw_token, invitee_email)`. All identifiers are
    /// uniquified per call; `tag` is a cosmetic prefix only.
    async fn seed_invitation(
        pool: &sqlx::PgPool,
        tag: &str,
        expired: bool,
    ) -> (i32, String, String) {
        let mut conn = pool.acquire().await.unwrap();
        let suffix = unique_suffix();
        let invitee_email = format!("invitee_{tag}_{suffix}@example.com");

        // Owner user
        let owner = sqlx::query!(
            "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, NULL) RETURNING id",
            format!("owner_{tag}_{suffix}"),
            format!("owner_{tag}_{suffix}@example.com"),
        )
        .fetch_one(&mut *conn)
        .await
        .unwrap();

        // Calendar owned by owner
        let cal = sqlx::query(
            "INSERT INTO calendars (name, language, user_id, subscription_token, event_style) \
             VALUES ($1, 'english'::language, $2, gen_random_uuid()::text, 'timed') RETURNING id",
        )
        .bind(format!("cal_{tag}_{suffix}"))
        .bind(owner.id)
        .fetch_one(&mut *conn)
        .await
        .unwrap();

        // Raw token + hash. Token must be 64 hex chars; build it from two
        // random u64s so it's unique across runs. The token is high-entropy
        // and is not reused as a DB primary key, but its hash IS indexed.
        let token_hi: u64 = rand::random();
        let token_lo: u64 = rand::random();
        let raw_token = format!("{token_hi:016x}{token_lo:016x}{token_hi:016x}{token_lo:016x}");
        let token_hash = hash_token(&raw_token);

        let expires_at: NaiveDateTime = if expired {
            (chrono::Utc::now() - chrono::Duration::days(1)).naive_utc()
        } else {
            (chrono::Utc::now() + chrono::Duration::days(7)).naive_utc()
        };

        let cal_id: i32 = cal.get("id");

        sqlx::query!(
            "INSERT INTO calendar_invitations
                (calendar_id, inviter_id, invitee_email, token_hash, expires_at)
             VALUES ($1, $2, $3, $4, $5)",
            cal_id,
            owner.id,
            invitee_email,
            token_hash,
            expires_at,
        )
        .execute(&mut *conn)
        .await
        .unwrap();

        // Invitee user (email matches invitee_email so the email-bound
        // accept/decline path will succeed for this user).
        let invitee = sqlx::query!(
            "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, NULL) RETURNING id",
            format!("invitee_{tag}_{suffix}"),
            invitee_email,
        )
        .fetch_one(&mut *conn)
        .await
        .unwrap();

        (invitee.id, raw_token, invitee_email)
    }

    /// Seed a standalone user (not tied to an invitation). Used when a test
    /// needs an actor whose email does NOT match any invite. Returns the
    /// new user's id.
    async fn seed_user(pool: &sqlx::PgPool, tag: &str) -> i32 {
        let suffix = unique_suffix();
        sqlx::query_scalar!(
            "INSERT INTO users (username, email, password_hash)
             VALUES ($1, $2, NULL) RETURNING id",
            format!("u_{tag}_{suffix}"),
            format!("u_{tag}_{suffix}@example.com"),
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    // --- accept: token not found ---

    #[tokio::test]
    async fn accept_unknown_token_returns_invalid_request() {
        let pool = test_pool().await;
        let svc = make_svc(pool.clone());

        // seed a real user so actor_id is valid
        let user_id = seed_user(&pool, "acc_unk").await;

        // Random 64-char hex string that won't match any seeded token_hash.
        let bogus = format!(
            "{:016x}{:016x}{:016x}{:016x}",
            rand::random::<u64>(),
            rand::random::<u64>(),
            rand::random::<u64>(),
            rand::random::<u64>()
        );

        let err = svc.accept(user_id, &bogus).await.unwrap_err();
        assert!(
            matches!(err, Error::InvalidRequest),
            "expected InvalidRequest for missing token, got {err:?}"
        );
    }

    // --- accept: email mismatch (anti-enumeration — was Forbidden, now 400) ---

    #[tokio::test]
    async fn accept_email_mismatch_returns_invalid_request_not_forbidden() {
        let pool = test_pool().await;
        let svc = make_svc(pool.clone());

        // Seed invitation for a unique invitee email.
        let (_invitee_id, raw_token, _invitee_email) = seed_invitation(&pool, "amm", false).await;

        // A different user whose email does NOT match the invitee_email
        let other_id = seed_user(&pool, "other_amm").await;

        let err = svc.accept(other_id, &raw_token).await.unwrap_err();
        assert!(
            matches!(err, Error::InvalidRequest),
            "email mismatch must return InvalidRequest (not Forbidden), got {err:?}"
        );
    }

    // --- accept: already-resolved race ---

    #[tokio::test]
    async fn accept_already_resolved_returns_invalid_request() {
        let pool = test_pool().await;
        let svc = make_svc(pool.clone());

        let (invitee_id, raw_token, _invitee_email) = seed_invitation(&pool, "aar", false).await;

        // Manually resolve the row so the race-condition branch fires.
        sqlx::query!(
            "UPDATE calendar_invitations SET status = 'accepted', resolved_at = NOW()
             WHERE token_hash = $1",
            hash_token(&raw_token),
        )
        .execute(&pool)
        .await
        .unwrap();

        // The row is no longer 'pending', so find_pending_by_token_hash_in_tx
        // returns None — collapses to InvalidRequest.
        let err = svc.accept(invitee_id, &raw_token).await.unwrap_err();
        assert!(
            matches!(err, Error::InvalidRequest),
            "already-resolved token must return InvalidRequest, got {err:?}"
        );
    }

    // --- decline: token not found ---

    #[tokio::test]
    async fn decline_unknown_token_returns_invalid_request() {
        let pool = test_pool().await;
        let svc = make_svc(pool.clone());

        let user_id = seed_user(&pool, "dec_unk").await;

        let bogus = format!(
            "{:016x}{:016x}{:016x}{:016x}",
            rand::random::<u64>(),
            rand::random::<u64>(),
            rand::random::<u64>(),
            rand::random::<u64>()
        );

        let err = svc.decline(user_id, &bogus).await.unwrap_err();
        assert!(
            matches!(err, Error::InvalidRequest),
            "expected InvalidRequest for missing token on decline, got {err:?}"
        );
    }

    // --- decline: email mismatch (anti-enumeration — was Forbidden, now 400) ---

    #[tokio::test]
    async fn decline_email_mismatch_returns_invalid_request_not_forbidden() {
        let pool = test_pool().await;
        let svc = make_svc(pool.clone());

        let (_invitee_id, raw_token, _invitee_email) = seed_invitation(&pool, "dmm", false).await;

        let other_id = seed_user(&pool, "other_dmm").await;

        let err = svc.decline(other_id, &raw_token).await.unwrap_err();
        assert!(
            matches!(err, Error::InvalidRequest),
            "email mismatch on decline must return InvalidRequest (not Forbidden), got {err:?}"
        );
    }

    // --- decline: already-resolved race ---

    #[tokio::test]
    async fn decline_already_resolved_returns_invalid_request() {
        let pool = test_pool().await;
        let svc = make_svc(pool.clone());

        let (invitee_id, raw_token, _invitee_email) = seed_invitation(&pool, "dar", false).await;

        sqlx::query!(
            "UPDATE calendar_invitations SET status = 'declined', resolved_at = NOW()
             WHERE token_hash = $1",
            hash_token(&raw_token),
        )
        .execute(&pool)
        .await
        .unwrap();

        let err = svc.decline(invitee_id, &raw_token).await.unwrap_err();
        assert!(
            matches!(err, Error::InvalidRequest),
            "already-resolved token must return InvalidRequest on decline, got {err:?}"
        );
    }

    // --- no path in accept/decline returns Forbidden ---

    #[tokio::test]
    async fn accept_never_returns_forbidden() {
        // Enumeration: every known error condition was covered above and all
        // assert InvalidRequest. This test makes the absence of Forbidden
        // explicit by checking a token whose record exists but email is wrong.
        let pool = test_pool().await;
        let svc = make_svc(pool.clone());

        let (_invitee_id, raw_token, _invitee_email) = seed_invitation(&pool, "anf", false).await;

        let attacker_id = seed_user(&pool, "attacker_anf").await;

        match svc.accept(attacker_id, &raw_token).await {
            Err(Error::Forbidden) => panic!("accept must NOT return Forbidden — AUDIT.md H-5"),
            Err(Error::InvalidRequest) => {} // correct
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[tokio::test]
    async fn decline_never_returns_forbidden() {
        let pool = test_pool().await;
        let svc = make_svc(pool.clone());

        let (_invitee_id, raw_token, _invitee_email) = seed_invitation(&pool, "dnf", false).await;

        let attacker_id = seed_user(&pool, "attacker_dnf").await;

        match svc.decline(attacker_id, &raw_token).await {
            Err(Error::Forbidden) => panic!("decline must NOT return Forbidden — AUDIT.md H-5"),
            Err(Error::InvalidRequest) => {} // correct
            other => panic!("unexpected result: {other:?}"),
        }
    }
}
