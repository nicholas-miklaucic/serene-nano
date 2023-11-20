use crate::{
    config::{TYPST_CLOSE_DELIM, TYPST_OPEN_DELIM},
    math_markup::TYPST_BASE,
    utils::{Context, Error},
};
use poise::{
    serenity_prelude::{AttachmentType, User},
    ChoiceParameter,
};
use regex::{escape, Regex};

use std::io::Cursor;
use std::sync::Arc;
use typst::{eval::Tracer, geom::Color};

use crate::math_markup::typst_base::{
    determine_pixels_per_point, format_diagnostics, Preamble, RenderErrors, ToCompile,
    TypstEssentials,
};

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

/// Renders a string. Used internally.
pub(crate) fn render_str(
    typst_base: Arc<TypstEssentials>,
    source: &str,
) -> Result<Vec<u8>, RenderErrors> {
    let mut source = source.to_string();

    source.insert_str(0, typst_base.preamble().as_str());
    let to_compile = ToCompile::new(typst_base, source.clone());
    let mut tracer = Tracer::default();
    let document = typst::compile(&to_compile, &mut tracer)
        .map_err(|errs| format_diagnostics(&to_compile, &errs))?;

    let frame = document.pages.get(0).ok_or(RenderErrors::NoPageError)?;

    let pixel_per_point = dbg!(determine_pixels_per_point(frame.size())?);

    let pixmap = typst::export::render(frame, pixel_per_point, Color::from_u8(0, 0, 0, 0));

    let mut writer = Cursor::new(Vec::new());

    image::write_buffer_with_format(
        &mut writer,
        bytemuck::cast_slice(pixmap.pixels()),
        pixmap.width(),
        pixmap.height(),
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .unwrap();
    // map_err(|_| RenderErrors::NotSourceError)?;

    let image = writer.into_inner();

    Ok(image)
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
    let im = render_str(TYPST_BASE.clone(), code.as_str())?;
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
    let im = render_str(TYPST_BASE.clone(), eqn_code.as_str())?;
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
