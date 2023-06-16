use crate::config::{TYPST_CLOSE_DELIM, TYPST_OPEN_DELIM};
use regex::{escape, Regex};
use std::io::Cursor;
use std::sync::Arc;
use typst::geom::RgbaColor;
use typst_library::text::lorem;

use typst::syntax::ErrorPos;

use crate::typst_base::{
    determine_pixels_per_point, Preamble, RenderErrors, ToCompile, TypstEssentials,
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
