use serde::{Deserialize, Serialize};

use crate::{id::Id, schedule::Schedule, title::Title};

#[derive(Serialize, Deserialize)]
pub struct Item {
    pub id: Id,
    pub id_mal: Option<i64>,
    pub title: Title,
    pub airing_schedule: Vec<Schedule>,
    pub episode_duration: i64,
}

pub trait Repository {
    fn get_item(&self, id: u64) -> impl std::future::Future<Output = Option<Item>> + Send; // TODO should input here be Id type
}
