use crate::{
    config::database::Database as DatabaseConfig,
    entity::calendar::{Calendar, Language},
    error::Error,
    mappers::database::Database,
    ServerResult,
};
use rand::{distr::Alphanumeric, Rng};
use sqlx::{Postgres, QueryBuilder};

#[derive(Clone)]
pub struct CalendarMapper {
    db: Database,
}

impl CalendarMapper {
    fn generate_subscription_token() -> String {
        rand::rng()
            .sample_iter(&Alphanumeric)
            .take(43)
            .map(char::from)
            .collect()
    }

    /// # Errors
    /// Fails if database connection fails
    pub async fn new(config: DatabaseConfig) -> ServerResult<Self> {
        Ok(Self {
            db: Database::new(config).await?,
        })
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn get_calendar_by_id(&self, id: i32, user_id: i32) -> ServerResult<Calendar> {
        let calendar = sqlx::query!(
            "SELECT
                id,
                language as \"language: Language\",
                name,
                subscription_token,
                user_id,
                created_at,
                updated_at FROM calendars WHERE id = $1 AND user_id = $2",
            id,
            user_id
        )
        .fetch_one(&self.db.pool)
        .await?;

        let item_ids: Vec<i32> = sqlx::query_scalar!(
            "SELECT item_id FROM calendar_items WHERE calendar_id = $1",
            id
        )
        .fetch_all(&self.db.pool)
        .await?;

        Ok(Calendar {
            id: calendar.id,
            name: calendar.name,
            item_ids,
            language: calendar.language,
            subscription_token: calendar.subscription_token,
            user_id: calendar.user_id,
            created_at: calendar.created_at,
            updated_at: calendar.updated_at,
        })
    }

    /// # Errors
    /// Returns an error if the query fails or the token is not found
    pub async fn get_calendar_by_token(&self, token: &str) -> ServerResult<Calendar> {
        let calendar = sqlx::query!(
            "SELECT
                id,
                language as \"language: Language\",
                name,
                subscription_token,
                user_id,
                created_at,
                updated_at FROM calendars WHERE subscription_token = $1",
            token
        )
        .fetch_one(&self.db.pool)
        .await?;

        let item_ids: Vec<i32> = sqlx::query_scalar!(
            "SELECT item_id FROM calendar_items WHERE calendar_id = $1",
            calendar.id
        )
        .fetch_all(&self.db.pool)
        .await?;

        Ok(Calendar {
            id: calendar.id,
            name: calendar.name,
            item_ids,
            language: calendar.language,
            subscription_token: calendar.subscription_token,
            user_id: calendar.user_id,
            created_at: calendar.created_at,
            updated_at: calendar.updated_at,
        })
    }

    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    /// # Errors
    /// Returns an error if the query fails
    pub async fn get_calendars_by_user_paginated(
        &self,
        user_id: i32,
        page: usize,
        page_size: usize,
    ) -> ServerResult<(Vec<Calendar>, usize)> {
        // First get the total count
        let total_count: i64 =
            match sqlx::query_scalar!("SELECT COUNT(*) FROM calendars WHERE user_id = $1", user_id)
                .fetch_one(&self.db.pool)
                .await?
            {
                Some(count) => count,
                None => return Err(Error::NotFound),
            };

        // Then get the paginated results
        let calendars: Vec<Calendar> = sqlx::query_as!(
            Calendar,
            "SELECT
                id,
                ARRAY[]::INTEGER[] as \"item_ids!\",
                language as \"language: Language\",
                name,
                subscription_token,
                user_id,
                created_at,
                updated_at FROM calendars WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            user_id,
            page_size as i64,
            ((page - 1) * page_size) as i64
        )
        .fetch_all(&self.db.pool)
        .await?;

        let mut result: Vec<Calendar> = Vec::new();

        for calendar_entity in calendars {
            let item_ids: Vec<i32> = sqlx::query_scalar!(
                "SELECT item_id FROM calendar_items WHERE calendar_id = $1",
                calendar_entity.id
            )
            .fetch_all(&self.db.pool)
            .await?;

            result.push(Calendar {
                id: calendar_entity.id,
                name: calendar_entity.name,
                item_ids,
                language: calendar_entity.language,
                subscription_token: calendar_entity.subscription_token,
                user_id: calendar_entity.user_id,
                created_at: calendar_entity.created_at,
                updated_at: calendar_entity.updated_at,
            });
        }

        Ok((result, total_count as usize))
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn save_calendar(&self, calendar: Calendar) -> ServerResult<Calendar> {
        match calendar.id {
            0 => self.insert_calendar(calendar).await,
            _ => self.update_calendar(calendar).await,
        }
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn insert_calendar(&self, calendar: Calendar) -> ServerResult<Calendar> {
        let token = Self::generate_subscription_token();
        let inserted_calendar = sqlx::query!(
            "INSERT INTO calendars (name, language, user_id, subscription_token) VALUES ($1, $2, $3, $4) RETURNING id, name, language as \"language: Language\", subscription_token, user_id, created_at, updated_at",
            calendar.name,
            calendar.language as Language,
            calendar.user_id,
            token,
        )
        .fetch_one(&self.db.pool)
        .await?;

        self.update_calendar_items(inserted_calendar.id, calendar.item_ids.clone())
            .await?;

        Ok(Calendar {
            id: inserted_calendar.id,
            name: inserted_calendar.name,
            item_ids: calendar.item_ids,
            language: inserted_calendar.language,
            subscription_token: inserted_calendar.subscription_token,
            user_id: inserted_calendar.user_id,
            created_at: inserted_calendar.created_at,
            updated_at: inserted_calendar.updated_at,
        })
    }

    /// # Errors
    /// Returns an error if the query fails.
    pub async fn update_calendar(&self, calendar: Calendar) -> ServerResult<Calendar> {
        let updated_calendar = sqlx::query!(
            "UPDATE calendars SET name = $1, language = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $3 AND user_id = $4 RETURNING id, name, language as \"language: Language\", subscription_token, user_id, created_at, updated_at",
            calendar.name,
            calendar.language as Language,
            calendar.id,
            calendar.user_id,
        )
        .fetch_one(&self.db.pool)
        .await?;

        self.update_calendar_items(updated_calendar.id, calendar.item_ids.clone())
            .await?;

        Ok(Calendar {
            id: updated_calendar.id,
            name: updated_calendar.name,
            item_ids: calendar.item_ids,
            language: updated_calendar.language,
            subscription_token: updated_calendar.subscription_token,
            user_id: updated_calendar.user_id,
            created_at: updated_calendar.created_at,
            updated_at: updated_calendar.updated_at,
        })
    }

    async fn update_calendar_items(
        &self,
        calendar_id: i32,
        item_ids: Vec<i32>,
    ) -> ServerResult<()> {
        let inserts: Vec<(i32, i32)> = item_ids
            .iter()
            .map(|item_id| (calendar_id, *item_id))
            .collect();

        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("INSERT INTO calendar_items (calendar_id, item_id) ");

        query_builder.push_values(inserts, |mut b, insert| {
            b.push_bind(insert.0).push_bind(insert.1);
        });

        query_builder.push(" ON CONFLICT (calendar_id, item_id) DO NOTHING");

        let query = query_builder.build();

        let mut transaction = self.db.pool.begin().await?;

        if let Err(e) = sqlx::query!(
            "DELETE FROM calendar_items WHERE calendar_id = $1",
            calendar_id
        )
        .execute(&mut *transaction)
        .await
        {
            transaction.rollback().await?;
            return Err(e.into());
        }

        if let Err(e) = query.execute(&mut *transaction).await {
            transaction.rollback().await?;
            return Err(e.into());
        }

        transaction.commit().await?;

        Ok(())
    }

    /// # Errors
    /// Returns an error if the query fails
    pub async fn delete_calendar(&self, id: i32, user_id: i32) -> ServerResult<()> {
        let mut tx = self.db.pool.begin().await?;

        if let Err(e) = sqlx::query!(
            "DELETE FROM calendar_items WHERE calendar_id = $1",
            id
        )
        .execute(&mut *tx)
        .await
        {
            tx.rollback().await?;
            return Err(e.into());
        }

        if let Err(e) = sqlx::query!(
            "DELETE FROM calendars WHERE id = $1 AND user_id = $2",
            id,
            user_id
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
}
