use crate::config::server::{CookieSettings, JwtSecret};
use crate::controllers::auth::build_auth_cookie;
use crate::mappers::google_oauth::GoogleOauth;
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
                        Ok(user) => {
                            // Mark the email as verified — Google has already confirmed ownership.
                            if let Err(e) = user_mapper.mark_email_verified(user.id).await {
                                error!("Failed to mark OAuth user {} as verified: {e}", user.id);
                                return e.error_response();
                            }
                            user
                        }
                        Err(e) => return e.error_response(),
                    }
                }
            };

            let token = match generate_token(&user.id, jwt_secret.expose_secret()) {
                Ok(token) => token,
                Err(e) => return e.error_response(),
            };

            let cookie = build_auth_cookie(token, &cookie_settings);
            let response = GoogleOAuthResponse {
                username: google_user.full_name,
                email: user.email,
                avatar: google_user.avatar_url,
            };

            HttpResponse::Ok().cookie(cookie).json(response)
        }
        Err(e) => {
            error!("{e}");
            e.error_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mappers::user::UserMapper;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../migrations")]
    async fn oauth_create_user_is_unverified_until_mark_called(pool: PgPool) {
        let mapper = UserMapper::from_pool(pool.clone());
        let user = mapper.create_user("oauthtest", "oauth@test.com", None).await.unwrap();
        assert!(user.email_verified_at.is_none(), "freshly created user is unverified");
        mapper.mark_email_verified(user.id).await.unwrap();
        let fetched = mapper.get_user_by_id(user.id).await.unwrap();
        assert!(fetched.email_verified_at.is_some());
    }
}
