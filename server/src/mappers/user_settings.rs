use crate::{
    config::database::Database as DatabaseConfig,
    entity::{
        calendar::Language,
        user_settings::{SiteLanguage, Theme, UserSettings},
    },
    mappers::database::Database,
    ServerResult,
};

#[derive(Clone)]
pub struct UserSettingsMapper {
    db: Database,
}

impl UserSettingsMapper {
    /// # Errors
    /// Fails if database connection fails
    pub async fn new(config: DatabaseConfig) -> ServerResult<Self> {
        Ok(Self {
            db: Database::new(config).await?,
        })
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
    /// Returns an error if the query fails
    pub async fn get_user_settings(&self, user_id: i32) -> ServerResult<UserSettings> {
        crate::metrics::db::timed("user_settings.get", async {
            Self::get_user_settings_with(&mut *self.db.pool.acquire().await?, user_id).await
        })
        .await
    }

    pub(crate) async fn get_user_settings_with(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
    ) -> ServerResult<UserSettings> {
        let settings: Option<UserSettings> = sqlx::query_as!(
            UserSettings,
            "SELECT user_id, theme_preference as \"theme_preference: Theme\", language_preference as \"language_preference: SiteLanguage\", title_language_preference as \"title_language_preference: Language\", timezone, created_at, updated_at FROM user_settings WHERE user_id = $1",
            user_id
        )
        .fetch_optional(conn)
        .await?;

        Ok(settings.unwrap_or_default())
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn update_user_settings(
        &self,
        user_id: i32,
        settings: &UserSettings,
    ) -> ServerResult<()> {
        crate::metrics::db::timed("user_settings.update", async {
            Self::update_user_settings_with(
                &mut *self.db.pool.acquire().await?,
                user_id,
                settings,
            )
            .await
        })
        .await
    }

    pub(crate) async fn update_user_settings_with(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
        settings: &UserSettings,
    ) -> ServerResult<()> {
        sqlx::query!(
                "INSERT INTO user_settings (user_id, theme_preference, language_preference, title_language_preference, timezone) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (user_id) DO UPDATE SET theme_preference = EXCLUDED.theme_preference, language_preference = EXCLUDED.language_preference, title_language_preference = EXCLUDED.title_language_preference, timezone = EXCLUDED.timezone",
                user_id,
                settings.theme_preference as Theme,
                settings.language_preference as SiteLanguage,
                settings.title_language_preference as Language,
                settings.timezone,
            )
            .execute(conn)
            .await?;

        Ok(())
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn delete_user_settings(&self, user_id: i32) -> ServerResult<()> {
        crate::metrics::db::timed("user_settings.delete", async {
            Self::delete_user_settings_with(&mut *self.db.pool.acquire().await?, user_id).await
        })
        .await
    }

    pub(crate) async fn delete_user_settings_with(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
    ) -> ServerResult<()> {
        sqlx::query!("DELETE FROM user_settings WHERE user_id = $1", user_id)
            .execute(conn)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_user(conn: &mut sqlx::PgConnection) -> i32 {
        let n: u64 = rand::random();
        sqlx::query_scalar(
            "INSERT INTO users (username, email) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("settingsuser_{n}"))
        .bind(format!("settings_{n}@example.com"))
        .fetch_one(conn)
        .await
        .unwrap()
    }

    fn custom_settings(user_id: i32) -> UserSettings {
        UserSettings {
            user_id,
            theme_preference: Theme::Light,
            language_preference: SiteLanguage::Pt,
            title_language_preference: Language::Native,
            timezone: "Europe/Lisbon".to_string(),
            created_at: chrono::NaiveDateTime::default(),
            updated_at: chrono::NaiveDateTime::default(),
        }
    }

    #[tokio::test]
    async fn get_settings_returns_default_when_no_row() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        let settings = UserSettingsMapper::get_user_settings_with(&mut *tx, user_id)
            .await
            .unwrap();
        assert!(matches!(settings.theme_preference, Theme::Dark));
        assert!(matches!(settings.language_preference, SiteLanguage::En));
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn update_settings_inserts_and_fetch_round_trips() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        UserSettingsMapper::update_user_settings_with(&mut *tx, user_id, &custom_settings(user_id))
            .await
            .unwrap();
        let fetched = UserSettingsMapper::get_user_settings_with(&mut *tx, user_id)
            .await
            .unwrap();
        assert!(matches!(fetched.theme_preference, Theme::Light));
        assert!(matches!(fetched.language_preference, SiteLanguage::Pt));
        assert_eq!(fetched.timezone, "Europe/Lisbon");
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn update_settings_upserts_on_conflict() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        UserSettingsMapper::update_user_settings_with(&mut *tx, user_id, &custom_settings(user_id))
            .await
            .unwrap();
        let updated = UserSettings {
            theme_preference: Theme::Dark,
            language_preference: SiteLanguage::En,
            timezone: "UTC".to_string(),
            ..custom_settings(user_id)
        };
        UserSettingsMapper::update_user_settings_with(&mut *tx, user_id, &updated)
            .await
            .unwrap();
        let fetched = UserSettingsMapper::get_user_settings_with(&mut *tx, user_id)
            .await
            .unwrap();
        assert!(matches!(fetched.theme_preference, Theme::Dark));
        assert_eq!(fetched.timezone, "UTC");
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn delete_settings_removes_row_so_default_is_returned() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        UserSettingsMapper::update_user_settings_with(&mut *tx, user_id, &custom_settings(user_id))
            .await
            .unwrap();
        UserSettingsMapper::delete_user_settings_with(&mut *tx, user_id)
            .await
            .unwrap();
        let settings = UserSettingsMapper::get_user_settings_with(&mut *tx, user_id)
            .await
            .unwrap();
        assert!(matches!(settings.theme_preference, Theme::Dark));
        assert_eq!(settings.user_id, 0);
        tx.rollback().await.unwrap();
    }
}
