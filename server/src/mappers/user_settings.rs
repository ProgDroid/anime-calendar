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

    /// # Errors
    /// Returns an error if the query fails
    pub async fn get_user_settings(&self, user_id: i32) -> ServerResult<UserSettings> {
        // First try to get user settings from database
        let settings: Option<UserSettings> = sqlx::query_as!(
            UserSettings,
            "SELECT user_id, theme_preference as \"theme_preference: Theme\", language_preference as \"language_preference: SiteLanguage\", title_language_preference as \"title_language_preference: Language\", timezone, created_at, updated_at FROM user_settings WHERE user_id = $1",
            user_id
        )
        .fetch_optional(&self.db.pool)
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
        sqlx::query!(
                "INSERT INTO user_settings (user_id, theme_preference, language_preference, title_language_preference, timezone) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (user_id) DO UPDATE SET theme_preference = EXCLUDED.theme_preference, language_preference = EXCLUDED.language_preference, title_language_preference = EXCLUDED.title_language_preference, timezone = EXCLUDED.timezone",
                user_id,
                settings.theme_preference as Theme,
                settings.language_preference as SiteLanguage,
                settings.title_language_preference as Language,
                settings.timezone,
            )
            .execute(&self.db.pool)
            .await?;

        Ok(())
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn delete_user_settings(&self, user_id: i32) -> ServerResult<()> {
        sqlx::query!("DELETE FROM user_settings WHERE user_id = $1", user_id)
            .execute(&self.db.pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    fn mapper(pool: PgPool) -> UserSettingsMapper {
        UserSettingsMapper { db: Database { pool } }
    }

    async fn create_test_user(pool: &PgPool) -> i32 {
        sqlx::query_scalar(
            "INSERT INTO users (username, email) VALUES ('settingsuser', 'settings@example.com') RETURNING id"
        )
        .fetch_one(pool)
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

    #[sqlx::test(migrations = "../migrations")]
    async fn get_settings_returns_default_when_no_row(pool: PgPool) {
        let user_id = create_test_user(&pool).await;
        let m = mapper(pool);
        let settings = m.get_user_settings(user_id).await.unwrap();
        assert!(matches!(settings.theme_preference, Theme::Dark));
        assert!(matches!(settings.language_preference, SiteLanguage::En));
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn update_settings_inserts_and_fetch_round_trips(pool: PgPool) {
        let user_id = create_test_user(&pool).await;
        let m = mapper(pool);
        m.update_user_settings(user_id, &custom_settings(user_id)).await.unwrap();
        let fetched = m.get_user_settings(user_id).await.unwrap();
        assert!(matches!(fetched.theme_preference, Theme::Light));
        assert!(matches!(fetched.language_preference, SiteLanguage::Pt));
        assert_eq!(fetched.timezone, "Europe/Lisbon");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn update_settings_upserts_on_conflict(pool: PgPool) {
        let user_id = create_test_user(&pool).await;
        let m = mapper(pool);
        m.update_user_settings(user_id, &custom_settings(user_id)).await.unwrap();
        let updated = UserSettings {
            theme_preference: Theme::Dark,
            language_preference: SiteLanguage::En,
            timezone: "UTC".to_string(),
            ..custom_settings(user_id)
        };
        m.update_user_settings(user_id, &updated).await.unwrap();
        let fetched = m.get_user_settings(user_id).await.unwrap();
        assert!(matches!(fetched.theme_preference, Theme::Dark));
        assert_eq!(fetched.timezone, "UTC");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn delete_settings_removes_row_so_default_is_returned(pool: PgPool) {
        let user_id = create_test_user(&pool).await;
        let m = mapper(pool);
        m.update_user_settings(user_id, &custom_settings(user_id)).await.unwrap();
        m.delete_user_settings(user_id).await.unwrap();
        let settings = m.get_user_settings(user_id).await.unwrap();
        assert!(matches!(settings.theme_preference, Theme::Dark));
        assert_eq!(settings.user_id, 0);
    }
}
