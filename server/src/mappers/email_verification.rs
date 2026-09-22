use crate::{ServerResult, error::Error, mappers::database::Database};

#[derive(Clone)]
pub struct EmailVerificationMapper {
    db: Database,
}

/// Identifies a valid, unexpired token row.
#[must_use]
pub struct EmailVerificationToken {
    pub id: i32,
    pub user_id: i32,
}

impl EmailVerificationMapper {
    /// Takes a **clone of the process-wide pool**, not a config to connect
    /// with — see [`Database::new`] for why there is only one.
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }

    /// Construct from a bare pool — for integration tests only.
    #[cfg(test)]
    #[must_use]
    pub const fn from_pool(pool: sqlx::PgPool) -> Self {
        Self {
            db: Database { pool },
        }
    }

    /// Atomically delete any existing token for `user_id` and insert a new one
    /// that expires 24 hours from now.
    ///
    /// # Errors
    /// Rolls back and propagates the error if either query fails.
    pub async fn replace_token(&self, user_id: i32, token_hash: &str) -> ServerResult<()> {
        crate::metrics::db::timed("email_verification.replace_token", async {
            let mut tx = self.db.pool.begin().await?;
            if let Err(e) = Self::replace_token_in_tx(&mut tx, user_id, token_hash).await {
                tx.rollback().await?;
                return Err(e);
            }
            tx.commit().await?;
            Ok(())
        })
        .await
    }

    pub(crate) async fn replace_token_in_tx(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
        token_hash: &str,
    ) -> ServerResult<()> {
        sqlx::query!(
            "DELETE FROM email_verification_tokens WHERE user_id = $1",
            user_id,
        )
        .execute(&mut *conn)
        .await?;

        sqlx::query!(
            "INSERT INTO email_verification_tokens (user_id, token_hash, expires_at) \
             VALUES ($1, $2, NOW() + INTERVAL '24 hours')",
            user_id,
            token_hash,
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Return the token row if it exists and has not expired.
    ///
    /// # Errors
    /// Returns `Error::InvalidVerificationToken` if no valid token matches.
    pub async fn find_valid_token(&self, token_hash: &str) -> ServerResult<EmailVerificationToken> {
        crate::metrics::db::timed("email_verification.find_valid_token", async {
            Self::find_valid_token_in_tx(&mut *self.db.pool.acquire().await?, token_hash).await
        })
        .await
    }

    pub(crate) async fn find_valid_token_in_tx(
        conn: &mut sqlx::PgConnection,
        token_hash: &str,
    ) -> ServerResult<EmailVerificationToken> {
        let row = sqlx::query!(
            "SELECT id, user_id FROM email_verification_tokens \
             WHERE token_hash = $1 AND expires_at > NOW()",
            token_hash,
        )
        .fetch_one(conn)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => Error::InvalidVerificationToken,
            other => Error::Database(other),
        })?;

        Ok(EmailVerificationToken {
            id: row.id,
            user_id: row.user_id,
        })
    }

    /// Atomically delete the verification token AND mark the user's email as
    /// verified. Both changes succeed or both are rolled back.
    ///
    /// # Errors
    /// Rolls back and propagates the error if either query fails.
    pub async fn consume_and_verify(&self, token_id: i32, user_id: i32) -> ServerResult<()> {
        crate::metrics::db::timed("email_verification.consume_and_verify", async {
            let mut tx = self.db.pool.begin().await?;
            if let Err(e) = Self::consume_and_verify_in_tx(&mut tx, token_id, user_id).await {
                tx.rollback().await?;
                return Err(e);
            }
            tx.commit().await?;
            Ok(())
        })
        .await
    }

    pub(crate) async fn consume_and_verify_in_tx(
        conn: &mut sqlx::PgConnection,
        token_id: i32,
        user_id: i32,
    ) -> ServerResult<()> {
        sqlx::query!(
            "DELETE FROM email_verification_tokens WHERE id = $1",
            token_id,
        )
        .execute(&mut *conn)
        .await?;

        sqlx::query!(
            "UPDATE users SET email_verified_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND deleted_at IS NULL",
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
    use crate::mappers::user::UserMapper;
    use crate::services::auth::hash_password;

    async fn seed_user(conn: &mut sqlx::PgConnection) -> i32 {
        let n: u64 = rand::random();
        UserMapper::create_user_in_tx(
            conn,
            &format!("evtest_{n}"),
            &format!("ev_{n}@test.com"),
            Some(&hash_password("TestPass12!@").unwrap()),
        )
        .await
        .unwrap()
        .id
    }

    #[tokio::test]
    async fn replace_token_creates_and_find_valid_returns_it() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;
        EmailVerificationMapper::replace_token_in_tx(&mut tx, user_id, "hash_abc")
            .await
            .unwrap();
        let token = EmailVerificationMapper::find_valid_token_in_tx(&mut tx, "hash_abc")
            .await
            .unwrap();
        assert_eq!(token.user_id, user_id);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn replace_token_removes_previous_token() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;
        EmailVerificationMapper::replace_token_in_tx(&mut tx, user_id, "old_hash")
            .await
            .unwrap();
        EmailVerificationMapper::replace_token_in_tx(&mut tx, user_id, "new_hash")
            .await
            .unwrap();
        assert!(
            EmailVerificationMapper::find_valid_token_in_tx(&mut tx, "old_hash")
                .await
                .is_err(),
            "old token must be gone"
        );
        let _tok = EmailVerificationMapper::find_valid_token_in_tx(&mut tx, "new_hash")
            .await
            .expect("new token must still be valid");
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn find_valid_token_not_found_returns_error() {
        let mut tx = crate::test_helpers::test_tx().await;
        assert!(
            EmailVerificationMapper::find_valid_token_in_tx(&mut tx, "nonexistent")
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn consume_and_verify_deletes_token_and_marks_user_verified() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = seed_user(&mut tx).await;
        EmailVerificationMapper::replace_token_in_tx(&mut tx, user_id, "consume_hash")
            .await
            .unwrap();
        let token = EmailVerificationMapper::find_valid_token_in_tx(&mut tx, "consume_hash")
            .await
            .unwrap();

        EmailVerificationMapper::consume_and_verify_in_tx(&mut tx, token.id, user_id)
            .await
            .unwrap();

        assert!(
            EmailVerificationMapper::find_valid_token_in_tx(&mut tx, "consume_hash")
                .await
                .is_err()
        );

        let verified_at: Option<chrono::NaiveDateTime> =
            sqlx::query_scalar("SELECT email_verified_at FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await
                .unwrap();
        assert!(
            verified_at.is_some(),
            "user should be verified after consume_and_verify"
        );
        tx.rollback().await.unwrap();
    }
}
