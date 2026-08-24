use std::path::Path;

use anyhow::Result;
use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

#[allow(dead_code)]
#[derive(Clone)]
pub struct Database {
    pub pool: Pool<Sqlite>,
}

impl Database {
    const MAX_CONNECTIONS: u32 = 8;

    pub async fn connect(database_url: &str) -> Result<Self> {
        let path = Path::new(database_url);
        if let Some(dir) = path.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }

        let options: SqliteConnectOptions = SqliteConnectOptions::new()
            .filename(database_url)
            .create_if_missing(true)
            .foreign_keys(true)
            .auto_vacuum(sqlx::sqlite::SqliteAutoVacuum::Full)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .pragma("temp_store", "MEMORY")
            .pragma("cache_size", "-16000")
            .optimize_on_close(true, None);

        let pool: Pool<Sqlite> = SqlitePoolOptions::new()
            .max_connections(Database::MAX_CONNECTIONS)
            .idle_timeout(std::time::Duration::from_secs(300))
            .max_lifetime(std::time::Duration::from_secs(1800))
            .test_before_acquire(true)
            .connect_with(options)
            .await?;

        Ok(Self { pool })
    }
}
