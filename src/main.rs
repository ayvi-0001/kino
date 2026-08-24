use std::sync::Arc;

use ::serenity::all::{ClientBuilder, GuildInfo};
use anyhow::{Context as _, Result};
use lazy_static::lazy_static;
use poise::{Command, serenity_prelude as serenity};
use serenity::all::{GatewayIntents, ShardManager};

use crate::state::{Data, Error};

pub(crate) mod macros;
crate::mod_flat!(commands, db, diff, state, utils);

lazy_static! {
    pub static ref DEV_GUILD: Option<serenity::GuildId> = match std::env::var("DEV_GUILD_ID") {
        Ok(value) => Some(serenity::GuildId::new(
            value.trim().parse().expect("DEV_GUILD_ID must be a numeric guild id"),
        )),
        Err(_) => None,
    };
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .init();

    let token: String = std::env::var("DISCORD_TOKEN").context("missing env var DISCORD_TOKEN")?;

    let db_url = std::env::var("DATABASE_URL").unwrap_or("data/watchlist.db".to_owned());
    let db = db::Database::connect(&db_url).await.context("could not open the database")?;

    let global_commands: Vec<Command<Data, Error>> = vec![];
    let commands: Vec<Command<Data, Error>> = vec![commands::register(), commands::watchlist()];

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions { commands, ..Default::default() })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                if let Some(dev_guild_id) = *DEV_GUILD {
                    poise::builtins::register_in_guild(
                        &ctx.http,
                        &framework.options().commands,
                        dev_guild_id,
                    )
                    .await?;
                } else {
                    poise::builtins::register_globally(&ctx.http, &global_commands).await?;

                    for guild in
                        ctx.http.get_guilds(None, None).await?.iter().collect::<Vec<&GuildInfo>>()
                    {
                        poise::builtins::register_in_guild(
                            &ctx.http,
                            &framework.options().commands,
                            guild.id,
                        )
                        .await?;
                    }
                }
                Ok(Data::new(db))
            })
        })
        .build();

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let mut client: serenity::Client = ClientBuilder::new(&token, intents)
        .framework(framework)
        .await
        .context("failed to build client")?;

    let shard_manager: Arc<ShardManager> = client.shard_manager.clone();

    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            tracing::info!("shutting down");
            shard_manager.shutdown_all().await;
        }
    });

    if let Err(why) = client.start().await.context("the Discord client stopped unexpectedly") {
        tracing::error!("client error: {why:?}");
    }

    Ok(())
}
