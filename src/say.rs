//! Command to echo given input text.

use serenity::utils::{content_safe, ContentSafeOptions};

use crate::utils::{Context, Error};

/// Says whatever is given to say.
///
/// Will remove mentions and other potential permissions issues.
#[poise::command(slash_command)]
pub(crate) async fn say(
    ctx: Context<'_>,
    #[description = "The words to say"] message: String,
) -> Result<(), Error> {
    let opts = match ctx.guild_id() {
        Some(id) => ContentSafeOptions::default().display_as_member_from(id),
        None => ContentSafeOptions::default(),
    };

    ctx.say(content_safe(ctx, message, &opts, &[])).await?;
    Ok(())
}
