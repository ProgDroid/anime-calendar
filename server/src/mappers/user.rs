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
        let user = sqlx::query!(
            "SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE email = $1 AND deleted_at IS NULL",
            email
        )
        .fetch_one(&self.db.pool)
        .await?;

        Ok(User {
            id: user.id,
            username: user.username,
            email: user.email,
            password_hash: user.password_hash,
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
        let user = sqlx::query!(
            "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id, username, email, password_hash, created_at, updated_at",
            username,
            email,
            password_hash
        )
        .fetch_one(&self.db.pool)
        .await?;

        Ok(User {
            id: user.id,
            username: user.username,
            email: user.email,
            password_hash: user.password_hash,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn get_user_by_id(&self, id: i32) -> ServerResult<User> {
        let user = sqlx::query!(
            "SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE id = $1 AND deleted_at IS NULL",
            id
        )
        .fetch_one(&self.db.pool)
        .await?;

        Ok(User {
            id: user.id,
            username: user.username,
            email: user.email,
            password_hash: user.password_hash,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn update_user(&self, id: i32, username: &str, email: &str) -> ServerResult<User> {
        let user = sqlx::query!(
            "UPDATE users SET username = $1, email = $2, updated_at = NOW() WHERE id = $3 AND deleted_at IS NULL RETURNING id, username, email, password_hash, created_at, updated_at",
            username,
            email,
            id
        )
        .fetch_one(&self.db.pool)
        .await?;

        Ok(User {
            id: user.id,
            username: user.username,
            email: user.email,
            password_hash: user.password_hash,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn update_user_password(&self, id: i32, password_hash: &str) -> ServerResult<()> {
        sqlx::query!(
            "UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2 AND deleted_at IS NULL",
            password_hash,
            id
        )
        .execute(&self.db.pool)
        .await?;

        Ok(())
    }

    /// Soft-deletes the user and cascades to all their calendars in a single transaction.
    ///
    /// # Errors
    /// Returns an error if the query fails
    pub async fn delete_user(&self, id: i32) -> ServerResult<()> {
        let mut tx = self.db.pool.begin().await?;

        if let Err(e) = sqlx::query!(
            "UPDATE calendars SET deleted_at = NOW() WHERE user_id = $1 AND deleted_at IS NULL",
            id
        )
        .execute(&mut *tx)
        .await
        {
            tx.rollback().await?;
            return Err(e.into());
        }

        if let Err(e) = sqlx::query!(
            "UPDATE users SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
            id
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
    use sqlx::PgPool;

    fn mapper(pool: PgPool) -> UserMapper {
        UserMapper { db: Database { pool } }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn create_user_returns_correct_fields(pool: PgPool) {
        let m = mapper(pool);
        let user = m.create_user("alice", "alice@example.com", Some("hash123")).await.unwrap();
        assert_eq!(user.username, "alice");
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.password_hash.as_deref(), Some("hash123"));
        assert!(user.id > 0);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn create_user_oauth_has_no_password(pool: PgPool) {
        let m = mapper(pool);
        let user = m.create_user("oauthuser", "oauth@example.com", None).await.unwrap();
        assert!(user.password_hash.is_none());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_user_by_email_finds_existing(pool: PgPool) {
        let m = mapper(pool);
        let created = m.create_user("bob", "bob@example.com", None).await.unwrap();
        let found = m.get_user_by_email("bob@example.com").await.unwrap();
        assert_eq!(found.id, created.id);
        assert_eq!(found.username, "bob");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_user_by_email_not_found(pool: PgPool) {
        let m = mapper(pool);
        assert!(m.get_user_by_email("nobody@example.com").await.is_err());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_user_by_id_finds_existing(pool: PgPool) {
        let m = mapper(pool);
        let created = m.create_user("carol", "carol@example.com", None).await.unwrap();
        let fetched = m.get_user_by_id(created.id).await.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.email, "carol@example.com");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_user_by_id_not_found(pool: PgPool) {
        let m = mapper(pool);
        assert!(m.get_user_by_id(i32::MAX).await.is_err());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn update_user_changes_username_and_email(pool: PgPool) {
        let m = mapper(pool);
        let created = m.create_user("dave", "dave@example.com", None).await.unwrap();
        let updated = m.update_user(created.id, "david", "david@example.com").await.unwrap();
        assert_eq!(updated.username, "david");
        assert_eq!(updated.email, "david@example.com");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn update_user_password_persists(pool: PgPool) {
        let m = mapper(pool);
        let created = m.create_user("eve", "eve@example.com", Some("oldhash")).await.unwrap();
        m.update_user_password(created.id, "newhash").await.unwrap();
        let fetched = m.get_user_by_id(created.id).await.unwrap();
        assert_eq!(fetched.password_hash.as_deref(), Some("newhash"));
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn delete_user_soft_deletes_so_lookup_fails(pool: PgPool) {
        let m = mapper(pool);
        let created = m.create_user("frank", "frank@example.com", None).await.unwrap();
        m.delete_user(created.id).await.unwrap();
        assert!(m.get_user_by_id(created.id).await.is_err());
        assert!(m.get_user_by_email("frank@example.com").await.is_err());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn delete_user_cascades_soft_delete_to_calendars(pool: PgPool) {
        let m = mapper(pool.clone());
        let user = m.create_user("grace", "grace@example.com", None).await.unwrap();

        sqlx::query(
            "INSERT INTO calendars (name, language, user_id, subscription_token) \
             VALUES ('Test Cal', 'english'::language, $1, 'tok-cascade')"
        )
        .bind(user.id)
        .execute(&pool)
        .await
        .unwrap();

        m.delete_user(user.id).await.unwrap();

        let deleted_at: Option<chrono::NaiveDateTime> =
            sqlx::query_scalar("SELECT deleted_at FROM calendars WHERE user_id = $1")
                .bind(user.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(deleted_at.is_some(), "calendar should be soft-deleted when user is deleted");
    }
}
