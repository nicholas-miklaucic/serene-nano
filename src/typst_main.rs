use crate::command_responder::Command;
use crate::config::{TYPST_CLOSE_DELIM, TYPST_OPEN_DELIM};
use regex::{escape, Regex};
use serenity::http::multipart::Multipart;
use serenity::model::application::command::CommandOptionType;
use std::io::Cursor;
use std::sync::Arc;
use typst::geom::RgbaColor;
use typst::syntax::ErrorPos;
use typst_library::text::lorem;

use crate::typst_base::{
    determine_pixels_per_point, Preamble, RenderErrors, ToCompile, TypstEssentials,
};

use serenity::{
    builder::{CreateApplicationCommandOption, CreateInteractionResponseData},
    model::{
        application::interaction::application_command::CommandDataOptionValue,
        channel::AttachmentType,
        prelude::interaction::application_command::ApplicationCommandInteraction,
    },
    prelude::Context,
};

pub(crate) fn my_lorem(num: usize) -> String {
    //Testing if I got the typst library in properly
    lorem(num).to_string()
}

/// Returns None if a message is not identifiable as Typst. If the message is
/// identifiable as Typst, then the cleaned message suitable for Typst rendering
/// is returned instead.
pub(crate) fn catch_typst_message(msg: &str) -> Option<String> {
    let typst_re = Regex::new(
        format!(
            r"(?s).*{}.*\S+.*{}.*",
            escape(TYPST_OPEN_DELIM),
            escape(TYPST_CLOSE_DELIM)
        )
        .as_str(),
    )
    .unwrap();
    if typst_re.is_match(msg) {
        Some(
            msg.replace(TYPST_OPEN_DELIM, "$")
                .replace(TYPST_CLOSE_DELIM, "$"),
        )
    } else {
        None
    }
}

pub(crate) fn render(
    typst_base: Arc<TypstEssentials>,
    source: &str,
) -> Result<Vec<u8>, RenderErrors> {
    let mut source = source.to_owned();

    source.insert_str(0, typst_base.preamble().as_str());
    let to_compile = ToCompile::new(typst_base, source.clone());
    let document = typst::compile(&to_compile).map_err(|err| RenderErrors::SourceError(err))?;

    let frame = document.pages.get(0).ok_or(RenderErrors::NoPageError)?;

    let pixel_per_point = dbg!(determine_pixels_per_point(frame.size())?);

    let pixmap = typst::export::render(frame, pixel_per_point, RgbaColor::new(0, 0, 0, 0).into());

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

    return Ok(image);
}

impl std::fmt::Display for RenderErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderErrors::NoPageError => {
                write!(f, "No pages found...")
            }
            RenderErrors::NotSourceError => {
                write!(f, "unreachable")
            }
            RenderErrors::PageSizeTooBig => {
                write!(f, "Page too big...")
            }
            RenderErrors::SourceError(err) => {
                write!(
                    f,
                    "{}",
                    err.iter()
                        .fold(String::from("Syntax error(s):\n"), |acc, se| acc
                            + se.message.as_str()
                            + "\n")
                )
            }
        }
    }
}

pub(crate) struct TypstEqtn {
    base: Arc<TypstEssentials>,
}

impl TypstEqtn {
    pub(crate) fn new(typst_base: Arc<TypstEssentials>) -> TypstEqtn {
        TypstEqtn { base: typst_base }
    }
}

impl Command for TypstEqtn {
    fn name(&self) -> &str {
        "typst_equation"
    }
    fn description(&self) -> &str {
        "Renders equations using typst"
    }

    fn options(
        &self,
    ) -> Vec<fn(&mut CreateApplicationCommandOption) -> &mut CreateApplicationCommandOption> {
        vec![|option: &mut CreateApplicationCommandOption| {
            option
                .name("code")
                .description("Equation to render")
                .kind(CommandOptionType::String)
                .required(true)
        }]
    }

    fn interaction<'a, 'b>(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
        msg: &'a mut CreateInteractionResponseData<'b>,
    ) -> &'a mut CreateInteractionResponseData<'b> {
        // let mut msg = serenity::builder::CreateInteractionResponseData::default();
        let mess = command
            .data
            .options
            .get(0)
            .and_then(|x| x.resolved.as_ref());

        if let Some(CommandDataOptionValue::String(source)) = mess {
            let source_with_limiters = format!("$\n{}\n$", source);

            match render(self.base.clone(), source_with_limiters.as_str()) {
                Ok(im) => {
                    msg.content(format!("```\n{}\n```", source))
                        .add_file(AttachmentType::Bytes {
                            data: im.into(),
                            filename: "Rendered.png".into(),
                        })
                }
                Err(e) => msg.content(format!("```\n{}\n```\n{}", source, e)),
            }
        } else {
            msg.content("Bigger oopsie")
        }
    }
}

pub(crate) struct TypstRender {
    base: Arc<TypstEssentials>,
}

impl TypstRender {
    pub(crate) fn new(typst_base: Arc<TypstEssentials>) -> TypstRender {
        TypstRender { base: typst_base }
    }
}

impl Command for TypstRender {
    fn name(&self) -> &str {
        "typst_render"
    }
    fn description(&self) -> &str {
        "renders with typst"
    }
    fn options(
        &self,
    ) -> Vec<fn(&mut CreateApplicationCommandOption) -> &mut CreateApplicationCommandOption> {
        vec![|option: &mut CreateApplicationCommandOption| {
            option
                .name("code")
                .description("typst code to render")
                .kind(CommandOptionType::String)
                .required(true)
        }]
    }
    fn interaction<'a, 'b>(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
        msg: &'a mut CreateInteractionResponseData<'b>,
    ) -> &'a mut CreateInteractionResponseData<'b> {
        let mess = command
            .data
            .options
            .get(0)
            .and_then(|x| x.resolved.as_ref());

        if let Some(CommandDataOptionValue::String(source)) = mess {
            match render(self.base.clone(), source.as_str()) {
                Ok(im) => {
                    msg.content(format!("```\n{}\n```", source))
                        .add_file(AttachmentType::Bytes {
                            data: im.into(),
                            filename: "Rendered.png".into(),
                        })
                }
                Err(e) => msg.content(format!("```\n{}\n```\n{}", source, e)),
            }
        } else {
            msg.content("Bigger oopsie")
        }
    }
}
