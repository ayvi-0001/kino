use anyhow::{Context as _, Result};
use reqwest::{self};
use serde_json::Value;

// static TMDB_API_READ_ACCESS_TOKEN: &str = env!("TMDB_API_READ_ACCESS_TOKEN");

pub async fn search_movies(name: String, year: Option<i64>) -> Result<Value, anyhow::Error> {
    let url = "https://api.themoviedb.org/3/search/movie";

    let mut querystring = vec![
        ("query", name.as_str()),
        ("include_adult", "true"),
        ("language", "en-US"),
    ];

    let mut year_str = String::default();
    {
        if let Some(year_value) = year {
            year_str = format!("{}", year_value);
        }
        if !year_str.is_empty() {
            querystring.push(("year", year_str.as_str()));
        }
    }

    let client = reqwest::Client::new();

    let tmdb_token: String = std::env::var("TMDB_API_READ_ACCESS_TOKEN")
        .context("missing env var TMDB_API_READ_ACCESS_TOKEN")?;

    let response = client
        .get(url)
        .query(&querystring)
        .header("Authorization", format!("Bearer {}", tmdb_token))
        .header("accept", "application/json")
        .send()
        .await?;

    response.json::<Value>().await.map_err(Into::into)
}

pub fn letterboxd_url(tmdb_id: i64) -> String {
    format!("https://letterboxd.com/tmdb/{}", tmdb_id)
}

pub fn get_response_results(response: Value) -> Vec<Value> {
    response["results"]
        .as_array()
        .map_or_else(|| vec![serde_json::Value::Null], |a| a.to_owned())
}
