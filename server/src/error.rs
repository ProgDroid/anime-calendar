use actix_web::{HttpResponse, ResponseError, http::StatusCode};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Requested resource not found")]
    NotFound,
    #[error("An unhandled database error occurred")]
    Database(#[from] sqlx::Error),
    #[error("An unhandled config error occurred")]
    Config(#[from] config::ConfigError),
    #[error("Could not start server")]
    Server(#[from] std::io::Error),
    #[error("Unauthorized")]
    Unauthorised,
    /// HTTP 403: caller is authenticated but lacks the role required for
    /// this action. Used by `SharingAuthz` to distinguish "not logged in"
    /// (Unauthorised, 401) from "logged in but wrong role" — e.g. an
    /// editor trying to mutate calendar metadata.
    #[error("Forbidden")]
    Forbidden,
    #[error("Invalid request data")]
    InvalidRequest,
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Invalid password")]
    InvalidPassword,
    #[error("Invalid token")]
    InvalidToken(#[from] google_oauth::Error),
    #[error("Cannot hash password")]
    CannotHashPassword(#[from] argon2::password_hash::Error),
    #[error("Cannot generate auth token")]
    CannotGenerateAuthToken(#[from] jsonwebtoken::errors::Error),
    #[error("Governor config invalid")]
    GovernorConfig,
    #[error("Invalid or expired reset token")]
    InvalidResetToken,
    #[error("Email not verified — please check your inbox")]
    EmailNotVerified,
    #[error("Invalid or expired verification token")]
    InvalidVerificationToken,
    #[error("Failed to send email: {0}")]
    EmailError(String),
    #[error("Stripe is not configured on this deployment")]
    StripeNotConfigured,
    #[error("Stripe API error: {0}")]
    Stripe(String),
    /// HTTP 402: caller's tier doesn't allow the requested resource.
    /// `required_tier` tells the frontend which surface to send the user
    /// to. `reason` provides a stable code (e.g. `cap_calendars`,
    /// `cap_shows`, `pro_accent`) so the modal can show the right copy.
    /// Reason is optional for backwards compatibility — frontend defaults
    /// to a generic "upgrade" string when absent.
    #[error("upgrade_required")]
    PaymentRequired {
        required_tier: &'static str,
        reason: Option<&'static str>,
    },
}

impl ResponseError for Error {
    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        // PaymentRequired carries an extra structured field. Other variants
        // keep the documented `{"error":"..."}` shape from CLAUDE.md.
        if let Self::PaymentRequired {
            required_tier,
            reason,
        } = self
        {
            let mut payload = serde_json::json!({
                "error": "upgrade_required",
                "required_tier": *required_tier,
            });
            if let Some(reason) = reason {
                payload["reason"] = serde_json::Value::String((*reason).to_owned());
            }
            return HttpResponse::build(self.status_code()).json(payload);
        }
        HttpResponse::build(self.status_code()).json(serde_json::json!({
            "error": self.to_string()
        }))
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorised | Self::InvalidToken(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden | Self::EmailNotVerified => StatusCode::FORBIDDEN,
            Self::PaymentRequired { .. } => StatusCode::PAYMENT_REQUIRED,
            Self::InvalidRequest
            | Self::UserAlreadyExists
            | Self::InvalidPassword
            | Self::InvalidResetToken
            | Self::InvalidVerificationToken
            | Self::CannotHashPassword(_)
            | Self::CannotGenerateAuthToken(_) => StatusCode::BAD_REQUEST,
            Self::Database(_)
            | Self::Config(_)
            | Self::Server(_)
            | Self::GovernorConfig
            | Self::EmailError(_)
            | Self::StripeNotConfigured
            | Self::Stripe(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn payment_required_with_reason_includes_reason_field() {
        let err = Error::PaymentRequired {
            required_tier: "paid",
            reason: Some("cap_shows"),
        };
        let resp = err.error_response();
        let bytes = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"], "upgrade_required");
        assert_eq!(json["required_tier"], "paid");
        assert_eq!(json["reason"], "cap_shows");
    }

    #[tokio::test]
    async fn payment_required_without_reason_omits_reason_field() {
        let err = Error::PaymentRequired {
            required_tier: "paid",
            reason: None,
        };
        let resp = err.error_response();
        let bytes = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"], "upgrade_required");
        assert_eq!(json["required_tier"], "paid");
        assert!(json.get("reason").is_none());
    }
}
