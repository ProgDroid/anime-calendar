use google_oauth::{AsyncClient, GooglePayload};

use crate::{ServerResult, error::Error};

#[derive(Clone)]
pub struct GoogleOauth {
    client: AsyncClient,
}

#[derive(Debug)]
pub struct GoogleUser {
    pub id: String,
    pub full_name: String,
    pub email: String,
    pub avatar_url: String,
}

impl GoogleOauth {
    #[must_use]
    pub fn new(client_id: &str) -> Self {
        Self {
            client: AsyncClient::new(client_id),
        }
    }

    /// # Errors
    /// Fails if the token is invalid or the asserted email is not verified.
    pub async fn validate_id_token(&self, id_token: &str) -> ServerResult<GoogleUser> {
        let payload = match self.client.validate_id_token(id_token).await {
            Ok(payload) => payload,
            Err(e) => return Err(e.into()),
        };

        google_user_from_payload(payload)
    }
}

/// Convert a validated Google ID-token payload into a [`GoogleUser`].
///
/// The OAuth controller trusts `email` to mark accounts verified and to link
/// the Google identity to a pre-existing local account with the same address,
/// so an *unverified* email here would let an attacker claim someone else's
/// local account by asserting their address on a fresh Google identity.
/// Google only vouches for the address when `email_verified` is `true`.
///
/// # Errors
/// Returns [`Error::Unauthorised`] when the payload carries no email or the
/// email is not verified (absent `email_verified` is treated as unverified).
fn google_user_from_payload(payload: GooglePayload) -> ServerResult<GoogleUser> {
    if payload.email_verified != Some(true) {
        log::warn!(
            "google_oauth: rejecting token with unverified email (sub: {})",
            payload.sub
        );
        return Err(Error::Unauthorised);
    }
    let Some(email) = payload.email.filter(|e| !e.is_empty()) else {
        log::warn!(
            "google_oauth: rejecting token without email claim (sub: {})",
            payload.sub
        );
        return Err(Error::Unauthorised);
    };

    Ok(GoogleUser {
        id: payload.sub,
        full_name: payload.name.unwrap_or_default(),
        email,
        avatar_url: payload.picture.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// `GooglePayload` is `#[non_exhaustive]`, so tests construct it through
    /// its `Deserialize` impl instead of a struct literal.
    fn payload(email: Option<&str>, email_verified: Option<bool>) -> GooglePayload {
        let mut value = json!({
            "aud": "client-id",
            "exp": 0,
            "iat": 0,
            "iss": "https://accounts.google.com",
            "sub": "google-sub-1",
            "name": "Test User",
            "picture": "https://example.com/a.png",
        });
        if let Some(email) = email {
            value["email"] = json!(email);
        }
        if let Some(verified) = email_verified {
            value["email_verified"] = json!(verified);
        }
        serde_json::from_value(value).expect("payload fixture must deserialize")
    }

    #[test]
    fn verified_email_is_accepted() {
        let user = google_user_from_payload(payload(Some("a@b.com"), Some(true)))
            .expect("verified email must be accepted");
        assert_eq!(user.email, "a@b.com");
        assert_eq!(user.id, "google-sub-1");
    }

    #[test]
    fn unverified_email_is_rejected_as_unauthorised() {
        let err = google_user_from_payload(payload(Some("a@b.com"), Some(false)))
            .expect_err("unverified email must be rejected");
        assert!(matches!(err, Error::Unauthorised));
    }

    #[test]
    fn missing_email_verified_claim_is_rejected() {
        let err = google_user_from_payload(payload(Some("a@b.com"), None))
            .expect_err("absent email_verified must be treated as unverified");
        assert!(matches!(err, Error::Unauthorised));
    }

    #[test]
    fn missing_email_is_rejected() {
        let err = google_user_from_payload(payload(None, Some(true)))
            .expect_err("payload without email must be rejected");
        assert!(matches!(err, Error::Unauthorised));
    }
}
