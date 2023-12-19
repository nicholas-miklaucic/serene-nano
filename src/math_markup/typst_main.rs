use crate::{
    config::{TYPST_CLOSE_DELIM, TYPST_OPEN_DELIM},
    math_markup::typst_render,
    utils::{Context, Error},
};
use poise::{
    serenity_prelude::{AttachmentType, User},
    ChoiceParameter,
};
use regex::{escape, Regex};

use super::preferred_markup::MathMarkup;

/// Returns None if a message is not identifiable as Typst. If the message is
/// identifiable as Typst, then the cleaned message suitable for Typst rendering
/// is returned instead.
pub(crate) fn catch_typst_message(msg: &str, author: &User) -> Option<String> {
    let pref = crate::math_markup::get_preferred_markup(author).unwrap_or_default();
    let (open, close) = match pref {
        MathMarkup::Typst => ("$", "$"),
        MathMarkup::Latex => (TYPST_OPEN_DELIM, TYPST_CLOSE_DELIM),
    };
    let typst_re =
        Regex::new(format!(r"(?s).*{}.*\S+.*{}.*", escape(open), escape(close)).as_str()).unwrap();
    if typst_re.is_match(msg) {
        Some(msg.replace(open, "$").replace(close, "$"))
    } else {
        None
    }
}

/// Parent command for rendering Typst code. Does nothing on its own.
#[poise::command(prefix_command, slash_command, subcommands("render", "equation"))]
pub(crate) async fn typst(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Renders Typst markup. To include math, use `$`: padding with a space sets
/// display.
#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    invoke_on_edit,
    reuse_response,
    track_deletion
)]
pub(crate) async fn render(
    ctx: Context<'_>,
    #[description = "Code to render. $ used for math."]
    #[rest]
    code: String,
) -> Result<(), Error> {
    // todo!();

    let im = typst_render(code.as_str()).await?;

    ctx.send(|m| {
        m.content(format!("`{}`", &code))
            .attachment(AttachmentType::Bytes {
                data: im.into(),
                filename: "Rendered.png".into(),
            })
    })
    .await?;
    Ok(())
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, ChoiceParameter)]
/// The math rendering mode for Typst code.
pub(crate) enum RenderMode {
    Display,
    Inline,
}

impl Default for RenderMode {
    fn default() -> Self {
        Self::Display
    }
}

/// Renders a Typst equation: $ not needed.
#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    invoke_on_edit,
    reuse_response,
    track_deletion
)]

pub(crate) async fn equation(
    ctx: Context<'_>,
    #[description = "Whether to render display: default true."] display: Option<RenderMode>,
    #[description = "Math code to render, $ not needed."]
    #[rest]
    code: String,
) -> Result<(), Error> {
    let eqn_code = match display.unwrap_or_default() {
        RenderMode::Display => format!("$ {code} $"),
        RenderMode::Inline => format!("${code}$"),
    };
    let im = typst_render(eqn_code.as_str()).await?;
    ctx.send(|m| {
        m.content(format!("`{}`", &code))
            .attachment(AttachmentType::Bytes {
                data: im.into(),
                filename: "Rendered.png".into(),
            })
    })
    .await?;
    Ok(())
}
