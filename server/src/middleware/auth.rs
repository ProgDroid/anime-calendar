use crate::config::server::JwtSecret;
use crate::error::Error;
use actix_web::{dev::Payload, FromRequest, HttpRequest};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

impl FromRequest for Claims {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let fut = async move {
            let auth_header = req
                .headers()
                .get("authorization")
                .ok_or(Error::Unauthorised)?
                .to_str()
                .map_err(|_| Error::Unauthorised)?;

            let token = auth_header
                .strip_prefix("Bearer ")
                .ok_or(Error::Unauthorised)?;

            let Some(jwt_secret) = req.app_data::<actix_web::web::Data<JwtSecret>>() else {
                return Err(Error::Unauthorised);
            };

            let secret = jwt_secret.expose_secret().as_bytes();
            let decoding_key = DecodingKey::from_secret(secret);
            let validation = Validation::default();

            let token_data = decode::<Self>(token, &decoding_key, &validation)
                .map_err(|_| Error::Unauthorised)?;

            Ok(token_data.claims)
        };

        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::server::JwtSecret;
    use crate::services::auth::generate_token;
    use actix_web::{get, http::StatusCode, test, web, App, HttpResponse};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use secrecy::SecretString;

    const SECRET: &str = "test-jwt-secret-at-least-32-bytes";

    /// Minimal protected endpoint: extracts Claims, proves the middleware passed.
    #[get("/protected")]
    async fn guarded(claims: Claims) -> HttpResponse {
        HttpResponse::Ok().json(serde_json::json!({ "sub": claims.sub }))
    }

    fn secret_data() -> web::Data<JwtSecret> {
        web::Data::new(JwtSecret::new(SecretString::from(SECRET)))
    }

    #[actix_web::test]
    async fn valid_jwt_allows_access() {
        let app = test::init_service(
            App::new().app_data(secret_data()).service(guarded),
        )
        .await;
        let token = generate_token(&99, SECRET).unwrap();
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn expired_jwt_returns_401() {
        let app = test::init_service(
            App::new().app_data(secret_data()).service(guarded),
        )
        .await;
        // exp = 0 → 1970-01-01, always expired.
        let claims = Claims { sub: "1".to_owned(), exp: 0 };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap();
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn missing_authorization_header_returns_401() {
        let app = test::init_service(
            App::new().app_data(secret_data()).service(guarded),
        )
        .await;
        let req = test::TestRequest::get().uri("/protected").to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn non_bearer_scheme_returns_401() {
        let app = test::init_service(
            App::new().app_data(secret_data()).service(guarded),
        )
        .await;
        let token = generate_token(&1, SECRET).unwrap();
        // "Token " prefix instead of "Bearer "
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Token {token}")))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn wrong_secret_returns_401() {
        let app = test::init_service(
            App::new().app_data(secret_data()).service(guarded),
        )
        .await;
        let token = generate_token(&1, "a-completely-different-secret!!").unwrap();
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
    }
}
