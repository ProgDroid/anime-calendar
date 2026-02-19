use crate::server::Repos;
use crate::services::auth::generate_token;
use actix_web::{post, web, HttpResponse, ResponseError};
use log::error;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct GoogleOAuthRequest {
    pub token: String,
}

#[derive(Deserialize, Serialize)]
pub struct GoogleOAuthResponse {
    pub token: String,
    pub username: String,
    pub email: String,
    pub avatar: String,
}

/// Google OAuth callback endpoint
#[post("/auth/google")]
pub async fn google_oauth(
    db: web::Data<Repos>,
    google_request: web::Json<GoogleOAuthRequest>,
) -> HttpResponse {
    match db
        .google_oauth
        .validate_id_token(&google_request.token)
        .await
    {
        Ok(google_user) => {
            let user = match db.database.get_user_by_email(&google_user.email).await {
                Ok(user) => user,
                Err(_) => {
                    // Create new user if doesn't exist
                    match db
                        .database
                        .create_user(&google_user.id, &google_user.email, None)
                        .await
                    {
                        Ok(user) => user,
                        Err(e) => return e.error_response(),
                    }
                }
            };

            let token = match generate_token(&user.id) {
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
