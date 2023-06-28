mod ask;
mod command_responder;
mod config;
mod dictionary;
mod geolocation;
mod poetry;
mod rep;
mod say;
mod set;
mod trace_moe;
mod translate;
mod typst_base;
mod typst_main;
mod utils;
mod weather;
mod wiki;

use crate::dictionary::Dictionary;
use crate::utils::log_err;
use command_responder::{Command, CommandResponder, StringContent};
use geolocation::find_location;
use lingua::Language;

use once_cell::sync::Lazy;
use poise::serenity_prelude::GuildId;
use poise::EventWrapper;
use rand::Rng;
use rand::{self, prelude::IteratorRandom};
use regex::Regex;
use serenity::collector::EventCollectorBuilder;
use serenity::futures::StreamExt;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::model::channel::AttachmentType;
use serenity::model::prelude::command::{Command as SerenityCommand, CommandOptionType};
use serenity::model::prelude::interaction::application_command::{
    CommandDataOptionValue, ResolvedTarget,
};
use serenity::model::prelude::interaction::Interaction;
use serenity::model::prelude::{AttachmentId, Event, EventType};
use serenity::{
    builder::CreateMessage,
    model::prelude::Activity,
    utils::{content_safe, Color, ContentSafeOptions, MessageBuilder},
};
use serenity_additions::RegisterAdditions;
use std::collections::{HashMap, HashSet};
use std::f32::consts::E;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs::File};
use translate::detection::detect_language;
use typst_base::{CustomisePage, RenderErrors};
use typst_main::{catch_typst_message, TypstEqtn, TypstRender};
use utils::Error;
use weather::WeatherResponse;

#[macro_use]
extern crate partial_application;

use serenity::{
    self, async_trait,
    http::Http,
    model::{channel::Message, gateway::Ready, id::UserId},
    prelude::*,
};

use crate::weather::{get_weather_forecast_from_loc, weather_forecast_msg, UnitSystem};

static TYPST_BASE: Lazy<Arc<typst_base::TypstEssentials>> =
    Lazy::new(|| Arc::new(typst_base::TypstEssentials::new()));

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

    //Translating has to be the last MessageTypes arm;
    if message
        .content
        .starts_with(&env::var("PREFIX").unwrap_or_else(|_| "nano, ".to_string()))
    {
        match detect_language(&message.content) {
            Some(Language::English) | None => MessageTypes::Normal,
            Some(other) => MessageTypes::Translate(other),
        }
    } else {
        MessageTypes::Normal
    }
}

struct Handler {}

#[serenity::async_trait]
impl serenity::prelude::EventHandler for Handler {
    async fn message(&self, _ctx: serenity::prelude::Context, _new_message: Message) {
        match get_message_type(&_new_message, &_ctx).await {
            MessageTypes::Normal | MessageTypes::BotMessage => (),
            MessageTypes::Thank => {
                if let Err(err) = rep::thank(&_ctx, &_new_message).await {
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
                match translate::translate(
                    &_new_message.content,
                    Some(other_language),
                    Language::English,
                )
                .await
                {
                    Some(result) => {
                        if result == _new_message.content {
                            println!("Translation detection failed");
                            dbg!(result.clone());
                        } else if let Err(why) = _new_message.reply(&_ctx, result).await {
                            println!("Error sending message: {}", why);
                        }
                    }
                    None => println!("Error translating"),
                }
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

                        let builder = EventCollectorBuilder::new(&_ctx)
                            .add_event_type(EventType::MessageUpdate)
                            .add_message_id(&_new_message.id)
                            .timeout(Duration::from_secs(180))
                            .build();

                        match builder {
                            Ok(mut collector) => {
                                while let Some(event) = collector.next().await {
                                    match event.as_ref() {
                                        // TODO should refactor get_message to work on edited messages too
                                        Event::MessageUpdate(e) => {
                                            if let Some(new_typst_content) = catch_typst_message(
                                                e.content.clone().unwrap().as_str(),
                                            ) {
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
                                                    .await;
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
    }
}

async fn handle_message(_ctx: &Context, _new_message: &Message) -> Result<(), Error> {
    match get_message_type(&_new_message, &_ctx).await {
        MessageTypes::Normal | MessageTypes::BotMessage => (),
        MessageTypes::Thank => {
            if let Err(err) = rep::thank(&_ctx, &_new_message).await {
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
            match translate::translate(
                &_new_message.content,
                Some(other_language),
                Language::English,
            )
            .await
            {
                Some(result) => {
                    if result == _new_message.content {
                        println!("Translation detection failed");
                        dbg!(result.clone());
                    } else if let Err(why) = _new_message.reply(&_ctx, result).await {
                        println!("Error sending message: {}", why);
                    }
                }
                None => println!("Error translating"),
            }
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

                    let builder = EventCollectorBuilder::new(&_ctx)
                        .add_event_type(EventType::MessageUpdate)
                        .add_message_id(&_new_message.id)
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
                                                            m.attachment(AttachmentType::Bytes {
                                                                data: im.into(),
                                                                filename: "Rendered.png".into(),
                                                            })
                                                        }
                                                        Err(e) => m.content(format!(
                                                            "`n{}n`\n{}",
                                                            typst_src, e
                                                        )),
                                                    }
                                                })
                                                .await;
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
                weather::weather(),
                wiki::wiki(),
                say::say(),
                ask::ask(),
                trace_moe::find_anime_source(),
                rep::reputation(),
                rep::leaderboard(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: env::var("PREFIX").ok(),
                edit_tracker: Some(poise::EditTracker::for_timespan(Duration::from_secs(3600))),
                ..Default::default()
            },
            event_handler: |ctx, event, framework_ctx, user| {
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
