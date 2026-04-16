use crate::{
    config::database::Database as DatabaseConfig, mappers::database::Database, ServerResult,
};

#[derive(Clone)]
pub struct PasswordResetMapper {
    db: Database,
}

#[must_use]
pub struct PasswordResetToken {
    pub id: i32,
    pub user_id: i32,
}

impl PasswordResetMapper {
    /// # Errors
    /// Fails if the database connection cannot be established.
    pub async fn new(config: DatabaseConfig) -> ServerResult<Self> {
        Ok(Self {
            db: Database::new(config).await?,
        })
    }

    /// Construct from a bare pool — for integration tests only.
    #[cfg(test)]
    #[must_use]
    pub const fn from_pool(pool: sqlx::PgPool) -> Self {
        Self {
            db: Database { pool },
        }
    }

    /// Delete all tokens for the given user (expired, used, or pending).
    /// Call before `create_token` to keep the table tidy.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn invalidate_previous_tokens(&self, user_id: i32) -> ServerResult<()> {
        sqlx::query!(
            "DELETE FROM password_reset_tokens WHERE user_id = $1",
            user_id,
        )
        .execute(&self.db.pool)
        .await?;
        Ok(())
    }

    /// Store a new reset token that expires 1 hour from now.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn create_token(&self, user_id: i32, token_hash: &str) -> ServerResult<()> {
        sqlx::query!(
            "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at) \
             VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
            user_id,
            token_hash,
        )
        .execute(&self.db.pool)
        .await?;
        Ok(())
    }

    /// Atomically invalidate all existing tokens for `user_id` and insert a new one.
    /// Prefer this over calling `invalidate_previous_tokens` + `create_token` separately
    /// to avoid a race window where two concurrent requests could leave two valid tokens.
    ///
    /// # Errors
    /// Rolls back and propagates the error if either query fails.
    pub async fn replace_token(&self, user_id: i32, token_hash: &str) -> ServerResult<()> {
        let mut tx = self.db.pool.begin().await?;

        if let Err(e) = sqlx::query!(
            "DELETE FROM password_reset_tokens WHERE user_id = $1",
            user_id,
        )
        .execute(&mut *tx)
        .await
        {
            tx.rollback().await?;
            return Err(e.into());
        }

        if let Err(e) = sqlx::query!(
            "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at) \
             VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
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

    /// Return the token row if it exists, is not yet used, and has not expired.
    ///
    /// # Errors
    /// Returns `Error::Database` (row-not-found) if no valid token matches.
    pub async fn find_valid_token(&self, token_hash: &str) -> ServerResult<PasswordResetToken> {
        let row = sqlx::query!(
            "SELECT id, user_id FROM password_reset_tokens \
             WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()",
            token_hash,
        )
        .fetch_one(&self.db.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => crate::error::Error::InvalidResetToken,
            other => crate::error::Error::Database(other),
        })?;

        Ok(PasswordResetToken {
            id: row.id,
            user_id: row.user_id,
        })
    }

    /// Mark the token as used AND update the user's password in one transaction.
    ///
    /// # Errors
    /// Rolls back and propagates the error if either query fails.
    pub async fn complete_reset(
        &self,
        token_id: i32,
        user_id: i32,
        password_hash: &str,
    ) -> ServerResult<()> {
        let mut tx = self.db.pool.begin().await?;

        if let Err(e) = sqlx::query!(
            "UPDATE password_reset_tokens SET used_at = NOW() WHERE id = $1",
            token_id,
        )
        .execute(&mut *tx)
        .await
        {
            tx.rollback().await?;
            return Err(e.into());
        }

        if let Err(e) = sqlx::query!(
            "UPDATE users SET password_hash = $1, updated_at = NOW() \
             WHERE id = $2 AND deleted_at IS NULL",
            password_hash,
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
    use crate::{mappers::user::UserMapper, services::auth::hash_password};
    use sqlx::PgPool;

    const PW: &str = "TestPass12!@";

    async fn seed_user(pool: &PgPool, email: &str) -> i32 {
        UserMapper::from_pool(pool.clone())
            .create_user("testuser", email, Some(&hash_password(PW).unwrap()))
            .await
            .unwrap()
            .id
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn create_and_find_valid_token(pool: PgPool) {
        let user_id = seed_user(&pool, "a@test.com").await;
        let mapper = PasswordResetMapper::from_pool(pool);
        mapper.create_token(user_id, "hash_abc").await.unwrap();

        let token = mapper.find_valid_token("hash_abc").await.unwrap();
        assert_eq!(token.user_id, user_id);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn find_valid_token_not_found_returns_error(pool: PgPool) {
        let mapper = PasswordResetMapper::from_pool(pool);
        assert!(mapper.find_valid_token("nonexistent").await.is_err());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn complete_reset_marks_token_used(pool: PgPool) {
        let user_id = seed_user(&pool, "b@test.com").await;
        let mapper = PasswordResetMapper::from_pool(pool);
        mapper.create_token(user_id, "hash_used").await.unwrap();
        let token = mapper.find_valid_token("hash_used").await.unwrap();

        let new_hash = hash_password("NewPass12!@").unwrap();
        mapper
            .complete_reset(token.id, user_id, &new_hash)
            .await
            .unwrap();

        // Token is now used — find_valid_token must fail
        assert!(mapper.find_valid_token("hash_used").await.is_err());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn invalidate_previous_tokens_removes_all_for_user(pool: PgPool) {
        let user_id = seed_user(&pool, "c@test.com").await;
        let mapper = PasswordResetMapper::from_pool(pool);
        mapper.create_token(user_id, "hash_1").await.unwrap();
        mapper.create_token(user_id, "hash_2").await.unwrap();

        mapper.invalidate_previous_tokens(user_id).await.unwrap();

        assert!(mapper.find_valid_token("hash_1").await.is_err());
        assert!(mapper.find_valid_token("hash_2").await.is_err());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn replace_token_is_atomic(pool: PgPool) {
        let user_id = seed_user(&pool, "d@test.com").await;
        let mapper = PasswordResetMapper::from_pool(pool);
        // Seed an old token
        mapper.create_token(user_id, "old_hash").await.unwrap();

        // replace_token atomically deletes old + inserts new
        mapper.replace_token(user_id, "new_hash").await.unwrap();

        // Old token is gone
        assert!(mapper.find_valid_token("old_hash").await.is_err());
        // New token is valid
        let token = mapper.find_valid_token("new_hash").await.unwrap();
        assert_eq!(token.user_id, user_id);
    }
}
