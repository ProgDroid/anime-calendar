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
        crate::metrics::db::timed("password_reset.invalidate_tokens", async {
            Self::invalidate_previous_tokens_with(&mut *self.db.pool.acquire().await?, user_id)
                .await
        })
        .await
    }

    pub(crate) async fn invalidate_previous_tokens_with(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
    ) -> ServerResult<()> {
        sqlx::query!(
            "DELETE FROM password_reset_tokens WHERE user_id = $1",
            user_id,
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Store a new reset token that expires 1 hour from now.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn create_token(&self, user_id: i32, token_hash: &str) -> ServerResult<()> {
        crate::metrics::db::timed("password_reset.create_token", async {
            Self::create_token_with(&mut *self.db.pool.acquire().await?, user_id, token_hash).await
        })
        .await
    }

    pub(crate) async fn create_token_with(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
        token_hash: &str,
    ) -> ServerResult<()> {
        sqlx::query!(
            "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at) \
             VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
            user_id,
            token_hash,
        )
        .execute(conn)
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
        crate::metrics::db::timed("password_reset.replace_token", async {
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
            "DELETE FROM password_reset_tokens WHERE user_id = $1",
            user_id,
        )
        .execute(&mut *conn)
        .await?;

        sqlx::query!(
            "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at) \
             VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
            user_id,
            token_hash,
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Return the token row if it exists, is not yet used, and has not expired.
    ///
    /// # Errors
    /// Returns `Error::Database` (row-not-found) if no valid token matches.
    pub async fn find_valid_token(&self, token_hash: &str) -> ServerResult<PasswordResetToken> {
        crate::metrics::db::timed("password_reset.find_valid_token", async {
            Self::find_valid_token_with(&mut *self.db.pool.acquire().await?, token_hash).await
        })
        .await
    }

    pub(crate) async fn find_valid_token_with(
        conn: &mut sqlx::PgConnection,
        token_hash: &str,
    ) -> ServerResult<PasswordResetToken> {
        let row = sqlx::query!(
            "SELECT id, user_id FROM password_reset_tokens \
             WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()",
            token_hash,
        )
        .fetch_one(conn)
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
        crate::metrics::db::timed("password_reset.complete_reset", async {
            let mut tx = self.db.pool.begin().await?;
            if let Err(e) =
                Self::complete_reset_with(&mut tx, token_id, user_id, password_hash).await
            {
                tx.rollback().await?;
                return Err(e);
            }
            tx.commit().await?;
            Ok(())
        })
        .await
    }

    pub(crate) async fn complete_reset_with(
        conn: &mut sqlx::PgConnection,
        token_id: i32,
        user_id: i32,
        password_hash: &str,
    ) -> ServerResult<()> {
        sqlx::query!(
            "UPDATE password_reset_tokens SET used_at = NOW() WHERE id = $1",
            token_id,
        )
        .execute(&mut *conn)
        .await?;

        sqlx::query!(
            "UPDATE users SET password_hash = $1, updated_at = NOW() \
             WHERE id = $2 AND deleted_at IS NULL",
            password_hash,
            user_id,
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mappers::user::UserMapper, services::auth::hash_password};

    const PW: &str = "TestPass12!@";

    async fn seed_user(conn: &mut sqlx::PgConnection, email: &str) -> i32 {
        let n: u64 = rand::random();
        UserMapper::create_user_with(
            conn,
            &format!("resetuser_{n}"),
            email,
            Some(&hash_password(PW).unwrap()),
        )
        .await
        .unwrap()
        .id
    }

    #[tokio::test]
    async fn create_and_find_valid_token() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx, "a@test.com").await;
        PasswordResetMapper::create_token_with(&mut tx, user_id, "hash_abc")
            .await
            .unwrap();

        let token = PasswordResetMapper::find_valid_token_with(&mut tx, "hash_abc")
            .await
            .unwrap();
        assert_eq!(token.user_id, user_id);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn find_valid_token_not_found_returns_error() {
        let mut tx = crate::test_helpers::test_tx().await;
        assert!(
            PasswordResetMapper::find_valid_token_with(&mut tx, "nonexistent")
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn complete_reset_marks_token_used() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx, "b@test.com").await;
        PasswordResetMapper::create_token_with(&mut tx, user_id, "hash_used")
            .await
            .unwrap();
        let token = PasswordResetMapper::find_valid_token_with(&mut tx, "hash_used")
            .await
            .unwrap();

        let new_hash = hash_password("NewPass12!@").unwrap();
        PasswordResetMapper::complete_reset_with(&mut tx, token.id, user_id, &new_hash)
            .await
            .unwrap();

        assert!(
            PasswordResetMapper::find_valid_token_with(&mut tx, "hash_used")
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn invalidate_previous_tokens_removes_all_for_user() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx, "c@test.com").await;
        PasswordResetMapper::create_token_with(&mut tx, user_id, "hash_1")
            .await
            .unwrap();
        PasswordResetMapper::create_token_with(&mut tx, user_id, "hash_2")
            .await
            .unwrap();

        PasswordResetMapper::invalidate_previous_tokens_with(&mut tx, user_id)
            .await
            .unwrap();

        assert!(
            PasswordResetMapper::find_valid_token_with(&mut tx, "hash_1")
                .await
                .is_err()
        );
        assert!(
            PasswordResetMapper::find_valid_token_with(&mut tx, "hash_2")
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn replace_token_is_atomic() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx, "d@test.com").await;
        PasswordResetMapper::create_token_with(&mut tx, user_id, "old_hash")
            .await
            .unwrap();

        PasswordResetMapper::replace_token_with(&mut tx, user_id, "new_hash")
            .await
            .unwrap();

        assert!(
            PasswordResetMapper::find_valid_token_with(&mut tx, "old_hash")
                .await
                .is_err()
        );
        let token = PasswordResetMapper::find_valid_token_with(&mut tx, "new_hash")
            .await
            .unwrap();
        assert_eq!(token.user_id, user_id);
        tx.rollback().await.unwrap();
    }
}
