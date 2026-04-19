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
    pub async fn get_calendar_by_id(&self, id: i32, user_id: i32) -> ServerResult<Calendar> {
        crate::metrics::db::timed("calendar.get_by_id", async {
            Self::get_calendar_by_id_with(&mut *self.db.pool.acquire().await?, id, user_id).await
        })
        .await
    }

    pub(crate) async fn get_calendar_by_id_with(
        conn: &mut sqlx::PgConnection,
        id: i32,
        user_id: i32,
    ) -> ServerResult<Calendar> {
        let calendar = sqlx::query!(
            "SELECT
                id,
                language as \"language: Language\",
                name,
                subscription_token,
                user_id,
                created_at,
                updated_at FROM calendars WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
            id,
            user_id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => Error::NotFound,
            other => Error::Database(other),
        })?;

        let item_ids: Vec<i32> = sqlx::query_scalar!(
            "SELECT item_id FROM calendar_items WHERE calendar_id = $1",
            id
        )
        .fetch_all(&mut *conn)
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
        crate::metrics::db::timed("calendar.get_by_token", async {
            Self::get_calendar_by_token_with(&mut *self.db.pool.acquire().await?, token).await
        })
        .await
    }

    pub(crate) async fn get_calendar_by_token_with(
        conn: &mut sqlx::PgConnection,
        token: &str,
    ) -> ServerResult<Calendar> {
        let calendar = sqlx::query!(
            "SELECT
                id,
                language as \"language: Language\",
                name,
                subscription_token,
                user_id,
                created_at,
                updated_at FROM calendars WHERE subscription_token = $1 AND deleted_at IS NULL",
            token
        )
        .fetch_one(&mut *conn)
        .await?;

        let item_ids: Vec<i32> = sqlx::query_scalar!(
            "SELECT item_id FROM calendar_items WHERE calendar_id = $1",
            calendar.id
        )
        .fetch_all(&mut *conn)
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
        crate::metrics::db::timed("calendar.list_paginated", async {
            Self::get_calendars_by_user_paginated_with(
                &mut *self.db.pool.acquire().await?,
                user_id,
                page,
                page_size,
            )
            .await
        })
        .await
    }

    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub(crate) async fn get_calendars_by_user_paginated_with(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
        page: usize,
        page_size: usize,
    ) -> ServerResult<(Vec<Calendar>, usize)> {
        let total_count: i64 = match sqlx::query_scalar!(
            "SELECT COUNT(*) FROM calendars WHERE user_id = $1 AND deleted_at IS NULL",
            user_id
        )
        .fetch_one(&mut *conn)
        .await?
        {
            Some(count) => count,
            None => return Err(Error::NotFound),
        };

        let calendars = sqlx::query_as!(
            Calendar,
            r#"SELECT
                c.id,
                c.language as "language: Language",
                c.name,
                c.subscription_token,
                c.user_id,
                c.created_at,
                c.updated_at,
                COALESCE(
                    ARRAY_AGG(ci.item_id) FILTER (WHERE ci.item_id IS NOT NULL),
                    '{}'::integer[]
                ) as "item_ids!: Vec<i32>"
            FROM calendars c
            LEFT JOIN calendar_items ci ON ci.calendar_id = c.id
            WHERE c.user_id = $1 AND c.deleted_at IS NULL
            GROUP BY c.id, c.language, c.name, c.subscription_token, c.user_id, c.created_at, c.updated_at
            ORDER BY c.created_at DESC
            LIMIT $2
            OFFSET $3"#,
            user_id,
            page_size as i64,
            ((page - 1) * page_size) as i64
        )
        .fetch_all(&mut *conn)
        .await?;

        Ok((calendars, total_count as usize))
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
        crate::metrics::db::timed("calendar.insert", async {
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
        })
        .await
    }

    /// # Errors
    /// Returns an error if the query fails.
    pub async fn update_calendar(&self, calendar: Calendar) -> ServerResult<Calendar> {
        crate::metrics::db::timed("calendar.update", async {
            let updated_calendar = sqlx::query!(
                "UPDATE calendars SET name = $1, language = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $3 AND user_id = $4 AND deleted_at IS NULL RETURNING id, name, language as \"language: Language\", subscription_token, user_id, created_at, updated_at",
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
        })
        .await
    }

    async fn update_calendar_items(
        &self,
        calendar_id: i32,
        item_ids: Vec<i32>,
    ) -> ServerResult<()> {
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

        // Skip the INSERT when there are no items — an empty VALUES list is invalid SQL.
        if !item_ids.is_empty() {
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

            if let Err(e) = query_builder.build().execute(&mut *transaction).await {
                transaction.rollback().await?;
                return Err(e.into());
            }
        }

        transaction.commit().await?;
        Ok(())
    }

    /// Soft-deletes the calendar. `calendar_items` rows are kept as an audit trail.
    /// Returns the `subscription_token` so the caller can invalidate the feed cache.
    ///
    /// # Errors
    /// Returns `Error::NotFound` if the calendar does not exist, is already deleted,
    /// or does not belong to `user_id`.
    pub async fn delete_calendar(&self, id: i32, user_id: i32) -> ServerResult<String> {
        crate::metrics::db::timed("calendar.delete", async {
            Self::delete_calendar_with(&mut *self.db.pool.acquire().await?, id, user_id).await
        })
        .await
    }

    pub(crate) async fn delete_calendar_with(
        conn: &mut sqlx::PgConnection,
        id: i32,
        user_id: i32,
    ) -> ServerResult<String> {
        let row = sqlx::query!(
            "UPDATE calendars SET deleted_at = NOW() WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL RETURNING subscription_token",
            id,
            user_id
        )
        .fetch_one(conn)
        .await
        .map_err(|_| Error::NotFound)?;

        Ok(row.subscription_token)
    }
}

