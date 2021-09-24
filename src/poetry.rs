//! Program to get poetry from the Web.
use std::error::Error;

use reqwest;
use scraper;

/// A poem with poet, link to site, and text.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct Poem {
    /// The poet's name.
    pub poet: String,
    /// The poem's text.
    pub poem: String,
    /// The poem's title.
    pub title: String,
    /// The URL to the poem.
    pub url: String,
}

/// Searches for a poem matching the given query and returns the text of the
/// first one that matches, or None if something went wrong.
pub(crate) async fn search_poem(query: &str) -> Option<Poem> {
    let client = reqwest::Client::new();
    let r = client
        .get("https://www.poetryfoundation.org/search")
        .query(&[("query", query)])
        .send()
        .await
        .ok()?;

    let link = scraper::Selector::parse("h2 > a").unwrap();
    let text = r.text().await.ok()?;
    let a = scraper::Html::parse_fragment(&text)
        .select(&link)
        .next()
        .and_then(|x| x.value().attr("href"))
        .and_then(|x| Some(x.to_string()))?;

    let mut url = "https://poetryfoundation.org".to_string();
    url.push_str(&a);
    get_poem(&url).await
}

/// Given a URL to a poem, returns the full Poem object.
pub(crate) async fn get_poem(url: &str) -> Option<Poem> {
    let content = reqwest::get(url).await.ok()?;
    let req_text = content.text().await.ok()?;
    parse_poem(url, req_text)
}

/// Parses a poem given the URL and the HTML of the URL.
fn parse_poem(url: &str, content: String) -> Option<Poem> {
    let html = scraper::Html::parse_fragment(&content);
    let div_poem = scraper::Selector::parse("div.o-poem").unwrap();
    let span_poet = scraper::Selector::parse("span.c-txt_attribution a").unwrap();
    let h1_title = scraper::Selector::parse("h1").unwrap();
    match html
        .select(&div_poem)
        .next()
        .zip(html.select(&span_poet).next())
        .zip(html.select(&h1_title).next())
    {
        None => None,
        Some(((div, poet_span), title_h1)) => {
            let text: Vec<_> = div.text().collect();
            let poem = text.join("\n");
            let poet = String::from(poet_span.text().next().unwrap_or("[Author not found]"));
            let title = String::from(title_h1.text().next().unwrap_or("[Title not found]"));
            Some(Poem {
                poet,
                poem,
                title,
                url: url.to_string(),
            })
        }
    }
}
