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
    #[error("Unauthorized")]
    Unauthorised,
    #[error("Invalid request data")]
    InvalidRequest,
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Not Implemented Yet")] // TODO remove once done
    NotImplemented,
}

impl ResponseError for Error {
    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code()).body(self.to_string()) // TODO also return original error?
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorised => StatusCode::UNAUTHORIZED,
            Self::InvalidRequest | Self::UserAlreadyExists => StatusCode::BAD_REQUEST,
            Self::Database(_) | Self::Config(_) | Self::Server(_) | Self::NotImplemented => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

// // server/src/error.rs
// use actix_web::{error, HttpResponse, Result};
// use serde::{Deserialize, Serialize};

// #[derive(Debug, Serialize, Deserialize)]
// pub struct ServiceError {
//     pub message: String,
//     pub status_code: u16,
// }

// impl error::ResponseError for ServiceError {
//     fn status_code(&self) -> actix_web::http::StatusCode {
//         actix_web::http::StatusCode::from_u16(self.status_code).unwrap()
//     }

//     fn error_response(&self) -> HttpResponse {
//         HttpResponse::build(self.status_code()).json(self)
//     }
// }

// impl ServiceError {
//     pub fn Unauthorized() -> Self {
//         ServiceError {
//             message: "Unauthorized".to_string(),
//             status_code: 401,
//         }
//     }

//     pub fn InternalServerError() -> Self {
//         ServiceError {
//             message: "Internal Server Error".to_string(),
//             status_code: 500,
//         }
//     }

//     pub fn NotFound() -> Self {
//         ServiceError {
//             message: "Not Found".to_string(),
//             status_code: 404,
//         }
//     }
// }
