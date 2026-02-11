#![allow(unused_variables)]
use crate::{
    error::Error,
    middleware::auth::{get_user_from_claims, Claims},
    server::Repos,
    services::auth::{hash_password, validate_password},
};

use actix_web::{delete, get, post, put, web, HttpResponse, ResponseError};
use log::{error, info};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct UserRequest {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub username: String,
    pub email: String,
    pub is_oauth: bool,
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub username: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct UpdatePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[get("/user/details")]
pub async fn get_user_details(data: web::Data<Repos>, claims: Claims) -> HttpResponse {
    match get_user_from_claims(&claims, &data.database).await {
        Ok(user) => {
            let response = UserResponse {
                username: if user.password_hash.is_none() {
                    String::new()
                } else {
                    user.username
                },
                email: user.email,
                is_oauth: user.password_hash.is_none(),
            };

            HttpResponse::Ok().json(response)
        }
        Err(e) => e.error_response(),
    }
}

#[put("/user")]
pub async fn update_user(
    data: web::Data<Repos>,
    claims: Claims,
    user_data: web::Json<UpdateUserRequest>,
) -> HttpResponse {
    let user = match get_user_from_claims(&claims, &data.database).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    let user = data
        .database
        .update_user(
            user.id,
            user_data.username.as_ref(),
            user_data.email.as_ref(),
        )
        .await;

    match user {
        Ok(user) => {
            let response = UserResponse {
                username: if user.password_hash.is_none() {
                    String::new()
                } else {
                    user.username
                },
                email: user.email,
                is_oauth: user.password_hash.is_none(),
            };

            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            error!("{e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/user/password")]
pub async fn update_password(
    data: web::Data<Repos>,
    claims: Claims,
    password_data: web::Json<UpdatePasswordRequest>,
) -> HttpResponse {
    let user = match get_user_from_claims(&claims, &data.database).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // Check that new password is different from current password
    if password_data.new_password == password_data.current_password {
        return Error::InvalidRequest.error_response();
    }

    // Verify current password
    if let Some(hash) = user.password_hash {
        if !validate_password(&password_data.current_password, &hash) {
            return Error::Unauthorised.error_response();
        }
    } else {
        info!("OAuth user attempted to change password: {}", user.id);
        return Error::InvalidRequest.error_response();
    }

    // Hash new password
    let hashed_password = hash_password(&password_data.new_password);

    // Update password in database
    match data
        .database
        .update_user_password(user.id, &hashed_password)
        .await
    {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("{e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[delete("/user/{id}")]
pub async fn delete_user(data: web::Data<Repos>, user_id: web::Path<i32>) -> HttpResponse {
    match data.database.delete_user(user_id.into_inner()).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => {
            error!("{e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[cfg(test)]
mod user_tests {
    use crate::controllers::user;
    use actix_web::{test, App, Result};
    use serde_json::json;

    #[actix_web::test]
    async fn test_update_user() -> Result<()> {
        let app = test::init_service(App::new().service(user::update_user)).await;

        let req = test::TestRequest::put()
            .uri("/user")
            .set_json(json!({
                "username": "updateduser",
                "email": "updated@example.com"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        // This should fail without authentication
        assert!(resp.status().is_client_error());

        Ok(())
    }

    #[actix_web::test]
    async fn test_delete_user() -> Result<()> {
        let app = test::init_service(App::new().service(user::delete_user)).await;

        let req = test::TestRequest::delete().uri("/user").to_request();

        let resp = test::call_service(&app, req).await;
        // This should fail without authentication
        assert!(resp.status().is_client_error());

        Ok(())
    }
}
