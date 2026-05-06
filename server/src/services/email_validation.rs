//! Email validation helpers shared by sharing endpoints.
//!
//! The auth controller already validates email shape on registration via the
//! `email_address` crate; this module exposes a `Result`-shaped wrapper plus
//! a self-invite check used by `InvitationService::send`.

use crate::ServerResult;
use crate::error::Error;

/// Maximum allowed email length (RFC 5321 §4.5.3.1.3).
const MAX_EMAIL_LEN: usize = 254;

/// Validates an invitee email address by shape.
///
/// Wraps the same `email_address` + dot-in-domain check used during user
/// registration so invitations can't be sent to local-only hostnames.
///
/// # Errors
/// Returns `Error::InvalidRequest` if the email is empty, too long, or
/// otherwise rejected by the validator.
pub fn validate_email(email: &str) -> ServerResult<()> {
    let trimmed = email.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_EMAIL_LEN {
        return Err(Error::InvalidRequest);
    }
    if !email_address::EmailAddress::is_valid(trimmed) {
        return Err(Error::InvalidRequest);
    }
    let has_dotted_domain = trimmed
        .rsplit_once('@')
        .is_some_and(|(_, domain)| domain.contains('.'));
    if !has_dotted_domain {
        return Err(Error::InvalidRequest);
    }
    Ok(())
}

/// Rejects an invitation when the invitee email matches the inviter's own.
///
/// Comparison is case-insensitive (ASCII) to match the `CITEXT` invitee
/// column. The two strings need not be normalized — the caller passes raw
/// owner email and incoming form input.
///
/// # Errors
/// Returns `Error::InvalidRequest` if the addresses match (the controller
/// reports `self_invite` to the client via the canonical 400 envelope).
pub fn check_not_self(invitee: &str, owner: &str) -> ServerResult<()> {
    if invitee.trim().eq_ignore_ascii_case(owner.trim()) {
        Err(Error::InvalidRequest)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert!(validate_email("").is_err());
        assert!(validate_email("   ").is_err());
    }

    #[test]
    fn rejects_no_at() {
        assert!(validate_email("foo").is_err());
    }

    #[test]
    fn rejects_local_only_domain() {
        assert!(validate_email("foo@localhost").is_err());
    }

    #[test]
    fn rejects_too_long() {
        let long = format!("{}@example.com", "a".repeat(260));
        assert!(validate_email(&long).is_err());
    }

    #[test]
    fn accepts_basic() {
        assert!(validate_email("a@b.co").is_ok());
        assert!(validate_email("Foo.Bar+tag@example.org").is_ok());
    }

    #[test]
    fn rejects_self_case_insensitive() {
        assert!(check_not_self("foo@bar.com", "FOO@bar.com").is_err());
        assert!(check_not_self("  foo@bar.com  ", "foo@bar.com").is_err());
    }

    #[test]
    fn accepts_different_emails() {
        assert!(check_not_self("a@x.com", "b@x.com").is_ok());
    }
}
