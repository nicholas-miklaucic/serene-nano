mod acarole;
mod config;
mod poetry;
mod rep;
mod set;
mod translate;
mod weather;

use lingua::{IsoCode639_1, Language};
use panmath;
use rand::Rng;
use rand::{self, prelude::IteratorRandom};
use regex::Regex;
use serenity::{
    builder::{CreateInteractionResponse, CreateInteractionResponseData, CreateMessage},
    cache::Cache,
    framework::standard::Delimiter,
    model::{
        interactions::application_command::ApplicationCommandInteractionDataOption,
        prelude::Activity,
    },
    utils::{content_safe, Color, ContentSafeOptions, MessageBuilder},
    Result,
};
use std::{
    collections::{HashMap, HashSet},
    string::ParseError,
};
use std::{env, fs::File};
use std::{
    io::{BufRead, BufReader},
    str::FromStr,
};
use translate::detection::detect_language;
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
                    .create_application_command(|command| {
                        let mut cmd = command
                            .name("add_elements")
                            .description("Add elements to a list (creating it if nonexistent)")
                            .create_option(|option| {
                                option
                                    .name("list_name")
                                    .description("The name of the list to add to")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(true)
                            });

                        for i in 1..=10 {
                            cmd = cmd.create_option(|option| {
                                option
                                    .name(format!("element_{}", i))
                                    .description("Value to add")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(false)
                            });
                        }

                        cmd
                    })
                    .create_application_command(|command| {
                        let mut cmd = command
                            .name("rem_elements")
                            .description("Remove elements from a list")
                            .create_option(|option| {
                                option
                                    .name("list_name")
                                    .description("The name of the list to add to")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(true)
                            });

                        for i in 1..=10 {
                            cmd = cmd.create_option(|option| {
                                option
                                    .name(format!("element_{}", i))
                                    .description("Value to remove")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(false)
                            });
                        }

                        cmd
                    })
                    .create_application_command(|command| {
                        command
                            .name("get_list")
                            .description("Get the elements from a user's list")
                            .create_option(|option| {
                                option
                                    .name("list_name")
                                    .description("The name of the list to add to")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(true)
                            })
                            .create_option(|option| {
                                option
                                    .name("user")
                                    .description("The user to get the list from (default: you)")
                                    .kind(ApplicationCommandOptionType::User)
                                    .required(false)
                            })
                    })
                    .create_application_command(|command| {
                        command
                            .name("reputation")
                            .description("Get the rep of a user")
                            .create_option(|option| {
                                option
                                    .name("user")
                                    .description("The user to get rep for (defaults to you)")
                                    .kind(ApplicationCommandOptionType::User)
                                    .required(false)
                            })
                    })
                    .create_application_command(|command| {
                        command
                            .name("thank")
                            .description("Thank a user")
                            .create_option(|option| {
                                option
                                    .name("user")
                                    .description("The user to thank")
                                    .kind(ApplicationCommandOptionType::User)
                                    .required(true)
                            })
                    })
                    .create_application_command(|command| {
                        command
                            .name("translate")
                            .description("Translate text")
                            .create_option(|option| {
                                option
                                    .name("text")
                                    .description("The text to translate")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(true)
                            })
                            .create_option(|option| {
                                let source = option
                                    .name("source")
                                    .description("The source language (auto-detects if not given)")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(false);
                                for lang_name in translate::available_langs::available_lang_names()
                                {
                                    source.add_string_choice(&lang_name, &lang_name);
                                }
                                source
                            })
                            .create_option(|option| {
                                let mut target = option
                                    .name("target")
                                    .description("The target language (defaults to English)")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(false);
                                for lang_name in translate::available_langs::available_lang_names()
                                {
                                    target = target.add_string_choice(&lang_name, &lang_name);
                                }
                                target
                            })
                    })
                    .create_application_command(|command| {
                        command
                            .name("texify")
                            .description("Translate to LaTeX")
                            .create_option(|option| {
                                option
                                    .name("message")
                                    .description("Input to translate")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(true)
                            })
                    })
                    .create_application_command(|command| {
                        command
                            .name("prettify")
                            .description("Translate to fancy Unicode")
                            .create_option(|option| {
                                option
                                    .name("message")
                                    .description("Input to translate")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(true)
                            })
                    })
                    .create_application_command(|command| {
                        command
                            .name("weather")
                            .description("Get the weather for a place")
                            .create_option(|opt| {
                                opt.name("location")
                                    .description("Location")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(true)
                            })
                            .create_option(|opt| {
                                opt.name("units")
                                    .description("Measurement units to use")
                                    .kind(ApplicationCommandOptionType::String)
                                    .add_string_choice("imperial", "imperial")
                                    .add_string_choice("metric", "metric")
                                    .add_string_choice("scientific", "standard")
                            })
                    })
                    .create_application_command(|command| {
                        command
                            .name("topic")
                            .description("Start a conversation with a random question")
                    })
            })
            .await;
        if let Err(e) = _commands {
            println!("Error making slash commands: {}", e);
        }
    }

    async fn message(&self, _ctx: Context, _new_message: Message) {
        // very important! this avoids infinite loops and whatnot
        if !_new_message.author.bot {
            let thank_re = Regex::new(r"(?i)(than[kx])|(tysm)|(^ty)|(\s+ty\s+)").unwrap();
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
            } else if !&_new_message.content.starts_with("nano,") {
                match detect_language(&_new_message.content) {
                    // only translate for non-English text detected with high probability
                    Some(Language::English) => (),
                    None => (),
                    Some(other) => {
                        match translate::translate(
                            &_new_message.content,
                            Some(other),
                            Language::English,
                        )
                        .await
                        {
                            Ok(result) => {
                                if result == _new_message.content {
                                    println!("Translation detection failed");
                                    dbg!(result.clone());
                                } else if let Err(why) = _new_message.reply(&_ctx, result).await {
                                    println!("Error sending message: {}", why);
                                }
                            }
                            Err(e) => println!("Error translating: {}", e),
                        }
                    }
                }
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let data = match command.data.name.as_str() {
                "translate" => {
                    let mut options = HashMap::new();
                    for opt in &command.data.options {
                        if let Some(serde_json::value::Value::String(val)) = &opt.value {
                            options.insert(opt.name.as_str(), val);
                        }
                    }
                    if !options.contains_key("text") {
                        "No text!".to_string()
                    } else {
                        match translate::translate(
                            options.get("text").unwrap_or(&&("Peligro!".to_string())),
                            options
                                .get("source")
                                .and_then(|l| translate::available_langs::get_language(l.as_str())),
                            // default target language is English
                            translate::available_langs::get_language(
                                options.get("target").unwrap_or(&&"English".to_string()),
                            )
                            .unwrap_or(Language::English),
                        )
                        .await
                        {
                            Ok(result) => {
                                format!("{}", result)
                            }
                            Err(e) => {
                                format!("Error translating: {}", e)
                            }
                        }
                    }
                }
                _ => "".to_string(),
            };
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
                            "add_elements" => {
                                let (msg, res) = set::add_elements_command(&command, msg);
                                match res {
                                    Ok(_) => msg,
                                    Err(_) => msg.content("An error occured. Pollards, why? WHYYYYY"),
                                }
                            }
                            "rem_elements" => {
                                let (msg, res) = set::rem_elements_command(&command, msg);
                                match res {
                                    Ok(_) => msg,
                                    Err(_) => msg.content("An error occured. Pollards, why? WHYYYYY"),
                                }
                            }
                            "get_list" => {
                                let (msg, res) = set::get_list_command(&command, msg);
                                match res {
                                    Ok(_) => msg,
                                    Err(_) => msg.content("An error occured. Pollards, why? WHYYYYY"),
                                }
                            }
                            "reputation" => {
                                let message = command
                                    .data
                                    .options
                                    .get(0)
                                    .and_then(|x| x.resolved.as_ref());

                                let user = match message {
                                    Some(ApplicationCommandInteractionDataOptionValue::User(
                                        usr,
                                        _,
                                    )) => usr,
                                    _ => &command.user,
                                };

                                if let Ok((rep, rank)) = rep::get_user_rep(&user) {
                                    msg.content(format!(
                                        "User **{}** has rep **{}** (rank **{}**)",
                                        user.name, rep, rank
                                    ))
                                } else {
                                    msg.content("Couldn't find user :(")
                                }
                            }
                            "thank" => {
                                let message = command
                                    .data
                                    .options
                                    .get(0)
                                    .and_then(|x| x.resolved.as_ref());

                                let user = match message {
                                    Some(ApplicationCommandInteractionDataOptionValue::User(
                                        usr,
                                        _,
                                    )) => usr,
                                    _ => &command.user,
                                };

                                if let Ok(string) = rep::thank_slash(&command.user, user) {
                                    msg.content(string)
                                } else {
                                    msg.content("The database broke! Pollards! POOOLLLLAAARRDDSSS!")
                                }
                            },
                            "weather" => {
                                // parse arguments
                                let mut units = "metric";
                                let mut location = "New York, NY";
                                for opt in &command.data.options {
                                    match &opt.value {
                                        Some(serde_json::Value::String(s)) => {
                                            if opt.name.as_str() == "units" {
                                                units = s.as_str();
                                            } else if opt.name.as_str() == "location" {
                                                location = s.as_str();
                                            }
                                        },
                                        _ => {}
                                    }
                                }
                                dbg!(location, units);
                                weather::weather_msg(location, units, msg)
                            }
                            "translate" => msg.content(data),
                            "texify" => {
                                let message = command
                                    .data
                                    .options
                                    .get(0)
                                    .and_then(|x| x.resolved.as_ref());

                                if let Some(ApplicationCommandInteractionDataOptionValue::String(
                                    content,
                                )) = message
                                {
                                    let raw_tex = panmath::texify(&content).unwrap_or("Couldn't parse <-<".to_string());
                                    let tex_url = format!(
                                        r"https://latex.codecogs.com/png.latex?\dpi{{300}}{{\color[rgb]{{0.7,0.7,0.7}}{}",
                                        raw_tex
                                    )
                                        .replace(" ", "&space;")
                                        .replace("\\", "%5C");
                                    msg.content(tex_url).allowed_mentions(|am| am.empty_parse())
                                } else {
                                    msg.content("Couldn't say nothin' :()")
                                }
                            }
                            "prettify" => {
                                let message = command
                                    .data
                                    .options
                                    .get(0)
                                    .and_then(|x| x.resolved.as_ref());

                                if let Some(ApplicationCommandInteractionDataOptionValue::String(
                                    content,
                                )) = message
                                {
                                    let texed = format!(
                                        "${}$",
                                        panmath::unicodeify(content)
                                            .unwrap_or("Couldn't parse <-<".to_string())
                                    );
                                    msg.content(texed).allowed_mentions(|am| am.empty_parse())
                                } else {
                                    msg.content("Couldn't say nothin' :()")
                                }
                            }
                            "topic" => {
                                let f_res = File::open("./topics.txt");
                                if let Ok(f) = f_res {
                                    let reader = BufReader::new(f);
                                    let mut rng = rand::thread_rng();
                                    let choice = reader.lines().choose(&mut rng);
                                    if let Some(Ok(topic)) = choice {
                                        msg.content(topic)
                                    }
                                    else {
                                        dbg!(choice);
                                        msg.content("Something went wrong...".to_string())
                                    }
                                } else {
                                    dbg!(f_res);
                                    msg.content("Something went wrong...".to_string())
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
#[commands(poetry_url, poem)]
struct Poetry;

#[group]
#[commands(wiki)]
struct Wiki;

#[group]
#[commands(texify, prettify, tex_source)]
struct Math;

#[group]
#[commands(say, tl, ask)]
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
async fn tex_source(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let message = args.rest().to_string();
    let texed = format!(
        "${}$",
        panmath::texify(&message).unwrap_or("Couldn't parse <-<".to_string())
    );
    let opts = match msg.guild_id {
        Some(id) => ContentSafeOptions::default().display_as_member_from(id),
        None => ContentSafeOptions::default(),
    };
    if let Err(why) = msg.reply(ctx, content_safe(ctx, texed, &opts).await).await {
        println!("Error saying message: {:?}", why);
    }

    Ok(())
}

#[command]
async fn texify(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let message = args.rest().to_string();
    let raw_tex = panmath::texify(&message).unwrap_or("Couldn't parse <-<".to_string());
    let tex_url = format!(
        r"https://latex.codecogs.com/png.latex?\dpi{{300}}{{\color[rgb]{{0.7,0.7,0.7}}{}",
        raw_tex
    )
    .replace(" ", "&space;")
    .replace("\\", "%5C");
    if let Err(why) = msg.reply(ctx, tex_url).await {
        println!("Error saying message: {:?}", why);
    }

    Ok(())
}

#[command]
async fn prettify(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let message = args.rest().to_string();
    let prettified = format!(
        "{}",
        panmath::unicodeify(&message).unwrap_or("Couldn't parse <-<".to_string())
    );
    let opts = match msg.guild_id {
        Some(id) => ContentSafeOptions::default().display_as_member_from(id),
        None => ContentSafeOptions::default(),
    };
    if let Err(why) = msg
        .reply(ctx, content_safe(ctx, prettified, &opts).await)
        .await
    {
        println!("Error saying message: {:?}", why);
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

#[command]
async fn tl(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut args = Args::new(args.message(), &[Delimiter::Single(' ')]);
    let source_lang;
    let target_lang;
    let text;
    if args.message().contains(">") {
        let source_str: String = args.parse().unwrap();
        let source = source_str.to_uppercase().parse();
        source_lang = source
            .and_then(|c| Ok(Language::from_iso_code_639_1(&c)))
            .ok();
        args.advance();
        args.advance();
        let target_str: String = args.parse().unwrap();
        dbg!(target_str.clone());
        let target = target_str.to_uppercase().parse();
        target_lang = target
            .and_then(|c| Ok(Language::from_iso_code_639_1(&c)))
            .unwrap_or(Language::English);

        args.advance();
        text = args.remains().unwrap_or("");
    } else {
        text = args.message();
        source_lang = None;
        target_lang = Language::English;
    }

    let reply = match translate::translate(text, source_lang.clone(), target_lang.clone()).await {
        Ok(result) => format!(
            "{:?} → {:?}\nSource: {}\nTranslation: {}",
            source_lang.unwrap_or(Language::English),
            target_lang,
            text,
            result
        ),
        Err(e) => format!("Error while translating: {:?}", e),
    };

    let opts = match msg.guild_id {
        Some(id) => ContentSafeOptions::default().display_as_member_from(id),
        None => ContentSafeOptions::default(),
    };
    if let Err(why) = msg.reply(ctx, content_safe(ctx, reply, &opts).await).await {
        println!("Error saying message: {:?}", why);
    }

    Ok(())
}

/// Gives very real, totally-not-random responses to any yes-or-no question your heart desires.
#[command]
async fn ask(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let random_i = rand::thread_rng().gen_range(0..20);
    const CHOICES: [&'static str; 20] = [
        "It is certain.",
        "It is decidedly so.",
        "Without a doubt.",
        "Yes, definitely.",
        "You may rely on it.",
        "As I see it, yes.",
        "Most likely.",
        "Outlook good.",
        "Yes.",
        "Signs point to yes.",
        "Reply hazy, try again...",
        "Ask again later...",
        "Better not tell you now!",
        "Cannot predict now...",
        "Concentrate and ask again.",
        "Don't count on it.",
        "My reply is no.",
        "My sources say no.",
        "Outlook not so good.",
        "Very doubtful.",
    ];

    let choice: String = CHOICES[random_i].to_string();

    if let Err(why) = msg.reply(ctx, choice).await {
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
        .group(&MATH_GROUP)
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
