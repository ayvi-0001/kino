#![cfg(feature = "tmdb")]

use anyhow::{Context, anyhow};
use kino_tmdb::{letterboxd_url, search_movies};

use crate::state::{Data, Error};

/// search for a movie and return the letterboxd url
#[poise::command(prefix_command, slash_command)]
pub async fn search(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "movie title"] name: String,
    #[description = "release year"] year: Option<i64>,
) -> Result<(), Error> {
    let response = search_movies(name, year).await?;
    let results = response["results"]
        .as_array()
        .map_or_else(|| vec![serde_json::Value::Null], |a| a.to_owned());

    if results.is_empty() {
        ctx.send(poise::CreateReply::default().content("no matches found!").ephemeral(true))
            .await?;
        return Ok(());
    }

    let tmdb_id = if let Some(id) = results[0].get("id") {
        id.as_i64().context("TMDB ID must be a number.")?
    } else {
        return Err(anyhow!("Could not determine TMDB ID."));
    };

    ctx.send(poise::CreateReply::default().content(letterboxd_url(tmdb_id)).ephemeral(true))
        .await?;

    Ok(())
}
