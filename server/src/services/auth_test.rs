#[cfg(test)]
mod auth_tests {
    use crate::auth;
    use bcrypt::verify;

    #[test]
    fn test_password_hashing() {
        let password = "password123";
        let hashed = auth::hash_password(password).unwrap();
        assert!(verify(password, &hashed).unwrap());
    }

    #[test]
    fn test_password_verification() {
        let password = "password123";
        let hashed = auth::hash_password(password).unwrap();
        assert!(auth::verify_password(password, &hashed).unwrap());
    }
}
