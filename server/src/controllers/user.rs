#![allow(unused_variables)]
use crate::{
    middleware::auth::{get_user_from_claims, Claims},
    server::Repos,
};

use actix_web::{delete, get, put, web, HttpResponse, ResponseError};
use log::error;
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
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub username: String,
    pub email: String,
}

#[get("/user/details")]
pub async fn get_user_details(data: web::Data<Repos>, claims: Claims) -> HttpResponse {
    match get_user_from_claims(&claims, &data.database).await {
        Ok(user) => {
            let response = UserResponse {
                username: user.username,
                email: user.email,
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
                username: user.username,
                email: user.email,
            };

            HttpResponse::Ok().json(response)
        }
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
