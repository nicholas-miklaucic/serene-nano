//! Support for dictionary API.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serenity::builder::CreateMessage;
use serenity_additions::menu::{MenuBuilder, Page};

use crate::utils::{Context, Error};

/// Base URL for API.
const API_URL: &str = "https://api.dictionaryapi.dev/api/v2/entries/en";

/// A dictionary definition.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DictionaryDefinition {
    pub word: String,
    pub phonetic: Option<String>,
    pub phonetics: Vec<Pronunciation>,
    pub meanings: Vec<Meaning>,
    pub source_urls: Vec<String>,
}

impl DictionaryDefinition {
    pub fn write_message<'a, 'b>(&self, m: &'a mut CreateMessage<'b>) -> &'a mut CreateMessage<'b> {
        m.embed(|e| {
            let mut e = e.title(self.word.clone()).description(
                self.phonetic
                    .clone()
                    .unwrap_or("No single pronunciation".to_string()),
            );
            if let Some(url) = self.source_urls.first() {
                e = e.url(url);
            };

            e = e
                .field(
                    "Pronunciations",
                    self.phonetics
                        .iter()
                        .map(|pro| pro.text.clone())
                        .collect::<Vec<String>>()
                        .join(", ")
                        .to_string(),
                    false,
                )
                .field(
                    "Audios",
                    self.phonetics
                        .iter()
                        .map(|pro| pro.audio.clone())
                        .collect::<Vec<String>>()
                        .join("\n"),
                    false,
                );

            for meaning in self.meanings.iter() {
                let mut def_str = String::new();
                for (i, def) in meaning.definitions.iter().enumerate() {
                    def_str.push_str(format!("{}. {}\n", i + 1, def.definition).as_str());
                    match &def.example {
                        Some(example) => {
                            def_str.push_str(format!("*\"{}\"*\n", example).as_str());
                        }
                        None => {}
                    }
                }
                e = e.field(format!("*{}*", meaning.part_of_speech), def_str, false);
            }

            e
        });

        m
    }
    pub fn write_page(&self) -> Page<'static> {
        let mut msg = CreateMessage::default();
        self.write_message(&mut msg);
        Page::new_static(msg)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pronunciation {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub audio: String,
    #[serde(default)]
    pub source_url: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meaning {
    pub part_of_speech: String,
    pub definitions: Vec<Definition>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Definition {
    pub definition: String,
    pub example: Option<String>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

async fn get_dictionary_definition(word: &str) -> Option<Vec<DictionaryDefinition>> {
    let client = reqwest::Client::new();
    let r = client.get(format!("{}/{}", API_URL, word)).send().await;

    let defs: Option<Vec<DictionaryDefinition>> = dbg!(r.ok()?.json().await).ok();
    defs
}

/// Define the given word using Wiktionary.
#[poise::command(
    slash_command,
    prefix_command,
    track_edits,
    invoke_on_edit,
    reuse_response,
    track_deletion
)]
pub(crate) async fn define(
    ctx: Context<'_>,
    #[description = "The word to define. Prefer headwords: \"serene\" instead of \"serenely.\""]
    word: String,
) -> Result<(), Error> {
    let defs_opt = get_dictionary_definition(word.as_str()).await;
    let menu = MenuBuilder::new_paginator().timeout(Duration::from_secs(120));

    match defs_opt {
        Some(defs) if defs.len() > 1 => {
            menu.add_pages(defs.into_iter().map(|d| d.write_page()))
                .show_help()
                .build(ctx.serenity_context(), ctx.channel_id())
                .await?;
        }
        Some(defs) if defs.len() == 1 => {
            let def = &defs[0];
            ctx.channel_id()
                .send_message(&ctx.http(), |m| def.write_message(m))
                .await?;
        }
        _other => {
            ctx.say("No definitions found. Try the word's root: for example, \"serene\" instead of \"serenely.\" ").await?;
        }
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_defs() {
        let defs = get_dictionary_definition("serenity").await;
        assert!(defs.is_some());
        assert_eq!(
            defs,
            Some(vec![DictionaryDefinition {
                word: "serenity".to_string(),
                phonetic: Some("/səˈɹɛnɪti/".to_string()),
                phonetics: vec![Pronunciation {
                    text: "/səˈɹɛnɪti/".to_string(),
                    audio: "https://api.dictionaryapi.dev/media/pronunciations/en/serenity-us.mp3"
                        .to_string(),
                    source_url: "https://commons.wikimedia.org/w/index.php?curid=1171246"
                        .to_string(),
                },],
                meanings: vec![Meaning {
                    part_of_speech: "noun".to_string(),
                    definitions: vec![
                        Definition {
                            definition: "The state of being serene; calmness; peacefulness."
                                .to_string(),
                            example: None,
                            synonyms: vec![],
                            antonyms: vec![],
                        },
                        Definition {
                            definition: "A lack of agitation or disturbance.".to_string(),
                            example: None,
                            synonyms: vec![],
                            antonyms: vec![],
                        },
                        Definition {
                            definition: "A title given to a reigning prince or similar dignitary."
                                .to_string(),
                            example: None,
                            synonyms: vec![],
                            antonyms: vec![],
                        },
                    ],
                    synonyms: vec![
                        "harmony".to_string(),
                        "peace".to_string(),
                        "sereneness".to_string(),
                        "tranquility".to_string(),
                        "tranquillity".to_string(),
                    ],
                    antonyms: vec![],
                },],
                source_urls: vec!["https://en.wiktionary.org/wiki/serenity".to_string()],
            },],)
        )
    }
}
