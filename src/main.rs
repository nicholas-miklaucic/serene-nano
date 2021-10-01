mod acarole;
mod config;
mod poetry;
mod rep;

use regex::Regex;
use serenity::{builder::CreateMessage, Result};
use std::collections::HashSet;
use std::env;
use wikipedia;

#[macro_use]
extern crate partial_application;

use serenity::{
    async_trait,
    client::bridge::gateway::{GatewayIntents, ShardId, ShardManager},
    framework::standard::{
        buckets::{LimitedFor, RevertBucket},
        help_commands,
        macros::{check, command, group, help, hook},
        Args, CommandGroup, CommandOptions, CommandResult, DispatchError, HelpOptions, Reason,
        StandardFramework,
    },
    http::Http,
    model::{
        channel::{Channel, Message},
        gateway::Ready,
        id::UserId,
        permissions::Permissions,
    },
};
use serenity::{http, prelude::*};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    async fn message(&self, _ctx: Context, _new_message: Message) {
        // very important! this avoids infinite loops and whatnot
        if !_new_message.author.bot {
            let thank_re = Regex::new(r"(?i)thank").unwrap();
            if thank_re.is_match(&_new_message.content) && !_new_message.mentions.is_empty() {
                if let Err(err) = rep::thank(&_ctx, &_new_message).await {
                    println!("Something went wrong! {}", err);
                }
            }
        }
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
#[commands(ping)]
struct General;

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

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
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(&token)
        .event_handler(Handler)
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
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let wiki = wikipedia::Wikipedia::<wikipedia::http::default::Client>::default();
    let page = wiki.page_from_title("Riemann zeta function".to_owned());
    let wiki_text = page.get_summary().unwrap();
    // hacky way of avoiding huge newlines around math text: work on making this better in the
    // future
    let re1 = Regex::new(r"\n +").unwrap();
    let re2 = Regex::new(r"(?P<first>\w+)\{.*\}").unwrap();
    let intermediate = re1.replace_all(&wiki_text, "");
    let processed = re2.replace_all(&intermediate, "$first");
    let sentences: Vec<&str> = processed.split(". ").take(3).collect();
    msg.reply(ctx, "Gimme a sec....").await?;
    if let Err(why) = msg.reply(ctx, sentences.join(". ")).await {
        dbg!(why);
    }

    Ok(())
}

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
