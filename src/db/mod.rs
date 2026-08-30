use serde::{Deserialize, Serialize};

#[cfg(feature = "postgres")]
mod postgres;
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
mod sqlite;

#[cfg(feature = "postgres")]
pub use postgres::Database;
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub use sqlite::Database;

#[cfg(not(any(feature = "sqlite", feature = "postgres")))]
compile_error!("either the `sqlite` or `postgres` feature must be enabled");

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::Decode, sqlx::Encode, sqlx::FromRow)]
pub struct Watchlist {
    pub id: i64,
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: i64,
    pub author_id: i64,
    pub revision: i64,
    pub created_at: i64,
    pub updated_at: i64,
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
