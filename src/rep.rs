//! Module that deals with the reputation system, processing "thanks" commands and setting reputation accordingly.

extern crate redis;
use crate::config::{REDIS_URL, THANK_COOLDOWN};
use redis::Commands;
use serenity::model::channel::Message;
use serenity::model::user::User;
use serenity::prelude::*;

/// Thanks the given user, returning the new reputation of that user. Does no checking on validity.
fn thank_user(user: &User, con: &mut redis::Connection) -> redis::RedisResult<usize> {
    con.zincr("reputation", user.id.0, 1_usize)
}

/// Checks if a given thanker-thankee relationship is allowed at this moment. The original server
/// prevented thanking bots, but I see no reason to do that. Thus, the two restrictions are that you
/// can't thank yourself and bots can't thank anyone.
fn can_thank(thanker: &User, thankee: &User) -> bool {
    !thanker.bot && thanker != thankee
}

/// Given a message, thanks all of the eligible mentions if the message author is not on cooldown,
/// starting a cooldown in the case of success. Replies to the message.
pub(crate) async fn thank(ctx: &Context, msg: &Message) -> redis::RedisResult<()> {
    let client = redis::Client::open(REDIS_URL)?;
    let mut con = client.get_connection()?;
    if con.exists(format!("on-cooldown:{}", msg.author.id.0))? {
        if let Err(e) = msg
            .reply(
                ctx,
                "You're still on cooldown. Wait 30 seconds and try again!",
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
            if can_thank(&msg.author, &user) {
                let new_rep: usize = thank_user(&user, &mut con)?;
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
