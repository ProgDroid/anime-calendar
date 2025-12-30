// server/src/database.rs
use sqlx::PgPool;

pub struct Database {
    pub pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Database { pool }
    }
}
