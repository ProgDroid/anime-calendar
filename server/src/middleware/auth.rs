use crate::config::server::JwtSecret;
use crate::error::Error;
use actix_web::{FromRequest, HttpRequest, dev::Payload};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

pub const ISS: &str = "anime-calendar";
pub const AUD: &str = "anime-calendar";

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iss: String,
    pub aud: String,
}

impl Claims {
    /// # Errors
    /// Returns `Error::Unauthorised` if `sub` is not a valid i32.
    pub fn user_id(&self) -> Result<i32, Error> {
        self.sub.parse().map_err(|_| Error::Unauthorised)
    }
}

impl FromRequest for Claims {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let fut = async move {
            let token = req
                .cookie("auth_token")
                .ok_or(Error::Unauthorised)?
                .value()
                .to_owned();

            let Some(jwt_secret) = req.app_data::<actix_web::web::Data<JwtSecret>>() else {
                return Err(Error::Unauthorised);
            };

            let secret = jwt_secret.expose_secret().as_bytes();
            let decoding_key = DecodingKey::from_secret(secret);
            let mut validation = Validation::default();
            validation.set_issuer(&[ISS]);
            validation.set_audience(&[AUD]);

            let token_data = decode::<Self>(&token, &decoding_key, &validation)
                .map_err(|_| Error::Unauthorised)?;

            Ok(token_data.claims)
        };

        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{App, HttpResponse, get, http::StatusCode, test, web};
    use jsonwebtoken::{EncodingKey, Header, encode};

    use super::*;
    use crate::config::server::JwtSecret;
    use crate::services::auth::generate_token;

    const SECRET: &str = "test-secret-key-for-testing-only";

    fn secret_data() -> web::Data<JwtSecret> {
        web::Data::new(JwtSecret::new(secrecy::SecretString::from(
            SECRET.to_owned(),
        )))
    }

    #[get("/protected")]
    async fn guarded(_claims: Claims) -> HttpResponse {
        HttpResponse::Ok().finish()
    }

    #[actix_web::test]
    async fn valid_jwt_allows_access() {
        let app = test::init_service(App::new().app_data(secret_data()).service(guarded)).await;
        let token = generate_token(&99, SECRET).unwrap();
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn expired_jwt_returns_401() {
        let app = test::init_service(App::new().app_data(secret_data()).service(guarded)).await;
        let claims = Claims {
            sub: "1".to_owned(),
            exp: 0,
            iss: ISS.to_owned(),
            aud: AUD.to_owned(),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap();
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[actix_web::test]
    async fn no_cookie_returns_401() {
        let app = test::init_service(App::new().app_data(secret_data()).service(guarded)).await;
        let req = test::TestRequest::get().uri("/protected").to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[actix_web::test]
    async fn wrong_cookie_name_returns_401() {
        let app = test::init_service(App::new().app_data(secret_data()).service(guarded)).await;
        let token = generate_token(&1, SECRET).unwrap();
        // "session" instead of "auth_token"
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Cookie", format!("session={token}")))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[actix_web::test]
    async fn wrong_secret_returns_401() {
        let app = test::init_service(App::new().app_data(secret_data()).service(guarded)).await;
        let token = generate_token(&1, "a-completely-different-secret!!").unwrap();
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
}
