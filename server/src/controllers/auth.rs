use crate::error::Error;
use crate::middleware::auth::Claims;
use crate::server::Repos;
use crate::services::auth::{generate_token, hash_password, validate_password, verify_token};
use actix_web::{get, post, web, HttpResponse, Result};
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
    db: web::Data<Repos>,
    credentials: web::Json<LoginRequest>,
) -> Result<HttpResponse, Error> {
    let user = db.database.get_user_by_email(&credentials.email).await?;

    match user.password_hash {
        Some(hash) => {
            if validate_password(&credentials.password, &hash) {
                let token = generate_token(&user.id);
                let response = LoginResponse {
                    token,
                    username: user.username,
                };
                Ok(HttpResponse::Ok().json(response))
            } else {
                Err(Error::Unauthorised)
            }
        }
        None => Err(Error::NotImplemented),
    }
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[post("/register")]
pub async fn register(
    db: web::Data<Repos>,
    user_data: web::Json<RegisterRequest>,
) -> Result<HttpResponse, Error> {
    // Check if user already exists
    if (db.database.get_user_by_email(&user_data.email).await).is_ok() {
        return Err(Error::UserAlreadyExists);
    }

    let hashed_password = hash_password(&user_data.password);

    let user = db
        .database
        .create_user(&user_data.username, &user_data.email, &hashed_password)
        .await?;

    let token = generate_token(&user.id);
    let response = LoginResponse {
        token,
        username: user.username,
    };

    Ok(HttpResponse::Ok().json(response))
}

#[get("/user")]
pub async fn get_current_user(db: web::Data<Repos>, claims: Claims) -> Result<HttpResponse, Error> {
    let user_id = claims.sub.parse::<i32>().map_err(|_| Error::Unauthorised)?;
    let user = db.database.get_user_by_id(user_id).await?;

    let response = LoginResponse {
        token: String::new(), // Not returning token for this endpoint
        username: user.username,
    };

    Ok(HttpResponse::Ok().json(response))
}

#[post("/auth/verify")]
pub async fn verify_token_endpoint(token: web::Json<String>) -> Result<HttpResponse, Error> {
    match verify_token(&token.0) {
        Ok(_) => Ok(HttpResponse::Ok().json("Token is valid")),
        Err(_) => Err(Error::Unauthorised),
    }
}

#[cfg(test)]
mod tests {
    use crate::controllers::auth;
    use actix_web::{test, App, Result};
    use serde_json::json;

    #[actix_web::test]
    async fn test_register() -> Result<()> {
        let app = test::init_service(App::new().service(auth::register)).await;

        let req = test::TestRequest::post()
            .uri("/register")
            .set_json(json!({
                "username": "testuser",
                "email": "test@example.com",
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        Ok(())
    }

    #[actix_web::test]
    async fn test_login() -> Result<()> {
        let app = test::init_service(App::new().service(auth::login)).await;

        let req = test::TestRequest::post()
            .uri("/login")
            .set_json(json!({
                "email": "test@example.com",
                "password": "password123"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        Ok(())
    }
}
