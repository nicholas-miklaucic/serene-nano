//! Gives very real, totally-not-random responses to any yes-or-no question your heart desires.

use rand::Rng;

use crate::utils::{Context, Error};

/// Gives very real, totally-not-random responses to any yes-or-no question your heart desires.
#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    invoke_on_edit,
    track_deletion,
    reuse_response
)]
pub(crate) async fn ask(
    ctx: Context<'_>,
    #[description = "The question to ask"]
    #[rest]
    question: String,
) -> Result<(), Error> {
    let random_i = rand::thread_rng().gen_range(0..20);
    const CHOICES: [&str; 20] = [
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

    // in slash commands, can't see message: add message in quotes
    let response: String = match ctx.prefix() {
        // slash command
        "/" => format!("> {}\n\n{}", &question.replace('\n', "\n>"), &choice),
        _ => format!("{}", &choice),
    };

    ctx.say(response).await?;
    Ok(())
}
