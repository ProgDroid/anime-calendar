mod calendar;

use common::{calendar::Calendar, id::Id, language::Language};
use sqlx::{Pool, Postgres};

use crate::{
    config::database::Database as Config, mappers::database::calendar::Calendar as DbCalendar,
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

    pub async fn get_calendar(&self, id: u64) -> ServerResult<Option<Calendar>> {
        match sqlx::query!("SELECT * FROM calendars WHERE id = ?", id)
            .fetch_optional(&self.client)
            .await?
        {
            Some(db_calendar) => {
                let items = Vec::default();

                Ok(Some(Calendar {
                    id: Id::from(db_calendar.id),
                    items,
                    language: Language::from(db_calendar.language),
                    name: db_calendar.name,
                }))
            }
            None => Ok(None),
        }
    }
}

// TODO queries
