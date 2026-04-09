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
    #[error("Not Implemented Yet")] // TODO remove once done
    NotImplemented,
    #[error("Invalid token")]
    InvalidToken(#[from] google_oauth::Error),
    #[error("Cannot hash password")]
    CannotHashPassword(#[from] argon2::password_hash::Error),
    #[error("Cannot generate auth token")]
    CannotGenerateAuthToken(#[from] jsonwebtoken::errors::Error),
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
            Self::InvalidRequest
            | Self::UserAlreadyExists
            | Self::InvalidPassword
            | Self::CannotHashPassword(_)
            | Self::CannotGenerateAuthToken(_) => StatusCode::BAD_REQUEST,
            Self::Database(_) | Self::Config(_) | Self::Server(_) | Self::NotImplemented => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}
