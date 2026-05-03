#![allow(
    clippy::cast_possible_truncation,
    clippy::missing_panics_doc,
    clippy::cast_sign_loss
)]

use argon2::{
    Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, encode};
use log::error;
use rand::RngCore as _;
use sha2::{Digest as _, Sha256};
use std::fmt::Write;

use crate::{ServerResult, middleware::auth::Claims};

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
pub fn generate_token<T: AsRef<[u8]>>(id: &i32, jwt_secret: T) -> ServerResult<String> {
    use crate::middleware::auth::{AUD, ISS};
    let claims = Claims {
        sub: id.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::minutes(30)).timestamp() as usize,
        iss: ISS.to_owned(),
        aud: AUD.to_owned(),
    };

    let encoding_key = EncodingKey::from_secret(jwt_secret.as_ref());
    Ok(encode(&Header::default(), &claims, &encoding_key)?)
}

/// Generate a cryptographically random 32-byte token as a 64-char hex string.
#[must_use]
pub fn generate_random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hash(&bytes)
}

/// SHA-256 hash a raw token, returning a 64-char hex string.
#[must_use]
pub fn hash_token(raw_token: &str) -> String {
    let sha_hash = Sha256::digest(raw_token.as_bytes());
    hash(&sha_hash)
}

fn hash(data: &[u8]) -> String {
    data.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

// Add a function to verify token
/// # Errors
/// Returns an error if the token is invalid or expired
pub fn verify_token<T: AsRef<[u8]>>(token: &str, jwt_secret: T) -> ServerResult<Claims> {
    use crate::middleware::auth::{AUD, ISS};
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_ref());
    let mut validation = jsonwebtoken::Validation::default();
    validation.set_issuer(&[ISS]);
    validation.set_audience(&[AUD]);
    let token_data = jsonwebtoken::decode::<Claims>(token, &decoding_key, &validation)?;
    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- password hashing / validation ---

    #[test]
    fn test_password_validation_correct_password() {
        let password = "password123";
        let hashed = hash_password(password).unwrap();
        assert!(validate_password(password, &hashed));
    }

    #[test]
    fn test_password_validation_wrong_password() {
        let hashed = hash_password("correcthorse").unwrap();
        assert!(!validate_password("wronghorse", &hashed));
    }

    #[test]
    fn test_password_validation_rejects_invalid_hash() {
        assert!(!validate_password("anything", "notahash"));
    }

    // --- password strength ---

    #[test]
    fn test_password_strength_valid() {
        assert!(validate_password_strength("Correct!Horse1Battery"));
    }

    #[test]
    fn test_password_strength_too_short() {
        assert!(!validate_password_strength("Sh0rt!"));
    }

    #[test]
    fn test_password_strength_no_uppercase() {
        assert!(!validate_password_strength("nouppercase1!aaaaaa"));
    }

    #[test]
    fn test_password_strength_no_lowercase() {
        assert!(!validate_password_strength("NOLOWERCASE1!AAAA"));
    }

    #[test]
    fn test_password_strength_no_digit() {
        assert!(!validate_password_strength("NoDigitsHere!abcde"));
    }

    #[test]
    fn test_password_strength_no_special_char() {
        assert!(!validate_password_strength("NoSpecialChar1abcd"));
    }

    // --- JWT generation / verification ---

    #[test]
    fn test_generate_and_verify_token_round_trip() {
        let secret = "test-jwt-secret";
        let user_id = 42_i32;
        let token = generate_token(&user_id, secret).unwrap();
        let claims = verify_token(&token, secret).unwrap();
        assert_eq!(claims.sub, "42");
    }

    #[test]
    fn test_verify_token_rejects_wrong_secret() {
        let token = generate_token(&1, "correct-secret").unwrap();
        let result = verify_token(&token, "wrong-secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_rejects_garbage_input() {
        let result = verify_token("not.a.jwt", "secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_token_sets_sub_to_user_id_string() {
        let token = generate_token(&99, "s3cr3t").unwrap();
        let claims = verify_token(&token, "s3cr3t").unwrap();
        assert_eq!(claims.sub, "99");
    }

    #[test]
    fn test_generate_token_sets_future_expiry() {
        let token = generate_token(&1, "s").unwrap();
        let claims = verify_token(&token, "s").unwrap();
        let now = chrono::Utc::now().timestamp() as usize;
        assert!(claims.exp > now);
    }

    // --- token generation / hashing ---

    #[test]
    fn hash_token_is_deterministic_and_64_hex_chars() {
        let h1 = hash_token("abc");
        let h2 = hash_token("abc");
        assert_eq!(h1, h2, "same input must produce same hash");
        assert_eq!(h1.len(), 64, "SHA-256 hex is 64 chars");
        assert_ne!(
            hash_token("abc"),
            hash_token("xyz"),
            "different inputs differ"
        );
    }

    #[test]
    fn generate_random_token_is_64_hex_and_unique() {
        let t1 = generate_random_token();
        let t2 = generate_random_token();
        assert_eq!(t1.len(), 64);
        assert_ne!(t1, t2, "two calls should produce different tokens");
        assert!(
            t1.chars().all(|c| c.is_ascii_hexdigit()),
            "output must be lowercase hex"
        );
    }
}
