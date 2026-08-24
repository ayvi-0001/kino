use crate::db;

#[allow(dead_code)]
pub struct Data {
    #[allow(dead_code)]
    pub(super) db: db::Database,
    pub(super) write_lock: tokio::sync::Mutex<()>,
}

// type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Error = anyhow::Error;

pub type Context<'a> = poise::Context<'a, Data, Error>;

impl Data {
    pub fn new(db: db::Database) -> Self {
        Data { db, write_lock: tokio::sync::Mutex::new(()) }
    }
}
