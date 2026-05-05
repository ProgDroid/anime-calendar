use crate::{
    ServerResult,
    config::database::Database as DatabaseConfig,
    entity::calendar::{Calendar, CalendarOwnerInfo, Language},
    error::Error,
    mappers::database::Database,
};
use rand::{Rng, distr::Alphanumeric};
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
                updated_at,
                event_style,
                frozen_subscribe_ics,
                meta_version FROM calendars WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
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
            event_style: calendar.event_style,
            frozen_subscribe_ics: calendar.frozen_subscribe_ics,
            meta_version: calendar.meta_version,
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
                updated_at,
                event_style,
                frozen_subscribe_ics,
                meta_version FROM calendars WHERE subscription_token = $1 AND deleted_at IS NULL",
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
            event_style: calendar.event_style,
            frozen_subscribe_ics: calendar.frozen_subscribe_ics,
            meta_version: calendar.meta_version,
        })
    }

    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    /// # Errors
    /// Returns an error if the query fails.
    ///
    /// Returns `(Vec<(Calendar, recent_item_ids, editor_count)>, total_count)`.
    /// `editor_count` is the number of currently-active editors on each row.
    pub async fn get_calendars_by_user_paginated(
        &self,
        user_id: i32,
        page: usize,
        page_size: usize,
    ) -> ServerResult<(Vec<(Calendar, Vec<i32>, i64)>, usize)> {
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
    ) -> ServerResult<(Vec<(Calendar, Vec<i32>, i64)>, usize)> {
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

        let rows = sqlx::query!(
            r#"SELECT
                c.id,
                c.language as "language: Language",
                c.name,
                c.subscription_token,
                c.user_id,
                c.created_at,
                c.updated_at,
                c.event_style,
                c.frozen_subscribe_ics,
                c.meta_version,
                COALESCE(
                    ARRAY_AGG(ci.item_id) FILTER (WHERE ci.item_id IS NOT NULL),
                    '{}'::integer[]
                ) as "item_ids!: Vec<i32>",
                COALESCE((
                    SELECT ARRAY_AGG(item_id ORDER BY added_at DESC)
                    FROM (
                        SELECT item_id, added_at
                        FROM calendar_items
                        WHERE calendar_id = c.id
                        ORDER BY added_at DESC
                        LIMIT 4
                    ) recent_inner
                ), '{}'::integer[]) as "recent_item_ids!: Vec<i32>",
                COALESCE((
                    SELECT COUNT(*)
                    FROM calendar_editors
                    WHERE calendar_id = c.id AND active = TRUE
                ), 0) as "editor_count!: i64"
            FROM calendars c
            LEFT JOIN calendar_items ci ON ci.calendar_id = c.id
            WHERE c.user_id = $1 AND c.deleted_at IS NULL
            GROUP BY c.id, c.language, c.name, c.subscription_token, c.user_id, c.created_at, c.updated_at, c.event_style, c.frozen_subscribe_ics, c.meta_version
            ORDER BY c.created_at DESC
            LIMIT $2
            OFFSET $3"#,
            user_id,
            page_size as i64,
            ((page - 1) * page_size) as i64
        )
        .fetch_all(&mut *conn)
        .await?;

        let calendars: Vec<(Calendar, Vec<i32>, i64)> = rows
            .into_iter()
            .map(|r| {
                (
                    Calendar {
                        id: r.id,
                        language: r.language,
                        name: r.name,
                        subscription_token: r.subscription_token,
                        user_id: r.user_id,
                        created_at: r.created_at,
                        updated_at: r.updated_at,
                        item_ids: r.item_ids,
                        event_style: r.event_style,
                        frozen_subscribe_ics: r.frozen_subscribe_ics,
                        meta_version: r.meta_version,
                    },
                    r.recent_item_ids,
                    r.editor_count,
                )
            })
            .collect();

        Ok((calendars, total_count as usize))
    }


    /// List calendars shared with `user_id` (i.e. where they are an active
    /// editor). Returns each calendar with its owner projection and the four
    /// most-recent item ids (mirrors the owned-list shape).
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub(crate) async fn list_shared_with_user_with(
        conn: &mut sqlx::PgConnection,
        user_id: i32,
    ) -> ServerResult<Vec<(Calendar, CalendarOwnerInfo, Vec<i32>)>> {
        let rows = sqlx::query!(
            r#"SELECT
                c.id,
                c.language as "language: Language",
                c.name,
                c.subscription_token,
                c.user_id,
                c.created_at,
                c.updated_at,
                c.event_style,
                c.frozen_subscribe_ics,
                c.meta_version,
                u.id as "owner_id!",
                u.username as "owner_username!",
                COALESCE(
                    ARRAY_AGG(ci.item_id) FILTER (WHERE ci.item_id IS NOT NULL),
                    '{}'::integer[]
                ) as "item_ids!: Vec<i32>",
                COALESCE((
                    SELECT ARRAY_AGG(item_id ORDER BY added_at DESC)
                    FROM (
                        SELECT item_id, added_at
                        FROM calendar_items
                        WHERE calendar_id = c.id
                        ORDER BY added_at DESC
                        LIMIT 4
                    ) recent_inner
                ), '{}'::integer[]) as "recent_item_ids!: Vec<i32>"
            FROM calendars c
            JOIN calendar_editors ce
                ON ce.calendar_id = c.id AND ce.user_id = $1 AND ce.active = TRUE
            JOIN users u ON u.id = c.user_id
            LEFT JOIN calendar_items ci ON ci.calendar_id = c.id
            WHERE c.deleted_at IS NULL
            GROUP BY c.id, c.language, c.name, c.subscription_token, c.user_id, c.created_at, c.updated_at, c.event_style, c.frozen_subscribe_ics, c.meta_version, u.id, u.username
            ORDER BY c.created_at DESC"#,
            user_id,
        )
        .fetch_all(&mut *conn)
        .await?;

        let calendars: Vec<(Calendar, CalendarOwnerInfo, Vec<i32>)> = rows
            .into_iter()
            .map(|r| {
                (
                    Calendar {
                        id: r.id,
                        language: r.language,
                        name: r.name,
                        subscription_token: r.subscription_token,
                        user_id: r.user_id,
                        created_at: r.created_at,
                        updated_at: r.updated_at,
                        item_ids: r.item_ids,
                        event_style: r.event_style,
                        frozen_subscribe_ics: r.frozen_subscribe_ics,
                        meta_version: r.meta_version,
                    },
                    CalendarOwnerInfo {
                        id: r.owner_id,
                        display: r.owner_username,
                        avatar: None,
                    },
                    r.recent_item_ids,
                )
            })
            .collect();

        Ok(calendars)
    }

    /// # Errors
    /// Returns an error if the query fails.
    pub async fn list_shared_with_user(
        &self,
        user_id: i32,
    ) -> ServerResult<Vec<(Calendar, CalendarOwnerInfo, Vec<i32>)>> {
        crate::metrics::db::timed("calendar.list_shared_with_user", async {
            Self::list_shared_with_user_with(
                &mut *self.db.pool.acquire().await?,
                user_id,
            )
            .await
        })
        .await
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
                "INSERT INTO calendars (name, language, user_id, subscription_token, event_style) VALUES ($1, $2, $3, $4, $5) RETURNING id, name, language as \"language: Language\", subscription_token, user_id, created_at, updated_at, event_style, frozen_subscribe_ics, meta_version",
                calendar.name,
                calendar.language as Language,
                calendar.user_id,
                token,
                calendar.event_style,
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
                event_style: inserted_calendar.event_style,
                frozen_subscribe_ics: inserted_calendar.frozen_subscribe_ics,
                meta_version: inserted_calendar.meta_version,
            })
        })
        .await
    }

    /// # Errors
    /// Returns an error if the query fails.
    ///
    /// Bumps `meta_version` only when `name`, `language`, or `event_style`
    /// differ from the stored row. Used by Phase 3 SSE fan-out as a
    /// per-calendar revision cursor; no-op writes do not invalidate caches.
    pub async fn update_calendar(&self, calendar: Calendar) -> ServerResult<Calendar> {
        crate::metrics::db::timed("calendar.update", async {
            let updated_calendar = sqlx::query!(
                "UPDATE calendars SET
                    name = $1,
                    language = $2,
                    event_style = $3,
                    updated_at = CURRENT_TIMESTAMP,
                    meta_version = meta_version + (CASE
                        WHEN name <> $6 OR language <> $7 OR event_style <> $8
                        THEN 1 ELSE 0
                    END)
                WHERE id = $4 AND user_id = $5 AND deleted_at IS NULL
                RETURNING id, name, language as \"language: Language\", subscription_token, user_id, created_at, updated_at, event_style, frozen_subscribe_ics, meta_version",
                calendar.name.clone(),
                calendar.language as Language,
                calendar.event_style.clone(),
                calendar.id,
                calendar.user_id,
                calendar.name,
                calendar.language as Language,
                calendar.event_style,
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
                event_style: updated_calendar.event_style,
                frozen_subscribe_ics: updated_calendar.frozen_subscribe_ics,
                meta_version: updated_calendar.meta_version,
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

// Write variants that run on a caller-supplied connection. Used by:
//   - `#[cfg(test)]` rollback-based mapper tests for isolation
//   - PUT /calendar so the cap-check + INSERT/UPDATE happen inside one
//     advisory-locked transaction (multi-tab race protection)
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

    pub(crate) async fn insert_calendar_with(
        conn: &mut sqlx::PgConnection,
        calendar: Calendar,
    ) -> ServerResult<Calendar> {
        let token = Self::generate_subscription_token();
        let inserted_calendar = sqlx::query!(
            "INSERT INTO calendars (name, language, user_id, subscription_token, event_style) VALUES ($1, $2, $3, $4, $5) RETURNING id, name, language as \"language: Language\", subscription_token, user_id, created_at, updated_at, event_style, frozen_subscribe_ics, meta_version",
            calendar.name,
            calendar.language as Language,
            calendar.user_id,
            token,
            calendar.event_style,
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
            event_style: inserted_calendar.event_style,
            frozen_subscribe_ics: inserted_calendar.frozen_subscribe_ics,
            meta_version: inserted_calendar.meta_version,
        })
    }

    async fn update_calendar_with(
        conn: &mut sqlx::PgConnection,
        calendar: Calendar,
    ) -> ServerResult<Calendar> {
        let updated_calendar = sqlx::query!(
            "UPDATE calendars SET
                name = $1,
                language = $2,
                event_style = $3,
                updated_at = CURRENT_TIMESTAMP,
                meta_version = meta_version + (CASE
                    WHEN name <> $6 OR language <> $7 OR event_style <> $8
                    THEN 1 ELSE 0
                END)
            WHERE id = $4 AND user_id = $5 AND deleted_at IS NULL
            RETURNING id, name, language as \"language: Language\", subscription_token, user_id, created_at, updated_at, event_style, frozen_subscribe_ics, meta_version",
            calendar.name.clone(),
            calendar.language as Language,
            calendar.event_style.clone(),
            calendar.id,
            calendar.user_id,
            calendar.name,
            calendar.language as Language,
            calendar.event_style,
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
            event_style: updated_calendar.event_style,
            frozen_subscribe_ics: updated_calendar.frozen_subscribe_ics,
            meta_version: updated_calendar.meta_version,
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
        sqlx::query_scalar("INSERT INTO users (username, email) VALUES ($1, $2) RETURNING id")
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
            event_style: "timed".to_owned(),
            frozen_subscribe_ics: None,
            meta_version: 1,
        }
    }

    #[tokio::test]
    async fn insert_calendar_returns_calendar_with_43_char_token() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let cal = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
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
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();
        let fetched = CalendarMapper::get_calendar_by_id_with(&mut tx, inserted.id, user_id)
            .await
            .unwrap();
        assert_eq!(fetched.id, inserted.id);
        assert_eq!(fetched.name, "My Calendar");
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_calendar_by_id_wrong_user_returns_error() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();
        assert!(
            CalendarMapper::get_calendar_by_id_with(&mut tx, inserted.id, i32::MAX)
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_calendar_by_token_finds_inserted() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();
        let fetched =
            CalendarMapper::get_calendar_by_token_with(&mut tx, &inserted.subscription_token)
                .await
                .unwrap();
        assert_eq!(fetched.id, inserted.id);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn update_calendar_changes_name_and_language() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();
        let to_update = Calendar {
            id: inserted.id,
            name: "Renamed".to_string(),
            language: Language::Native,
            item_ids: vec![],
            ..inserted.clone()
        };
        let updated = CalendarMapper::update_calendar_with(&mut tx, to_update)
            .await
            .unwrap();
        assert_eq!(updated.name, "Renamed");
        assert!(matches!(updated.language, Language::Native));
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn insert_calendar_starts_at_meta_version_one() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();
        assert_eq!(inserted.meta_version, 1);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn update_calendar_bumps_meta_version_when_name_changes() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();
        assert_eq!(inserted.meta_version, 1);

        let to_update = Calendar {
            name: "Renamed".to_string(),
            ..inserted.clone()
        };
        let updated = CalendarMapper::update_calendar_with(&mut tx, to_update)
            .await
            .unwrap();
        assert_eq!(updated.meta_version, 2);
        assert_eq!(updated.name, "Renamed");
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn update_calendar_bumps_meta_version_when_language_changes() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();

        let to_update = Calendar {
            language: Language::Native,
            ..inserted.clone()
        };
        let updated = CalendarMapper::update_calendar_with(&mut tx, to_update)
            .await
            .unwrap();
        assert_eq!(updated.meta_version, 2);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn update_calendar_bumps_meta_version_when_event_style_changes() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();

        let to_update = Calendar {
            event_style: "all_day".to_owned(),
            ..inserted.clone()
        };
        let updated = CalendarMapper::update_calendar_with(&mut tx, to_update)
            .await
            .unwrap();
        assert_eq!(updated.meta_version, 2);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn update_calendar_no_op_does_not_bump_meta_version() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();

        let updated = CalendarMapper::update_calendar_with(&mut tx, inserted.clone())
            .await
            .unwrap();
        assert_eq!(updated.meta_version, inserted.meta_version);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn save_calendar_with_item_ids_round_trips() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let cal = Calendar {
            item_ids: vec![101, 202, 303],
            ..new_calendar(user_id)
        };
        let saved = CalendarMapper::save_calendar_with(&mut tx, cal)
            .await
            .unwrap();
        let fetched = CalendarMapper::get_calendar_by_id_with(&mut tx, saved.id, user_id)
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
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();
        CalendarMapper::delete_calendar_with(&mut tx, inserted.id, user_id)
            .await
            .unwrap();
        assert!(
            CalendarMapper::get_calendar_by_id_with(&mut tx, inserted.id, user_id)
                .await
                .is_err()
        );
        assert!(
            CalendarMapper::get_calendar_by_token_with(&mut tx, &inserted.subscription_token)
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn delete_calendar_returns_subscription_token() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();
        let token = CalendarMapper::delete_calendar_with(&mut tx, inserted.id, user_id)
            .await
            .unwrap();
        assert_eq!(token, inserted.subscription_token);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn delete_calendar_preserves_calendar_items_as_audit_trail() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let cal = Calendar {
            item_ids: vec![42, 43],
            ..new_calendar(user_id)
        };
        let inserted = CalendarMapper::save_calendar_with(&mut tx, cal)
            .await
            .unwrap();
        CalendarMapper::delete_calendar_with(&mut tx, inserted.id, user_id)
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
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();
        assert!(
            CalendarMapper::delete_calendar_with(&mut tx, inserted.id, i32::MAX)
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_calendars_paginated_returns_all_active() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();
        CalendarMapper::insert_calendar_with(
            &mut tx,
            Calendar {
                name: "Second".to_string(),
                ..new_calendar(user_id)
            },
        )
        .await
        .unwrap();
        let (cals, total) =
            CalendarMapper::get_calendars_by_user_paginated_with(&mut tx, user_id, 1, 10)
                .await
                .unwrap();
        assert_eq!(total, 2);
        assert_eq!(cals.len(), 2);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_calendars_paginated_returns_recent_item_ids_most_recent_first() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(
            &mut tx,
            Calendar {
                item_ids: vec![],
                ..new_calendar(user_id)
            },
        )
        .await
        .unwrap();
        // Insert 5 items with ascending added_at; ids are 10..14, expect newest 4 in DESC order.
        for (i, item_id) in [10, 11, 12, 13, 14].iter().enumerate() {
            sqlx::query!(
                "INSERT INTO calendar_items (calendar_id, item_id, added_at) VALUES ($1, $2, NOW() + ($3 || ' seconds')::interval)",
                inserted.id,
                item_id,
                i.to_string(),
            )
            .execute(&mut *tx)
            .await
            .unwrap();
        }
        let (cals, _) =
            CalendarMapper::get_calendars_by_user_paginated_with(&mut tx, user_id, 1, 10)
                .await
                .unwrap();
        assert_eq!(cals.len(), 1);
        let (_cal, recent, editor_count) = &cals[0];
        assert_eq!(recent, &vec![14, 13, 12, 11]);
        assert_eq!(*editor_count, 0);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_calendars_paginated_returns_empty_recent_item_ids_for_empty_calendar() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        CalendarMapper::insert_calendar_with(
            &mut tx,
            Calendar {
                item_ids: vec![],
                ..new_calendar(user_id)
            },
        )
        .await
        .unwrap();
        let (cals, _) =
            CalendarMapper::get_calendars_by_user_paginated_with(&mut tx, user_id, 1, 10)
                .await
                .unwrap();
        assert_eq!(cals.len(), 1);
        let (_cal, recent, editor_count) = &cals[0];
        assert!(recent.is_empty());
        assert_eq!(*editor_count, 0);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn get_calendars_paginated_excludes_deleted() {
        let mut tx = crate::test_helpers::test_tx().await;
        let user_id = create_test_user(&mut tx).await;
        let inserted = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(user_id))
            .await
            .unwrap();
        CalendarMapper::delete_calendar_with(&mut tx, inserted.id, user_id)
            .await
            .unwrap();
        let (cals, total) =
            CalendarMapper::get_calendars_by_user_paginated_with(&mut tx, user_id, 1, 10)
                .await
                .unwrap();
        assert_eq!(total, 0);
        assert!(cals.is_empty());
        tx.rollback().await.unwrap();
    }


    #[tokio::test]
    async fn get_calendars_paginated_returns_editor_count() {
        use crate::mappers::calendar_editor::CalendarEditorMapper;
        let mut tx = crate::test_helpers::test_tx().await;
        let owner_id = create_test_user(&mut tx).await;
        let editor_a = create_test_user(&mut tx).await;
        let editor_b = create_test_user(&mut tx).await;
        let cal = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(owner_id))
            .await
            .unwrap();
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal.id, editor_a)
            .await
            .unwrap();
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal.id, editor_b)
            .await
            .unwrap();

        let (cals, _) =
            CalendarMapper::get_calendars_by_user_paginated_with(&mut tx, owner_id, 1, 10)
                .await
                .unwrap();
        assert_eq!(cals.len(), 1);
        assert_eq!(cals[0].2, 2, "two active editors should be counted");
        tx.rollback().await.unwrap();
    }


    #[tokio::test]
    async fn list_shared_with_user_returns_only_active_editor_calendars_with_owner() {
        use crate::mappers::calendar_editor::CalendarEditorMapper;
        let mut tx = crate::test_helpers::test_tx().await;
        let owner_id = create_test_user(&mut tx).await;
        let editor_id = create_test_user(&mut tx).await;
        let other_id = create_test_user(&mut tx).await;

        let owner_username: String = sqlx::query_scalar!(
            "SELECT username FROM users WHERE id = $1",
            owner_id,
        )
        .fetch_one(&mut *tx)
        .await
        .unwrap();

        // Owner has 2 calendars; editor is active on cal_a only.
        let cal_a = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(owner_id))
            .await
            .unwrap();
        let _cal_b = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(owner_id))
            .await
            .unwrap();
        // `other_id` also has a calendar — must NOT show up for the editor either.
        let _cal_other = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(other_id))
            .await
            .unwrap();

        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal_a.id, editor_id)
            .await
            .unwrap();

        let shared = CalendarMapper::list_shared_with_user_with(&mut tx, editor_id)
            .await
            .unwrap();
        assert_eq!(shared.len(), 1, "only cal_a (active editor) shows");
        let (cal, owner, _recent) = &shared[0];
        assert_eq!(cal.id, cal_a.id);
        assert_eq!(owner.id, owner_id);
        assert_eq!(owner.display, owner_username);
        assert!(owner.avatar.is_none());
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn list_shared_with_user_excludes_suspended_editor_rows() {
        use crate::mappers::calendar_editor::CalendarEditorMapper;
        let mut tx = crate::test_helpers::test_tx().await;
        let owner_id = create_test_user(&mut tx).await;
        let editor_id = create_test_user(&mut tx).await;
        let cal = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(owner_id))
            .await
            .unwrap();
        CalendarEditorMapper::upsert_active_in_tx(&mut tx, cal.id, editor_id)
            .await
            .unwrap();
        CalendarEditorMapper::suspend_for_calendar_in_tx(&mut tx, cal.id)
            .await
            .unwrap();

        let shared = CalendarMapper::list_shared_with_user_with(&mut tx, editor_id)
            .await
            .unwrap();
        assert!(shared.is_empty(), "suspended editor row should not appear");
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn list_shared_with_user_excludes_owners_own_calendars() {
        let mut tx = crate::test_helpers::test_tx().await;
        let owner_id = create_test_user(&mut tx).await;
        let _cal = CalendarMapper::insert_calendar_with(&mut tx, new_calendar(owner_id))
            .await
            .unwrap();

        let shared = CalendarMapper::list_shared_with_user_with(&mut tx, owner_id)
            .await
            .unwrap();
        assert!(shared.is_empty(), "owners do not appear in their own shared list");
        tx.rollback().await.unwrap();
    }
}
