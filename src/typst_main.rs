use std::io::Cursor;
use std::sync::Arc;
use typst::geom::{RgbaColor};
use typst_library::text::lorem;

use crate::typst_base::{TypstEssentials, RenderErrors, ToCompile, determine_pixels_per_point, Preamble};

pub(crate) fn my_lorem(num: usize) -> String {
    //Testing if I got the typst library in properly
    lorem(num).to_string()
}


pub(crate) fn render(
    typst_base: Arc<TypstEssentials>,
    source: &str,
) -> Result<Vec<u8>, RenderErrors> {
    let mut source = source.to_owned();
    
    source.insert_str(0, typst_base.preamble().as_str());
    let to_compile = ToCompile::new(typst_base, source.clone());
    let document = typst::compile(&to_compile).map_err(|_| RenderErrors::SourceError)?;

    let frame = document.pages.get(0).ok_or(RenderErrors::NoPageError)?;

    let pixel_per_point = determine_pixels_per_point(frame.size())?;

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

