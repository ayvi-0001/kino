use std::sync::Arc;

use ::serenity::all::{ClientBuilder, GuildInfo};
use anyhow::{Context as _, Result};
use poise::{Command, serenity_prelude as serenity};
use serenity::all::{GatewayIntents, ShardManager};

use crate::state::{Data, Error};

pub(crate) mod macros;
crate::mod_flat!(commands, state);

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .init();

    let token: String = std::env::var("DISCORD_TOKEN").context("missing env var DISCORD_TOKEN")?;

    let global_commands: Vec<Command<Data, Error>> = vec![];
    let commands: Vec<Command<Data, Error>> = vec![commands::register()];

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions { commands, ..Default::default() })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
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

                Ok(Data::new())
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
