mod ask;
mod config;
mod dictionary;
mod geolocation;
mod message_filter;
mod message_handler;
mod rep;
mod say;
mod trace_moe;
mod translate;
mod typst_base;
mod typst_main;
mod utils;
mod weather;
mod wiki;

use poise::serenity_prelude::GuildId;
use serenity::model::prelude::Activity;
use serenity::prelude::GatewayIntents;
use std::env;
use std::time::Duration;

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
                        message_handler::handle_message(ctx, new_message).await?;
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
