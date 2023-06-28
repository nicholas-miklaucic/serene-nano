//! Support for dictionary API.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serenity::{
    builder::{CreateApplicationCommandOption, CreateInteractionResponseData, CreateMessage},
    model::prelude::{
        command::CommandOptionType, interaction::application_command::CommandDataOptionValue,
    },
};
use serenity_additions::menu::{MenuBuilder, Page};

use crate::{command_responder::Command, utils::log_err};

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
            if let Some(url) = self.source_urls.get(0) {
                e = e.url(url);
            };

            e = e
                .field(
                    "Pronunciations",
                    format!(
                        "{}",
                        self.phonetics
                            .iter()
                            .map(|pro| pro.text.clone())
                            .collect::<Vec<String>>()
                            .join(", ")
                    ),
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

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) struct Dictionary {}

#[async_trait]
impl Command for Dictionary {
    fn name(&self) -> &str {
        "define"
    }

    fn description(&self) -> &str {
        "Gets the dictionary definition for a word"
    }

    async fn interaction<'b>(
        &self,
        ctx: &serenity::prelude::Context,
        command: &serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction,
    ) -> serenity::builder::CreateInteractionResponseData<'b> {
        let mess = command
            .data
            .options
            .get(0)
            .and_then(|x| x.resolved.as_ref());
        let mut msg = CreateInteractionResponseData::default();

        println!(
            "{}",
            serde_json::to_string_pretty(&command.data.options).unwrap()
        );

        msg.content("Default message");
        if let Some(CommandDataOptionValue::String(word)) = mess {
            let def_opt = get_dictionary_definition(word.as_str()).await;
            let menu = MenuBuilder::new_paginator().timeout(Duration::from_secs(120));
            match def_opt {
                Some(defs) => {
                    if defs.len() > 1 {
                        log_err(
                            menu.add_pages(defs.into_iter().map(|d| d.write_page()))
                                .show_help()
                                .build(ctx, command.channel_id)
                                .await,
                        );

                        msg.content("See above");
                    } else if defs.len() == 1 {
                        let def = &defs[0];
                        log_err(
                            command
                                .channel_id
                                .send_message(&ctx.http, |m| def.write_message(m))
                                .await,
                        );

                        msg.content("See above");
                    } else {
                        msg.content("No definitions");
                    }
                }
                None => {
                    msg.content("Could not find definition");
                }
            }
        } else {
            msg.content("AAH! Something terrible happened.");
        }

        msg
    }

    fn options(
        &self,
    ) -> Vec<
        fn(
            &mut serenity::builder::CreateApplicationCommandOption,
        ) -> &mut serenity::builder::CreateApplicationCommandOption,
    > {
        vec![|option: &mut CreateApplicationCommandOption| {
            option
                .name("word")
                .description("Base word (e.g., \"serene\" instead of \"serenely\")")
                .kind(CommandOptionType::String)
                .required(true)
        }]
    }
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
