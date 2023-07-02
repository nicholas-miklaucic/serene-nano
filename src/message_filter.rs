//! Message filtering logic.

use crate::{math_markup::catch_typst_message, translate::detection::detect_language};
use lingua::Language;
use poise::serenity_prelude::Message;
use pomsky_macro::pomsky;
use regex::{Regex, RegexBuilder};
use serenity::prelude::Context;
use std::sync::OnceLock;

const THANK_RE_PATTERN: &str = pomsky!(
(^ | %)
(| ("than" ("k"|"x"|"ks"))
 | "tysm"
 | "ty")
($ | %)
);

//             Regex::new(r"(?i)(good bot)|(good job)|(nice work)|(nailed it)|(nice job)")
const GOOD_RE_PATTERN: &str = pomsky!(
    ("good" | "nice" | "awesome") " " ("bot" | "job" | "work")
);

static THANK_RE: OnceLock<Regex> = OnceLock::new();
static GOOD_RE: OnceLock<Regex> = OnceLock::new();
static BAD_RE: OnceLock<Regex> = OnceLock::new();

#[derive(Debug, Clone)]
pub(crate) enum MessageType {
    BotMessage,
    Thank,
    GoodNano,
    BadNano,
    Typst(String),
    Translate(Language),
    Normal,
}

/// Determines if the message matches any of the categories that prompt a response.
pub(crate) async fn get_message_type(message: &Message, ctx: &Context) -> MessageType {
    if message.author.bot {
        return MessageType::BotMessage;
    }

    if !message.mentions.is_empty() {
        let thank_re = THANK_RE.get_or_init(|| {
            RegexBuilder::new(THANK_RE_PATTERN)
                .case_insensitive(true)
                .build()
                .unwrap()
        });
        if dbg!(thank_re).is_match(&message.content) {
            return MessageType::Thank;
        }

        if message.mentions_me(ctx).await.unwrap_or(false) {
            let bad_re = BAD_RE.get_or_init(|| {
                RegexBuilder::new(r"bad\b")
                    .case_insensitive(true)
                    .build()
                    .unwrap()
            });
            if bad_re.is_match(&message.content) {
                return MessageType::BadNano;
            }

            let good_re = GOOD_RE.get_or_init(|| {
                RegexBuilder::new(GOOD_RE_PATTERN)
                    .case_insensitive(true)
                    .build()
                    .unwrap()
            });
            if good_re.is_match(&message.content) {
                return MessageType::GoodNano;
            }
        }
    }

    if let Some(s) = catch_typst_message(&message.content, &message.author) {
        return MessageType::Typst(s);
    }

    match detect_language(&message.content) {
        Some(Language::English) | None => MessageType::Normal,
        Some(other) => MessageType::Translate(other),
    }
}
