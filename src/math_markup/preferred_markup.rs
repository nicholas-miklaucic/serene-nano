//! Lets users configure the preferred markup.

use crate::{
    config::{REDIS_URL, TYPST_CLOSE_DELIM, TYPST_OPEN_DELIM},
    utils::{Context, Error},
};
use poise::{serenity_prelude::User, ChoiceParameter};
use redis::{Commands, ErrorKind, FromRedisValue, RedisError, RedisResult, ToRedisArgs};
use regex::{escape, Regex};

use std::io::Cursor;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, ChoiceParameter)]
/// The preferred math markup to use inside dollar signs.
pub(crate) enum MathMarkup {
    Latex,
    Typst,
}

impl Default for MathMarkup {
    fn default() -> Self {
        Self::Latex
    }
}

impl FromRedisValue for MathMarkup {
    fn from_redis_value(v: &redis::Value) -> RedisResult<Self> {
        let str_v: String = FromRedisValue::from_redis_value(v)?;
        match str_v.as_str() {
            "latex" => Ok(Self::Latex),
            "typst" => Ok(Self::Typst),
            _ => Err(RedisError::from((
                ErrorKind::TypeError,
                "Not valid math markup lang",
                str_v,
            ))),
        }
    }
}

impl ToRedisArgs for MathMarkup {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        out.write_arg(match *self {
            Self::Latex => b"latex",
            Self::Typst => b"typst",
        })
    }
}

/// The name of the math markup preference DB.
const MATH_MARKUP: &str = "math_markup";

/// Get the preferred markup language for a user.
pub(crate) fn get_preferred_markup(user: &User) -> RedisResult<MathMarkup> {
    let mut client = redis::Client::open(REDIS_URL)?;
    let pref: Option<MathMarkup> = client.hget(MATH_MARKUP, &user.name)?;
    Ok(pref.unwrap_or_default())
}

/// Set the preferred markup language for a user.
fn set_preferred_markup(user: &User, pref: &MathMarkup) -> RedisResult<()> {
    let mut client = redis::Client::open(REDIS_URL)?;
    client.hset(MATH_MARKUP, &user.name, pref)?;
    Ok(())
}

/// Set what markup language you want $$ to be interpreted as. Only applies to
/// messages you send.
///
/// The other can be accessed using `<.` and `.>`.
#[poise::command(slash_command)]
pub(crate) async fn set_default_math_markup(
    ctx: Context<'_>,
    #[description = "Math markup language to render inside $$"] preference: MathMarkup,
) -> Result<(), Error> {
    match set_preferred_markup(ctx.author(), &preference) {
        Ok(()) => {
            ctx.say(format!(
                "Success! Your preferred math markup is now {}.
If you want to disable TeXit's LaTeX rendering, do
so with `,config latex_level CODEBLOCK.`",
                preference
            ))
            .await?;
        }
        Err(e) => {
            ctx.say(format!("An error occurred! <-<\nError: {}", e))
                .await?;
        }
    };
    Ok(())
}
