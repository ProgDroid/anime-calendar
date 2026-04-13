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
