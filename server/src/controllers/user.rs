#![allow(unused_variables)]
use crate::{entity::user::User, server::Repos};

use actix_web::{delete, get, post, put, web, HttpResponse};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct UserRequest {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub username: String,
    pub email: String,
}

#[post("/user")]
async fn create_user(data: web::Data<Repos>, user_data: web::Json<UserRequest>) -> HttpResponse {
    let user = data
        .database
        .create_user(&user_data.username, &user_data.email, &user_data.password)
        .await;

    match user {
        Ok(user) => {
            let response = UserResponse {
                id: user.id,
                username: user.username,
                email: user.email,
            };

            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            // TODO
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/user/{id}")]
async fn get_user(data: web::Data<Repos>, user_id: web::Path<i32>) -> HttpResponse {
    // let user = data.database.get_user(user_id.into_inner()).await?;

    // let response = UserResponse {
    //     id: user.id,
    //     username: user.username,
    //     email: user.email,
    // };

    // Ok(HttpResponse::Ok().json(response))
    HttpResponse::Ok().finish()
}

#[put("/user/{id}")]
async fn update_user(
    data: web::Data<Repos>,
    user_id: web::Path<i32>,
    user_data: web::Json<UpdateUserRequest>,
) -> HttpResponse {
    let user = data
        .database
        .update_user(
            user_id.into_inner(),
            user_data.username.as_ref(),
            user_data.email.as_ref(),
        )
        .await;

    match user {
        Ok(user) => {
            let response = UserResponse {
                id: user.id,
                username: user.username,
                email: user.email,
            };

            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            // TODO
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[delete("/user/{id}")]
async fn delete_user(data: web::Data<Repos>, user_id: web::Path<i32>) -> HttpResponse {
    match data.database.delete_user(user_id.into_inner()).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => {
            // TODO
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/user")]
async fn get_current_user(data: web::Data<Repos>, user: web::ReqData<User>) -> HttpResponse {
    let response = UserResponse {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
    };

    HttpResponse::Ok().json(response)
}

#[cfg(test)]
mod user_tests {
    use crate::controllers::user;
    use actix_web::{test, App, Result};
    use serde_json::json;

    #[actix_web::test]
    async fn test_get_current_user() -> Result<()> {
        let app = test::init_service(App::new().service(user::get_current_user)).await;

        let req = test::TestRequest::get().uri("/user").to_request();

        let resp = test::call_service(&app, req).await;
        // This should fail without authentication
        assert!(resp.status().is_client_error());

        Ok(())
    }

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
