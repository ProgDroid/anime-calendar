use crate::{entity::user::User, error::Error, server::Repos};

use actix_web::{get, post, web, HttpResponse, ResponseError, Result};
use log::error;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
}

#[post("/auth/login")]
async fn login(data: web::Data<Repos>, form: web::Json<LoginRequest>) -> HttpResponse {
    // TODO: Implement proper authentication with password hashing
    // For now, we'll just return an error to indicate this is not implemented yet
    error!("Authentication not yet implemented");
    Error::Unauthorised.error_response()
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[post("/auth/register")]
async fn register(data: web::Data<Repos>, form: web::Json<RegisterRequest>) -> HttpResponse {
    // TODO: Implement proper registration with password hashing
    // For now, we'll just return an error to indicate this is not implemented yet
    error!("Registration not yet implemented");
    Error::Unauthorised.error_response()
}

#[get("/auth/me")]
async fn get_current_user(data: web::Data<Repos>) -> HttpResponse {
    let user_id = 1; // TODO implement getting logged in user

    match data.database.get_user_by_id(user_id as i32).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => Error::NotFound.error_response(),
        Err(e) => {
            error!("Database error: {e}");
            e.error_response()
        }
    }
}

// OAuth endpoints
#[get("/auth/google")]
async fn google_oauth() -> HttpResponse {
    // TODO: Implement Google OAuth
    error!("Google OAuth not yet implemented");
    Error::Unauthorised.error_response()
}

#[get("/auth/github")]
async fn github_oauth() -> HttpResponse {
    // TODO: Implement GitHub OAuth
    error!("GitHub OAuth not yet implemented");
    Error::Unauthorised.error_response()
}
