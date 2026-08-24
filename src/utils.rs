use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

pub fn message_link(guild_id: i64, channel_id: i64, message_id: i64) -> String {
    format!("https://discord.com/channels/{guild_id}/{channel_id}/{message_id}")
}
