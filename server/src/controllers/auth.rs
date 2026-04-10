use crate::config::server::Server as ServerConfig;
use crate::error::Error;
use crate::mappers::user::UserMapper;
use crate::middleware::auth::Claims;
use crate::services::auth::{
    generate_token, hash_password, validate_password, validate_password_strength, verify_token,
};
use actix_web::{get, post, web, HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
}

#[post("/login")]
pub async fn login(
    db: web::Data<UserMapper>,
    credentials: web::Json<LoginRequest>,
    config: web::Data<ServerConfig>,
) -> HttpResponse {
    let Ok(user) = db.get_user_by_email(&credentials.email).await else {
        return Error::Unauthorised.error_response();
    };

    match user.password_hash {
        Some(hash) => {
            if validate_password(&credentials.password, &hash) {
                let token = match generate_token(&user.id, config.jwt_secret.clone()) {
                    Ok(token) => token,
                    Err(e) => return e.error_response(),
                };

                let response = LoginResponse {
                    token,
                    username: user.username,
                };
                HttpResponse::Ok().json(response)
            } else {
                Error::Unauthorised.error_response()
            }
        }
        None => Error::Unauthorised.error_response(),
    }
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String, // TODO secret?
}

#[post("/register")]
pub async fn register(
    db: web::Data<UserMapper>,
    user_data: web::Json<RegisterRequest>,
    config: web::Data<ServerConfig>,
) -> HttpResponse {
    // Check if user already exists
    if (db.get_user_by_email(&user_data.email).await).is_ok() {
        return Error::UserAlreadyExists.error_response();
    }

    // Validate username length
    if user_data.username.len() > 50 {
        return Error::InvalidRequest.error_response();
    }

    // Validate password length
    if user_data.password.len() > 128 {
        return Error::InvalidRequest.error_response();
    }

    // Validate password strength
    if !validate_password_strength(&user_data.password) {
        return Error::InvalidPassword.error_response();
    }

    let hashed_password = match hash_password(&user_data.password) {
        Ok(hashed_password) => hashed_password,
        Err(e) => return e.error_response(),
    };

    let user = match db
        .create_user(
            &user_data.username,
            &user_data.email,
            Some(&hashed_password),
        )
        .await
    {
        Ok(user) => user,
        Err(e) => return e.error_response(),
    };

    let token = match generate_token(&user.id, config.jwt_secret.clone()) {
        Ok(token) => token,
        Err(e) => return e.error_response(),
    };

    let response = LoginResponse {
        token,
        username: user.username,
    };

    HttpResponse::Ok().json(response)
}

#[get("/user")]
pub async fn get_current_user(db: web::Data<UserMapper>, claims: Claims) -> HttpResponse {
    let user = match db.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => return e.error_response(),
    };

    let response = LoginResponse {
        token: String::new(), // Not returning token for this endpoint
        username: user.username,
    };

    HttpResponse::Ok().json(response)
}

#[derive(Deserialize)]
struct AuthVerifyRequest {
    token: String,
}

#[post("/auth/verify")]
pub async fn verify_token_endpoint(
    token: web::Json<AuthVerifyRequest>,
    config: web::Data<ServerConfig>,
) -> HttpResponse {
    match verify_token(&token.token, config.jwt_secret.clone()) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => Error::Unauthorised.error_response(),
    }
}
