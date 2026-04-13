use crate::{
    cache::Cache,
    error::Error,
    mappers::user::UserMapper,
    middleware::auth::Claims,
    services::auth::{hash_password, validate_password},
};

use crate::entity::user_settings::UserSettings;
use crate::mappers::user_settings::UserSettingsMapper;
use actix_web::{delete, get, post, put, web, HttpResponse, ResponseError};
use log::{error, info};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UserRequest {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct UserResponse {
    pub username: String,
    pub email: String,
    pub is_oauth: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateUserRequest {
    pub username: String,
    pub email: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdatePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[utoipa::path(
    get,
    path = "/user/details",
    tag = "user",
    responses(
        (status = 200, body = UserResponse),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[get("/user/details")]
pub async fn get_user_details(user_mapper: web::Data<UserMapper>, claims: Claims) -> HttpResponse {
    match user_mapper.get_user_from_claims(&claims).await {
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

#[utoipa::path(
    put,
    path = "/user",
    tag = "user",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, body = UserResponse),
        (status = 400, body = crate::controllers::auth::ErrorResponse),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[put("/user")]
pub async fn update_user(
    user_mapper: web::Data<UserMapper>,
    cache: web::Data<Cache>,
    claims: Claims,
    user_data: web::Json<UpdateUserRequest>,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // Validate username length
    if user_data.username.len() > 50 {
        return Error::InvalidRequest.error_response();
    }

    let user = user_mapper
        .update_user(
            user.id,
            user_data.username.as_ref(),
            user_data.email.as_ref(),
        )
        .await;

    match user {
        Ok(user) => {
            // Invalidate user cache
            let _ = cache.invalidate_user_details(user.id).await;

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

#[utoipa::path(
    post,
    path = "/user/password",
    tag = "user",
    request_body = UpdatePasswordRequest,
    responses(
        (status = 200, description = "Password updated"),
        (status = 400, body = crate::controllers::auth::ErrorResponse),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[post("/user/password")]
pub async fn update_password(
    user_mapper: web::Data<UserMapper>,
    cache: web::Data<Cache>,
    claims: Claims,
    password_data: web::Json<UpdatePasswordRequest>,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // Check that new password is different from current password
    if password_data.new_password == password_data.current_password {
        return Error::InvalidRequest.error_response();
    }

    // Validate password length
    if password_data.new_password.len() > 128 {
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
    let hashed_password = match hash_password(&password_data.new_password) {
        Ok(hashed_password) => hashed_password,
        Err(e) => return e.error_response(),
    };

    // Update password in database
    match user_mapper
        .update_user_password(user.id, &hashed_password)
        .await
    {
        Ok(()) => {
            // Invalidate user cache
            let _ = cache.invalidate_user_details(user.id).await;
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            error!("{e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[utoipa::path(
    delete,
    path = "/user/{id}",
    tag = "user",
    params(("id" = i32, Path, description = "User ID to delete (must match authenticated user)")),
    responses(
        (status = 204, description = "User deleted"),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
        (status = 403, description = "Forbidden — cannot delete another user"),
    ),
    security(("bearer_auth" = []))
)]
#[delete("/user/{id}")]
pub async fn delete_user(
    user_mapper: web::Data<UserMapper>,
    cache: web::Data<Cache>,
    user_id: web::Path<i32>,
    claims: Claims,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    let user_id_inner = user_id.into_inner();

    if user.id != user_id_inner {
        return HttpResponse::Forbidden().finish();
    }

    match user_mapper.delete_user(user_id_inner).await {
        Ok(()) => {
            // Invalidate user cache
            let _ = cache.invalidate_user_details(user_id_inner).await;
            HttpResponse::NoContent().finish()
        }
        Err(e) => {
            error!("{e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

// User settings endpoints
#[utoipa::path(
    get,
    path = "/user/settings",
    tag = "user",
    responses(
        (status = 200, body = crate::entity::user_settings::UserSettings),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[get("/user/settings")]
pub async fn get_user_settings(
    user_settings_mapper: web::Data<UserSettingsMapper>,
    claims: Claims,
) -> HttpResponse {
    let Ok(user_id) = claims.sub.parse::<i32>() else {
        return Error::Unauthorised.error_response();
    };

    match user_settings_mapper.get_user_settings(user_id).await {
        Ok(settings) => HttpResponse::Ok().json(settings),
        Err(e) => e.error_response(),
    }
}

#[utoipa::path(
    put,
    path = "/user/settings",
    tag = "user",
    request_body = crate::entity::user_settings::UserSettings,
    responses(
        (status = 200, description = "Settings updated"),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[put("/user/settings")]
pub async fn update_user_settings(
    user_settings_mapper: web::Data<UserSettingsMapper>,
    cache: web::Data<Cache>,
    claims: Claims,
    settings_data: web::Json<UserSettings>,
) -> HttpResponse {
    let Ok(user_id) = claims.sub.parse::<i32>() else {
        return Error::Unauthorised.error_response();
    };

    info!("{settings_data:?}");
    match user_settings_mapper
        .update_user_settings(user_id, &settings_data)
        .await
    {
        Ok(()) => {
            let _ = cache.invalidate_user_settings(user_id).await;
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            error!("Failed to update user settings: {e:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

// TODO actually use these settings
// TODO default to user's browser locale (navigator.langauges?)
// TODO finish translations for rest of files (it refuses to go beyond Login and Register)
// TODO refactor frontend. Use frontend and Vue expertise skills. It's a mess rn
