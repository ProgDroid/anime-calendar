mod calendar;

use sqlx::{Pool, Postgres};

use crate::{
    config::database::Database as Config, entity::calendar::Calendar,
    mappers::database::calendar::Calendar as DbCalendar, ServerResult,
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
        match sqlx::query_as!(
            DbCalendar,
            "SELECT * FROM calendars WHERE id = $1",
            id as i32
        )
        .fetch_optional(&self.client)
        .await?
        {
            Some(db_calendar) => {
                let item_ids = sqlx::query_scalar!(
                    "SELECT item_id FROM calendar_items WHERE calendar_id = $1",
                    id as i32
                )
                .fetch_all(&self.client)
                .await?;

                let item_ids: Vec<u64> = item_ids
                    .iter()
                    .map(|id| (*id).try_into().unwrap())
                    .collect();

                Ok(Some(Calendar {
                    id: db_calendar.id as u64,
                    item_ids,
                    language: db_calendar.language,
                    name: db_calendar.name,
                }))
            }
            None => Ok(None),
        }
    }
}
