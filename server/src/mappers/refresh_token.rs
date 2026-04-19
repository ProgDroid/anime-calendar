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
        crate::metrics::db::timed("refresh_token.replace_token", async {
            let mut tx = self.db.pool.begin().await?;
            if let Err(e) = Self::replace_token_with(&mut tx, user_id, token_hash).await {
                tx.rollback().await?;
                return Err(e);
            }
            tx.commit().await?;
            Ok(())
        })
        .await
    }

    pub(crate) async fn replace_token_with(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
        token_hash: &str,
    ) -> ServerResult<()> {
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE user_id = $1",
            user_id,
        )
        .execute(&mut *conn)
        .await?;

        sqlx::query!(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) \
             VALUES ($1, $2, NOW() + INTERVAL '30 days')",
            user_id,
            token_hash,
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Return the token row if it exists, has not been used, and has not expired.
    /// Used by the refresh endpoint to validate an incoming refresh token.
    ///
    /// # Errors
    /// Returns `Error::Unauthorised` if no valid token matches.
    pub async fn find_valid_token(&self, token_hash: &str) -> ServerResult<RefreshToken> {
        crate::metrics::db::timed("refresh_token.find_valid_token", async {
            Self::find_valid_token_with(&mut *self.db.pool.acquire().await?, token_hash).await
        })
        .await
    }

    pub(crate) async fn find_valid_token_with(
        conn: &mut sqlx::PgConnection,
        token_hash: &str,
    ) -> ServerResult<RefreshToken> {
        let row = sqlx::query!(
            "SELECT id, user_id FROM refresh_tokens \
             WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()",
            token_hash,
        )
        .fetch_one(conn)
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
        crate::metrics::db::timed("refresh_token.rotate_token", async {
            let mut tx = self.db.pool.begin().await?;
            if let Err(e) =
                Self::rotate_token_with(&mut tx, old_token_id, user_id, new_token_hash).await
            {
                tx.rollback().await?;
                return Err(e);
            }
            tx.commit().await?;
            Ok(())
        })
        .await
    }

    pub(crate) async fn rotate_token_with(
        conn: &mut sqlx::PgConnection,
        old_token_id: i32,
        user_id: i32,
        new_token_hash: &str,
    ) -> ServerResult<()> {
        sqlx::query!(
            "UPDATE refresh_tokens SET used_at = NOW() WHERE id = $1",
            old_token_id,
        )
        .execute(&mut *conn)
        .await?;

        sqlx::query!(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) \
             VALUES ($1, $2, NOW() + INTERVAL '30 days')",
            user_id,
            new_token_hash,
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Invalidate all refresh tokens for a user — called on logout.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn invalidate_all_for_user(&self, user_id: i32) -> ServerResult<()> {
        crate::metrics::db::timed("refresh_token.invalidate_all", async {
            Self::invalidate_all_for_user_with(&mut *self.db.pool.acquire().await?, user_id).await
        })
        .await
    }

    pub(crate) async fn invalidate_all_for_user_with(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
    ) -> ServerResult<()> {
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE user_id = $1",
            user_id,
        )
        .execute(conn)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(raw: &str) -> String {
        use sha2::{Digest, Sha256};
        let h = Sha256::digest(raw.as_bytes());
        h.iter().fold(String::new(), |mut s, b| {
            let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{b:02x}"));
            s
        })
    }

    async fn seed_user(conn: &mut sqlx::PgConnection) -> i32 {
        let n: u64 = rand::random();
        sqlx::query_scalar(
            "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, 'hash') RETURNING id",
        )
        .bind(format!("tester_{n}"))
        .bind(format!("tester_{n}@test.com"))
        .fetch_one(conn)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn replace_token_stores_and_can_be_found() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;
        let raw = "raw_token_abc";
        let h = hash(raw);
        RefreshTokenMapper::replace_token_with(&mut tx, user_id, &h)
            .await
            .unwrap();
        let tok = RefreshTokenMapper::find_valid_token_with(&mut tx, &h)
            .await
            .unwrap();
        assert_eq!(tok.user_id, user_id);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn replace_token_clears_previous() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;
        let h1 = hash("first_token");
        let h2 = hash("second_token");
        RefreshTokenMapper::replace_token_with(&mut tx, user_id, &h1)
            .await
            .unwrap();
        RefreshTokenMapper::replace_token_with(&mut tx, user_id, &h2)
            .await
            .unwrap();
        assert!(RefreshTokenMapper::find_valid_token_with(&mut tx, &h1)
            .await
            .is_err());
        assert!(RefreshTokenMapper::find_valid_token_with(&mut tx, &h2)
            .await
            .is_ok());
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn rotate_token_marks_old_used_and_stores_new() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;
        let h1 = hash("first_token");
        RefreshTokenMapper::replace_token_with(&mut tx, user_id, &h1)
            .await
            .unwrap();
        let old = RefreshTokenMapper::find_valid_token_with(&mut tx, &h1)
            .await
            .unwrap();

        let h2 = hash("rotated_token");
        RefreshTokenMapper::rotate_token_with(&mut tx, old.id, user_id, &h2)
            .await
            .unwrap();

        assert!(RefreshTokenMapper::find_valid_token_with(&mut tx, &h1)
            .await
            .is_err());
        assert!(RefreshTokenMapper::find_valid_token_with(&mut tx, &h2)
            .await
            .is_ok());
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn invalidate_all_removes_tokens() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;
        let h = hash("some_token");
        RefreshTokenMapper::replace_token_with(&mut tx, user_id, &h)
            .await
            .unwrap();
        RefreshTokenMapper::invalidate_all_for_user_with(&mut tx, user_id)
            .await
            .unwrap();
        assert!(RefreshTokenMapper::find_valid_token_with(&mut tx, &h)
            .await
            .is_err());
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn missing_token_returns_unauthorised() {
        let mut tx = crate::test_helpers::test_tx().await;
        let result = RefreshTokenMapper::find_valid_token_with(&mut tx, "nonexistent_hash").await;
        assert!(matches!(result, Err(Error::Unauthorised)));
        tx.rollback().await.unwrap();
    }
}
