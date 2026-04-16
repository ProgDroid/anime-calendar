use crate::{
    config::database::Database as DatabaseConfig,
    error::Error,
    mappers::database::Database,
    ServerResult,
};

#[derive(Clone)]
pub struct EmailVerificationMapper {
    db: Database,
}

/// Identifies a valid, unexpired token row.
pub struct EmailVerificationToken {
    pub id: i32,
    pub user_id: i32,
}

impl EmailVerificationMapper {
    /// # Errors
    /// Fails if the database connection cannot be established.
    pub async fn new(config: DatabaseConfig) -> ServerResult<Self> {
        Ok(Self { db: Database::new(config).await? })
    }

    /// Construct from a bare pool — for integration tests only.
    #[cfg(test)]
    #[must_use]
    pub const fn from_pool(pool: sqlx::PgPool) -> Self {
        Self { db: Database { pool } }
    }

    /// Atomically delete any existing token for `user_id` and insert a new one
    /// that expires 24 hours from now.
    ///
    /// # Errors
    /// Rolls back and propagates the error if either query fails.
    pub async fn replace_token(&self, user_id: i32, token_hash: &str) -> ServerResult<()> {
        let mut tx = self.db.pool.begin().await?;

        if let Err(e) = sqlx::query!(
            "DELETE FROM email_verification_tokens WHERE user_id = $1",
            user_id,
        )
        .execute(&mut *tx)
        .await
        {
            tx.rollback().await?;
            return Err(e.into());
        }

        if let Err(e) = sqlx::query!(
            "INSERT INTO email_verification_tokens (user_id, token_hash, expires_at) \
             VALUES ($1, $2, NOW() + INTERVAL '24 hours')",
            user_id,
            token_hash,
        )
        .execute(&mut *tx)
        .await
        {
            tx.rollback().await?;
            return Err(e.into());
        }

        tx.commit().await?;
        Ok(())
    }

    /// Return the token row if it exists and has not expired.
    ///
    /// # Errors
    /// Returns `Error::InvalidVerificationToken` if no valid token matches.
    pub async fn find_valid_token(
        &self,
        token_hash: &str,
    ) -> ServerResult<EmailVerificationToken> {
        let row = sqlx::query!(
            "SELECT id, user_id FROM email_verification_tokens \
             WHERE token_hash = $1 AND expires_at > NOW()",
            token_hash,
        )
        .fetch_one(&self.db.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => Error::InvalidVerificationToken,
            other => Error::Database(other),
        })?;

        Ok(EmailVerificationToken { id: row.id, user_id: row.user_id })
    }

    /// Atomically delete the verification token AND mark the user's email as
    /// verified. Both changes succeed or both are rolled back.
    ///
    /// # Errors
    /// Rolls back and propagates the error if either query fails.
    pub async fn consume_and_verify(&self, token_id: i32, user_id: i32) -> ServerResult<()> {
        let mut tx = self.db.pool.begin().await?;

        if let Err(e) = sqlx::query!(
            "DELETE FROM email_verification_tokens WHERE id = $1",
            token_id,
        )
        .execute(&mut *tx)
        .await
        {
            tx.rollback().await?;
            return Err(e.into());
        }

        if let Err(e) = sqlx::query!(
            "UPDATE users SET email_verified_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND deleted_at IS NULL",
            user_id,
        )
        .execute(&mut *tx)
        .await
        {
            tx.rollback().await?;
            return Err(e.into());
        }

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mappers::user::UserMapper;
    use crate::services::auth::hash_password;
    use sqlx::PgPool;

    async fn seed_user(pool: &PgPool) -> i32 {
        let mapper = UserMapper::from_pool(pool.clone());
        let user = mapper
            .create_user("evtest", "ev@test.com", Some(&hash_password("TestPass12!@").unwrap()))
            .await
            .unwrap();
        user.id
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn replace_token_creates_and_find_valid_returns_it(pool: PgPool) {
        let user_id = seed_user(&pool).await;
        let mapper = EmailVerificationMapper::from_pool(pool);
        mapper.replace_token(user_id, "hash_abc").await.unwrap();
        let token = mapper.find_valid_token("hash_abc").await.unwrap();
        assert_eq!(token.user_id, user_id);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn replace_token_removes_previous_token(pool: PgPool) {
        let user_id = seed_user(&pool).await;
        let mapper = EmailVerificationMapper::from_pool(pool);
        mapper.replace_token(user_id, "old_hash").await.unwrap();
        mapper.replace_token(user_id, "new_hash").await.unwrap();
        assert!(mapper.find_valid_token("old_hash").await.is_err(), "old token must be gone");
        mapper.find_valid_token("new_hash").await.unwrap();
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn find_valid_token_not_found_returns_error(pool: PgPool) {
        let mapper = EmailVerificationMapper::from_pool(pool);
        assert!(mapper.find_valid_token("nonexistent").await.is_err());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn consume_and_verify_deletes_token_and_marks_user_verified(pool: PgPool) {
        let user_id = seed_user(&pool).await;
        let mapper = EmailVerificationMapper::from_pool(pool.clone());
        mapper.replace_token(user_id, "consume_hash").await.unwrap();
        let token = mapper.find_valid_token("consume_hash").await.unwrap();

        mapper.consume_and_verify(token.id, user_id).await.unwrap();

        // Token is gone
        assert!(mapper.find_valid_token("consume_hash").await.is_err());

        // User is now verified
        let verified_at: Option<chrono::NaiveDateTime> =
            sqlx::query_scalar("SELECT email_verified_at FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(verified_at.is_some(), "user should be verified after consume_and_verify");
    }
}