// Test-only write variants that run on a caller-supplied connection.
// These exist solely for rollback-based test isolation and are not part of the
// production code path.
#[cfg(test)]
impl CalendarMapper {
    pub(crate) async fn save_calendar_with(
        conn: &mut sqlx::PgConnection,
        calendar: Calendar,
    ) -> ServerResult<Calendar> {
        match calendar.id {
            0 => Self::insert_calendar_with(conn, calendar).await,
            _ => Self::update_calendar_with(conn, calendar).await,
        }
    }

    async fn insert_calendar_with(
        conn: &mut sqlx::PgConnection,
        calendar: Calendar,
    ) -> ServerResult<Calendar> {
        let token = Self::generate_subscription_token();
        let inserted_calendar = sqlx::query!(
            "INSERT INTO calendars (name, language, user_id, subscription_token) VALUES ($1, $2, $3, $4) RETURNING id, name, language as \"language: Language\", subscription_token, user_id, created_at, updated_at",
            calendar.name,
            calendar.language as Language,
            calendar.user_id,
            token,
        )
        .fetch_one(&mut *conn)
        .await?;

        Self::update_calendar_items_with(conn, inserted_calendar.id, calendar.item_ids.clone())
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

    async fn update_calendar_with(
        conn: &mut sqlx::PgConnection,
        calendar: Calendar,
    ) -> ServerResult<Calendar> {
        let updated_calendar = sqlx::query!(
            "UPDATE calendars SET name = $1, language = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $3 AND user_id = $4 AND deleted_at IS NULL RETURNING id, name, language as \"language: Language\", subscription_token, user_id, created_at, updated_at",
            calendar.name,
            calendar.language as Language,
            calendar.id,
            calendar.user_id,
        )
        .fetch_one(&mut *conn)
        .await?;

        Self::update_calendar_items_with(conn, updated_calendar.id, calendar.item_ids.clone())
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

    async fn update_calendar_items_with(
        conn: &mut sqlx::PgConnection,
        calendar_id: i32,
        item_ids: Vec<i32>,
    ) -> ServerResult<()> {
        sqlx::query!(
            "DELETE FROM calendar_items WHERE calendar_id = $1",
            calendar_id
        )
        .execute(&mut *conn)
        .await?;

        if !item_ids.is_empty() {
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

            query_builder.build().execute(&mut *conn).await?;
        }

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
        .bind(format!("caltest_{n}"))
        .bind(format!("caltest_{n}@example.com"))
        .fetch_one(conn)
        .await
        .unwrap()
    }

    fn new_calendar(user_id: i32) -> Calendar {
        Calendar {
            id: 0,
            name: "My Calendar".to_string(),
            item_ids: vec![],
            language: Language::English,
            subscription_token: String::new(),
            user_id,
            created_at: chrono::NaiveDateTime::default(),
            updated_at: chrono::NaiveDateTime::default(),
        }
    }

    #[tokio::test]
    async fn insert_calendar_returns_calendar_with_43_char_token() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        let cal = CalendarMapper::insert_calendar_with(&mut *tx, new_calendar(user_id))
            .await
            .unwrap();
        assert!(cal.id > 0);
        assert_eq!(cal.subscription_token.len(), 43);
        assert_eq!(cal.name, "My Calendar");
        assert_eq!(cal.user_id, user_id);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_calendar_by_id_finds_inserted() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut *tx, new_calendar(user_id))
            .await
            .unwrap();
        let fetched =
            CalendarMapper::get_calendar_by_id_with(&mut *tx, inserted.id, user_id)
                .await
                .unwrap();
        assert_eq!(fetched.id, inserted.id);
        assert_eq!(fetched.name, "My Calendar");
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_calendar_by_id_wrong_user_returns_error() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut *tx, new_calendar(user_id))
            .await
            .unwrap();
        assert!(
            CalendarMapper::get_calendar_by_id_with(&mut *tx, inserted.id, i32::MAX)
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_calendar_by_token_finds_inserted() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut *tx, new_calendar(user_id))
            .await
            .unwrap();
        let fetched =
            CalendarMapper::get_calendar_by_token_with(&mut *tx, &inserted.subscription_token)
                .await
                .unwrap();
        assert_eq!(fetched.id, inserted.id);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn update_calendar_changes_name_and_language() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut *tx, new_calendar(user_id))
            .await
            .unwrap();
        let to_update = Calendar {
            id: inserted.id,
            name: "Renamed".to_string(),
            language: Language::Native,
            item_ids: vec![],
            ..inserted.clone()
        };
        let updated = CalendarMapper::update_calendar_with(&mut *tx, to_update)
            .await
            .unwrap();
        assert_eq!(updated.name, "Renamed");
        assert!(matches!(updated.language, Language::Native));
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn save_calendar_with_item_ids_round_trips() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        let cal = Calendar {
            item_ids: vec![101, 202, 303],
            ..new_calendar(user_id)
        };
        let saved = CalendarMapper::save_calendar_with(&mut *tx, cal).await.unwrap();
        let fetched =
            CalendarMapper::get_calendar_by_id_with(&mut *tx, saved.id, user_id)
                .await
                .unwrap();
        let mut ids = fetched.item_ids;
        ids.sort_unstable();
        assert_eq!(ids, vec![101, 202, 303]);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn delete_calendar_soft_deletes_so_lookup_fails() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut *tx, new_calendar(user_id))
            .await
            .unwrap();
        CalendarMapper::delete_calendar_with(&mut *tx, inserted.id, user_id)
            .await
            .unwrap();
        assert!(
            CalendarMapper::get_calendar_by_id_with(&mut *tx, inserted.id, user_id)
                .await
                .is_err()
        );
        assert!(
            CalendarMapper::get_calendar_by_token_with(&mut *tx, &inserted.subscription_token)
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn delete_calendar_returns_subscription_token() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut *tx, new_calendar(user_id))
            .await
            .unwrap();
        let token =
            CalendarMapper::delete_calendar_with(&mut *tx, inserted.id, user_id)
                .await
                .unwrap();
        assert_eq!(token, inserted.subscription_token);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn delete_calendar_preserves_calendar_items_as_audit_trail() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        let cal = Calendar {
            item_ids: vec![42, 43],
            ..new_calendar(user_id)
        };
        let inserted = CalendarMapper::save_calendar_with(&mut *tx, cal).await.unwrap();
        CalendarMapper::delete_calendar_with(&mut *tx, inserted.id, user_id)
            .await
            .unwrap();
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM calendar_items WHERE calendar_id = $1")
                .bind(inserted.id)
                .fetch_one(&mut *tx)
                .await
                .unwrap();
        assert_eq!(
            count, 2,
            "calendar_items should be preserved after soft delete"
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn delete_calendar_wrong_user_returns_not_found() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut *tx, new_calendar(user_id))
            .await
            .unwrap();
        assert!(
            CalendarMapper::delete_calendar_with(&mut *tx, inserted.id, i32::MAX)
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_calendars_paginated_returns_all_active() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        CalendarMapper::insert_calendar_with(&mut *tx, new_calendar(user_id))
            .await
            .unwrap();
        CalendarMapper::insert_calendar_with(
            &mut *tx,
            Calendar {
                name: "Second".to_string(),
                ..new_calendar(user_id)
            },
        )
        .await
        .unwrap();
        let (cals, total) =
            CalendarMapper::get_calendars_by_user_paginated_with(&mut *tx, user_id, 1, 10)
                .await
                .unwrap();
        assert_eq!(total, 2);
        assert_eq!(cals.len(), 2);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_calendars_paginated_excludes_deleted() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut *tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut *tx, new_calendar(user_id))
            .await
            .unwrap();
        CalendarMapper::delete_calendar_with(&mut *tx, inserted.id, user_id)
            .await
            .unwrap();
        let (cals, total) =
            CalendarMapper::get_calendars_by_user_paginated_with(&mut *tx, user_id, 1, 10)
                .await
                .unwrap();
        assert_eq!(total, 0);
        assert!(cals.is_empty());
        tx.rollback().await.unwrap();
    }
}
