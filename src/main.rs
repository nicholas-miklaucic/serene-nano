mod ask;
mod config;
mod dictionary;
mod geolocation;
mod rep;
mod say;
mod trace_moe;
mod translate;
mod typst_base;
mod typst_main;
mod utils;
mod weather;
mod wiki;

use crate::utils::log_err;

use lingua::Language;

use once_cell::sync::Lazy;
use poise::serenity_prelude::GuildId;

use regex::Regex;
use serenity::collector::EventCollectorBuilder;
use serenity::futures::StreamExt;

use serenity::model::channel::AttachmentType;

use serenity::model::prelude::{AttachmentId, Event, EventType};
use serenity::{model::prelude::Activity, utils::MessageBuilder};

use std::env;
use std::sync::Arc;
use std::time::Duration;
use translate::detection::detect_language;

use typst_main::catch_typst_message;
use utils::Error;

use serenity::{self, model::channel::Message, prelude::*};

static TYPST_BASE: Lazy<Arc<typst_base::TypstEssentials>> =
    Lazy::new(|| Arc::new(typst_base::TypstEssentials::new()));

#[derive(Debug, Clone)]
enum MessageTypes {
    BotMessage,
    Thank,
    GoodNano,
    BadNano,
    Typst(String),
    Translate(Language),
    Normal,
}

async fn get_message_type(message: &Message, ctx: &Context) -> MessageTypes {
    if message.author.bot {
        return MessageTypes::BotMessage;
    }

    if !message.mentions.is_empty() {
        let thank_re = Regex::new(r"(?i)(than[kx])|(tysm)|(^ty)|(\s+ty\s+)").unwrap();
        if thank_re.is_match(&message.content) {
            return MessageTypes::Thank;
        }

        if message.mentions_me(ctx).await.unwrap_or(false) {
            let bad_re = Regex::new(r"(?i)(bad)").unwrap();
            if bad_re.is_match(&message.content) {
                return MessageTypes::BadNano;
            }

            let good_re =
                Regex::new(r"(?i)(good bot)|(good job)|(nice work)|(nailed it)|(nice job)")
                    .unwrap();
            if good_re.is_match(&message.content) {
                return MessageTypes::GoodNano;
            }
        }
    }

    if let Some(s) = catch_typst_message(&message.content) {
        return MessageTypes::Typst(s);
    }

    match detect_language(&message.content) {
        Some(Language::English) | None => MessageTypes::Normal,
        Some(other) => MessageTypes::Translate(other),
    }
}

async fn handle_message(_ctx: &Context, _new_message: &Message) -> Result<(), Error> {
    match get_message_type(_new_message, _ctx).await {
        MessageTypes::Normal | MessageTypes::BotMessage => (),
        MessageTypes::Thank => {
            if let Err(err) = rep::thank(_ctx, _new_message).await {
                println!("Something went wrong! {}", err);
            }
        }
        MessageTypes::GoodNano => {
            log_err(
                _new_message
                    .reply(
                        &_ctx,
                        MessageBuilder::new()
                            .push("https://i.imgur.com/bgiANhm.gif")
                            .build(),
                    )
                    .await,
            );
        }
        MessageTypes::BadNano => {
            log_err(
                _new_message
                    .reply(
                        &_ctx,
                        MessageBuilder::new()
                            .push("https://c.tenor.com/8QjR5hC91b0AAAAC/nichijou-nano.gif")
                            .build(),
                    )
                    .await,
            );
        }
        MessageTypes::Translate(other_language) => {
            let res = translate::translate(
                &_new_message.content,
                Some(other_language),
                Language::English,
            )
            .await;
            log_err(
                _new_message
                    .reply(
                        &_ctx,
                        res.unwrap_or_else(|err| dbg!(format!("Translation failed: {err}"))),
                    )
                    .await,
            );
        }
        MessageTypes::Typst(typst_src) => {
            let res = _new_message
                .channel_id
                .send_message(&_ctx.http, |m| {
                    match typst_main::render(TYPST_BASE.clone(), typst_src.as_str()) {
                        Ok(im) => m.add_file(AttachmentType::Bytes {
                            data: im.into(),
                            filename: "Rendered.png".into(),
                        }),
                        Err(e) => m.content(format!("`n{}n`\n{}", typst_src, e)),
                    }
                })
                .await;

            match res {
                Ok(mut typst_reply) => {
                    let prev_img_id = match typst_reply.attachments.get(0) {
                        Some(img) => img.id,
                        None => {
                            println!("No image!");
                            AttachmentId(0)
                        }
                    };

                    let builder = EventCollectorBuilder::new(_ctx)
                        .add_event_type(EventType::MessageUpdate)
                        .add_message_id(_new_message.id)
                        .timeout(Duration::from_secs(180))
                        .build();

                    match builder {
                        Ok(mut collector) => {
                            while let Some(event) = collector.next().await {
                                match event.as_ref() {
                                    // TODO should refactor get_message to work on edited messages too
                                    Event::MessageUpdate(e) => {
                                        if let Some(new_typst_content) =
                                            catch_typst_message(e.content.clone().unwrap().as_str())
                                        {
                                            log_err(
                                                typst_reply
                                                    .edit(&_ctx, |m| {
                                                        match typst_main::render(
                                                            TYPST_BASE.clone(),
                                                            new_typst_content.as_str(),
                                                        ) {
                                                            Ok(im) => {
                                                                m.remove_existing_attachment(
                                                                    prev_img_id,
                                                                );
                                                                m.attachment(
                                                                    AttachmentType::Bytes {
                                                                        data: im.into(),
                                                                        filename: "Rendered.png"
                                                                            .into(),
                                                                    },
                                                                )
                                                            }
                                                            Err(e) => m.content(format!(
                                                                "`n{}n`\n{}",
                                                                typst_src, e
                                                            )),
                                                        }
                                                    })
                                                    .await,
                                            );
                                        }
                                    }
                                    _ => {
                                        println!("Somehow a different event got through!");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            dbg!(e);
                            println!("An error occurred!");
                        }
                    }
                }
                Err(e) => {
                    dbg!(e);
                    println!("An error occurred!");
                }
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::DIRECT_MESSAGE_REACTIONS
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_EMOJIS_AND_STICKERS;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                ask::ask(),
                dictionary::define(),
                rep::leaderboard(),
                rep::reputation(),
                say::say(),
                trace_moe::find_anime_source(),
                weather::weather(),
                wiki::wiki(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: env::var("PREFIX").ok(),
                edit_tracker: Some(poise::EditTracker::for_timespan(Duration::from_secs(3600))),
                ..Default::default()
            },
            event_handler: |ctx, event, _framework_ctx, _user| {
                Box::pin(async move {
                    if let poise::Event::Message { new_message } = event {
                        handle_message(ctx, new_message).await?;
                    }

                    Ok(())
                })
            },
            ..Default::default()
        })
        .token(token)
        .intents(intents)
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                ctx.set_activity(Activity::playing("with Sakamoto")).await;
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    // Nano's Lab server
                    GuildId(1079226248263368814),
                )
                .await?;
                Ok(())
            })
        });

    framework.run().await.unwrap();
}
