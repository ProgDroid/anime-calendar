use crate::{
    config::database::Database as DatabaseConfig, entity::user::User, error::Error,
    mappers::database::Database, middleware::auth::Claims, ServerResult,
};

#[derive(Clone)]
pub struct UserMapper {
    db: Database,
}

impl UserMapper {
    /// # Errors
    /// Fails if database connection fails
    pub async fn new(config: DatabaseConfig) -> ServerResult<Self> {
        Ok(Self {
            db: Database::new(config).await?,
        })
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn get_user_by_email(&self, email: &str) -> ServerResult<User> {
        Self::get_user_by_email_with(&mut *self.db.pool.acquire().await?, email).await
    }

    pub(crate) async fn get_user_by_email_with(
        conn: &mut sqlx::PgConnection,
        email: &str,
    ) -> ServerResult<User> {
        let user = sqlx::query!(
            "SELECT id, username, email, password_hash, email_verified_at, created_at, updated_at FROM users WHERE email = $1 AND deleted_at IS NULL",
            email
        )
        .fetch_one(conn)
        .await?;

        Ok(User {
            id: user.id,
            username: user.username,
            email: user.email,
            password_hash: user.password_hash,
            email_verified_at: user.email_verified_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn create_user(
        &self,
        username: &str,
        email: &str,
        password_hash: Option<&str>,
    ) -> ServerResult<User> {
        Self::create_user_with(&mut *self.db.pool.acquire().await?, username, email, password_hash)
            .await
    }

    pub(crate) async fn create_user_with(
        conn: &mut sqlx::PgConnection,
        username: &str,
        email: &str,
        password_hash: Option<&str>,
    ) -> ServerResult<User> {
        let user = sqlx::query!(
            "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id, username, email, password_hash, email_verified_at, created_at, updated_at",
            username,
            email,
            password_hash
        )
        .fetch_one(conn)
        .await?;

        Ok(User {
            id: user.id,
            username: user.username,
            email: user.email,
            password_hash: user.password_hash,
            email_verified_at: user.email_verified_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn get_user_by_id(&self, id: i32) -> ServerResult<User> {
        Self::get_user_by_id_with(&mut *self.db.pool.acquire().await?, id).await
    }

    pub(crate) async fn get_user_by_id_with(
        conn: &mut sqlx::PgConnection,
        id: i32,
    ) -> ServerResult<User> {
        let user = sqlx::query!(
            "SELECT id, username, email, password_hash, email_verified_at, created_at, updated_at FROM users WHERE id = $1 AND deleted_at IS NULL",
            id
        )
        .fetch_one(conn)
        .await?;

        Ok(User {
            id: user.id,
            username: user.username,
            email: user.email,
            password_hash: user.password_hash,
            email_verified_at: user.email_verified_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn update_user(&self, id: i32, username: &str, email: &str) -> ServerResult<User> {
        Self::update_user_with(&mut *self.db.pool.acquire().await?, id, username, email).await
    }

    pub(crate) async fn update_user_with(
        conn: &mut sqlx::PgConnection,
        id: i32,
        username: &str,
        email: &str,
    ) -> ServerResult<User> {
        let user = sqlx::query!(
            "UPDATE users SET username = $1, email = $2, updated_at = NOW() WHERE id = $3 AND deleted_at IS NULL RETURNING id, username, email, password_hash, email_verified_at, created_at, updated_at",
            username,
            email,
            id
        )
        .fetch_one(conn)
        .await?;

        Ok(User {
            id: user.id,
            username: user.username,
            email: user.email,
            password_hash: user.password_hash,
            email_verified_at: user.email_verified_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn update_user_password(&self, id: i32, password_hash: &str) -> ServerResult<()> {
        Self::update_user_password_with(&mut *self.db.pool.acquire().await?, id, password_hash)
            .await
    }

    pub(crate) async fn update_user_password_with(
        conn: &mut sqlx::PgConnection,
        id: i32,
        password_hash: &str,
    ) -> ServerResult<()> {
        sqlx::query!(
            "UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2 AND deleted_at IS NULL",
            password_hash,
            id
        )
        .execute(conn)
        .await?;

        Ok(())
    }

    /// Set `email_verified_at` to the current timestamp for the given user.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn mark_email_verified(&self, user_id: i32) -> ServerResult<()> {
        Self::mark_email_verified_with(&mut *self.db.pool.acquire().await?, user_id).await
    }

    pub(crate) async fn mark_email_verified_with(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
    ) -> ServerResult<()> {
        sqlx::query!(
            "UPDATE users SET email_verified_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND deleted_at IS NULL",
            user_id,
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Soft-deletes the user and cascades to all their calendars in a single transaction.
    ///
    /// # Errors
    /// Returns an error if the query fails
    pub async fn delete_user(&self, id: i32) -> ServerResult<()> {
        let mut tx = self.db.pool.begin().await?;

        if let Err(e) = Self::delete_user_with(&mut *tx, id).await {
            tx.rollback().await?;
            return Err(e);
        }

        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn delete_user_with(
        conn: &mut sqlx::PgConnection,
        id: i32,
    ) -> ServerResult<()> {
        sqlx::query!(
            "UPDATE calendars SET deleted_at = NOW() WHERE user_id = $1 AND deleted_at IS NULL",
            id
        )
        .execute(&mut *conn)
        .await?;

        sqlx::query!(
            "UPDATE users SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
            id
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Construct a mapper from a bare pool — for integration tests only.
    #[cfg(test)]
    #[must_use]
    pub const fn from_pool(pool: sqlx::PgPool) -> Self {
        Self {
            db: Database { pool },
        }
    }

    /// # Errors
    /// Fails if user ID cannot be extracted from Claims
    pub async fn get_user_from_claims(&self, claims: &Claims) -> ServerResult<User> {
        let user_id = claims.sub.parse::<i32>().map_err(|_| Error::Unauthorised)?;

        self.get_user_by_id(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_user_returns_correct_fields() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user = UserMapper::create_user_with(
            &mut *tx,
            "alice",
            "alice@example.com",
            Some("hash123"),
        )
        .await
        .unwrap();
        assert_eq!(user.username, "alice");
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.password_hash.as_deref(), Some("hash123"));
        assert!(user.id > 0);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn create_user_oauth_has_no_password() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user = UserMapper::create_user_with(&mut *tx, "oauthuser", "oauth@example.com", None)
            .await
            .unwrap();
        assert!(user.password_hash.is_none());
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_user_by_email_finds_existing() {
        let mut tx = crate::test_helpers::test_tx().await;
        let created = UserMapper::create_user_with(&mut *tx, "bob", "bob@example.com", None)
            .await
            .unwrap();
        let found = UserMapper::get_user_by_email_with(&mut *tx, "bob@example.com")
            .await
            .unwrap();
        assert_eq!(found.id, created.id);
        assert_eq!(found.username, "bob");
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_user_by_email_not_found() {
        let mut tx = crate::test_helpers::test_tx().await;
        assert!(
            UserMapper::get_user_by_email_with(&mut *tx, "nobody@example.com")
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_user_by_id_finds_existing() {
        let mut tx = crate::test_helpers::test_tx().await;
        let created =
            UserMapper::create_user_with(&mut *tx, "carol", "carol@example.com", None)
                .await
                .unwrap();
        let fetched = UserMapper::get_user_by_id_with(&mut *tx, created.id)
            .await
            .unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.email, "carol@example.com");
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_user_by_id_not_found() {
        let mut tx = crate::test_helpers::test_tx().await;
        assert!(UserMapper::get_user_by_id_with(&mut *tx, i32::MAX)
            .await
            .is_err());
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn update_user_changes_username_and_email() {
        let mut tx = crate::test_helpers::test_tx().await;
        let created = UserMapper::create_user_with(&mut *tx, "dave", "dave@example.com", None)
            .await
            .unwrap();
        let updated =
            UserMapper::update_user_with(&mut *tx, created.id, "david", "david@example.com")
                .await
                .unwrap();
        assert_eq!(updated.username, "david");
        assert_eq!(updated.email, "david@example.com");
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn update_user_password_persists() {
        let mut tx = crate::test_helpers::test_tx().await;
        let created =
            UserMapper::create_user_with(&mut *tx, "eve", "eve@example.com", Some("oldhash"))
                .await
                .unwrap();
        UserMapper::update_user_password_with(&mut *tx, created.id, "newhash")
            .await
            .unwrap();
        let fetched = UserMapper::get_user_by_id_with(&mut *tx, created.id)
            .await
            .unwrap();
        assert_eq!(fetched.password_hash.as_deref(), Some("newhash"));
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn delete_user_soft_deletes_so_lookup_fails() {
        let mut tx = crate::test_helpers::test_tx().await;
        let created =
            UserMapper::create_user_with(&mut *tx, "frank", "frank@example.com", None)
                .await
                .unwrap();
        UserMapper::delete_user_with(&mut *tx, created.id)
            .await
            .unwrap();
        assert!(UserMapper::get_user_by_id_with(&mut *tx, created.id)
            .await
            .is_err());
        assert!(
            UserMapper::get_user_by_email_with(&mut *tx, "frank@example.com")
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn mark_email_verified_sets_timestamp() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user =
            UserMapper::create_user_with(&mut *tx, "vera", "vera@example.com", Some("hash"))
                .await
                .unwrap();
        assert!(
            user.email_verified_at.is_none(),
            "new user should be unverified"
        );
        UserMapper::mark_email_verified_with(&mut *tx, user.id)
            .await
            .unwrap();
        let fetched = UserMapper::get_user_by_id_with(&mut *tx, user.id)
            .await
            .unwrap();
        assert!(
            fetched.email_verified_at.is_some(),
            "should be verified after mark"
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn delete_user_cascades_soft_delete_to_calendars() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user =
            UserMapper::create_user_with(&mut *tx, "grace", "grace@example.com", None)
                .await
                .unwrap();

        sqlx::query(
            "INSERT INTO calendars (name, language, user_id, subscription_token) \
             VALUES ('Test Cal', 'english'::language, $1, 'tok-cascade')",
        )
        .bind(user.id)
        .execute(&mut *tx)
        .await
        .unwrap();

        UserMapper::delete_user_with(&mut *tx, user.id)
            .await
            .unwrap();

        let deleted_at: Option<chrono::NaiveDateTime> =
            sqlx::query_scalar("SELECT deleted_at FROM calendars WHERE user_id = $1")
                .bind(user.id)
                .fetch_one(&mut *tx)
                .await
                .unwrap();
        assert!(
            deleted_at.is_some(),
            "calendar should be soft-deleted when user is deleted"
        );
        tx.rollback().await.unwrap();
    }
}
