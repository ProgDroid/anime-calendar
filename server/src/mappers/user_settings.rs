use crate::{
    config::database::Database as DatabaseConfig,
    entity::{
        calendar::Language,
        user_settings::{DateFormat, DateSeparator, SiteLanguage, Theme, UserSettings},
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
            "SELECT user_id, theme_preference as \"theme_preference: Theme\", language_preference as \"language_preference: SiteLanguage\", title_language_preference as \"title_language_preference: Language\", date_display_preference as \"date_display_preference: DateFormat\", date_separator_preference as \"date_separator_preference: DateSeparator\", timezone, created_at, updated_at FROM user_settings WHERE user_id = $1",
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
                "INSERT INTO user_settings (user_id, theme_preference, language_preference, title_language_preference, date_display_preference, date_separator_preference, timezone) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (user_id) DO UPDATE SET theme_preference = EXCLUDED.theme_preference, language_preference = EXCLUDED.language_preference, title_language_preference = EXCLUDED.title_language_preference, date_display_preference = EXCLUDED.date_display_preference, date_separator_preference = EXCLUDED.date_separator_preference, timezone = EXCLUDED.timezone",
                user_id,
                settings.theme_preference as Theme,
                settings.language_preference as SiteLanguage,
                settings.title_language_preference as Language,
                settings.date_display_preference as DateFormat,
                settings.date_separator_preference as DateSeparator,
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
