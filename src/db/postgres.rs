use anyhow::Result;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions, query_builder::Separated};

use super::{Entry, Watchlist};
use crate::utils::now;

#[derive(Clone)]
pub struct Database {
    pub pool: Pool<Postgres>,
}

impl Database {
    const MAX_CONNECTIONS: u32 = 8;

    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool: Pool<Postgres> = PgPoolOptions::new()
            .max_connections(Database::MAX_CONNECTIONS)
            .idle_timeout(std::time::Duration::from_secs(300))
            .max_lifetime(std::time::Duration::from_secs(1800))
            .test_before_acquire(true)
            .connect(database_url)
            .await?;

        if let Err(migrate_result) = sqlx::migrate!("migrations/postgres").run(&pool).await {
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
                   created_at AS "created_at!",
                   updated_at AS "updated_at!"
               FROM
                   guild.lists
               WHERE
                   guild_id = $1
                   AND channel_id = $2;"#,
            guild_id,
            channel_id,
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
                   guild.entries
               WHERE
                   list_id = $1
               ORDER BY
                   ordinal;"#,
            list_id,
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
        let mut transaction: sqlx::Transaction<'_, Postgres> = self.pool.begin().await?;

        let timestamp: i64 = now();

        let revision_id: i64 = sqlx::query_scalar!(
            r#"INSERT INTO guild.revisions (guild_id, channel_id, author_id, content, created_at)
                   VALUES ($1, $2, $3, $4, $5)
                   RETURNING id;"#,
            guild_id,
            channel_id,
            author_id,
            content.unwrap_or_default(),
            timestamp as f64,
        )
        .fetch_one(&mut *transaction)
        .await?;

        sqlx::query!(
                r#"MERGE INTO guild.lists e
                USING (
                    VALUES ($1::bigint, $2::bigint, $3::bigint, $4::bigint, $5::bigint, $6::bigint)
                ) n(guild_id, channel_id, message_id, author_id, revision, updated_at)
                ON
                    e.guild_id = n.guild_id
                    AND e.channel_id = n.channel_id
                WHEN MATCHED THEN
                    UPDATE SET
                        message_id = n.message_id,
                        updated_at = n.updated_at,
                        revision = n.revision
                WHEN NOT MATCHED THEN
                    INSERT (guild_id, channel_id, message_id, author_id, revision, updated_at)
                        VALUES (n.guild_id, n.channel_id, n.message_id, n.author_id, n.revision, n.updated_at);"#,
            guild_id,
            channel_id,
            message_id,
            author_id,
            revision_id,
            timestamp as f64,
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(revision_id)
    }

    pub async fn delete_list_entries(&self, list_id: i64, entries: &Vec<&str>) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut transaction: sqlx::Transaction<'_, Postgres> = self.pool.begin().await?;

        let mut query_builder =
            sqlx::QueryBuilder::<Postgres>::new("DELETE FROM guild.entries WHERE list_id = ");

        query_builder.push_bind(list_id);
        query_builder.push(" AND name IN (");

        let mut separated: Separated<'_, Postgres, &str> = query_builder.separated(",");

        entries.iter().for_each(|e| {
            separated.push_bind(e);
        });

        separated.push_unseparated(")");

        query_builder.build().execute(&mut *transaction).await?;

        transaction.commit().await?;

        Ok(())
    }

    pub async fn delete_all_list_entries(&self, list_id: i64) -> Result<()> {
        let mut transaction: sqlx::Transaction<'_, Postgres> = self.pool.begin().await?;

        sqlx::query!(r#"DELETE FROM guild.entries WHERE list_id = $1;"#, list_id)
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
        let mut transaction: sqlx::Transaction<'_, Postgres> = self.pool.begin().await?;

        let timestamp = now();

        for (idx, entry) in entries.into_iter().enumerate() {
            sqlx::query!(
                r#"MERGE INTO guild.entries e
                USING (
                    VALUES ($1::bigint, $2::bigint, $3::bigint, $4::text, $5::bigint, $5::bigint)
                ) n(ordinal, list_id, author_id, "name", created_at, updated_at)
                ON
                    e.list_id = n.list_id
                    AND e.name = n.name
                WHEN MATCHED THEN
                    UPDATE SET
                    ordinal = n.ordinal,
                    updated_at = n.updated_at
                WHEN NOT MATCHED THEN
                    INSERT (ordinal, list_id, author_id, "name", created_at, updated_at)
                        VALUES (n.ordinal, n.list_id, n.author_id, n.name, n.created_at, n.updated_at);"#,
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
