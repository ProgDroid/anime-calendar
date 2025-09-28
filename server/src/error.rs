use actix_web::{http::StatusCode, HttpResponse, ResponseError};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Requested resource not found")]
    NotFound,
    #[error("An unhandled database error occurred")] // TODO more granular DB errors
    Database(#[from] sqlx::Error),
    #[error("An unhandled config error occurred")] // TODO more granular/specific
    Config(#[from] config::ConfigError),
    #[error("Could not start server")] // TODO ?
    Server(#[from] std::io::Error),
}

impl ResponseError for Error {
    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code()).body(self.to_string()) // TODO also return original error?
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Database(_) | Self::Config(_) | Self::Server(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}
