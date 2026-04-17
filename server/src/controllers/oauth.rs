use crate::config::server::{CookieSettings, JwtSecret};
use crate::controllers::auth::{build_auth_cookie, build_refresh_cookie, generate_raw_token, hash_refresh_token};
use crate::mappers::google_oauth::GoogleOauth;
use crate::mappers::refresh_token::RefreshTokenMapper;
use crate::mappers::user::UserMapper;
use crate::services::auth::generate_token;
use actix_web::{post, web, HttpResponse, ResponseError};
use log::error;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct GoogleOAuthRequest {
    pub token: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct GoogleOAuthResponse {
    pub username: String,
    pub email: String,
    pub avatar: String,
}

#[utoipa::path(
    post,
    path = "/auth/google",
    tag = "auth",
    request_body = GoogleOAuthRequest,
    responses(
        (status = 200, description = "OAuth login successful", body = GoogleOAuthResponse),
        (status = 401, description = "Invalid Google ID token", body = crate::controllers::auth::ErrorResponse),
    )
)]
#[post("/auth/google")]
pub async fn google_oauth(
    user_mapper: web::Data<UserMapper>,
    google_oauth: web::Data<GoogleOauth>,
    refresh_mapper: web::Data<RefreshTokenMapper>,
    google_request: web::Json<GoogleOAuthRequest>,
    jwt_secret: web::Data<JwtSecret>,
    cookie_settings: web::Data<CookieSettings>,
) -> HttpResponse {
    match google_oauth.validate_id_token(&google_request.token).await {
        Ok(google_user) => {
            let user = match user_mapper.get_user_by_email(&google_user.email).await {
                Ok(user) => user,
                Err(_) => {
                    match user_mapper
                        .create_user(&google_user.id, &google_user.email, None)
                        .await
                    {
                        Ok(user) => user,
                        Err(e) => return e.error_response(),
                    }
                }
            };

            let token = match generate_token(&user.id, jwt_secret.expose_secret()) {
                Ok(token) => token,
                Err(e) => return e.error_response(),
            };

            let raw_refresh = generate_raw_token();
            let refresh_hash = hash_refresh_token(&raw_refresh);
            if let Err(e) = refresh_mapper.replace_token(user.id, &refresh_hash).await {
                return e.error_response();
            }

            let auth_cookie = build_auth_cookie(token, &cookie_settings);
            let refresh_cookie = build_refresh_cookie(raw_refresh, &cookie_settings);
            let response = GoogleOAuthResponse {
                username: google_user.full_name,
                email: user.email,
                avatar: google_user.avatar_url,
            };

            HttpResponse::Ok()
                .cookie(auth_cookie)
                .cookie(refresh_cookie)
                .json(response)
        }
        Err(e) => {
            error!("{e}");
            e.error_response()
        }
    }
}
