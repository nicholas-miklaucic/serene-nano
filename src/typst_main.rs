use comemo::Prehashed;
use image;
use std::convert::TryInto;
use std::io::Cursor;
use std::sync::Arc;
use typst::eval::Library;
use typst::font::{Font, FontBook};
use typst::geom::{Color, RgbaColor, Size};
use typst::syntax::{Source, SourceId};
use typst::util::Buffer;
use typst_library::text::lorem;

pub(crate) fn my_lorem(num: usize) -> String {
    //Testing if I got the typst library in properly
    lorem(num).to_string()
}

pub(crate) struct TypstEssentials {
    library: Prehashed<Library>,
    fontbook: Prehashed<FontBook>,
    fonts: Vec<Font>,
}

fn get_fonts() -> Vec<Font> {
    std::fs::read_dir("fonts")
        .unwrap()
        .map(Result::unwrap)
        .flat_map(|entry| {
            let bytes = std::fs::read(entry.path()).unwrap();
            let buffer = Buffer::from(bytes);
            Font::iter(buffer)
        })
        .collect()
}

impl TypstEssentials {
    pub(crate) fn new() -> Self {
        let fonts = get_fonts();

        Self {
            library: Prehashed::new(typst_library::build()),
            fontbook: Prehashed::new(FontBook::from_fonts(&fonts)),
            fonts,
        }
    }
}

struct ToCompile {
    te: Arc<TypstEssentials>,
    source: Source,
    time: time::OffsetDateTime,
}

fn string2source(source: String) -> Source {
    Source::new(SourceId::from_u16(0), "not needed".as_ref(), source)
}

impl ToCompile {
    fn new(te: Arc<TypstEssentials>, source: String) -> Self {
        ToCompile {
            te,
            source: string2source(source),
            time: time::OffsetDateTime::now_utc(),
        }
    }
}

impl typst::World for ToCompile {
    fn book(&self) -> &Prehashed<FontBook> {
        &self.te.fontbook
    }

    fn file(&self, path: &std::path::Path) -> typst::diag::FileResult<Buffer> {
        Err(typst::diag::FileError::NotFound(path.into()))
    }

    fn font(&self, id: usize) -> Option<Font> {
        self.te.fonts.get(id).cloned()
    }
    fn library(&self) -> &Prehashed<Library> {
        &self.te.library
    }
    fn main(&self) -> &Source {
        &self.source
    }
    fn resolve(&self, path: &std::path::Path) -> typst::diag::FileResult<SourceId> {
        Err(typst::diag::FileError::NotFound(path.into()))
    }

    fn source(&self, id: SourceId) -> &Source {
        &self.source
    }
    fn today(&self, offset: Option<i64>) -> Option<typst::eval::Datetime> {
        let offset = offset.unwrap_or(0);
        let offset = time::UtcOffset::from_hms(offset.try_into().ok()?, 0, 0).ok()?;
        let time = self.time.checked_to_offset(offset)?;
        Some(typst::eval::Datetime::Date(time.date()))
    }
}

pub(crate) enum RenderErrors {
    SourceError,
    NoPageError,
    PageSizeTooBig,
    NotSourceError,
}

pub(crate) fn render(
    typst_base: Arc<TypstEssentials>,
    source: &String,
) -> Result<Vec<u8>, RenderErrors> {
    let mut source = source.clone();
    source.insert_str(0, "#set page(width: auto, height: auto, margin: 10pt)\n\n");
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

const DESIRED_RESOLUTION: f32 = 1000.0;
const MAX_SIZE: f32 = 10000.0;
const MAX_PIXELS_PER_POINT: f32 = 5.0;

fn determine_pixels_per_point(size: Size) -> Result<f32, RenderErrors> {
    let x = size.x.to_pt() as f32;
    let y = size.y.to_pt() as f32;

    if x > MAX_SIZE || y > MAX_SIZE {
        Err(RenderErrors::PageSizeTooBig)
    } else {
        let area = x * y;
        Ok((DESIRED_RESOLUTION / area.sqrt()).min(MAX_PIXELS_PER_POINT))
    }
}
