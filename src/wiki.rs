//! Wikipedia parsing command.

use regex::Regex;

use crate::utils::{Context, Error};

/// Get the Wikipedia entry for the given search term.
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
    #[description = "The search term"]
    #[rest]
    search: String,
) -> Result<(), Error> {
    let wiki = wikipedia::Wikipedia::<wikipedia::http::default::Client>::default();
    let titles = wiki.search(search.as_str());
    let page = wiki.page_from_title(titles.unwrap().first().unwrap().to_string());
    let wiki_text = page.get_summary().unwrap();
    // hacky way of avoiding huge newlines around math text: work on making this better in the
    // future
    let re1 = Regex::new(r"\n +").unwrap();
    let re2 = Regex::new(r"(?P<first>\w+)\{.*\}").unwrap();
    let intermediate = re1.replace_all(&wiki_text, "");
    let processed = re2.replace_all(&intermediate, "$first");
    let sentences: Vec<&str> = processed.split(". ").take(3).collect();

    ctx.say(sentences.join(". ")).await?;

    Ok(())
}
