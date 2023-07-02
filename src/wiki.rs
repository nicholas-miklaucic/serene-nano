//! Functionality for parsing Wikipedia pages.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::utils::{Context, Error};

fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(format!(
            "{} {} <nicholas.miklaucic@gmail.com>",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()
}

/// Finds the name of a search title corresponding to the given query.
async fn search_title(query: &str) -> Result<String> {
    let results: Value = client()?
        .get("https://en.wikipedia.org/w/api.php")
        .query(&[
            ("action", "opensearch"),
            ("search", query),
            ("limit", "1"),
            ("namespace", "0"), // main pages
            ("profile", "fuzzy"),
            ("redirects", "resolve"),
            ("format", "json"),
        ])
        .send()
        .await?
        .json()
        .await?;

    if let Value::Array(vals) = &results {
        if let [_search, _terms, _descs, Value::Array(urls)] = &vals[..] {
            // url is of form https://en.wikipedia.org/wiki/Serenity
            if let Value::String(url) = urls.get(0).ok_or(anyhow!("No link found for {}", query))? {
                return url
                    .clone()
                    .strip_prefix("https://en.wikipedia.org/wiki/")
                    .ok_or(anyhow!("Invalid URL: {}", url))
                    .map(|s| s.to_string());
            }
        }
    }

    Err(anyhow!("Parsing results for {query} failed:\n{results}"))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Summary {
    title: String,
    extract: String,
}

async fn summary(title: &str) -> Result<Summary> {
    let extract: Summary = client()?
        .get(format!(
            "https://en.wikipedia.org/api/rest_v1/page/summary/{title}"
        ))
        .query(&[("redirect", "true")])
        .send()
        .await?
        .json()
        .await?;

    Ok(extract)
}

/// Gets the Wikipedia summary of the given search.
#[poise::command(
    slash_command,
    prefix_command,
    track_edits,
    invoke_on_edit,
    reuse_response,
    track_deletion
)]
pub(crate) async fn wiki(
    ctx: Context<'_>,
    #[description = "The search query"]
    #[rest]
    query: String,
) -> Result<(), Error> {
    let title = search_title(&query).await?;
    let extract = summary(&title).await?;
    ctx.say(format!("## {}\n{}", extract.title, extract.extract))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search() {
        assert_eq!(
            search_title("horeshoe theory").await.unwrap(),
            "Horseshoe_theory"
        );
    }

    #[tokio::test]
    async fn test_summary() {
        assert!(summary("Horseshoe_theory")
            .await
            .unwrap()
            .extract
            .starts_with(
                "In popular discourse, the horseshoe theory asserts that the extreme left"
            ));
    }
}
