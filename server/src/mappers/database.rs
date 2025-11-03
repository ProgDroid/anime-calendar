use sqlx::{Pool, Postgres, QueryBuilder};

use crate::{
    config::database::Database as Config,
    entity::calendar::{Calendar, Language},
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
                config.pass,
                format_args!("{}:{}", config.host, config.port),
                config.database
            )
            .as_str(),
        )
        .await?;

        Ok(Self { client: pool })
    }

    // TODO resolve these sign shenanigans
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub async fn get_calendar(&self, id: u64) -> ServerResult<Option<Calendar>> {
        let mut db_calendar = sqlx::query_as!(
            Calendar,
            "SELECT
                id,
                ARRAY[]::INTEGER[] as \"item_ids!\",
                language as \"language: Language\",
                name
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

        Ok(Calendar { id, ..calendar })
    }

    async fn update_calendar(&self, calendar: Calendar) -> ServerResult<Calendar> {
        sqlx::query!(
            "UPDATE calendars
            SET
                language = $1,
                name = $2
            WHERE id = $3",
            calendar.language.clone() as Language,
            calendar.name.clone(),
            calendar.id.clone()
        )
        .execute(&self.client)
        .await?;

        let calendar_id = calendar.id;

        let inserts: Vec<(i32, i32)> = calendar
            .item_ids
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

        Ok(calendar)
    }
}
