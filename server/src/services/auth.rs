#![allow(
    clippy::cast_possible_truncation,
    clippy::missing_panics_doc,
    clippy::cast_sign_loss
)]

use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier,
};
use jsonwebtoken::{encode, DecodingKey, EncodingKey, Header};
use log::error;
use serde::{Deserialize, Serialize};
use std::env;

use crate::ServerResult;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

/// # Errors
/// Fails if password cannot be hashed
pub fn hash_password(password: &str) -> ServerResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let params = Params::default();
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    Ok(argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

#[must_use]
pub fn validate_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(hash) => hash,
        Err(e) => {
            error!("{e}");
            return false;
        }
    };

    let argon2 = Argon2::default();

    argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[must_use]
pub fn validate_password_strength(password: &str) -> bool {
    // Password must be at least 12 characters long
    if password.len() < 12 {
        return false;
    }

    // Password must contain at least one uppercase letter
    if !password.chars().any(char::is_uppercase) {
        return false;
    }

    // Password must contain at least one lowercase letter
    if !password.chars().any(char::is_lowercase) {
        return false;
    }

    // Password must contain at least one digit
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }

    // Password must contain at least one special character
    if !password.chars().any(|c| !c.is_alphanumeric()) {
        return false;
    }

    true
}

/// # Errors
/// Fails if claims cannot be encoded
pub fn generate_token(id: &i32) -> ServerResult<String> {
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string()); // TODO ?
    let claims = Claims {
        sub: id.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
    };

    let encoding_key = EncodingKey::from_secret(secret.as_ref());
    Ok(encode(&Header::default(), &claims, &encoding_key)?)
}

// Add a function to verify token
/// # Errors
/// Returns an error if the token is invalid or expired
pub fn verify_token(token: &str) -> ServerResult<Claims> {
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string()); // TODO probably shouldn't get this far if we have no secret
    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    let token_data =
        jsonwebtoken::decode::<Claims>(token, &decoding_key, &jsonwebtoken::Validation::default())?;
    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_validation() {
        let password = "password123";
        let hashed = hash_password(password).unwrap();
        assert!(validate_password(password, &hashed));
    }
}
