mod acarole;
mod config;
mod poetry;
mod rep;

use regex::Regex;
use serenity::{
    builder::{CreateInteractionResponse, CreateInteractionResponseData, CreateMessage},
    cache::Cache,
    model::{
        interactions::application_command::ApplicationCommandInteractionDataOption,
        prelude::Activity,
    },
    utils::{content_safe, Color, ContentSafeOptions, MessageBuilder},
    Result,
};
use std::collections::HashSet;
use std::env;
use wikipedia;

#[macro_use]
extern crate partial_application;

use serenity::{
    self, async_trait,
    client::bridge::gateway::{GatewayIntents, ShardId, ShardManager},
    framework::standard::{
        buckets::{LimitedFor, RevertBucket},
        help_commands,
        macros::{check, command, group, help, hook},
        Args, CommandGroup, CommandOptions, CommandResult, DispatchError, HelpOptions, Reason,
        StandardFramework,
    },
    http::{self, Http},
    model::{
        channel::{Channel, Message},
        gateway::Ready,
        id::{GuildId, UserId},
        interactions::{
            application_command::{
                ApplicationCommand, ApplicationCommandInteractionDataOptionValue,
                ApplicationCommandOptionType,
            },
            Interaction, InteractionResponseType,
        },
        permissions::Permissions,
    },
    prelude::*,
};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        ctx.set_activity(Activity::playing("with Sakamoto")).await;

        let _commands =
            ApplicationCommand::set_global_application_commands(&ctx.http, |commands| {
                commands
                    .create_application_command(|command| {
                        command.name("ping").description("A ping command")
                    })
                    .create_application_command(|command| {
                        command
                            .name("leaderboard")
                            .description("Get the reputation leaderboard")
                            .create_option(|option| {
                                option
                                    .name("how_many_users")
                                    .description("The number of leaders to show (default 10)")
                                    .kind(ApplicationCommandOptionType::Integer)
                                    .required(false)
                            })
                    })
                    .create_application_command(|command| {
                        command
                            .name("say")
                            .description("Have Nano say something")
                            .create_option(|option| {
                                option
                                    .name("message")
                                    .description("What to say")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(true)
                            })
                    })
            })
            .await;
    }

    async fn message(&self, _ctx: Context, _new_message: Message) {
        // very important! this avoids infinite loops and whatnot
        if !_new_message.author.bot {
            let thank_re = Regex::new(r"(?i)(than[kx])|(tysm)").unwrap();
            if thank_re.is_match(&_new_message.content) && !_new_message.mentions.is_empty() {
                if let Err(err) = rep::thank(&_ctx, &_new_message).await {
                    println!("Something went wrong! {}", err);
                }
            } else if _new_message.mentions_me(&_ctx).await.unwrap_or(false) {
                let bad_re = Regex::new(r"(?i)(bad)").unwrap();
                let good_re =
                    Regex::new(r"(?i)(good bot)|(good job)|(nice work)|(nailed it)|(nice job)")
                        .unwrap();
                if bad_re.is_match(&_new_message.content) {
                    _new_message
                        .reply(
                            &_ctx,
                            MessageBuilder::new()
                                .push("https://c.tenor.com/8QjR5hC91b0AAAAC/nichijou-nano.gif")
                                .build(),
                        )
                        .await;
                } else if good_re.is_match(&_new_message.content) {
                    _new_message
                        .reply(
                            &_ctx,
                            MessageBuilder::new()
                                .push("https://i.imgur.com/bgiANhm.gif")
                                .build(),
                        )
                        .await;
                }
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(match command.data.name.as_str() {
                            _ => InteractionResponseType::ChannelMessageWithSource,
                        })
                        .interaction_response_data(|msg| match command.data.name.as_str() {
                            "ping" => msg.content("Pong!"),
                            "leaderboard" => {
                                let n = command
                                    .data
                                    .options
                                    .get(0)
                                    .and_then(|x| x.resolved.as_ref())
                                    .unwrap_or(
                                        &ApplicationCommandInteractionDataOptionValue::Integer(10),
                                    );

                                if let &ApplicationCommandInteractionDataOptionValue::Integer(n) = n
                                {
                                    if let Ok(leaders) = rep::top_rep(n as isize) {
                                        let mut list = String::from("");
                                        for (user_name, rep) in leaders {
                                            list.push_str(&format!(
                                                "**{}** \u{2014} **{}** rep\n",
                                                user_name, rep
                                            ));
                                        }
                                        msg.create_embed(|emb| {
                                            emb.title("Reputation Leaderboard")
                                                .color(Color::PURPLE)
                                                .field("Leaders", list, false)
                                        })
                                    } else {
                                        msg.content("Uh-oh! I'm having a moment...")
                                    }
                                } else {
                                    msg.content("Couldn't get leaderboard :(")
                                }
                            }
                            "say" => {
                                let message = command
                                    .data
                                    .options
                                    .get(0)
                                    .and_then(|x| x.resolved.as_ref());

                                if let Some(ApplicationCommandInteractionDataOptionValue::String(
                                    content,
                                )) = message
                                {
                                    msg.content(content).allowed_mentions(|am| am.empty_parse())
                                } else {
                                    msg.content("Couldn't say nothin' :()")
                                }
                            }
                            _ => msg.content("Drawing a blank...".to_string()),
                        })
                })
                .await
            {
                println!("Cannot respond to slash command: {}", why);
            }
        };
    }
}

