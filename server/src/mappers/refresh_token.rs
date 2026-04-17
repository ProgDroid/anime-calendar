use crate::{
    config::database::Database as DatabaseConfig, error::Error, mappers::database::Database,
    ServerResult,
};

#[derive(Clone)]
pub struct RefreshTokenMapper {
    db: Database,
}

#[must_use]
pub struct RefreshToken {
    pub id: i32,
    pub user_id: i32,
}

impl RefreshTokenMapper {
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

    /// Atomically invalidate all existing refresh tokens for `user_id` and insert a new one.
    /// Prevents a race window where two concurrent logins could leave two valid tokens.
    ///
    /// # Errors
    /// Rolls back and propagates the error if either query fails.
    pub async fn replace_token(&self, user_id: i32, token_hash: &str) -> ServerResult<()> {
        let mut tx = self.db.pool.begin().await?;

        if let Err(e) = sqlx::query!(
            "DELETE FROM refresh_tokens WHERE user_id = $1",
            user_id,
        )
        .execute(&mut *tx)
        .await
        {
            tx.rollback().await?;
            return Err(e.into());
        }

        if let Err(e) = sqlx::query!(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) \
             VALUES ($1, $2, NOW() + INTERVAL '30 days')",
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

    /// Return the token row if it exists, has not been used, and has not expired.
    /// Used by the refresh endpoint to validate an incoming refresh token.
    ///
    /// # Errors
    /// Returns `Error::Unauthorised` if no valid token matches.
    pub async fn find_valid_token(&self, token_hash: &str) -> ServerResult<RefreshToken> {
        let row = sqlx::query!(
            "SELECT id, user_id FROM refresh_tokens \
             WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()",
            token_hash,
        )
        .fetch_one(&self.db.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => Error::Unauthorised,
            other => Error::Database(other),
        })?;

        Ok(RefreshToken {
            id: row.id,
            user_id: row.user_id,
        })
    }

    /// Mark a token as used.  Called atomically inside `rotate_token`.
    async fn mark_used(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, token_id: i32) -> ServerResult<()> {
        sqlx::query!(
            "UPDATE refresh_tokens SET used_at = NOW() WHERE id = $1",
            token_id,
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Rotate a token: atomically mark the old one used and insert a new one.
    /// This is the correct operation for the refresh endpoint.
    ///
    /// # Errors
    /// Rolls back and propagates the error if any query fails.
    pub async fn rotate_token(
        &self,
        old_token_id: i32,
        user_id: i32,
        new_token_hash: &str,
    ) -> ServerResult<()> {
        let mut tx = self.db.pool.begin().await?;

        if let Err(e) = self.mark_used(&mut tx, old_token_id).await {
            tx.rollback().await?;
            return Err(e);
        }

        if let Err(e) = sqlx::query!(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) \
             VALUES ($1, $2, NOW() + INTERVAL '30 days')",
            user_id,
            new_token_hash,
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

    /// Invalidate all refresh tokens for a user — called on logout.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn invalidate_all_for_user(&self, user_id: i32) -> ServerResult<()> {
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE user_id = $1",
            user_id,
        )
        .execute(&self.db.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    fn hash(raw: &str) -> String {
        use sha2::{Digest, Sha256};
        let h = Sha256::digest(raw.as_bytes());
        h.iter().fold(String::new(), |mut s, b| {
            let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{b:02x}"));
            s
        })
    }

    async fn seed_user(pool: &PgPool) -> i32 {
        sqlx::query_scalar!(
            "INSERT INTO users (username, email, password_hash) \
             VALUES ('tester', 'tester@test.com', 'hash') RETURNING id"
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn replace_token_stores_and_can_be_found(pool: PgPool) {
        let user_id = seed_user(&pool).await;
        let mapper = RefreshTokenMapper::from_pool(pool);
        let raw = "raw_token_abc";
        let h = hash(raw);
        mapper.replace_token(user_id, &h).await.unwrap();
        let tok = mapper.find_valid_token(&h).await.unwrap();
        assert_eq!(tok.user_id, user_id);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn replace_token_clears_previous(pool: PgPool) {
        let user_id = seed_user(&pool).await;
        let mapper = RefreshTokenMapper::from_pool(pool);
        let h1 = hash("first_token");
        let h2 = hash("second_token");
        mapper.replace_token(user_id, &h1).await.unwrap();
        mapper.replace_token(user_id, &h2).await.unwrap();
        // old token should no longer be valid
        assert!(mapper.find_valid_token(&h1).await.is_err());
        // new token should be valid
        assert!(mapper.find_valid_token(&h2).await.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn rotate_token_marks_old_used_and_stores_new(pool: PgPool) {
        let user_id = seed_user(&pool).await;
        let mapper = RefreshTokenMapper::from_pool(pool);
        let h1 = hash("first_token");
        mapper.replace_token(user_id, &h1).await.unwrap();
        let old = mapper.find_valid_token(&h1).await.unwrap();

        let h2 = hash("rotated_token");
        mapper.rotate_token(old.id, user_id, &h2).await.unwrap();

        // old must be invalid
        assert!(mapper.find_valid_token(&h1).await.is_err());
        // new must be valid
        assert!(mapper.find_valid_token(&h2).await.is_ok());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn invalidate_all_removes_tokens(pool: PgPool) {
        let user_id = seed_user(&pool).await;
        let mapper = RefreshTokenMapper::from_pool(pool);
        let h = hash("some_token");
        mapper.replace_token(user_id, &h).await.unwrap();
        mapper.invalidate_all_for_user(user_id).await.unwrap();
        assert!(mapper.find_valid_token(&h).await.is_err());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn missing_token_returns_unauthorised(pool: PgPool) {
        let mapper = RefreshTokenMapper::from_pool(pool);
        let result = mapper.find_valid_token("nonexistent_hash").await;
        assert!(matches!(result, Err(Error::Unauthorised)));
    }
}
