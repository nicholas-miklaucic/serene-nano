//! Module that deals with the reputation system, processing "thanks" commands and setting reputation accordingly.

extern crate redis;
use crate::config::{REDIS_URL, THANK_COOLDOWN};
use crate::utils::{Context, Error};
use anyhow::anyhow;
use redis::Commands;

use serenity::model::channel::Message;
use serenity::model::user::User;

/// Thanks the given user, returning the new reputation of that user. Does no checking on validity.
fn thank_user(user: &User, con: &mut redis::Connection) -> redis::RedisResult<usize> {
    con.zincr("reputation", &user.name, 1_usize)
}

/// Checks if a given thanker-thankee relationship is allowed at this moment. The original server
/// prevented thanking bots, but I see no reason to do that. Thus, the two restrictions are that you
/// can't thank yourself and bots can't thank anyone.
fn can_thank(thanker: &User, thankee: &User) -> bool {
    !thanker.bot && thanker != thankee
}

/// Gets the reputation and rank of a user, in that order.
pub(crate) fn get_user_rep(user: &User) -> redis::RedisResult<(usize, usize)> {
    let client = redis::Client::open(REDIS_URL)?;
    let mut con = client.get_connection()?;
    let score = con.zscore("reputation", &user.name);
    let rank = con.zrevrank("reputation", &user.name);
    score.and_then(|s| rank.map(|r: usize| (s, r + 1)))
}

/// Returns a list of the top n users and their reputations.
pub(crate) fn top_rep(n: isize) -> redis::RedisResult<Vec<(String, usize)>> {
    let client = redis::Client::open(REDIS_URL)?;
    let mut con = client.get_connection()?;

    con.zrevrange_withscores("reputation", 0, n - 1)
}

/// Given two users, as might be in a slash command, returns an output message to reply with.
pub(crate) fn thank_slash(thanker: &User, thankee: &User) -> redis::RedisResult<String> {
    let client = redis::Client::open(REDIS_URL)?;
    let mut con = client.get_connection()?;

    let on_cooldown: bool = con.exists(format!("on-cooldown:{}", thanker.id.0))?;
    if can_thank(thanker, thankee) && !on_cooldown {
        con.set_ex(format!("on-cooldown:{}", thanker.id.0), "", THANK_COOLDOWN)?;
        let new_rep = thank_user(thankee, &mut con)?;

        if new_rep == 1000 {
            let fireworks_url = "https://tenor.com/view/happy-new-year2021version-gif-19777838";
            Ok(format!(
                "**{}** has helped **1000** people!!! In recognition of this achievement, {} can redeem these points for a book of your choosing: contact PollardsRho for more information. \n{}",
                thankee.name, thankee.name, fireworks_url
            ))
        } else {
            Ok(format!(
                "Thanked **{}** (new rep: **{}**)\n",
                thankee.name, new_rep
            ))
        }
    } else if on_cooldown {
        Ok(format!(
            "You're still on cooldown: wait {} seconds, please!",
            THANK_COOLDOWN
        ))
    } else {
        Ok("That's not someone you're allowed to thank <-<".to_string())
    }
}

/// Get the top users by reputation.
#[poise::command(slash_command)]
pub(crate) async fn leaderboard(
    ctx: Context<'_>,
    #[description = "Number of users to show (default 10)"]
    #[min = 0_isize]
    #[max = 50_isize]
    num_users: Option<isize>,
) -> Result<(), Error> {
    let leaders = top_rep(num_users.unwrap_or(10))
        .ok()
        .ok_or(anyhow!("Redis error: contact Pollards!"))?;

    let board: String = leaders
        .into_iter()
        .map(|(user, rep)| format!("1. **{}** — **{:>5}** points", user, rep))
        .collect::<Vec<String>>()
        .join("\n");

    ctx.say(format!("# Leaderboard\n\n{}", board)).await?;
    Ok(())
}

/// Get the reputation of a user.
#[poise::command(slash_command)]
pub(crate) async fn reputation(
    ctx: Context<'_>,
    #[description = "User to get reputation of"] user: User,
) -> Result<(), Error> {
    let (rep, rank) = get_user_rep(&user)
        .ok()
        .ok_or(anyhow!("Redis error: contact Pollards!"))?;

    ctx.say(format!(
        "User {} has **{}** points (ranked *{}*)",
        user.nick_in(ctx, ctx.guild_id().unwrap_or_default())
            .await
            .unwrap_or(user.name),
        rep,
        rank
    ))
    .await?;
    Ok(())
}

/// Given a message, thanks all of the eligible mentions if the message author is not on cooldown,
/// starting a cooldown in the case of success. Replies to the message.
pub(crate) async fn thank(
    ctx: &serenity::prelude::Context,
    msg: &Message,
) -> redis::RedisResult<()> {
    let client = redis::Client::open(REDIS_URL)?;
    let mut con = client.get_connection()?;
    if con.exists(format!("on-cooldown:{}", msg.author.id.0))? {
        if let Err(e) = msg
            .reply(
                ctx,
                format!(
                    "You're still on cooldown. Wait {} seconds and try again!",
                    THANK_COOLDOWN
                )
                .as_str(),
            )
            .await
        {
            println!("Error sending message: {}", e);
        }
    } else {
        if msg.mentions_user(&msg.author) {
            if let Err(e) = msg.reply(ctx, "You can't thank *yourself*, silly!").await {
                println!("Error sending message: {}", e);
            }
        }

        let mut reps = vec![];
        for user in &msg.mentions {
            if can_thank(&msg.author, user) {
                let new_rep: usize = thank_user(user, &mut con)?;
                reps.push((&user.name, new_rep));
            }
        }

        let mut content = String::from("");
        for (username, new_rep) in reps {
            content.push_str(&format!(
                "Thanked **{}** (new rep: **{}**)\n",
                username, new_rep
            ));
        }

        if !content.is_empty() {
            con.set_ex(
                format!("on-cooldown:{}", msg.author.id.0),
                "",
                THANK_COOLDOWN,
            )?;
            if let Err(e) = msg.reply(ctx, content).await {
                println!("Error sending message: {}", e);
            }
        }
    }

    Ok(())
}