#[group]
#[commands(poetry_url)]
#[commands(poem)]
struct Poetry;

#[group]
#[commands(wiki)]
struct Wiki;

#[group]
#[commands(say)]
struct General;

#[help]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

// #[tokio::main]
// async fn main() {
//     let framework = StandardFramework::new()
//         .configure(|c| c.prefix("~"))
//         .group(&GENERAL_GROUP);

//     // Login with a bot token from the environment
//     let token = env::var("DISCORD_TOKEN").expect("token");
//     let mut client = Client::builder(token)
//         .event_handler(Handler)
//         .framework(framework)
//         .await
//         .expect("Error creating client");

//     // start listening for events by starting a single shard
//     if let Err(why) = client.start().await {
//         println!("An error occurred while running the client: {:?}", why);
//     }
// }

#[command]
async fn wiki(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let wiki = wikipedia::Wikipedia::<wikipedia::http::default::Client>::default();
    let titles = wiki.search(args.rest());
    let page = wiki.page_from_title(titles.unwrap().iter().next().unwrap().to_string());
    let wiki_text = page.get_summary().unwrap();
    // hacky way of avoiding huge newlines around math text: work on making this better in the
    // future
    let re1 = Regex::new(r"\n +").unwrap();
    let re2 = Regex::new(r"(?P<first>\w+)\{.*\}").unwrap();
    let intermediate = re1.replace_all(&wiki_text, "");
    let processed = re2.replace_all(&intermediate, "$first");
    let sentences: Vec<&str> = processed.split(". ").take(3).collect();
    if let Err(why) = msg.reply(ctx, sentences.join(". ")).await {
        dbg!(why);
    }

    Ok(())
}

fn send_poem<'a, 'b>(
    poem_opt: Option<poetry::Poem>,
    m: &'a mut CreateMessage<'b>,
) -> &'a mut CreateMessage<'b> {
    match poem_opt {
        None => m.content("Couldn't find poem!"),
        Some(poem) => m.embed(|e| {
            e.title(&poem.title);
            let mut desc = "By ".to_string();
            desc.push_str(&poem.poet);
            e.description(&desc);
            e.field("Poem", &poem.poem, false);
            e.url(&poem.url)
        }),
    };

    m
}

#[command]
async fn poem(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let query = args.rest();
    let poem_opt = poetry::search_poem(query).await;
    let res = msg
        .channel_id
        .send_message(&ctx.http, partial!(send_poem => poem_opt, _))
        .await;

    if let Err(why) = res {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}

#[command]
async fn poetry_url(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let url = args.rest();
    let poem_opt = poetry::get_poem(url).await;
    let res = msg
        .channel_id
        .send_message(&ctx.http, partial!(send_poem => poem_opt, _))
        .await;

    if let Err(why) = res {
        println!("Error sending message: {:?}", why);
    }

    Ok(())
}

#[command]
async fn say(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let message = args.rest();
    let opts = match msg.guild_id {
        Some(id) => ContentSafeOptions::default().display_as_member_from(id),
        None => ContentSafeOptions::default(),
    };
    if let Err(why) = msg
        .reply(ctx, content_safe(ctx, message, &opts).await)
        .await
    {
        println!("Error saying message: {:?}", why);
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // The Application Id is usually the Bot User Id.
    let application_id: u64 = env::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .expect("application id is not a valid id");

    let http = Http::new_with_token(&token);

    // We will fetch your bot's owners and id
    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            if let Some(team) = info.team {
                owners.insert(team.owner_user_id);
            } else {
                owners.insert(info.owner.id);
            }
            match http.get_current_user().await {
                Ok(bot_id) => (owners, bot_id.id),
                Err(why) => panic!("Could not access the bot id: {:?}", why),
            }
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let framework = StandardFramework::new()
        .configure(|c| {
            c.with_whitespace(true)
                .on_mention(Some(bot_id))
                .prefix("nano, ")
                // In this case, if "," would be first, a message would never
                // be delimited at ", ", forcing you to trim your arguments if you
                // want to avoid whitespaces at the start of each.
                .delimiters(vec![", ", ","])
                // Sets the bot's owners. These will be used for commands that
                // are owners only.
                .owners(owners)
        })
        .group(&POETRY_GROUP)
        .group(&WIKI_GROUP)
        .group(&GENERAL_GROUP)
        .help(&MY_HELP);

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .application_id(application_id)
        .framework(framework)
        .await
        .expect("Error creating client!");
    // For this example to run properly, the "Presence Intent" and "Server Members Intent"
    // options need to be enabled.
    // These are needed so the `required_permissions` macro works on the commands that need to
    // use it.
    // You will need to enable these 2 options on the bot application, and possibly wait up to 5
    // minutes.
    // .intents(GatewayIntents::all())
    // .await
    // .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
