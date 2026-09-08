use poise::CreateReply;

use crate::state::{Data, Error};

const BOT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// show version info
#[poise::command(prefix_command, slash_command)]
pub async fn version(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    let content = format!(
        "```\nv{}\n{}\n({}) {}{}\n{}```",
        BOT_VERSION,
        env!("VERGEN_CARGO_TARGET_TRIPLE"),
        env!("VERGEN_GIT_BRANCH"),
        env!("GIT_DESCRIBE"),
        git_shortstat().unwrap_or_default(),
        vergen_build_timestamp(),
    );

    ctx.send(CreateReply::default().content(content).ephemeral(true)).await?;

    Ok(())
}

fn git_shortstat() -> Option<String> {
    if env!("VERGEN_GIT_DIRTY") == "true" {
        Some(format!(
            "+{}-{}",
            env!("GIT_INSERTIONS"),
            env!("GIT_DELETIONS")
        ))
    } else {
        None
    }
}

fn vergen_build_timestamp() -> String {
    let build_timestamp: chrono::DateTime<chrono::FixedOffset> =
        chrono::DateTime::parse_from_rfc3339(env!("VERGEN_BUILD_TIMESTAMP")).unwrap_or_else(|_| {
            panic!(
                "vergen build script failed to run, missing env var: {}",
                "VERGEN_BUILD_TIMESTAMP"
            )
        });

    build_timestamp
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%dT%H:%M:%S%z")
        .to_string()
}
