#![allow(dead_code)]

use std::path::Path;

use anyhow::{Result, anyhow};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{
    Arguments, Pool, Sqlite,
    query::Query,
    query_builder::Separated,
    sqlite::{SqliteArguments, SqliteConnectOptions, SqlitePoolOptions},
};

use crate::utils::now;

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::Decode, sqlx::Encode, sqlx::FromRow)]
pub struct Watchlist {
    pub id: i64,
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: i64,
    pub author_id: i64,
    pub revision: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::Decode, sqlx::Encode, sqlx::FromRow)]
pub struct Entry {
    pub id: i64,
    pub list_id: i64,
    pub ordinal: i64,
    pub name: String,
    pub author_id: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

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

        if let Err(migrate_result) = sqlx::migrate!().run(&pool).await {
            // TODO(ayvi0001): handle if migration fails
            tracing::error!("Sqlx migration error: {:?}", migrate_result.to_string());
        };

        Ok(Self { pool })
    }

    pub async fn get_list(&self, guild_id: i64, channel_id: i64) -> Result<Option<Watchlist>> {
        let watchlist: Option<Watchlist> = sqlx::query_as!(
            Watchlist,
            r#"SELECT
                   id AS "id!",
                   guild_id AS "guild_id!",
                   channel_id AS "channel_id!",
                   message_id AS "message_id!",
                   author_id AS "author_id!",
                   revision AS "revision!",
                   created_at AS "created_at!: NaiveDateTime",
                   updated_at AS "updated_at!: NaiveDateTime"
               FROM
                   lists
               WHERE
                   guild_id = $1
                   AND channel_id = $2;"#,
            guild_id,
            channel_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(watchlist)
    }

    pub async fn get_list_entries(&self, list_id: i64) -> Result<Vec<Entry>> {
        let entries: Vec<Entry> = sqlx::query_as!(
            Entry,
            r#"SELECT
                   id AS "id!",
                   list_id AS "list_id!",
                   ordinal AS "ordinal!",
                   name AS "name!",
                   author_id AS "author_id!",
                   created_at AS "created_at!",
                   updated_at AS "updated_at!"
               FROM
                   "entries"
               WHERE
                   list_id = $1
               ORDER BY
                   ordinal;"#,
            list_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(entries)
    }

    pub async fn update_list(
        &self,
        guild_id: i64,
        channel_id: i64,
        message_id: i64,
        author_id: i64,
        content: Option<&str>,
    ) -> Result<i64> {
        let mut transaction: sqlx::Transaction<'_, Sqlite> = self.pool.begin().await?;

        let timestamp: i64 = now();

        let revision_id: i64 = sqlx::query!(
            r#"INSERT INTO revisions (guild_id, channel_id, author_id, content, created_at) VALUES ($1, $2, $3, $4, $5);"#,
            guild_id,
            channel_id,
            author_id,
            content.unwrap_or_default(),
            timestamp,
        )
        .execute(&mut *transaction)
        .await?
        .last_insert_rowid();

        sqlx::query!(
            r#"INSERT INTO lists (guild_id, channel_id, message_id, author_id, revision, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6)
                   ON CONFLICT (channel_id)
                   DO UPDATE SET
                       message_id = excluded.message_id,
                       updated_at = excluded.updated_at;"#,
            guild_id,
            channel_id,
            message_id,
            author_id,
            revision_id,
            timestamp,
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(revision_id)
    }

    pub async fn delete_list_entries(&self, list_id: i64, entries: &Vec<&str>) -> Result<()> {
        let mut transaction: sqlx::Transaction<'_, Sqlite> = self.pool.begin().await?;

        let mut arguments = SqliteArguments::default();

        arguments.add(list_id).map_err(|_| anyhow!("failed to bind list id"))?;

        let mut query_builder = sqlx::QueryBuilder::with_arguments(
            "DELETE FROM entries WHERE list_id = ? AND name IN (",
            arguments,
        );

        let mut separated: Separated<'_, Sqlite, &str> = query_builder.separated(",");

        entries.iter().for_each(|e| {
            separated.push_bind(e);
        });

        separated.push_unseparated(")");

        let query: Query<'_, Sqlite, SqliteArguments> = query_builder.build();

        query.execute(&mut *transaction).await?;

        transaction.commit().await?;

        Ok(())
    }

    pub async fn delete_all_list_entries(&self, list_id: i64) -> Result<()> {
        let mut transaction: sqlx::Transaction<'_, Sqlite> = self.pool.begin().await?;

        sqlx::query!(r#"DELETE FROM entries WHERE list_id = ?;"#, list_id,)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;

        Ok(())
    }

    pub async fn update_list_entries(
        &self,
        list_id: i64,
        auther_id: i64,
        entries: Vec<&str>,
    ) -> Result<()> {
        let mut transaction: sqlx::Transaction<'_, Sqlite> = self.pool.begin().await?;

        let timestamp = now();

        for (idx, entry) in entries.into_iter().enumerate() {
            sqlx::query!(
                r#"INSERT INTO entries (ordinal, list_id, author_id, name, created_at, updated_at)
                       VALUES ($1, $2, $3, $4, $5, $5)
                       ON CONFLICT("list_id", "name")
                       DO UPDATE SET
                           ordinal = $1,
                           updated_at = $5;"#,
                idx as i64,
                list_id,
                auther_id,
                entry,
                timestamp,
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(())
    }
}
