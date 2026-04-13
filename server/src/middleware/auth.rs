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
