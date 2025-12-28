use secrecy::ExposeSecret;
use sqlx::{Pool, Postgres, QueryBuilder};

use crate::{
    config::database::Database as Config,
    entity::calendar::{Calendar, Language},
    entity::user::User,
    ServerResult,
};

#[derive(Clone)]
pub struct Database {
    client: Pool<Postgres>,
}

impl Database {
    pub async fn new(config: Config) -> ServerResult<Self> {
        let pool = Pool::connect(
            format!(
                "postgres://{}:{}@{}/{}",
                config.user,
                config.pass.expose_secret(),
                format_args!("{}:{}", config.host, config.port),
                config.database
            )
            .as_str(),
        )
        .await?;

        Ok(Self { client: pool })
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub async fn get_calendar(&self, id: u64) -> ServerResult<Option<Calendar>> {
        let mut db_calendar = sqlx::query_as!(
            Calendar,
            "SELECT
                id,
                ARRAY[]::INTEGER[] as \"item_ids!\",
                language as \"language: Language\",
                name,
                user_id,
                created_at,
                updated_at
            FROM calendars WHERE id = $1",
            id as i32
        )
        .fetch_one(&self.client)
        .await?;

        db_calendar.item_ids = sqlx::query_scalar!(
            "SELECT item_id FROM calendar_items WHERE calendar_id = $1",
            id as i32
        )
        .fetch_all(&self.client)
        .await?;

        Ok(Some(db_calendar))
    }

    pub async fn save_calendar(&self, calendar: Calendar) -> ServerResult<Calendar> {
        match calendar.id {
            0 => self.insert_calendar(calendar).await,
            _ => self.update_calendar(calendar).await,
        }
    }

    async fn insert_calendar(&self, calendar: Calendar) -> ServerResult<Calendar> {
        let id = sqlx::query_scalar!(
            "INSERT INTO calendars (
                language,
                name
            ) VALUES ($1, $2)
            RETURNING id",
            calendar.language.clone() as Language,
            calendar.name.clone()
        )
        .fetch_one(&self.client)
        .await?;

        self.update_calendar_items(id, calendar.item_ids.clone())
            .await?;

        Ok(Calendar { id, ..calendar })
    }

    async fn update_calendar(&self, calendar: Calendar) -> ServerResult<Calendar> {
        sqlx::query!(
            "UPDATE calendars
            SET
                language = $1,
                name = $2,
                user_id = $3
            WHERE id = $4",
            calendar.language.clone() as Language,
            calendar.name.clone(),
            calendar.user_id,
            calendar.id.clone()
        )
        .execute(&self.client)
        .await?;

        self.update_calendar_items(calendar.id, calendar.item_ids.clone())
            .await?;

        Ok(calendar)
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

        query_builder.build().execute(&self.client).await?;

        Ok(())
    }

    // User methods
    pub async fn get_user_by_id(&self, id: i32) -> ServerResult<Option<User>> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE id = $1",
            id
        )
        .fetch_optional(&self.client)
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_email(&self, email: &str) -> ServerResult<Option<User>> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.client)
        .await?;

        Ok(user)
    }

    pub async fn create_user(
        &self,
        username: String,
        email: String,
        password_hash: Option<String>,
    ) -> ServerResult<User> {
        let user = sqlx::query_as!(
            User,
            "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id, username, email, password_hash, created_at, updated_at",
            username,
            email,
            password_hash
        )
        .fetch_one(&self.client)
        .await?;

        Ok(user)
    }
}
