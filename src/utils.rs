pub use kino_db::now;

pub fn message_link(guild_id: i64, channel_id: i64, message_id: i64) -> String {
    format!("https://discord.com/channels/{guild_id}/{channel_id}/{message_id}")
}
