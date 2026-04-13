use crate::config::server::JwtSecret;
use crate::mappers::google_oauth::GoogleOauth;
use crate::mappers::user::UserMapper;
use crate::services::auth::generate_token;
use actix_web::{post, web, HttpResponse, ResponseError};
use log::error;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct GoogleOAuthRequest {
    /// Google ID token from the GSI client library
    pub token: String,
}

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
pub struct GoogleOAuthResponse {
    pub token: String,
    pub username: String,
    pub email: String,
    pub avatar: String,
}

/// Google OAuth callback endpoint
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
    google_request: web::Json<GoogleOAuthRequest>,
    jwt_secret: web::Data<JwtSecret>,
) -> HttpResponse {
    match google_oauth.validate_id_token(&google_request.token).await {
        Ok(google_user) => {
            let user = match user_mapper.get_user_by_email(&google_user.email).await {
                Ok(user) => user,
                Err(_) => {
                    // Create new user if doesn't exist
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

            let response = GoogleOAuthResponse {
                token,
                username: google_user.full_name,
                email: user.email,
                avatar: google_user.avatar_url,
            };

            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            error!("{e}");
            e.error_response()
        }
    }
}
