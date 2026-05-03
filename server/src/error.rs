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
    /// HTTP 402: caller's tier doesn't allow the requested resource
    /// (e.g. a free user trying to set a Pro accent on `PUT /user/settings`).
    /// The body shape extends the `{"error": "..."}` convention with a
    /// `required_tier` field so the frontend can route the user to the right
    /// upgrade surface without parsing the message string.
    #[error("upgrade_required")]
    PaymentRequired { required_tier: &'static str },
}

impl ResponseError for Error {
    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        // PaymentRequired carries an extra structured field. Other variants
        // keep the documented `{"error":"..."}` shape from CLAUDE.md.
        if let Self::PaymentRequired { required_tier } = self {
            return HttpResponse::build(self.status_code()).json(serde_json::json!({
                "error": "upgrade_required",
                "required_tier": *required_tier,
            }));
        }
        HttpResponse::build(self.status_code()).json(serde_json::json!({
            "error": self.to_string()
        }))
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorised | Self::InvalidToken(_) => StatusCode::UNAUTHORIZED,
            Self::EmailNotVerified => StatusCode::FORBIDDEN,
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
