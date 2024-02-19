use crate::{
    math_markup::{get_preferred_markup, set_preferred_markup, typst_base::typst_render},
    utils::{Context, Error},
};
use poise::{
    serenity_prelude::{AttachmentType, User},
    ChoiceParameter,
};
use regex::Regex;

use super::preferred_markup::MathMarkup;

/// Returns None if a message is not identifiable as Typst. If the message is
/// identifiable as Typst, then the cleaned message suitable for Typst rendering
/// is returned instead.
pub(crate) fn catch_typst_message(msg: &str) -> Option<String> {
    if msg.contains("#ce") {
        return Some(msg.to_string());
    }
    let math_check_regex = Regex::new(r"(?s).*\$.+\$.*").unwrap();
    if math_check_regex.is_match(msg) {
        Some(msg.to_string())
    } else {
        None
    }
}

fn latex2typst(msg: &str) -> String {
    "#mitext(`\n".to_string() + msg + "\n`)"
}

/// Checks if the text is latex or typst, presently just checking if there is a \ in between 2 $ signs and a non-whitespace immediately after; meh it works
fn latex_or_typst(msg: &str) -> MathMarkup {
    let latex_regex = Regex::new(r"(?s).*\$.*\\S.*\$.*").unwrap();
    if latex_regex.is_match(msg) {
        MathMarkup::Latex
    } else {
        MathMarkup::Typst
    }
}

pub(crate) async fn render_math(
    msg: &str,
    author: &User,
) -> Result<Vec<u8>, crate::math_markup::typst_base::RenderErrors> {
    let pref = get_preferred_markup(author)
        .unwrap()
        .unwrap_or_else(|| latex_or_typst(msg));
    // let pref = None;
    match pref {
        MathMarkup::Typst => typst_render(msg).await,
        MathMarkup::Latex => typst_render(latex2typst(msg).as_str()).await,
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
