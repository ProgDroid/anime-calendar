use actix_web::{http::StatusCode, HttpResponse, ResponseError};

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
}

impl ResponseError for Error {
    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code()).json(serde_json::json!({
            "error": self.to_string()
        }))
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorised | Self::InvalidToken(_) => StatusCode::UNAUTHORIZED,
            Self::EmailNotVerified => StatusCode::FORBIDDEN,
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
