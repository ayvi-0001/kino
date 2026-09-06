use serde::{Deserialize, Serialize};
use sqlx::{Decode, Encode, FromRow};

#[derive(Clone, Debug, Serialize, Deserialize, Decode, Encode, FromRow)]
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

#[derive(Clone, Debug, Serialize, Deserialize, Decode, Encode, FromRow)]
pub struct Entry {
    pub id: i64,
    pub list_id: i64,
    pub ordinal: i64,
    pub name: String,
    pub author_id: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
