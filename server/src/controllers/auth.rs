use crate::error::Error;
use crate::middleware::auth::{get_user_from_claims, Claims};
use crate::server::Repos;
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
pub async fn login(db: web::Data<Repos>, credentials: web::Json<LoginRequest>) -> HttpResponse {
    let user = match db.database.get_user_by_email(&credentials.email).await {
        Ok(user) => user,
        Err(e) => return e.error_response(),
    };

    match user.password_hash {
        Some(hash) => {
            if validate_password(&credentials.password, &hash) {
                let token = match generate_token(&user.id) {
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
        None => Error::NotImplemented.error_response(),
    }
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String, // TODO secret?
}

#[post("/register")]
pub async fn register(db: web::Data<Repos>, user_data: web::Json<RegisterRequest>) -> HttpResponse {
    // Check if user already exists
    if (db.database.get_user_by_email(&user_data.email).await).is_ok() {
        return Error::UserAlreadyExists.error_response();
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
        .database
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

    let token = match generate_token(&user.id) {
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
pub async fn get_current_user(db: web::Data<Repos>, claims: Claims) -> HttpResponse {
    let user = match get_user_from_claims(&claims, &db.database).await {
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
pub async fn verify_token_endpoint(token: web::Json<AuthVerifyRequest>) -> HttpResponse {
    match verify_token(&token.token) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => Error::Unauthorised.error_response(),
    }
}

// TODO commented out until I work out how to create fake repos to set up application for tests
// #[cfg(test)]
// mod tests {
//     use crate::controllers::auth;
//     use actix_web::{test, App, Result};
//     use serde_json::json;

//     #[actix_web::test]
//     async fn test_register() -> Result<()> {
//         let app = test::init_service(App::new().service(auth::register)).await;

//         let req = test::TestRequest::post()
//             .uri("/register")
//             .set_json(json!({
//                 "username": "testuser",
//                 "email": "test@example.com",
//                 "password": "password123"
//             }))
//             .to_request();

//         let resp = test::call_service(&app, req).await;
//         assert!(resp.status().is_success());

//         Ok(())
//     }

//     #[actix_web::test]
//     async fn test_login() -> Result<()> {
//         let app = test::init_service(App::new().service(auth::login)).await;

//         let req = test::TestRequest::post()
//             .uri("/login")
//             .set_json(json!({
//                 "email": "test@example.com",
//                 "password": "password123"
//             }))
//             .to_request();

//         let resp = test::call_service(&app, req).await;
//         assert!(resp.status().is_success());

//         Ok(())
//     }
// }
