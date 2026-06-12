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
    /// HTTP 409: the requested operation conflicts with current state and
    /// cannot be retried as-is. `reason` is a stable code the frontend
    /// maps to copy (e.g. `editor_cap_reached`, `invite_already_pending`).
    /// Used by the sharing layer to surface cap and uniqueness rejections.
    #[error("conflict")]
    Conflict { reason: &'static str },
    /// HTTP 429: caller has exceeded a server-side rate budget. Used by
    /// `InvitationService::send` to enforce per-inviter hourly invite
    /// caps without standing up a separate middleware (the controller
    /// path already has the user identity from `Claims`).
    #[error("rate_limited")]
    TooManyRequests,
    /// Redis operation or connection error.
    #[error("Redis error: {0}")]
    Redis(String),
}

impl ResponseError for Error {
    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        // PaymentRequired carries an extra structured field. Conflict surfaces
        // a `reason` code so the frontend can map to copy. Other variants
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
        if let Self::Conflict { reason } = self {
            return HttpResponse::build(self.status_code()).json(serde_json::json!({
                "error": *reason,
            }));
        }
        // Variants whose Display embeds an internal / third-party detail string.
        // Log the full detail server-side, but return only a stable generic code
        // so SMTP, Stripe, and Redis internals never reach the client
        // (M-2, M-3, F2-18).
        if let Self::EmailError(detail) = self {
            log::error!("email send error: {detail}");
            return HttpResponse::build(self.status_code())
                .json(serde_json::json!({ "error": "email_error" }));
        }
        if let Self::Stripe(detail) = self {
            log::error!("stripe error: {detail}");
            return HttpResponse::build(self.status_code())
                .json(serde_json::json!({ "error": "stripe_error" }));
        }
        if let Self::Redis(detail) = self {
            log::error!("redis error: {detail}");
            return HttpResponse::build(self.status_code())
                .json(serde_json::json!({ "error": "internal_error" }));
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
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            Self::InvalidRequest
            | Self::UserAlreadyExists
            | Self::InvalidPassword
            | Self::InvalidResetToken
            | Self::InvalidVerificationToken => StatusCode::BAD_REQUEST,
            // Server-side crypto failures (argon2 hashing / JWT signing) are
            // internal faults, not client errors — they carry no caller-fixable
            // detail and must not be advertised as 400 Bad Request.
            Self::CannotHashPassword(_)
            | Self::CannotGenerateAuthToken(_)
            | Self::Database(_)
            | Self::Config(_)
            | Self::Server(_)
            | Self::EmailError(_)
            | Self::StripeNotConfigured
            | Self::Stripe(_)
            | Self::Redis(_) => StatusCode::INTERNAL_SERVER_ERROR,
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

    #[tokio::test]
    async fn conflict_returns_409_with_reason_as_error() {
        let err = Error::Conflict {
            reason: "editor_cap_reached",
        };
        assert_eq!(err.status_code(), StatusCode::CONFLICT);
        let resp = err.error_response();
        let bytes = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"], "editor_cap_reached");
    }

    #[tokio::test]
    async fn too_many_requests_returns_429() {
        let err = Error::TooManyRequests;
        assert_eq!(err.status_code(), StatusCode::TOO_MANY_REQUESTS);
        let resp = err.error_response();
        let bytes = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"], "rate_limited");
    }

    #[tokio::test]
    async fn stripe_error_returns_generic_code_without_leaking_detail() {
        let err = Error::Stripe("No such customer: cus_SECRET123 (price_1abc)".to_owned());
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        let resp = err.error_response();
        let bytes = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"], "stripe_error");
        assert!(
            !json.to_string().contains("cus_SECRET123"),
            "Stripe internal detail leaked to client: {json}"
        );
    }

    #[tokio::test]
    async fn email_error_returns_generic_code_without_leaking_detail() {
        let err = Error::EmailError("smtp connect failed: relay.internal:587 timed out".to_owned());
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        let resp = err.error_response();
        let bytes = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"], "email_error");
        assert!(
            !json.to_string().contains("relay.internal"),
            "SMTP internal detail leaked to client: {json}"
        );
    }

    #[tokio::test]
    async fn redis_error_returns_generic_code_without_leaking_detail() {
        let err = Error::Redis("connection refused: 10.0.0.5:6379".to_owned());
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        let resp = err.error_response();
        let bytes = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"], "internal_error");
        assert!(
            !json.to_string().contains("10.0.0.5"),
            "Redis internal detail leaked to client: {json}"
        );
    }
}
