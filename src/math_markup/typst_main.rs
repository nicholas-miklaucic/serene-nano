use crate::{
    config::{REDIS_URL, TYPST_CLOSE_DELIM, TYPST_OPEN_DELIM},
    utils::{Context, Error},
};
use poise::{serenity_prelude::User, ChoiceParameter};
use redis::{Commands, ErrorKind, FromRedisValue, RedisError, RedisResult, ToRedisArgs};
use regex::{escape, Regex};

use std::io::Cursor;
use std::sync::Arc;
use typst::geom::RgbaColor;

use crate::math_markup::typst_base::{
    determine_pixels_per_point, Preamble, RenderErrors, ToCompile, TypstEssentials,
};

use super::preferred_markup::MathMarkup;

/// Returns None if a message is not identifiable as Typst. If the message is
/// identifiable as Typst, then the cleaned message suitable for Typst rendering
/// is returned instead.
pub(crate) fn catch_typst_message(msg: &str, author: &User) -> Option<String> {
    let pref = crate::math_markup::get_preferred_markup(&author).unwrap_or_default();
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

pub(crate) fn render(typst_base: Arc<TypstEssentials>, source: &str) -> anyhow::Result<Vec<u8>> {
    let mut source = source.to_string();

    source.insert_str(0, typst_base.preamble().as_str());
    let to_compile = ToCompile::new(typst_base, source.clone());
    let document = typst::compile(&to_compile).map_err(|errs| RenderErrors::SourceError(*errs))?;

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

    Ok(image)
}